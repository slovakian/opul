use super::*;

pub(super) fn snapshot_view(projection: AppProjection) -> MachineSnapshot {
    MachineSnapshot {
        recent_emoji: projection.recent_emoji,
        revision: projection.revision,
        boot_id: projection.boot.map(|id| id.to_string()),
        sequence: projection.sequence,
        session: match projection.session {
            SessionPhase::Booting => MachineSessionState::Booting,
            SessionPhase::Reconciling => MachineSessionState::Reconciling,
            SessionPhase::Running => MachineSessionState::Running,
            SessionPhase::Quiescing { .. } => MachineSessionState::Quiescing,
            SessionPhase::Stopped => MachineSessionState::Stopped,
            SessionPhase::RecoveryRequired => MachineSessionState::RecoveryRequired,
            SessionPhase::ReadOnlyDurabilityFault => MachineSessionState::ReadOnly,
        },
        workspaces: projection
            .workspaces
            .into_iter()
            .map(workspace_view)
            .collect(),
        recovery_notices: projection
            .recovery
            .into_iter()
            .map(|item| format!("{} {}", item.summary, item.detail))
            .collect(),
    }
}

pub(super) fn workspace_view(workspace: WorkspaceProjection) -> MachineWorkspace {
    MachineWorkspace {
        id: workspace.id.to_string(),
        name: workspace.name,
        appearance: workspace.appearance,
        icon: match workspace.icon {
            WorkspaceIcon::Symbol(value) => MachineWorkspaceIcon {
                kind: MachineWorkspaceIconKind::Symbol,
                value,
            },
            WorkspaceIcon::Emoji(value) => MachineWorkspaceIcon {
                kind: MachineWorkspaceIconKind::Emoji,
                value,
            },
        },
        folders: workspace
            .folders
            .into_iter()
            .map(|folder| MachineFolder {
                id: folder.id.to_string(),
                name: folder.name,
                path: folder.path,
                is_default: folder.id == workspace.default_folder,
            })
            .collect(),
        default_format: format_view(workspace.default_format),
        file_organization: organization_view(workspace.file_organization),
        layout: match workspace.view.layout {
            WorkspaceLayout::List => MachineLayout::List,
            WorkspaceLayout::Table => MachineLayout::Table,
        },
        sort: match workspace.view.sort {
            WorkspaceSort::Order => MachineSort::Order,
            WorkspaceSort::Name => MachineSort::Name,
            WorkspaceSort::State => MachineSort::State,
            WorkspaceSort::Source => MachineSort::Source,
        },
        grouping: match workspace.view.grouping {
            WorkspaceGrouping::None => MachineGrouping::None,
            WorkspaceGrouping::State => MachineGrouping::State,
            WorkspaceGrouping::Folder => MachineGrouping::Folder,
            WorkspaceGrouping::Kind => MachineGrouping::Kind,
            WorkspaceGrouping::Source => MachineGrouping::Source,
        },
        downloads: workspace.downloads.into_iter().map(download_view).collect(),
    }
}

pub(super) fn download_view(download: DownloadProjection) -> MachineDownload {
    let telemetry = download.telemetry.clone().unwrap_or_default();
    let (
        source,
        source_url,
        page_url,
        format,
        info_hash,
        thumbnail,
        extractor,
        playlist,
        files,
        total,
    ) = actor_source_fields(&download);
    let (state, hold_cause, error_message) = lifecycle_view(&download.lifecycle);
    let (seed_ratio_limit, piece_length, piece_count, comment, trackers) =
        match &download.source_data {
            DownloadSource::Torrent(torrent) => (
                torrent.seed_ratio_limit,
                torrent
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.piece_length)
                    .unwrap_or_default(),
                torrent
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.piece_count)
                    .unwrap_or_default(),
                torrent
                    .metadata
                    .as_ref()
                    .and_then(|metadata| metadata.comment.clone()),
                torrent
                    .metadata
                    .as_ref()
                    .map(|metadata| metadata.trackers.clone())
                    .unwrap_or_default(),
            ),
            DownloadSource::Media(_) => (None, 0, 0, None, Vec::new()),
        };
    MachineDownload {
        needs_video_tools: download.needs_video_tools,
        relies_on_video_tools: download.relies_on_video_tools,
        id: download.id.to_string(),
        folder_id: download.folder.to_string(),
        name: download.name,
        source,
        source_url,
        page_url,
        format,
        state,
        hold_cause,
        info_hash,
        total_size: telemetry.total_bytes.max(total),
        bytes_downloaded: telemetry.downloaded_bytes,
        bytes_uploaded: telemetry.uploaded_bytes,
        seed_ratio: if telemetry.downloaded_bytes == 0 {
            0.0
        } else {
            telemetry.uploaded_bytes as f64 / telemetry.downloaded_bytes as f64
        },
        seed_ratio_limit,
        download_rate: telemetry.download_rate,
        upload_rate: telemetry.upload_rate,
        eta_seconds: telemetry.eta_seconds,
        peers: telemetry.peers,
        seeds: telemetry.seeds,
        progress_millionths: telemetry.progress_millionths,
        piece_length,
        piece_count,
        comment,
        trackers,
        output_path: download.output.as_ref().map(|output| output.path.clone()),
        content_folder: download
            .output
            .as_ref()
            .and_then(|output| output.content_folder.clone()),
        thumbnail_url: thumbnail,
        extractor,
        playlist_index: playlist,
        error_message,
        waiting_label: download.waiting_label,
        files,
        capabilities: MachineCapabilities {
            can_set_up_video_tools: download.capabilities.can_set_up_video_tools,
            can_retry_video_tools: download.capabilities.can_retry_video_tools,
            can_update_video_tools: download.capabilities.can_update_video_tools,
            can_pause: download.capabilities.can_pause,
            can_resume: download.capabilities.can_resume,
            can_retry: download.capabilities.can_retry,
            can_cancel: download.capabilities.can_cancel,
            can_stop_seeding: download.capabilities.can_stop_seeding,
            can_change_file_selection: download.capabilities.can_change_file_selection,
            can_remove: download.capabilities.can_remove,
            can_reveal_output: download.capabilities.can_reveal_output,
            can_move: download.capabilities.can_move,
        },
    }
}

