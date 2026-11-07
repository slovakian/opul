use super::*;

impl OpulStateMachine {
    pub fn pause_download(
        &self,
        request: MachineDownloadCommand,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.download_command(request, |download| Command::Pause { download })
    }

    pub fn resume_download(
        &self,
        request: MachineDownloadCommand,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.download_command(request, |download| Command::Resume { download })
    }

    pub fn retry_download(
        &self,
        request: MachineDownloadCommand,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.download_command(request, |download| Command::Retry { download })
    }

    pub fn cancel_download(
        &self,
        request: MachineDownloadCommand,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.download_command(request, |download| Command::Cancel { download })
    }

    pub fn stop_seeding(
        &self,
        request: MachineDownloadCommand,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.download_command(request, |download| Command::StopSeeding { download })
    }

    pub fn remove_downloads(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        download_ids: Vec<String>,
        move_files_to_trash: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let downloads = download_ids
            .iter()
            .map(|value| download_id(value))
            .collect::<Result<Vec<_>, _>>()?;
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::Remove {
                downloads,
                move_files_to_trash,
            },
        )
    }

    pub fn set_seed_ratio(
        &self,
        request: MachineDownloadCommand,
        limit: Option<f64>,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let download = download_id(&request.download_id)?;
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::SetSeedRatio { download, limit },
        )
    }

    pub fn set_torrent_files(
        &self,
        request: MachineDownloadCommand,
        files: Vec<MachineTorrentFile>,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let download = download_id(&request.download_id)?;
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::SetTorrentFiles {
                download,
                files: files.into_iter().map(file_from_view).collect(),
            },
        )
    }

    /// Records a finished download whose files the shell already moved into `folder_id`.
    pub fn move_download(
        &self,
        request: MachineDownloadCommand,
        folder_id_value: String,
        output_path: String,
        content_folder: Option<String>,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let download = download_id(&request.download_id)?;
        // The shell has already moved the files, so measure them where they now live.
        let identity = output_artifact(output_path.clone(), content_folder.clone()).identity;
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::MoveDownload {
                download,
                folder: folder_id(&folder_id_value)?,
                output_path,
                content_folder,
                identity,
            },
        )
    }

    pub fn probe_media(&self, url: String) -> Result<MediaProbe, StateMachineError> {
        self.media.probe_url(url).map_err(engine_error)
    }

    pub fn clear_guided_login_data(&self) -> Result<(), StateMachineError> {
        self.media.clear_guided_session_data().map_err(engine_error)
    }

    pub fn video_tools(&self) -> Option<Arc<crate::video_tools::VideoTools>> {
        self.video_tools.clone()
    }

    pub fn built_in_tool_versions(&self) -> crate::media::BuiltInToolVersions {
        self.media.built_in_tool_versions()
    }

    pub fn set_speed_limits(&self, download_bps: u64, upload_bps: u64) {
        self.torrent.set_speed_limits(download_bps, upload_bps);
        self.media.set_max_download_bps(download_bps);
    }

    pub fn set_max_concurrent_media_downloads(&self, maximum: u32) {
        self.media.set_max_concurrent_downloads(maximum.max(1));
    }

    pub fn platform_effects(&self) -> Vec<MachinePlatformEffect> {
        self.platform_effects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn complete_platform_effect(
        &self,
        effect_id_value: String,
        succeeded: bool,
    ) -> Result<(), StateMachineError> {
        let effect_id = effect_id(&effect_id_value)?;
        let claimed = {
            let effects = self
                .platform_effects
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let position = effects
                .iter()
                .position(|effect| effect.effect_id == effect_id_value)
                .ok_or_else(|| StateMachineError::Rejected {
                    reason: "that platform effect is not pending".to_string(),
                })?;
            effects[position].clone()
        };
        // Either outcome resolves the retirement. On failure, the shell leaves the file untouched
        // and surfaces the reason; retaining an unexecutable effect would wedge the aggregate.
        let _file_was_moved = succeeded;
        self.actor
            .complete_effect(
                effect_id,
                vec![Fact::FilesResolved {
                    download: download_id(&claimed.download_id)?,
                    control_generation: self
                        .actor
                        .execution_context(download_id(&claimed.download_id)?)
                        .map_err(actor_error)?
                        .ok_or_else(|| StateMachineError::Rejected {
                            reason: "the retiring download is no longer present".to_string(),
                        })?
                        .download
                        .control_generation,
                }],
            )
            .map_err(actor_error)?;
        self.platform_effects
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|effect| effect.effect_id != effect_id_value);
        Ok(())
    }

    pub fn begin_shutdown(&self) -> Result<MachineCommandReceipt, StateMachineError> {
        if let Some(tools) = &self.video_tools {
            tools
                .execute(crate::video_tools::VideoToolsOperation::CancelSetup)
                .map_err(|reason| StateMachineError::Open { reason })?;
        }
        let boot = self.actor.snapshot().boot.unwrap_or_else(BootId::new);
        let receipt = self.dispatch(None, Command::BeginShutdown { boot })?;
        self.torrent.stop_all();
        self.media.stop_all();
        for _ in 0..100 {
            observe_adapters(&self.actor, &self.torrent, &self.media);
            let pausing = self
                .actor
                .all_execution_contexts()
                .map_err(actor_error)?
                .iter()
                .any(|context| {
                    matches!(
                        context.download.lifecycle,
                        DownloadLifecycle::Pausing {
                            cause: HoldCause::Shutdown,
                            ..
                        }
                    )
                });
            if !pausing {
                let _ = self.dispatch(None, Command::CompleteShutdown)?;
                break;
            }
            thread::sleep(Duration::from_millis(25));
        }
        Ok(receipt)
    }
}
