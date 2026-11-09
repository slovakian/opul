use super::*;

impl ActorCore {
    pub(super) fn run(mut self, receiver: mpsc::Receiver<Message>) {
        while let Ok(message) = receiver.recv() {
            match message {
                Message::ProposeWorkspaceAppearance(id, response) => {
                    let _ = response.send(self.state.propose_folder_appearance(id));
                }
                Message::Command(command, response) => {
                    let _ = response.send(self.handle_command(*command));
                }
                Message::Fact(fact, response) => {
                    let _ = response.send(self.handle_fact(fact));
                }
                Message::VideoUpdateAvailable(available) => {
                    if self.overlay.video_update_available != available {
                        self.overlay.video_update_available = available;
                        self.publish();
                    }
                }
                Message::Telemetry(batch) => {
                    let mut changed = false;
                    for (download, telemetry) in batch {
                        if self.state.locate_download(download).is_some()
                            && self.overlay.telemetry.get(&download) != Some(&telemetry)
                        {
                            self.overlay.telemetry.insert(download, telemetry);
                            changed = true;
                        }
                    }
                    if changed {
                        self.publish();
                    }
                }
                Message::ClaimEffect(response) => {
                    let _ = response.send(self.handle_claim());
                }
                Message::CompleteEffect {
                    effect,
                    facts,
                    response,
                } => {
                    let _ = response.send(self.handle_completion(effect, facts));
                }
                Message::ExecutionContext(download, response) => {
                    let context =
                        self.state
                            .locate_download(download)
                            .and_then(|(workspace_id, item)| {
                                let workspace = self.state.workspace(workspace_id)?;
                                let folder = workspace
                                    .folders
                                    .iter()
                                    .find(|folder| folder.id == item.folder)?;
                                Some(ExecutionContext {
                                    download: item.clone(),
                                    output_folder: folder.path.clone(),
                                    organization: workspace.file_organization,
                                })
                            });
                    let _ = response.send(Ok(context));
                }
                Message::AllExecutionContexts(response) => {
                    let contexts = self
                        .state
                        .workspaces
                        .iter()
                        .flat_map(|workspace| {
                            workspace.downloads.iter().filter_map(|item| {
                                let folder = workspace
                                    .folders
                                    .iter()
                                    .find(|folder| folder.id == item.folder)?;
                                Some(ExecutionContext {
                                    download: item.clone(),
                                    output_folder: folder.path.clone(),
                                    organization: workspace.file_organization,
                                })
                            })
                        })
                        .collect();
                    let _ = response.send(Ok(contexts));
                }
                Message::Stop => break,
            }
        }
    }

    fn handle_command(&mut self, envelope: CommandEnvelope) -> Result<CommandReceipt, ActorError> {
        self.require_writable()?;
        if let Some(receipt) = self.state.receipts.get(&envelope.id).cloned() {
            return if receipt.fingerprint == envelope.fingerprint {
                Ok(receipt)
            } else {
                Err(ActorError::Rejected(
                    "the command ID was reused with different input".to_string(),
                ))
            };
        }
        let decision = decide(&self.state, &envelope)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        let created = decision.created_downloads.clone();
        let mut next = self.prepare_next(&decision)?;
        let receipt = CommandReceipt {
            command_id: envelope.id,
            fingerprint: envelope.fingerprint,
            revision: next.revision,
            created_downloads: created,
        };
        next.remember_receipt(receipt.clone());
        self.commit(next)?;
        Ok(receipt)
    }

    fn handle_fact(&mut self, fact: Fact) -> Result<u64, ActorError> {
        self.require_writable()?;
        let decision = decide_fact(&self.state, &fact)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        if decision.events.is_empty() && decision.effects.is_empty() {
            return Ok(self.state.revision);
        }
        let next = self.prepare_next(&decision)?;
        let revision = next.revision;
        self.commit(next)?;
        Ok(revision)
    }

    fn handle_claim(&mut self) -> Result<Option<PendingEffect>, ActorError> {
        self.require_writable()?;
        self.discard_obsolete_pending()?;
        let candidate = self
            .state
            .outbox
            .values()
            .find(|effect| {
                effect.status == EffectStatus::Pending
                    && super::super::effect_is_eligible(&self.state, effect)
                    && !self.target_in_flight(effect)
            })
            .cloned();
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let mut next = self.state.clone();
        if let Some(effect) = next.outbox.get_mut(&candidate.id) {
            effect.status = EffectStatus::InFlight;
        }
        next.revision += 1;
        self.commit(next)?;
        Ok(Some(candidate))
    }