pub(super) type SourceFields = (
    MachineSource,
    String,
    Option<String>,
    Option<MachineFormat>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<u32>,
    Vec<MachineTorrentFile>,
    u64,
);

pub(super) fn actor_source_fields(download: &DownloadProjection) -> SourceFields {
    match &download.source_data {
        DownloadSource::Torrent(torrent) => {
            let source_url = match &torrent.origin {
                TorrentOrigin::Magnet { uri } => uri.clone(),
                TorrentOrigin::Metainfo { original_name, .. } => original_name.clone(),
                TorrentOrigin::RemoteMetainfo { url } => url.clone(),
            };
            let total = torrent
                .metadata
                .as_ref()
                .map(|metadata| metadata.total_size)
                .unwrap_or_else(|| torrent.files.iter().map(|file| file.size).sum());
            (
                MachineSource::Torrent,
                source_url,
                None,
                None,
                torrent.info_hash.clone(),
                None,
                None,
                None,
                torrent
                    .files
                    .iter()
                    .map(|file| MachineTorrentFile {
                        index: file.index,
                        path: file.path.clone(),
                        size: file.size,
                        priority: match file.priority {
                            TorrentFilePriority::Skip => "Skip",
                            TorrentFilePriority::Low => "Low",
                            TorrentFilePriority::Normal => "Normal",
                            TorrentFilePriority::High => "High",
                        }
                        .to_string(),
                    })
                    .collect(),
                total,
            )
        }
        DownloadSource::Media(media) => {
            let (source, source_url, page_url, playlist_index) = match &media.origin {
                MediaOrigin::PendingPage { url } => (
                    MachineSource::VideoPage,
                    url.clone(),
                    Some(url.clone()),
                    None,
                ),
                MediaOrigin::Page {
                    page_url,
                    item_url,
                    playlist_index,
                } => (
                    MachineSource::VideoPage,
                    item_url.clone(),
                    Some(page_url.clone()),
                    *playlist_index,
                ),
                MediaOrigin::Direct { url, referer } => {
                    (MachineSource::Direct, url.clone(), referer.clone(), None)
                }
                MediaOrigin::Guided { page_url, url, .. } => (
                    MachineSource::Guided,
                    url.clone(),
                    Some(page_url.clone()),
                    None,
                ),
            };
            (
                source,
                source_url,
                page_url,
                Some(format_view(media.format)),
                None,
                media.thumbnail_url.clone(),
                media.extractor.clone(),
                playlist_index,
                Vec::new(),
                0,
            )
        }
    }
}

pub(super) fn lifecycle_view(
    lifecycle: &DownloadLifecycle,
) -> (
    MachineDownloadState,
    Option<MachineHoldCause>,
    Option<String>,
) {
    match lifecycle {
        DownloadLifecycle::WaitingForVideoTools => {
            (MachineDownloadState::WaitingForVideoTools, None, None)
        }
        DownloadLifecycle::Queued => (MachineDownloadState::Queued, None, None),
        DownloadLifecycle::Starting => (MachineDownloadState::Starting, None, None),
        DownloadLifecycle::Active(phase) => (phase_view(*phase), None, None),
        DownloadLifecycle::Pausing { cause, .. } => {
            (MachineDownloadState::Pausing, Some(hold_view(*cause)), None)
        }
        DownloadLifecycle::Paused { cause, .. } => {
            (MachineDownloadState::Paused, Some(hold_view(*cause)), None)
        }
        DownloadLifecycle::Finished => (MachineDownloadState::Finished, None, None),
        DownloadLifecycle::Failed { message, .. } => {
            (MachineDownloadState::Failed, None, Some(message.clone()))
        }
        DownloadLifecycle::Cancelled => (MachineDownloadState::Cancelled, None, None),
        DownloadLifecycle::Retiring { .. } => (MachineDownloadState::Cancelled, None, None),
    }
}

pub(super) fn phase_view(phase: ActivePhase) -> MachineDownloadState {
    match phase {
        ActivePhase::Fetching => MachineDownloadState::Fetching,
        ActivePhase::Processing => MachineDownloadState::Processing,
        ActivePhase::Verifying => MachineDownloadState::Verifying,
        ActivePhase::Seeding => MachineDownloadState::Seeding,
    }
}

pub(super) fn hold_view(cause: HoldCause) -> MachineHoldCause {
    match cause {
        HoldCause::Person => MachineHoldCause::Person,
        HoldCause::Shutdown => MachineHoldCause::Shutdown,
        HoldCause::Recovery => MachineHoldCause::Recovery,
    }
}
