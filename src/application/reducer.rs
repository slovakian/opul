use super::domain::*;

mod commands;
mod downloads;
mod effects;
mod evolution;
mod facts;
mod session;
mod types;
mod waiting;
mod workspaces;

pub(crate) use commands::decide;

pub(crate) use facts::decide_fact;

use downloads::*;
use effects::{control_effect, trash_effect};
pub(crate) use effects::{effect_is_eligible, effect_is_obsolete};
pub(crate) use evolution::{apply, evolve};
use session::*;
pub(crate) use types::*;
use waiting::*;
use workspaces::*;

fn simple(event: Event) -> Decision {
    Decision {
        events: vec![event],
        effects: Vec::new(),
        created_downloads: Vec::new(),
    }
}

fn phase_allowed(source: SourceKind, phase: ActivePhase) -> bool {
    match source {
        SourceKind::Torrent => !matches!(phase, ActivePhase::Processing),
        SourceKind::Media => !matches!(phase, ActivePhase::Verifying | ActivePhase::Seeding),
    }
}

fn lifecycle_name(lifecycle: &DownloadLifecycle) -> &'static str {
    match lifecycle {
        DownloadLifecycle::WaitingForVideoTools => "waiting for video tools",
        DownloadLifecycle::Queued => "queued",
        DownloadLifecycle::Starting => "starting",
        DownloadLifecycle::Active(_) => "active",
        DownloadLifecycle::Pausing { .. } => "pausing",
        DownloadLifecycle::Paused { .. } => "paused",
        DownloadLifecycle::Finished => "finished",
        DownloadLifecycle::Failed { .. } => "failed",
        DownloadLifecycle::Cancelled => "cancelled",
        DownloadLifecycle::Retiring { .. } => "retiring",
    }
}

fn invalid(action: &'static str, lifecycle: &DownloadLifecycle) -> Rejection {
    Rejection::InvalidTransition {
        action,
        lifecycle: lifecycle_name(lifecycle),
    }
}

#[cfg(test)]
mod tests;