    fn handle_completion(
        &mut self,
        effect_id: EffectId,
        facts: Vec<Fact>,
    ) -> Result<u64, ActorError> {
        self.require_writable()?;
        let Some(completed) = self.state.outbox.get(&effect_id).cloned() else {
            return Ok(self.state.revision);
        };
        let mut next = self.state.clone();
        next.outbox.remove(&effect_id);
        for fact in facts {
            let decision = decide_fact(&next, &fact)
                .map_err(|error| ActorError::Rejected(error.to_string()))?;
            for event in &decision.events {
                super::super::apply(&mut next, event);
            }
            append_effects(&mut next, decision.effects);
        }

        // If a control operation completed after being superseded, schedule convergence to the
        // latest desired state. This prevents an old pause or start from winning externally even
        // though its result is correctly ignored by the reducer.
        if let Some(target) = completed.target()
            && let Some((_, item)) = next.locate_download(target)
            && completed.control_generation != Some(item.control_generation)
        {
            let already_planned = next.outbox.values().any(|effect| {
                effect.target() == Some(target)
                    && effect.control_generation == Some(item.control_generation)
            });
            if !already_planned {
                let effect = PendingEffect {
                    id: EffectId::new(),
                    created_revision: next.revision + 1,
                    attempt: Some(item.attempt),
                    control_generation: Some(item.control_generation),
                    status: EffectStatus::Pending,
                    kind: super::super::EffectKind::ConvergeControl { download: target },
                };
                next.outbox.insert(effect.id, effect);
            }
        }
        next.revision += 1;
        let revision = next.revision;
        self.commit(next)?;
        Ok(revision)
    }

    fn prepare_next(&self, decision: &Decision) -> Result<AppState, ActorError> {
        let mut next = evolve(&self.state, decision)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        append_effects(&mut next, decision.effects.clone());
        next.validate()
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        Ok(next)
    }

    fn commit(&mut self, next: AppState) -> Result<(), ActorError> {
        if let Err(error) = self.store.commit(&next) {
            if error.installed() {
                // The rename made `next` authoritative even though syncing the directory failed.
                // Keep memory aligned with the visible checkpoint before fencing further writes.
                self.state = next;
            }
            self.writable = false;
            self.state.session = SessionPhase::ReadOnlyDurabilityFault;
            self.state.recovery.push(RecoveryItem {
                key: "durability-fault".to_string(),
                summary: "Application state is read-only.".to_string(),
                detail: error.to_string(),
            });
            self.publish();
            return Err(ActorError::Durability(error.to_string()));
        }
        self.state = next;
        self.publish();
        Ok(())
    }

    fn publish(&mut self) {
        self.overlay.sequence = self.overlay.sequence.saturating_add(1);
        self.publisher.publish(project(&self.state, &self.overlay));
    }

    fn require_writable(&self) -> Result<(), ActorError> {
        if self.writable {
            Ok(())
        } else {
            Err(ActorError::ReadOnly(
                self.state
                    .recovery
                    .last()
                    .map(|item| item.detail.clone())
                    .unwrap_or_else(|| "recovery is required".to_string()),
            ))
        }
    }

    fn target_in_flight(&self, candidate: &PendingEffect) -> bool {
        let Some(target) = candidate.target() else {
            return false;
        };
        self.state.outbox.values().any(|effect| {
            effect.id != candidate.id
                && effect.target() == Some(target)
                && effect.status == EffectStatus::InFlight
        })
    }

    fn discard_obsolete_pending(&mut self) -> Result<(), ActorError> {
        let obsolete = self
            .state
            .outbox
            .values()
            .filter(|effect| {
                effect.status == EffectStatus::Pending
                    && super::super::effect_is_obsolete(&self.state, effect)
            })
            .map(|effect| effect.id)
            .collect::<Vec<_>>();
        if obsolete.is_empty() {
            return Ok(());
        }
        let mut next = self.state.clone();
        for id in obsolete {
            next.outbox.remove(&id);
        }
        next.revision += 1;
        self.commit(next)
    }
}
