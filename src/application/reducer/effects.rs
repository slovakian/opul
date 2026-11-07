use super::*;

pub(crate) fn effect_is_eligible(state: &AppState, effect: &PendingEffect) -> bool {
    if matches!(
        state.session,
        SessionPhase::Booting
            | SessionPhase::RecoveryRequired
            | SessionPhase::ReadOnlyDurabilityFault
    ) {
        return false;
    }
    match &effect.kind {
        EffectKind::StopAllTransfers => matches!(
            state.session,
            SessionPhase::Quiescing { .. } | SessionPhase::Stopped
        ),
        EffectKind::EnsureTransfer { download } => {
            let Some((_, item)) = state.locate_download(*download) else {
                return false;
            };
            matches!(state.session, SessionPhase::Running)
                && effect.attempt == Some(item.attempt)
                && effect.control_generation == Some(item.control_generation)
                && matches!(
                    item.lifecycle,
                    DownloadLifecycle::Queued | DownloadLifecycle::Starting
                )
        }
        EffectKind::ConvergeControl { download } => {
            let Some((_, item)) = state.locate_download(*download) else {
                return false;
            };
            matches!(
                state.session,
                SessionPhase::Running | SessionPhase::Quiescing { .. }
            ) && effect.attempt == Some(item.attempt)
                && effect.control_generation == Some(item.control_generation)
        }
        EffectKind::RemoveTransfer { download } => {
            state.locate_download(*download).is_some_and(|(_, item)| {
                effect.attempt == Some(item.attempt)
                    && effect.control_generation == Some(item.control_generation)
                    && item.lifecycle.is_retired()
            })
        }
        EffectKind::MoveOutputToTrash {
            download,
            artifact,
            leftovers,
        } => state.locate_download(*download).is_some_and(|(_, item)| {
            effect.control_generation == Some(item.control_generation)
                && item.verified_output() == artifact.as_ref()
                && item.uncovered_leftovers() == *leftovers
                && matches!(
                    item.lifecycle,
                    DownloadLifecycle::Retiring {
                        writer_stopped: true,
                        adapter_absent: true,
                        files_resolved: false,
                        ..
                    }
                )
        }),
        EffectKind::FinalizeRetirement { download } => {
            state.locate_download(*download).is_some_and(|(_, item)| {
                matches!(
                    item.lifecycle,
                    DownloadLifecycle::Retiring {
                        writer_stopped: true,
                        adapter_absent: true,
                        files_resolved: true,
                        ..
                    }
                )
            })
        }
    }
}

pub(crate) fn effect_is_obsolete(state: &AppState, effect: &PendingEffect) -> bool {
    match &effect.kind {
        EffectKind::StopAllTransfers => matches!(state.session, SessionPhase::Running),
        EffectKind::EnsureTransfer { download } => {
            state.locate_download(*download).is_none_or(|(_, item)| {
                effect.attempt != Some(item.attempt)
                    || effect.control_generation != Some(item.control_generation)
                    || !matches!(
                        item.lifecycle,
                        DownloadLifecycle::Queued | DownloadLifecycle::Starting
                    )
            })
        }
        EffectKind::ConvergeControl { download } | EffectKind::RemoveTransfer { download } => {
            state.locate_download(*download).is_none_or(|(_, item)| {
                effect.attempt != Some(item.attempt)
                    || effect.control_generation != Some(item.control_generation)
            })
        }
        EffectKind::MoveOutputToTrash {
            download,
            artifact,
            leftovers,
        } => state.locate_download(*download).is_none_or(|(_, item)| {
            effect.control_generation != Some(item.control_generation)
                || item.verified_output() != artifact.as_ref()
                || item.uncovered_leftovers() != *leftovers
                || !item.lifecycle.is_retired()
        }),
        EffectKind::FinalizeRetirement { download } => state
            .locate_download(*download)
            .is_none_or(|(_, item)| !item.lifecycle.is_retired()),
    }
}

pub(super) fn trash_effect(item: &Download) -> EffectKind {
    EffectKind::MoveOutputToTrash {
        download: item.id,
        artifact: item.verified_output().cloned(),
        leftovers: item.uncovered_leftovers(),
    }
}

pub(super) fn control_effect(item: &Download, generation: u64) -> EffectPlan {
    EffectPlan {
        attempt: Some(item.attempt),
        control_generation: Some(generation),
        kind: EffectKind::ConvergeControl { download: item.id },
    }
}
