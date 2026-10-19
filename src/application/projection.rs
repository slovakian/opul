use super::domain::*;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RuntimeOverlay {
    pub(crate) boot: Option<BootId>,
    pub(crate) sequence: u64,
    pub(crate) telemetry: HashMap<DownloadId, TransferTelemetry>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TransferTelemetry {
    pub(crate) downloaded_bytes: u64,
    pub(crate) uploaded_bytes: u64,
    pub(crate) total_bytes: u64,
    pub(crate) download_rate: u64,
    pub(crate) upload_rate: u64,
    pub(crate) eta_seconds: Option<u64>,
    pub(crate) peers: u32,
    pub(crate) seeds: u32,
    pub(crate) progress_millionths: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppProjection {
    pub(crate) revision: u64,
    pub(crate) boot: Option<BootId>,
    pub(crate) sequence: u64,
    pub(crate) session: SessionPhase,
    pub(crate) workspaces: Vec<WorkspaceProjection>,
    pub(crate) recovery: Vec<RecoveryItem>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceProjection {
    pub(crate) id: WorkspaceId,
    pub(crate) name: String,
    pub(crate) icon: WorkspaceIcon,
    pub(crate) folders: Vec<SavedFolder>,
    pub(crate) default_folder: FolderId,
    pub(crate) default_format: MediaFormat,
    pub(crate) file_organization: FileOrganization,
    pub(crate) view: WorkspaceView,
    pub(crate) downloads: Vec<DownloadProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DownloadProjection {
    pub(crate) id: DownloadId,
    pub(crate) folder: FolderId,
    pub(crate) name: String,
    pub(crate) attempt: u32,
    pub(crate) source: SourceKind,
    pub(crate) source_data: DownloadSource,
    pub(crate) lifecycle: DownloadLifecycle,
    pub(crate) output: Option<OutputArtifact>,
    pub(crate) telemetry: Option<TransferTelemetry>,
    pub(crate) capabilities: DownloadCapabilities,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct DownloadCapabilities {
    pub(crate) can_pause: bool,
    pub(crate) can_resume: bool,
    pub(crate) can_retry: bool,
    pub(crate) can_cancel: bool,
    pub(crate) can_stop_seeding: bool,
    pub(crate) can_change_file_selection: bool,
    pub(crate) can_remove: bool,
    pub(crate) can_reveal_output: bool,
}

pub(crate) fn project(state: &AppState, overlay: &RuntimeOverlay) -> AppProjection {
    AppProjection {
        revision: state.revision,
        boot: overlay.boot,
        sequence: overlay.sequence,
        session: state.session.clone(),
        workspaces: state
            .workspaces
            .iter()
            .map(|workspace| WorkspaceProjection {
                id: workspace.id,
                name: workspace.name.clone(),
                icon: workspace.icon.clone(),
                folders: workspace.folders.clone(),
                default_folder: workspace.default_folder,
                default_format: workspace.default_format,
                file_organization: workspace.file_organization,
                view: workspace.view.clone(),
                downloads: workspace
                    .downloads
                    .iter()
                    .filter(|download| !download.lifecycle.is_retired())
                    .map(|download| DownloadProjection {
                        id: download.id,
                        folder: download.folder,
                        name: download.name.clone(),
                        attempt: download.attempt,
                        source: download.source_kind(),
                        source_data: download.source.clone(),
                        lifecycle: download.lifecycle.clone(),
                        output: download.output.clone(),
                        telemetry: overlay.telemetry.get(&download.id).cloned(),
                        capabilities: capabilities(state, download),
                    })
                    .collect(),
            })
            .collect(),
        recovery: state.recovery.clone(),
    }
}

pub(crate) fn capabilities(state: &AppState, download: &Download) -> DownloadCapabilities {
    let mutable = matches!(state.session, SessionPhase::Running);
    DownloadCapabilities {
        can_pause: mutable
            && matches!(
                download.lifecycle,
                DownloadLifecycle::Queued
                    | DownloadLifecycle::Starting
                    | DownloadLifecycle::Active(_)
            ),
        can_resume: mutable && matches!(download.lifecycle, DownloadLifecycle::Paused { .. }),
        can_retry: mutable
            && matches!(
                download.lifecycle,
                DownloadLifecycle::Failed {
                    retryable: true,
                    ..
                }
            ),
        can_cancel: mutable
            && matches!(
                download.lifecycle,
                DownloadLifecycle::Queued
                    | DownloadLifecycle::Starting
                    | DownloadLifecycle::Active(_)
                    | DownloadLifecycle::Pausing { .. }
                    | DownloadLifecycle::Paused { .. }
                    | DownloadLifecycle::Failed { .. }
            ),
        can_stop_seeding: mutable
            && matches!(download.source, DownloadSource::Torrent(_))
            && matches!(
                download.lifecycle,
                DownloadLifecycle::Active(ActivePhase::Seeding)
            ),
        can_change_file_selection: mutable
            && matches!(download.source, DownloadSource::Torrent(_))
            && !matches!(
                download.lifecycle,
                DownloadLifecycle::Finished
                    | DownloadLifecycle::Failed { .. }
                    | DownloadLifecycle::Cancelled
                    | DownloadLifecycle::Retiring { .. }
            ),
        can_remove: mutable && !download.lifecycle.is_retired(),
        can_reveal_output: download.output.is_some(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_download_can_resume_while_running() {
        let mut state = AppState::fresh("/tmp/Downloads".to_string());
        state.session = SessionPhase::Running;
        let folder = state.workspaces[0].default_folder;
        let download = Download {
            id: DownloadId::new(),
            folder,
            name: "Film".to_string(),
            attempt: 0,
            control_generation: 1,
            source: DownloadSource::Media(MediaDownload {
                origin: MediaOrigin::Direct {
                    url: "http://example.com/film.mp4".to_string(),
                    referer: None,
                },
                format: MediaFormat::Original,
                policy: MediaPolicy::default(),
                legacy_binding: None,
                thumbnail_url: None,
                extractor: None,
            }),
            lifecycle: DownloadLifecycle::Paused {
                resume: ActivePhase::Fetching,
                cause: HoldCause::Person,
            },
            output: None,
        };
        assert!(capabilities(&state, &download).can_resume);
    }
}
