use super::*;

pub(super) fn decide_reconciled(state: &AppState) -> Result<Decision, Rejection> {
    if !matches!(state.session, SessionPhase::Reconciling) {
        return Err(Rejection::InvalidInput(
            "only a reconciling session can enter running".to_string(),
        ));
    }
    let mut decision = Decision {
        events: vec![Event::SessionChanged(SessionPhase::Running)],
        effects: Vec::new(),
        created_downloads: Vec::new(),
    };
    for workspace in &state.workspaces {
        for item in &workspace.downloads {
            let already_planned = |kind: &EffectKind, generation: u64| {
                state.outbox.values().any(|effect| {
                    &effect.kind == kind
                        && effect.attempt == Some(item.attempt)
                        && effect.control_generation == Some(generation)
                })
            };
            match &item.lifecycle {
                DownloadLifecycle::WaitingForVideoTools
                    if state.video_tools == VideoToolsAvailability::Ready =>
                {
                    queue_waiting(item, &mut decision);
                }
                DownloadLifecycle::Queued | DownloadLifecycle::Starting => {
                    let kind = EffectKind::EnsureTransfer { download: item.id };
                    if !already_planned(&kind, item.control_generation) {
                        decision.effects.push(EffectPlan {
                            attempt: Some(item.attempt),
                            control_generation: Some(item.control_generation),
                            kind,
                        });
                    }
                }
                DownloadLifecycle::Active(_)
                | DownloadLifecycle::Pausing {
                    cause: HoldCause::Shutdown,
                    ..
                }
                | DownloadLifecycle::Paused {
                    cause: HoldCause::Shutdown,
                    ..
                } => {
                    let generation = item.control_generation + 1;
                    decision.events.push(Event::DownloadLifecycleChanged {
                        download: item.id,
                        lifecycle: DownloadLifecycle::Starting,
                        control_generation: generation,
                    });
                    decision.effects.push(EffectPlan {
                        attempt: Some(item.attempt),
                        control_generation: Some(generation),
                        kind: EffectKind::EnsureTransfer { download: item.id },
                    });
                }
                DownloadLifecycle::Pausing { .. } => {
                    let kind = EffectKind::ConvergeControl { download: item.id };
                    if !already_planned(&kind, item.control_generation) {
                        decision.effects.push(EffectPlan {
                            attempt: Some(item.attempt),
                            control_generation: Some(item.control_generation),
                            kind,
                        });
                    }
                }
                DownloadLifecycle::Retiring {
                    writer_stopped,
                    adapter_absent,
                    files_resolved,
                    move_files_to_trash,
                    ..
                } => {
                    let kind = if !writer_stopped || !adapter_absent {
                        Some(EffectKind::RemoveTransfer { download: item.id })
                    } else if !files_resolved && *move_files_to_trash && item.has_trash_targets() {
                        Some(trash_effect(item))
                    } else {
                        if !files_resolved {
                            decision.events.push(Event::RetirementProgressed {
                                download: item.id,
                                writer_stopped: None,
                                adapter_absent: None,
                                files_resolved: Some(true),
                            });
                        }
                        Some(EffectKind::FinalizeRetirement { download: item.id })
                    };
                    if let Some(kind) = kind
                        && !already_planned(&kind, item.control_generation)
                    {
                        decision.effects.push(EffectPlan {
                            attempt: Some(item.attempt),
                            control_generation: Some(item.control_generation),
                            kind,
                        });
                    }
                }
                DownloadLifecycle::Paused { .. }
                | DownloadLifecycle::WaitingForVideoTools
                | DownloadLifecycle::Finished
                | DownloadLifecycle::Failed { .. }
                | DownloadLifecycle::Cancelled => {}
            }
        }
    }
    Ok(decision)
}

pub(super) fn decide_shutdown(state: &AppState, boot: BootId) -> Result<Decision, Rejection> {
    if matches!(
        state.session,
        SessionPhase::Stopped | SessionPhase::Quiescing { .. }
    ) {
        return Ok(Decision::none());
    }
    let mut decision = Decision {
        events: vec![Event::SessionChanged(SessionPhase::Quiescing { boot })],
        effects: vec![EffectPlan {
            attempt: None,
            control_generation: None,
            kind: EffectKind::StopAllTransfers,
        }],
        created_downloads: Vec::new(),
    };
    for workspace in &state.workspaces {
        for item in &workspace.downloads {
            if let DownloadLifecycle::Active(phase) = item.lifecycle {
                let generation = item.control_generation + 1;
                decision.events.push(Event::DownloadLifecycleChanged {
                    download: item.id,
                    lifecycle: DownloadLifecycle::Pausing {
                        from: phase,
                        cause: HoldCause::Shutdown,
                    },
                    control_generation: generation,
                });
                decision.effects.push(control_effect(item, generation));
            }
        }
    }
    Ok(decision)
}

pub(super) fn retirement_progress(
    state: &AppState,
    id: DownloadId,
    attempt: u32,
    generation: Option<u64>,
    writer_stopped: Option<bool>,
    adapter_absent: Option<bool>,
    files_resolved: Option<bool>,
) -> Result<Decision, Rejection> {
    let (_, item) = match state.locate_download(id) {
        Some(found) => found,
        None => return Ok(Decision::none()),
    };
    if item.attempt != attempt || generation.is_some_and(|value| value != item.control_generation) {
        return Ok(Decision::none());
    }
    let DownloadLifecycle::Retiring {
        writer_stopped: old_stopped,
        adapter_absent: old_absent,
        files_resolved: old_resolved,
        move_files_to_trash,
        ..
    } = item.lifecycle
    else {
        return Ok(Decision::none());
    };
    let stopped = writer_stopped.unwrap_or(old_stopped);
    let absent = adapter_absent.unwrap_or(old_absent);
    let needs_trash = move_files_to_trash && item.has_trash_targets();
    // Nothing left to trash resolves the files, so a retirement can never wait on them.
    let files_resolved = files_resolved.or((!old_resolved && !needs_trash).then_some(true));
    let resolved = files_resolved.unwrap_or(old_resolved);
    let mut effects = Vec::new();
    if stopped && absent && !resolved {
        effects.push(EffectPlan {
            attempt: Some(item.attempt),
            control_generation: Some(item.control_generation),
            kind: trash_effect(item),
        });
    }
    if stopped && absent && resolved {
        effects.push(EffectPlan {
            attempt: Some(item.attempt),
            control_generation: Some(item.control_generation),
            kind: EffectKind::FinalizeRetirement { download: id },
        });
    }
    Ok(Decision {
        events: vec![Event::RetirementProgressed {
            download: id,
            writer_stopped,
            adapter_absent,
            files_resolved,
        }],
        effects,
        created_downloads: Vec::new(),
    })
}
