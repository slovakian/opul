use super::*;
use crate::destination::DownloadContentLayout;
use crate::engine::{EngineConfig, TorrentEngine, TorrentFileInput};
use crate::media::{
    GuidedDownloadSelection, GuidedExtractionSession, MediaDownloadOptions, MediaEngine,
    MediaEngineConfig, MediaFormat as EngineMediaFormat, MediaProbe, MediaProbeItem,
    MediaTransferState,
};
use crate::meta::{TorrentState, parse_magnet_uri, parse_torrent_bytes};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, uniffi::Record)]
pub struct OpulMachineConfig {
    pub application_support_folder: String,
    pub downloads_folder: String,
    pub torrent: EngineConfig,
    pub media: MediaEngineConfig,
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum StateMachineError {
    #[error("{reason}")]
    Open { reason: String },
    #[error("{reason}")]
    Rejected { reason: String },
    #[error("{reason}")]
    Engine { reason: String },
    #[error("{reason}")]
    InvalidIdentifier { reason: String },
    #[error("{reason}")]
    Persistence { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineSessionState {
    Booting,
    Reconciling,
    Running,
    Quiescing,
    Stopped,
    RecoveryRequired,
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineWorkspaceIconKind {
    Symbol,
    Emoji,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineWorkspaceIcon {
    pub kind: MachineWorkspaceIconKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineFormat {
    Best,
    BestMp4,
    P1080,
    P720,
    Original,
    AudioMp3,
    AudioM4a,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineFileOrganization {
    Automatic,
    AlwaysCreateFolder,
    KeepInSaveLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineLayout {
    List,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineSort {
    Order,
    Name,
    State,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineGrouping {
    None,
    State,
    Folder,
    Kind,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineSource {
    Torrent,
    VideoPage,
    Direct,
    Guided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineDownloadState {
    Queued,
    Starting,
    Fetching,
    Processing,
    Verifying,
    Seeding,
    Pausing,
    Paused,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum MachineHoldCause {
    Person,
    Shutdown,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineCapabilities {
    pub can_pause: bool,
    pub can_resume: bool,
    pub can_retry: bool,
    pub can_cancel: bool,
    pub can_stop_seeding: bool,
    pub can_change_file_selection: bool,
    pub can_remove: bool,
    pub can_reveal_output: bool,
    pub can_move: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineTorrentFile {
    pub index: u32,
    pub path: String,
    pub size: u64,
    pub priority: String,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MachineDownload {
    pub id: String,
    pub folder_id: String,
    pub name: String,
    pub source: MachineSource,
    pub source_url: String,
    pub page_url: Option<String>,
    pub format: Option<MachineFormat>,
    pub state: MachineDownloadState,
    pub hold_cause: Option<MachineHoldCause>,
    pub info_hash: Option<String>,
    pub total_size: u64,
    pub bytes_downloaded: u64,
    pub bytes_uploaded: u64,
    pub seed_ratio: f64,
    pub seed_ratio_limit: Option<f64>,
    pub download_rate: u64,
    pub upload_rate: u64,
    pub eta_seconds: Option<u64>,
    pub peers: u32,
    pub seeds: u32,
    pub progress_millionths: u32,
    pub piece_length: u64,
    pub piece_count: u32,
    pub comment: Option<String>,
    pub trackers: Vec<String>,
    pub output_path: Option<String>,
    pub content_folder: Option<String>,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub playlist_index: Option<u32>,
    pub error_message: Option<String>,
    pub files: Vec<MachineTorrentFile>,
    pub capabilities: MachineCapabilities,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MachineWorkspace {
    pub id: String,
    pub name: String,
    pub icon: MachineWorkspaceIcon,
    pub folders: Vec<MachineFolder>,
    pub default_format: MachineFormat,
    pub file_organization: MachineFileOrganization,
    pub layout: MachineLayout,
    pub sort: MachineSort,
    pub grouping: MachineGrouping,
    pub downloads: Vec<MachineDownload>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MachineSnapshot {
    pub revision: u64,
    pub boot_id: Option<String>,
    pub sequence: u64,
    pub session: MachineSessionState,
    pub workspaces: Vec<MachineWorkspace>,
    pub recovery_notices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineCommandReceipt {
    pub revision: u64,
    pub created_download_ids: Vec<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineWorkspaceRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub name: String,
    pub icon: MachineWorkspaceIcon,
    pub folder_id: String,
    pub folder_name: String,
    pub folder_path: String,
    pub default_format: MachineFormat,
    pub file_organization: MachineFileOrganization,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineFolderRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub folder_id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineDownloadCommand {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub download_id: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineTorrentRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub folder_id: String,
    pub source: String,
    pub metainfo: Option<Vec<u8>>,
    pub seed_ratio_limit: Option<f64>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineMediaPolicy {
    pub download_subtitles: bool,
    pub subtitle_languages: Vec<String>,
    pub embed_subtitles: bool,
    pub write_thumbnail: bool,
    pub write_metadata: bool,
    pub filename_template: String,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineMediaItem {
    pub url: String,
    pub title: String,
    pub page_url: String,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub playlist_index: Option<u32>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MachineMediaRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub folder_id: String,
    pub format: MachineFormat,
    pub policy: MachineMediaPolicy,
    pub items: Vec<MachineMediaItem>,
}

/// A file or folder the shell may move to the Trash once it still matches the identity
/// recorded for it.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachineTrashTarget {
    pub path: String,
    pub expected_byte_length: Option<u64>,
    pub expected_modified_at_millis: Option<u64>,
    pub expected_file_identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct MachinePlatformEffect {
    pub effect_id: String,
    pub download_id: String,
    pub targets: Vec<MachineTrashTarget>,
}

struct GuidedInput {
    session: Arc<GuidedExtractionSession>,
    selection: GuidedDownloadSelection,
}

#[derive(uniffi::Object)]
pub struct OpulStateMachine {
    actor: Arc<ApplicationActor>,
    torrent: Arc<TorrentEngine>,
    media: Arc<MediaEngine>,
    guided: Arc<Mutex<HashMap<DownloadId, GuidedInput>>>,
    platform_effects: Arc<Mutex<Vec<MachinePlatformEffect>>>,
    stop: Arc<AtomicBool>,
    runner: Mutex<Option<JoinHandle<()>>>,
    support_directory: PathBuf,
}

#[uniffi::export]
impl OpulStateMachine {
    #[uniffi::constructor]
    pub fn open(config: OpulMachineConfig) -> Result<Arc<Self>, StateMachineError> {
        let support_directory = PathBuf::from(&config.application_support_folder);
        let actor = Arc::new(
            ApplicationActor::open(support_directory.clone(), config.downloads_folder)
                .map_err(open_error)?,
        );
        let torrent = TorrentEngine::new_quiescent(config.torrent).map_err(engine_error)?;
        let media = MediaEngine::new_quiescent(config.media).map_err(engine_error)?;
        let machine = Arc::new(Self {
            actor,
            torrent,
            media,
            guided: Arc::new(Mutex::new(HashMap::new())),
            platform_effects: Arc::new(Mutex::new(Vec::new())),
            stop: Arc::new(AtomicBool::new(false)),
            runner: Mutex::new(None),
            support_directory,
        });
        machine.start_runner()?;
        if matches!(machine.actor.snapshot().session, SessionPhase::Reconciling) {
            machine.reconcile_bindings()?;
            let _ = machine.dispatch(None, Command::MarkReconciled)?;
        }
        Ok(machine)
    }

    pub fn snapshot(&self) -> MachineSnapshot {
        snapshot_view(self.actor.snapshot())
    }

    pub fn wait_for_snapshot(
        &self,
        after_boot_id: Option<String>,
        after_sequence: u64,
        timeout_milliseconds: u32,
    ) -> MachineSnapshot {
        let after_boot = after_boot_id
            .and_then(|value| Uuid::parse_str(&value).ok())
            .map(BootId);
        snapshot_view(self.actor.wait_for_projection(
            after_boot,
            after_sequence,
            Duration::from_millis(u64::from(timeout_milliseconds.min(30_000))),
        ))
    }

    pub fn acknowledge_recovery(&self) -> Result<MachineCommandReceipt, StateMachineError> {
        let receipt = self.dispatch(None, Command::AcknowledgeRecovery)?;
        if matches!(self.actor.snapshot().session, SessionPhase::Reconciling) {
            self.reconcile_bindings()?;
            let _ = self.dispatch(None, Command::MarkReconciled)?;
        }
        Ok(receipt)
    }

    pub fn create_workspace(
        &self,
        request: MachineWorkspaceRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let workspace_id = workspace_id(&request.workspace_id)?;
        let folder_id = folder_id(&request.folder_id)?;
        let command = Command::CreateWorkspace {
            workspace: Workspace {
                id: workspace_id,
                name: request.name,
                icon: icon_from_view(request.icon),
                folders: vec![SavedFolder {
                    id: folder_id,
                    name: request.folder_name,
                    path: request.folder_path,
                }],
                default_folder: folder_id,
                default_format: format_from_view(request.default_format),
                file_organization: organization_from_view(request.file_organization),
                view: WorkspaceView::default(),
                downloads: Vec::new(),
            },
        };
        self.dispatch_request(request.request_id, request.expected_revision, command)
    }

    pub fn update_workspace(
        &self,
        request: MachineWorkspaceRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RenameWorkspace {
                workspace: workspace_id(&request.workspace_id)?,
                name: request.name,
                icon: icon_from_view(request.icon),
                organization: organization_from_view(request.file_organization),
            },
        )
    }

    pub fn delete_workspace(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        replacement: Option<MachineWorkspaceRequest>,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let replacement = replacement
            .map(|request| {
                let workspace_id = workspace_id(&request.workspace_id)?;
                let folder_id = folder_id(&request.folder_id)?;
                Ok(Workspace {
                    id: workspace_id,
                    name: request.name,
                    icon: icon_from_view(request.icon),
                    folders: vec![SavedFolder {
                        id: folder_id,
                        name: request.folder_name,
                        path: request.folder_path,
                    }],
                    default_folder: folder_id,
                    default_format: format_from_view(request.default_format),
                    file_organization: organization_from_view(request.file_organization),
                    view: WorkspaceView::default(),
                    downloads: Vec::new(),
                })
            })
            .transpose()?;
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::DeleteWorkspace {
                workspace: workspace_id(&workspace_id_value)?,
                replacement,
            },
        )
    }

    pub fn add_folder(
        &self,
        request: MachineFolderRequest,
        make_default: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: SavedFolder {
                    id: folder_id(&request.folder_id)?,
                    name: request.name,
                    path: request.path,
                },
                make_default,
            },
        )
    }

    pub fn set_default_folder(
        &self,
        request: MachineFolderRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::SetDefaultFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
            },
        )
    }

    pub fn relink_folder(
        &self,
        request: MachineFolderRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RelinkFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
                name: request.name,
                path: request.path,
            },
        )
    }

    pub fn remove_folder(
        &self,
        request: MachineFolderRequest,
        move_files_to_trash: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::RemoveFolder {
                workspace: workspace_id(&request.workspace_id)?,
                folder: folder_id(&request.folder_id)?,
                move_files_to_trash,
            },
        )
    }

    pub fn set_workspace_view(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        layout: MachineLayout,
        sort: MachineSort,
        grouping: MachineGrouping,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::SetWorkspaceView {
                workspace: workspace_id(&workspace_id_value)?,
                view: WorkspaceView {
                    layout: match layout {
                        MachineLayout::List => WorkspaceLayout::List,
                        MachineLayout::Table => WorkspaceLayout::Table,
                    },
                    sort: match sort {
                        MachineSort::Order => WorkspaceSort::Order,
                        MachineSort::Name => WorkspaceSort::Name,
                        MachineSort::State => WorkspaceSort::State,
                        MachineSort::Source => WorkspaceSort::Source,
                    },
                    grouping: match grouping {
                        MachineGrouping::None => WorkspaceGrouping::None,
                        MachineGrouping::State => WorkspaceGrouping::State,
                        MachineGrouping::Folder => WorkspaceGrouping::Folder,
                        MachineGrouping::Kind => WorkspaceGrouping::Kind,
                        MachineGrouping::Source => WorkspaceGrouping::Source,
                    },
                },
            },
        )
    }

    pub fn set_default_format(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        workspace_id_value: String,
        format: MachineFormat,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(
            request_id,
            expected_revision,
            Command::SetDefaultFormat {
                workspace: workspace_id(&workspace_id_value)?,
                format: format_from_view(format),
            },
        )
    }

    pub fn add_torrent(
        &self,
        request: MachineTorrentRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let id = DownloadId::new();
        let (origin, name, info_hash) = if let Some(bytes) = request.metainfo {
            let meta = parse_torrent_bytes(&bytes).map_err(engine_error)?;
            let blob = self.stage_torrent_blob(&bytes)?;
            (
                TorrentOrigin::Metainfo {
                    blob,
                    original_name: Path::new(&request.source)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("download.torrent")
                        .to_string(),
                },
                meta.name,
                Some(meta.info_hash),
            )
        } else if request.source.to_ascii_lowercase().starts_with("magnet:") {
            let meta = parse_magnet_uri(&request.source).map_err(engine_error)?;
            (
                TorrentOrigin::Magnet {
                    uri: request.source.clone(),
                },
                meta.name.unwrap_or_else(|| request.source.clone()),
                Some(meta.info_hash),
            )
        } else {
            let name = Path::new(&request.source)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&request.source)
                .to_string();
            (
                TorrentOrigin::RemoteMetainfo {
                    url: request.source.clone(),
                },
                name,
                None,
            )
        };
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddDownload {
                workspace: workspace_id(&request.workspace_id)?,
                download: Box::new(Download {
                    id,
                    folder: folder_id(&request.folder_id)?,
                    name,
                    attempt: 0,
                    control_generation: 0,
                    source: DownloadSource::Torrent(TorrentDownload {
                        origin,
                        info_hash,
                        metadata: None,
                        seed_ratio_limit: request.seed_ratio_limit,
                        files: Vec::new(),
                    }),
                    lifecycle: DownloadLifecycle::Queued,
                    output: None,
                }),
            },
        )
    }

    pub fn add_media(
        &self,
        request: MachineMediaRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let folder = folder_id(&request.folder_id)?;
        let downloads = request
            .items
            .into_iter()
            .map(|item| Download {
                id: DownloadId::new(),
                folder,
                name: if item.title.trim().is_empty() {
                    item.url.clone()
                } else {
                    item.title
                },
                attempt: 0,
                control_generation: 0,
                source: DownloadSource::Media(MediaDownload {
                    origin: MediaOrigin::Page {
                        page_url: item.page_url,
                        item_url: item.url,
                        playlist_index: item.playlist_index,
                    },
                    format: format_from_view(request.format),
                    policy: policy_from_view(&request.policy),
                    legacy_binding: None,
                    thumbnail_url: item.thumbnail_url,
                    extractor: item.extractor,
                }),
                lifecycle: DownloadLifecycle::Queued,
                output: None,
            })
            .collect();
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddMediaItems {
                workspace: workspace_id(&request.workspace_id)?,
                downloads,
            },
        )
    }

    pub fn add_guided_media(
        &self,
        request: MachineMediaRequest,
        session: Arc<GuidedExtractionSession>,
        selection: GuidedDownloadSelection,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let item = request
            .items
            .first()
            .ok_or_else(|| StateMachineError::Rejected {
                reason: "guided media requires one selected item".to_string(),
            })?;
        let id = DownloadId::new();
        self.guided
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, GuidedInput { session, selection });
        let result = self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddDownload {
                workspace: workspace_id(&request.workspace_id)?,
                download: Box::new(Download {
                    id,
                    folder: folder_id(&request.folder_id)?,
                    name: item.title.clone(),
                    attempt: 0,
                    control_generation: 0,
                    source: DownloadSource::Media(MediaDownload {
                        origin: MediaOrigin::Guided {
                            page_url: item.page_url.clone(),
                            url: item.url.clone(),
                            discovery_id: id.to_string(),
                        },
                        format: format_from_view(request.format),
                        policy: policy_from_view(&request.policy),
                        legacy_binding: None,
                        thumbnail_url: item.thumbnail_url.clone(),
                        extractor: Some("User-guided extraction".to_string()),
                    }),
                    lifecycle: DownloadLifecycle::Queued,
                    output: None,
                }),
            },
        );
        if result.is_err() {
            self.guided
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id);
        }
        result
    }

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

    pub fn media_tool_status(&self) -> crate::media::MediaToolStatus {
        self.media.tool_status()
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
        // Either outcome resolves the retirement. On failure, Swift leaves the file untouched
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

impl OpulStateMachine {
    fn start_runner(self: &Arc<Self>) -> Result<(), StateMachineError> {
        let actor = Arc::clone(&self.actor);
        let torrent = Arc::clone(&self.torrent);
        let media = Arc::clone(&self.media);
        let guided = Arc::clone(&self.guided);
        let platform = Arc::clone(&self.platform_effects);
        let stop = Arc::clone(&self.stop);
        let runner = thread::Builder::new()
            .name("opul-effect-runner".to_string())
            .spawn(move || {
                while !stop.load(Ordering::Acquire) {
                    match actor.claim_effect() {
                        Ok(Some(effect)) => {
                            execute_effect(&actor, &torrent, &media, &guided, &platform, effect)
                        }
                        Ok(None) => {
                            observe_adapters(&actor, &torrent, &media);
                            thread::sleep(Duration::from_millis(250));
                        }
                        Err(_) => thread::sleep(Duration::from_millis(250)),
                    }
                }
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

    fn dispatch(
        &self,
        expected_revision: Option<u64>,
        command: Command,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.dispatch_request(Uuid::new_v4().to_string(), expected_revision, command)
    }

    fn dispatch_request(
        &self,
        request_id: String,
        expected_revision: Option<u64>,
        command: Command,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
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

    fn download_command(
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

    fn stage_torrent_blob(&self, bytes: &[u8]) -> Result<String, StateMachineError> {
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

    fn reconcile_bindings(&self) -> Result<(), StateMachineError> {
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

fn execute_effect(
    actor: &ApplicationActor,
    torrent: &TorrentEngine,
    media: &MediaEngine,
    guided: &Mutex<HashMap<DownloadId, GuidedInput>>,
    platform: &Mutex<Vec<MachinePlatformEffect>>,
    effect: PendingEffect,
) {
    let result = match &effect.kind {
        EffectKind::EnsureTransfer { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                ensure_transfer(torrent, media, guided, &context)
                    .map_err(|error| ActorError::Rejected(error.to_string()))?;
                actor.complete_effect(
                    effect.id,
                    vec![Fact::TransferStarted {
                        download: context.download.id,
                        attempt: context.download.attempt,
                        control_generation: context.download.control_generation,
                        phase: ActivePhase::Fetching,
                    }],
                )
            }),
        EffectKind::ConvergeControl { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                converge_control(torrent, media, &context);
                actor.complete_effect(effect.id, Vec::new())
            }),
        EffectKind::RemoveTransfer { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                // Ask the adapter what it wrote before its record goes away: that is the only
                // evidence of partial files, and of output the aggregate never verified.
                let owned = if matches!(
                    context.download.lifecycle,
                    DownloadLifecycle::Retiring {
                        move_files_to_trash: true,
                        ..
                    }
                ) {
                    owned_paths(torrent, media, &context)
                } else {
                    Vec::new()
                };
                remove_transfer(torrent, media, &context);
                let mut facts = Vec::new();
                if !owned.is_empty() {
                    facts.push(Fact::LeftoversFound {
                        download: context.download.id,
                        attempt: context.download.attempt,
                        leftovers: leftover_artifacts(owned),
                    });
                }
                facts.push(Fact::WriterStopped {
                    download: context.download.id,
                    attempt: context.download.attempt,
                    control_generation: context.download.control_generation,
                });
                facts.push(Fact::AdapterRecordAbsent {
                    download: context.download.id,
                    attempt: context.download.attempt,
                });
                actor.complete_effect(effect.id, facts)
            }),
        EffectKind::MoveOutputToTrash {
            download,
            artifact,
            leftovers,
        } => {
            let platform_effect = MachinePlatformEffect {
                effect_id: effect.id.to_string(),
                download_id: download.to_string(),
                targets: artifact
                    .iter()
                    .chain(leftovers)
                    .map(|artifact| MachineTrashTarget {
                        path: artifact
                            .content_folder
                            .clone()
                            .unwrap_or_else(|| artifact.path.clone()),
                        expected_byte_length: artifact
                            .identity
                            .as_ref()
                            .map(|identity| identity.byte_length),
                        expected_modified_at_millis: artifact
                            .identity
                            .as_ref()
                            .map(|identity| identity.modified_at_millis),
                        expected_file_identifier: artifact
                            .identity
                            .as_ref()
                            .and_then(|identity| identity.file_identifier.clone()),
                    })
                    .collect(),
            };
            let mut effects = platform
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !effects
                .iter()
                .any(|pending| pending.effect_id == platform_effect.effect_id)
            {
                effects.push(platform_effect);
            }
            Ok(0)
        }
        EffectKind::FinalizeRetirement { download } => actor.complete_effect(
            effect.id,
            vec![Fact::RetirementFinalized {
                download: *download,
                control_generation: effect.control_generation.unwrap_or_default(),
            }],
        ),
        EffectKind::StopAllTransfers => {
            torrent.stop_all();
            media.stop_all();
            actor.complete_effect(effect.id, Vec::new())
        }
    };
    if let Err(error) = result {
        // Engine failure becomes a typed fact when the target still exists. The effect is
        // acknowledged so it cannot busy-loop; Retry creates a fresh incarnation.
        if let Some(download) = effect.target()
            && let Ok(Some(context)) = actor.execution_context(download)
        {
            let _ = actor.complete_effect(
                effect.id,
                vec![Fact::TransferFailed {
                    download,
                    attempt: context.download.attempt,
                    message: error.to_string(),
                    retryable: true,
                }],
            );
        }
    }
}

fn ensure_transfer(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    guided: &Mutex<HashMap<DownloadId, GuidedInput>>,
    context: &ExecutionContext,
) -> Result<(), crate::CoreError> {
    let id = context.download.id.to_string();
    let layout = content_layout(context.organization);
    match &context.download.source {
        DownloadSource::Torrent(download) => match &download.origin {
            TorrentOrigin::Magnet { uri } => {
                torrent.ensure_magnet(
                    &id,
                    context.download.attempt,
                    uri.clone(),
                    context.output_folder.clone(),
                    layout,
                    download.seed_ratio_limit,
                )?;
            }
            TorrentOrigin::Metainfo { blob, .. } => {
                let path = blob.strip_prefix("legacy-path:").unwrap_or(blob);
                let bytes = fs::read(path).map_err(|error| crate::CoreError::Engine {
                    reason: format!("could not read torrent source {path}: {error}"),
                })?;
                torrent.ensure_torrent_file(TorrentFileInput {
                    download_id: id,
                    attempt: context.download.attempt,
                    bytes,
                    source: path.to_string(),
                    save_folder: context.output_folder.clone(),
                    content_layout: layout,
                    seed_ratio_limit: download.seed_ratio_limit,
                })?;
            }
            TorrentOrigin::RemoteMetainfo { url } => torrent.ensure_remote_torrent(
                &id,
                context.download.attempt,
                url.clone(),
                context.output_folder.clone(),
                layout,
                download.seed_ratio_limit,
            )?,
        },
        DownloadSource::Media(download) => {
            let options = media_options(download, layout);
            if matches!(download.origin, MediaOrigin::Guided { .. }) {
                let input = guided
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&context.download.id)
                    .ok_or_else(|| crate::CoreError::Engine {
                        reason:
                            "guided credentials expired before the transfer started; add it again"
                                .to_string(),
                    })?;
                media.ensure_guided_download(
                    &id,
                    context.download.attempt,
                    input.session,
                    input.selection,
                    context.output_folder.clone(),
                    options,
                )?;
            } else {
                let (url, page_url, playlist_index) = match &download.origin {
                    MediaOrigin::Page {
                        page_url,
                        item_url,
                        playlist_index,
                    } => (item_url.clone(), page_url.clone(), *playlist_index),
                    MediaOrigin::Direct { url, referer } => (
                        url.clone(),
                        referer.clone().unwrap_or_else(|| url.clone()),
                        None,
                    ),
                    MediaOrigin::Guided { .. } => unreachable!(),
                };
                media.ensure_download(
                    &id,
                    context.download.attempt,
                    MediaProbeItem {
                        webpage_url: url,
                        title: context.download.name.clone(),
                        thumbnail_url: download.thumbnail_url.clone(),
                        extractor: download.extractor.clone(),
                        duration_seconds: None,
                        playlist_index,
                    },
                    context.output_folder.clone(),
                    MediaDownloadOptions {
                        referer: Some(page_url),
                        ..options
                    },
                )?;
            }
            media.resume_download(&id, context.download.attempt);
        }
    }
    Ok(())
}

fn converge_control(torrent: &TorrentEngine, media: &MediaEngine, context: &ExecutionContext) {
    let id = context.download.id.to_string();
    match &context.download.source {
        DownloadSource::Torrent(download) => {
            let binding = torrent
                .binding_for(&id, context.download.attempt)
                .or_else(|| download.info_hash.clone());
            let Some(binding) = binding else { return };
            torrent.set_seed_ratio_limit(binding.clone(), download.seed_ratio_limit);
            if !download.files.is_empty() {
                let length = download
                    .files
                    .iter()
                    .map(|file| file.index as usize)
                    .max()
                    .unwrap_or_default()
                    + 1;
                let mut priorities = vec![crate::meta::FilePriority::Normal; length];
                for file in &download.files {
                    priorities[file.index as usize] = match file.priority {
                        TorrentFilePriority::Skip => crate::meta::FilePriority::Skip,
                        TorrentFilePriority::Low => crate::meta::FilePriority::Low,
                        TorrentFilePriority::Normal => crate::meta::FilePriority::Normal,
                        TorrentFilePriority::High => crate::meta::FilePriority::High,
                    };
                }
                torrent.set_file_priorities(binding.clone(), priorities);
            }
            match context.download.lifecycle {
                DownloadLifecycle::Starting | DownloadLifecycle::Active(_) => {
                    torrent.resume(binding)
                }
                DownloadLifecycle::Finished => torrent.stop_seeding(binding),
                _ => torrent.pause(binding),
            }
        }
        DownloadSource::Media(_) => match context.download.lifecycle {
            DownloadLifecycle::Starting | DownloadLifecycle::Active(_) => {
                media.resume_download(&id, context.download.attempt)
            }
            DownloadLifecycle::Cancelled => media.cancel_download(&id, context.download.attempt),
            _ => media.pause_download(&id, context.download.attempt),
        },
    }
}

fn remove_transfer(torrent: &TorrentEngine, media: &MediaEngine, context: &ExecutionContext) {
    let id = context.download.id.to_string();
    match context.download.source {
        DownloadSource::Torrent(_) => torrent.remove_download(&id, context.download.attempt),
        DownloadSource::Media(_) => media.remove_download(&id, context.download.attempt),
    }
}

/// Every path the adapters wrote for this Download, across all of its attempts, kept only
/// when it lies strictly inside the Download's saved folder.
fn owned_paths(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    context: &ExecutionContext,
) -> Vec<PathBuf> {
    let id = context.download.id.to_string();
    let paths = match context.download.source {
        DownloadSource::Torrent(_) => torrent.owned_paths(&id),
        DownloadSource::Media(_) => media.owned_paths(&id),
    };
    let folder = Path::new(&context.output_folder);
    let mut owned = Vec::<PathBuf>::new();
    for path in paths {
        let inside = path != folder
            && path.starts_with(folder)
            && !path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir));
        // A folder already listed covers everything in it.
        if inside && !owned.iter().any(|known| path.starts_with(known)) {
            owned.retain(|known| !known.starts_with(&path));
            owned.push(path);
        }
    }
    owned
}

/// Leftovers that exist once the writer has stopped, with the identity they have now.
fn leftover_artifacts(paths: Vec<PathBuf>) -> Vec<OutputArtifact> {
    paths
        .into_iter()
        .filter_map(|path| {
            let metadata = fs::symlink_metadata(&path).ok()?;
            let path = path.to_string_lossy().into_owned();
            let content_folder = metadata.is_dir().then(|| path.clone());
            Some(output_artifact(path, content_folder))
        })
        .collect()
}

fn observe_adapters(actor: &ApplicationActor, torrent: &TorrentEngine, media: &MediaEngine) {
    for snapshot in torrent.snapshots() {
        let Some(download_id_value) = snapshot.download_id.as_deref() else {
            continue;
        };
        let Ok(download) = download_id(download_id_value) else {
            continue;
        };
        let phase = match snapshot.state {
            TorrentState::Queued | TorrentState::Fetching => ActivePhase::Fetching,
            TorrentState::Paused => {
                if let Ok(Some(context)) = actor.execution_context(download) {
                    let _ = actor.observe(Fact::TransferPaused {
                        download,
                        attempt: snapshot.attempt,
                        control_generation: context.download.control_generation,
                    });
                }
                ActivePhase::Fetching
            }
            TorrentState::Seeding => ActivePhase::Seeding,
            TorrentState::Finished => {
                if let Some(path) = snapshot.content_folder.clone() {
                    let _ = actor.observe(Fact::TransferFinished {
                        download,
                        attempt: snapshot.attempt,
                        output: output_artifact(path, snapshot.content_folder.clone()),
                    });
                }
                ActivePhase::Seeding
            }
            TorrentState::Failed => {
                let _ = actor.observe(Fact::TransferFailed {
                    download,
                    attempt: snapshot.attempt,
                    message: snapshot
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "The torrent transfer failed.".to_string()),
                    retryable: true,
                });
                ActivePhase::Fetching
            }
        };
        if !snapshot.info_hash.is_empty() {
            let _ = actor.observe(Fact::TorrentResolved {
                download,
                attempt: snapshot.attempt,
                info_hash: snapshot.info_hash.clone(),
                metadata: TorrentMetadata {
                    total_size: snapshot.total_size,
                    piece_length: snapshot.piece_length,
                    piece_count: snapshot.piece_count,
                    comment: snapshot.comment.clone(),
                    trackers: snapshot.trackers.clone(),
                },
                files: snapshot
                    .files
                    .iter()
                    .map(|file| TorrentFileSelection {
                        index: file.file_index,
                        path: file.path.clone(),
                        size: file.size,
                        priority: match file.priority.as_str() {
                            "Skip" => TorrentFilePriority::Skip,
                            "Low" => TorrentFilePriority::Low,
                            "High" => TorrentFilePriority::High,
                            _ => TorrentFilePriority::Normal,
                        },
                    })
                    .collect(),
            });
        }
        if !matches!(
            snapshot.state,
            TorrentState::Paused | TorrentState::Finished | TorrentState::Failed
        ) {
            let _ = actor.observe(Fact::TransferAdvanced {
                download,
                attempt: snapshot.attempt,
                phase,
            });
        }
        let _ = actor.observe_telemetry(
            download,
            TransferTelemetry {
                downloaded_bytes: snapshot.bytes_downloaded,
                uploaded_bytes: snapshot.bytes_uploaded,
                total_bytes: snapshot.selected_size,
                download_rate: snapshot.download_rate,
                upload_rate: snapshot.upload_rate,
                eta_seconds: snapshot.eta_seconds,
                peers: snapshot.peers,
                seeds: snapshot.seeds,
                progress_millionths: fraction_to_millionths(snapshot.progress),
            },
        );
    }
    for snapshot in media.snapshots() {
        let Some(download_id_value) = snapshot.download_id.as_deref() else {
            continue;
        };
        let Ok(download) = download_id(download_id_value) else {
            continue;
        };
        match snapshot.state {
            MediaTransferState::Queued | MediaTransferState::Downloading => {
                let _ = actor.observe(Fact::TransferAdvanced {
                    download,
                    attempt: snapshot.attempt,
                    phase: ActivePhase::Fetching,
                });
            }
            MediaTransferState::Processing => {
                let _ = actor.observe(Fact::TransferAdvanced {
                    download,
                    attempt: snapshot.attempt,
                    phase: ActivePhase::Processing,
                });
            }
            MediaTransferState::Paused => {
                if let Ok(Some(context)) = actor.execution_context(download) {
                    let _ = actor.observe(Fact::TransferPaused {
                        download,
                        attempt: snapshot.attempt,
                        control_generation: context.download.control_generation,
                    });
                }
            }
            MediaTransferState::Finished => {
                if let Some(path) = snapshot.output_path.clone() {
                    let _ = actor.observe(Fact::TransferFinished {
                        download,
                        attempt: snapshot.attempt,
                        output: output_artifact(path, snapshot.content_folder.clone()),
                    });
                }
            }
            MediaTransferState::Failed => {
                let _ = actor.observe(Fact::TransferFailed {
                    download,
                    attempt: snapshot.attempt,
                    message: snapshot
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "The media transfer failed.".to_string()),
                    retryable: true,
                });
            }
            MediaTransferState::Cancelled => {}
        }
        let _ = actor.observe_telemetry(
            download,
            TransferTelemetry {
                downloaded_bytes: snapshot.downloaded_bytes,
                uploaded_bytes: 0,
                total_bytes: snapshot.total_bytes,
                download_rate: snapshot.download_rate,
                upload_rate: 0,
                eta_seconds: snapshot.eta_seconds,
                peers: 0,
                seeds: 0,
                progress_millionths: fraction_to_millionths(snapshot.progress),
            },
        );
    }
}

fn snapshot_view(projection: AppProjection) -> MachineSnapshot {
    MachineSnapshot {
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

fn workspace_view(workspace: WorkspaceProjection) -> MachineWorkspace {
    MachineWorkspace {
        id: workspace.id.to_string(),
        name: workspace.name,
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

fn download_view(download: DownloadProjection) -> MachineDownload {
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
        files,
        capabilities: MachineCapabilities {
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

type SourceFields = (
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

fn actor_source_fields(download: &DownloadProjection) -> SourceFields {
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

fn lifecycle_view(
    lifecycle: &DownloadLifecycle,
) -> (
    MachineDownloadState,
    Option<MachineHoldCause>,
    Option<String>,
) {
    match lifecycle {
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

fn phase_view(phase: ActivePhase) -> MachineDownloadState {
    match phase {
        ActivePhase::Fetching => MachineDownloadState::Fetching,
        ActivePhase::Processing => MachineDownloadState::Processing,
        ActivePhase::Verifying => MachineDownloadState::Verifying,
        ActivePhase::Seeding => MachineDownloadState::Seeding,
    }
}

fn hold_view(cause: HoldCause) -> MachineHoldCause {
    match cause {
        HoldCause::Person => MachineHoldCause::Person,
        HoldCause::Shutdown => MachineHoldCause::Shutdown,
        HoldCause::Recovery => MachineHoldCause::Recovery,
    }
}

fn icon_from_view(icon: MachineWorkspaceIcon) -> WorkspaceIcon {
    match icon.kind {
        MachineWorkspaceIconKind::Symbol => WorkspaceIcon::Symbol(icon.value),
        MachineWorkspaceIconKind::Emoji => WorkspaceIcon::Emoji(icon.value),
    }
}

fn format_from_view(format: MachineFormat) -> MediaFormat {
    match format {
        MachineFormat::Best => MediaFormat::Best,
        MachineFormat::BestMp4 => MediaFormat::BestMp4,
        MachineFormat::P1080 => MediaFormat::P1080,
        MachineFormat::P720 => MediaFormat::P720,
        MachineFormat::Original => MediaFormat::Original,
        MachineFormat::AudioMp3 => MediaFormat::AudioMp3,
        MachineFormat::AudioM4a => MediaFormat::AudioM4a,
    }
}

fn format_view(format: MediaFormat) -> MachineFormat {
    match format {
        MediaFormat::Best => MachineFormat::Best,
        MediaFormat::BestMp4 => MachineFormat::BestMp4,
        MediaFormat::P1080 => MachineFormat::P1080,
        MediaFormat::P720 => MachineFormat::P720,
        MediaFormat::Original => MachineFormat::Original,
        MediaFormat::AudioMp3 => MachineFormat::AudioMp3,
        MediaFormat::AudioM4a => MachineFormat::AudioM4a,
    }
}

fn engine_format(format: MediaFormat) -> EngineMediaFormat {
    match format {
        MediaFormat::Best => EngineMediaFormat::Best,
        MediaFormat::BestMp4 => EngineMediaFormat::BestMp4,
        MediaFormat::P1080 => EngineMediaFormat::P1080,
        MediaFormat::P720 => EngineMediaFormat::P720,
        MediaFormat::Original => EngineMediaFormat::Original,
        MediaFormat::AudioMp3 => EngineMediaFormat::AudioMp3,
        MediaFormat::AudioM4a => EngineMediaFormat::AudioM4a,
    }
}

fn organization_from_view(value: MachineFileOrganization) -> FileOrganization {
    match value {
        MachineFileOrganization::Automatic => FileOrganization::Automatic,
        MachineFileOrganization::AlwaysCreateFolder => FileOrganization::AlwaysCreateFolder,
        MachineFileOrganization::KeepInSaveLocation => FileOrganization::KeepInSaveLocation,
    }
}

fn organization_view(value: FileOrganization) -> MachineFileOrganization {
    match value {
        FileOrganization::Automatic => MachineFileOrganization::Automatic,
        FileOrganization::AlwaysCreateFolder => MachineFileOrganization::AlwaysCreateFolder,
        FileOrganization::KeepInSaveLocation => MachineFileOrganization::KeepInSaveLocation,
    }
}

fn content_layout(value: FileOrganization) -> DownloadContentLayout {
    match value {
        FileOrganization::Automatic => DownloadContentLayout::Original,
        FileOrganization::AlwaysCreateFolder => DownloadContentLayout::Subfolder,
        FileOrganization::KeepInSaveLocation => DownloadContentLayout::NoSubfolder,
    }
}

fn policy_from_view(value: &MachineMediaPolicy) -> MediaPolicy {
    MediaPolicy {
        download_subtitles: value.download_subtitles,
        subtitle_languages: value.subtitle_languages.clone(),
        embed_subtitles: value.embed_subtitles,
        write_thumbnail: value.write_thumbnail,
        write_metadata: value.write_metadata,
        filename_template: value.filename_template.clone(),
        user_agent: value.user_agent.clone(),
    }
}

fn media_options(download: &MediaDownload, layout: DownloadContentLayout) -> MediaDownloadOptions {
    let referer = match &download.origin {
        MediaOrigin::Page { page_url, .. } | MediaOrigin::Guided { page_url, .. } => {
            Some(page_url.clone())
        }
        MediaOrigin::Direct { referer, .. } => referer.clone(),
    };
    MediaDownloadOptions {
        format: engine_format(download.format),
        content_layout: layout,
        download_subtitles: download.policy.download_subtitles,
        subtitle_languages: download.policy.subtitle_languages.clone(),
        embed_subtitles: download.policy.embed_subtitles,
        write_thumbnail: download.policy.write_thumbnail,
        write_metadata: download.policy.write_metadata,
        filename_template: download.policy.filename_template.clone(),
        referer,
        user_agent: download.policy.user_agent.clone(),
    }
}

fn file_from_view(file: MachineTorrentFile) -> TorrentFileSelection {
    TorrentFileSelection {
        index: file.index,
        path: file.path,
        size: file.size,
        priority: match file.priority.as_str() {
            "skip" | "Skip" => TorrentFilePriority::Skip,
            "low" | "Low" => TorrentFilePriority::Low,
            "high" | "High" => TorrentFilePriority::High,
            _ => TorrentFilePriority::Normal,
        },
    }
}

fn output_artifact(path: String, content_folder: Option<String>) -> OutputArtifact {
    let identity_path = content_folder.as_deref().unwrap_or(&path);
    let identity = fs::metadata(identity_path)
        .ok()
        .map(|metadata| OutputIdentity {
            byte_length: metadata.len(),
            modified_at_millis: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
            #[cfg(unix)]
            file_identifier: {
                use std::os::unix::fs::MetadataExt;
                Some(format!("{}:{}", metadata.dev(), metadata.ino()))
            },
            #[cfg(not(unix))]
            file_identifier: None,
        });
    OutputArtifact {
        path,
        content_folder,
        identity,
    }
}

fn fraction_to_millionths(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

fn fingerprint(command: &Command) -> Result<String, StateMachineError> {
    let bytes = serde_json::to_vec(command).map_err(|error| StateMachineError::Rejected {
        reason: format!("could not fingerprint command: {error}"),
    })?;
    Ok(hex::encode(Sha1::digest(bytes)))
}

fn parse_id(value: &str, kind: &str) -> Result<Uuid, StateMachineError> {
    Uuid::parse_str(value).map_err(|error| StateMachineError::InvalidIdentifier {
        reason: format!("invalid {kind} ID: {error}"),
    })
}

fn workspace_id(value: &str) -> Result<WorkspaceId, StateMachineError> {
    parse_id(value, "workspace").map(WorkspaceId)
}

fn folder_id(value: &str) -> Result<FolderId, StateMachineError> {
    parse_id(value, "folder").map(FolderId)
}

fn download_id(value: &str) -> Result<DownloadId, StateMachineError> {
    parse_id(value, "download").map(DownloadId)
}

fn command_id(value: &str) -> Result<CommandId, StateMachineError> {
    parse_id(value, "command").map(CommandId)
}

fn effect_id(value: &str) -> Result<EffectId, StateMachineError> {
    parse_id(value, "effect").map(EffectId)
}

fn actor_error(error: ActorError) -> StateMachineError {
    match error {
        ActorError::Open(reason) => StateMachineError::Open { reason },
        ActorError::AlreadyOpen => StateMachineError::Open {
            reason: "the application state is already open".to_string(),
        },
        ActorError::Rejected(reason) | ActorError::ReadOnly(reason) => {
            StateMachineError::Rejected { reason }
        }
        ActorError::Durability(reason) => StateMachineError::Persistence { reason },
        ActorError::Stopped => StateMachineError::Open {
            reason: "the application state machine stopped".to_string(),
        },
    }
}

fn open_error(error: ActorError) -> StateMachineError {
    actor_error(error)
}

fn engine_error(error: crate::CoreError) -> StateMachineError {
    StateMachineError::Engine {
        reason: error.to_string(),
    }
}

fn secure_write_new(path: &Path, bytes: &[u8]) -> Result<(), StateMachineError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| StateMachineError::Persistence {
            reason: format!("could not create {}: {error}", path.display()),
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| StateMachineError::Persistence {
            reason: format!("could not write {}: {error}", path.display()),
        })?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| StateMachineError::Persistence {
                reason: format!("could not sync {}: {error}", parent.display()),
            })?;
    }
    Ok(())
}
