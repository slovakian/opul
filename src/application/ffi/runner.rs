use super::*;

impl OpulStateMachine {
    pub(super) fn start_runner(self: &Arc<Self>) -> Result<(), StateMachineError> {
        let actor = Arc::clone(&self.actor);
        let torrent = Arc::clone(&self.torrent);
        let media = Arc::clone(&self.media);
        let guided = Arc::clone(&self.guided);
        let platform = Arc::clone(&self.platform_effects);
        let stop = Arc::clone(&self.stop);
        let execution_gate = Arc::clone(&self.execution_gate);
        let video_tools = self.video_tools.clone();
        let (probe_sender, probe_worker) = super::page_probes::start_worker(
            Arc::clone(&actor),
            Arc::clone(&media),
            Arc::clone(&stop),
        )?;
        let runner = thread::Builder::new()
            .name("opul-effect-runner".to_string())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    let _execution = execution_gate
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    // Picks up tool changes that no command caused, such as setup finishing.
                    let _ = observe_video_tools(&actor, &video_tools);
                    match actor.claim_effect() {
                        Ok(Some(effect)) => {
                            if super::page_probes::is_page_probe(&actor, &effect) {
                                let _ = probe_sender.send(effect);
                            } else {
                                execute_effect(
                                    &actor, &torrent, &media, &guided, &platform, effect,
                                );
                            }
                        }
                        Ok(None) => {
                            observe_adapters(&actor, &torrent, &media);
                            // Keep credentials only while a Guided Download can still use them.
                            guided
                                .lock()
                                .unwrap_or_else(std::sync::PoisonError::into_inner)
                                .retain(|id, _| {
                                    actor.execution_context(*id).ok().flatten().is_some_and(
                                        |context| {
                                            !matches!(
                                                context.download.lifecycle,
                                                DownloadLifecycle::Finished
                                                    | DownloadLifecycle::Cancelled
                                                    | DownloadLifecycle::Retiring { .. }
                                            )
                                        },
                                    )
                                });
                            drop(_execution);
                            thread::sleep(Duration::from_millis(250));
                        }
                        Err(_) => {
                            drop(_execution);
                            thread::sleep(Duration::from_millis(250));
                        }
                    }
                }
                drop(probe_sender);
                let _ = probe_worker.join();
                torrent.stop_all();
                media.stop_all();
            })
            .map_err(|error| StateMachineError::Open {
                reason: format!("could not start effect runner: {error}"),
            })?;
        *self
            .runner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(runner);
        Ok(())
    }

    pub(super) fn dispatch(
        &self,
        expected_revision: Option<u64>,
        command: Command,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(Uuid::new_v4().to_string(), expected_revision, command)
    }

    pub(super) fn dispatch_request(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        command: Command,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.refresh_video_tools()?;
        let fingerprint = fingerprint(&command)?;
        let receipt = self
            .actor
            .dispatch(CommandEnvelope {
                id: command_id(&request_id)?,
                fingerprint,
                expected_revision,
                command,
            })
            .map_err(actor_error)?;
        Ok(MachineCommandReceipt {
            revision: receipt.revision,
            created_download_ids: receipt
                .created_downloads
                .into_iter()
                .map(|id| id.to_string())
                .collect(),
        })
    }

    pub(super) fn download_command(
        &self,
        request: MachineDownloadCommand,
        build: impl FnOnce(DownloadId) -> Command,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let download = download_id(&request.download_id)?;
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            build(download),
        )
    }

    pub(super) fn stage_torrent_blob(&self, bytes: &[u8]) -> Result<String, StateMachineError> {
        let digest = hex::encode(Sha1::digest(bytes));
        let directory = self.support_directory.join("torrent-blobs");
        fs::create_dir_all(&directory).map_err(|error| StateMachineError::Persistence {
            reason: format!("could not create {}: {error}", directory.display()),
        })?;
        let path = directory.join(format!("{digest}.torrent"));
        if !path.exists() {
            secure_write_new(&path, bytes)?;
        }
        Ok(path.to_string_lossy().into_owned())
    }

    pub(super) fn reconcile_bindings(&self) -> Result<(), StateMachineError> {
        let contexts = self.actor.all_execution_contexts().map_err(actor_error)?;
        for context in contexts {
            if let DownloadSource::Media(media) = &context.download.source
                && let Some(legacy) = &media.legacy_binding
                && self
                    .media
                    .transfer_id_for(&context.download.id.to_string(), context.download.attempt)
                    .is_none()
            {
                let _ = self.media.claim_legacy_transfer(
                    legacy,
                    &context.download.id.to_string(),
                    context.download.attempt,
                );
            }
        }
        Ok(())
    }
}

impl Drop for OpulStateMachine {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(runner) = self
            .runner
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
        {
            let _ = runner.join();
        }
    }
}

pub(super) fn observe_video_tools(
    actor: &ApplicationActor,
    tools: &crate::video_tools::VideoTools,
) -> Result<(), StateMachineError> {
    use crate::video_tools::VideoToolsState;
    let snapshot = tools.snapshot();
    actor
        .observe_video_update(!snapshot.keep_up_to_date && snapshot.update_available)
        .map_err(actor_error)?;
    let availability = match snapshot.state {
        VideoToolsState::NotSetUp => VideoToolsAvailability::NotSetUp,
        VideoToolsState::TurnedOff => VideoToolsAvailability::TurnedOff,
        VideoToolsState::SettingUp { .. } => VideoToolsAvailability::SettingUp,
        VideoToolsState::Ready { .. } => VideoToolsAvailability::Ready,
        VideoToolsState::NotWorking { .. } => VideoToolsAvailability::NotWorking,
    };
    actor
        .observe(Fact::VideoToolsChanged(availability))
        .map_err(actor_error)?;
    Ok(())
}

impl OpulStateMachine {
    /// Every decision that depends on the tools sees their current state.
    pub(super) fn refresh_video_tools(&self) -> Result<(), StateMachineError> {
        observe_video_tools(&self.actor, &self.video_tools)
    }
}
