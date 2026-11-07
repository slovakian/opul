use super::*;

#[derive(Debug, Clone)]
pub struct OpulMachineConfig {
    pub application_support_folder: String,
    pub downloads_folder: String,
    pub torrent: EngineConfig,
    pub media: MediaEngineConfig,
}

#[derive(Debug, thiserror::Error)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineSessionState {
    Booting,
    Reconciling,
    Running,
    Quiescing,
    Stopped,
    RecoveryRequired,
    ReadOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineWorkspaceIconKind {
    Symbol,
    Emoji,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineWorkspaceIcon {
    pub kind: MachineWorkspaceIconKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineFormat {
    Best,
    BestMp4,
    P1080,
    P720,
    Original,
    AudioMp3,
    AudioM4a,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineFileOrganization {
    Automatic,
    AlwaysCreateFolder,
    KeepInSaveLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineLayout {
    List,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineSort {
    Order,
    Name,
    State,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineGrouping {
    None,
    State,
    Folder,
    Kind,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineSource {
    Torrent,
    VideoPage,
    Direct,
    Guided,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineDownloadState {
    WaitingForVideoTools,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MachineHoldCause {
    Person,
    Shutdown,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineCapabilities {
    pub can_set_up_video_tools: bool,
    pub can_retry_video_tools: bool,
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

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineTorrentFile {
    pub index: u32,
    pub path: String,
    pub size: u64,
    pub priority: String,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MachineDownload {
    pub needs_video_tools: bool,
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
    pub waiting_label: Option<String>,
    pub files: Vec<MachineTorrentFile>,
    pub capabilities: MachineCapabilities,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MachineWorkspace {
    pub id: String,
    pub name: String,
    pub icon: MachineWorkspaceIcon,
    pub appearance: WorkspaceAppearance,
    pub folders: Vec<MachineFolder>,
    pub default_format: MachineFormat,
    pub file_organization: MachineFileOrganization,
    pub layout: MachineLayout,
    pub sort: MachineSort,
    pub grouping: MachineGrouping,
    pub downloads: Vec<MachineDownload>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MachineSnapshot {
    pub revision: u64,
    pub boot_id: Option<String>,
    pub sequence: u64,
    pub session: MachineSessionState,
    pub workspaces: Vec<MachineWorkspace>,
    pub recent_emoji: Vec<String>,
    pub recovery_notices: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineCommandReceipt {
    pub revision: u64,
    pub created_download_ids: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineWorkspaceRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub name: String,
    pub icon: MachineWorkspaceIcon,
    #[serde(default)]
    pub appearance: WorkspaceAppearance,
    pub folder_id: String,
    pub folder_name: String,
    pub folder_path: String,
    pub default_format: MachineFormat,
    pub file_organization: MachineFileOrganization,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineWorkspaceDraft {
    pub workspace_id: String,
    pub folder_id: String,
    pub appearance: WorkspaceAppearance,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineWorkspaceAppearanceRequest {
    pub request_id: String,
    pub workspace_id: String,
    pub emoji: String,
    pub appearance: WorkspaceAppearance,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineFolderRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub folder_id: String,
    pub name: String,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineDownloadCommand {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub download_id: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineTorrentRequest {
    pub request_id: String,
    pub expected_revision: Option<u64>,
    pub workspace_id: String,
    pub folder_id: String,
    pub source: String,
    pub metainfo: Option<Vec<u8>>,
    pub seed_ratio_limit: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineMediaPolicy {
    pub download_subtitles: bool,
    pub subtitle_languages: Vec<String>,
    pub embed_subtitles: bool,
    pub write_thumbnail: bool,
    pub write_metadata: bool,
    pub filename_template: String,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MachineMediaItem {
    pub url: String,
    pub title: String,
    pub page_url: String,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub playlist_index: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachineTrashTarget {
    pub path: String,
    pub expected_byte_length: Option<u64>,
    pub expected_modified_at_millis: Option<u64>,
    pub expected_file_identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MachinePlatformEffect {
    pub effect_id: String,
    pub download_id: String,
    pub targets: Vec<MachineTrashTarget>,
}

#[derive(Clone)]
pub(super) struct GuidedInput {
    pub(super) session: Arc<GuidedExtractionSession>,
    pub(super) selection: GuidedDownloadSelection,
}

pub struct OpulStateMachine {
    pub(super) actor: Arc<ApplicationActor>,
    pub(super) torrent: Arc<TorrentEngine>,
    pub(super) media: Arc<MediaEngine>,
    pub(super) video_tools: Option<Arc<crate::video_tools::VideoTools>>,
    pub(super) guided: Arc<Mutex<HashMap<DownloadId, GuidedInput>>>,
    pub(super) platform_effects: Arc<Mutex<Vec<MachinePlatformEffect>>>,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) execution_gate: Arc<Mutex<()>>,
    pub(super) runner: Mutex<Option<JoinHandle<()>>>,
    pub(super) support_directory: PathBuf,
}
