use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CommandEnvelope {
    pub(crate) id: CommandId,
    pub(crate) fingerprint: String,
    pub(crate) expected_revision: Option<u64>,
    pub(crate) command: Command,
}

impl Eq for CommandEnvelope {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Command {
    RemoveVideoTools,
    SetWorkspaceAppearance {
        workspace: WorkspaceId,
        emoji: String,
        appearance: crate::application::WorkspaceAppearance,
    },
    CreateWorkspace {
        workspace: Workspace,
    },
    RenameWorkspace {
        workspace: WorkspaceId,
        name: String,
        icon: WorkspaceIcon,
        organization: FileOrganization,
    },
    DeleteWorkspace {
        workspace: WorkspaceId,
        replacement: Option<Workspace>,
        move_files_to_trash: bool,
    },
    AddFolder {
        workspace: WorkspaceId,
        folder: SavedFolder,
        make_default: bool,
    },
    SetDefaultFolder {
        workspace: WorkspaceId,
        folder: FolderId,
    },
    RelinkFolder {
        workspace: WorkspaceId,
        folder: FolderId,
        name: String,
        path: String,
    },
    RemoveFolder {
        workspace: WorkspaceId,
        folder: FolderId,
        move_files_to_trash: bool,
    },
    SetWorkspaceView {
        workspace: WorkspaceId,
        view: WorkspaceView,
    },
    SetDefaultFormat {
        workspace: WorkspaceId,
        format: MediaFormat,
    },
    AddDownload {
        workspace: WorkspaceId,
        download: Box<Download>,
    },
    AddMediaItems {
        workspace: WorkspaceId,
        downloads: Vec<Download>,
    },
    Pause {
        download: DownloadId,
    },
    Resume {
        download: DownloadId,
    },
    Retry {
        download: DownloadId,
    },
    Cancel {
        download: DownloadId,
    },
    StopSeeding {
        download: DownloadId,
    },
    Remove {
        downloads: Vec<DownloadId>,
        move_files_to_trash: bool,
    },
    SetSeedRatio {
        download: DownloadId,
        limit: Option<f64>,
    },
    SetTorrentFiles {
        download: DownloadId,
        files: Vec<TorrentFileSelection>,
    },
    MoveDownload {
        download: DownloadId,
        folder: FolderId,
        output_path: String,
        content_folder: Option<String>,
        #[serde(default)]
        identity: Option<OutputIdentity>,
    },
    AcknowledgeRecovery,
    MarkReconciled,
    BeginShutdown {
        boot: BootId,
    },
    CompleteShutdown,
}

impl Eq for Command {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum Fact {
    VideoToolsChanged(VideoToolsAvailability),
    MediaPageFailed {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
        message: String,
    },
    MediaPageResolved {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
        items: Vec<Download>,
    },
    TransferStarted {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
        phase: ActivePhase,
    },
    TransferAdvanced {
        download: DownloadId,
        attempt: u32,
        phase: ActivePhase,
    },
    TransferPaused {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
    },
    TransferFinished {
        download: DownloadId,
        attempt: u32,
        output: OutputArtifact,
    },
    TransferFailed {
        download: DownloadId,
        attempt: u32,
        message: String,
        retryable: bool,
    },
    TorrentResolved {
        download: DownloadId,
        attempt: u32,
        info_hash: String,
        metadata: TorrentMetadata,
        files: Vec<TorrentFileSelection>,
    },
    LeftoversFound {
        download: DownloadId,
        attempt: u32,
        leftovers: Vec<OutputArtifact>,
    },
    WriterStopped {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
    },
    AdapterRecordAbsent {
        download: DownloadId,
        attempt: u32,
    },
    FilesResolved {
        download: DownloadId,
        control_generation: u64,
    },
    RetirementFinalized {
        download: DownloadId,
        control_generation: u64,
    },
}

impl Eq for Fact {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Decision {
    pub(crate) events: Vec<Event>,
    pub(crate) effects: Vec<EffectPlan>,
    pub(crate) created_downloads: Vec<DownloadId>,
}

impl Decision {
    pub(super) fn none() -> Self {
        Self {
            events: Vec::new(),
            effects: Vec::new(),
            created_downloads: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    VideoToolsChanged(VideoToolsAvailability),
    MediaPageExpanded {
        workspace: WorkspaceId,
        download: DownloadId,
        items: Vec<Download>,
    },
    WorkspaceAppearanceChanged {
        workspace: WorkspaceId,
        emoji: String,
        appearance: crate::application::WorkspaceAppearance,
    },
    WorkspaceCreated(Workspace),
    WorkspaceRenamed {
        workspace: WorkspaceId,
        name: String,
        icon: WorkspaceIcon,
        organization: FileOrganization,
        appearance: crate::application::WorkspaceAppearance,
    },
    WorkspaceDeleted {
        workspace: WorkspaceId,
        replacement: Option<Workspace>,
    },
    FolderAdded {
        workspace: WorkspaceId,
        folder: SavedFolder,
        make_default: bool,
    },
    DefaultFolderChanged {
        workspace: WorkspaceId,
        folder: FolderId,
    },
    FolderRelinked {
        workspace: WorkspaceId,
        folder: FolderId,
        name: String,
        path: String,
    },
    FolderRemoved {
        workspace: WorkspaceId,
        folder: FolderId,
    },
    WorkspaceDeletionRequested(PendingWorkspaceDeletion),
    FolderRemovalRequested(PendingFolderRemoval),
    WorkspaceViewChanged {
        workspace: WorkspaceId,
        view: WorkspaceView,
    },
    DefaultFormatChanged {
        workspace: WorkspaceId,
        format: MediaFormat,
    },
    DownloadAdded {
        workspace: WorkspaceId,
        download: Box<Download>,
    },
    DownloadLifecycleChanged {
        download: DownloadId,
        lifecycle: DownloadLifecycle,
        control_generation: u64,
    },
    DownloadRetried {
        download: DownloadId,
        attempt: u32,
        control_generation: u64,
    },
    DownloadsBeganRetiring {
        downloads: Vec<DownloadId>,
        move_files_to_trash: bool,
    },
    RetirementLeftoversRecorded {
        download: DownloadId,
        leftovers: Vec<OutputArtifact>,
    },
    RetirementProgressed {
        download: DownloadId,
        writer_stopped: Option<bool>,
        adapter_absent: Option<bool>,
        files_resolved: Option<bool>,
    },
    DownloadRetired {
        download: DownloadId,
    },
    SeedRatioChanged {
        download: DownloadId,
        limit: Option<f64>,
        control_generation: u64,
    },
    TorrentFilesChanged {
        download: DownloadId,
        files: Vec<TorrentFileSelection>,
        control_generation: u64,
    },
    DownloadMoved {
        download: DownloadId,
        folder: FolderId,
        output_path: String,
        content_folder: Option<String>,
        identity: Option<OutputIdentity>,
    },
    TorrentResolved {
        download: DownloadId,
        info_hash: String,
        metadata: TorrentMetadata,
        files: Vec<TorrentFileSelection>,
    },
    DownloadOutputRecorded {
        download: DownloadId,
        output: OutputArtifact,
    },
    SessionChanged(SessionPhase),
    RecoveryAcknowledged,
}

impl Eq for Event {}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct EffectPlan {
    pub(crate) attempt: Option<u32>,
    pub(crate) control_generation: Option<u64>,
    pub(crate) kind: EffectKind,
}

impl Eq for EffectPlan {}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Rejection {
    #[error("the command was based on revision {expected}, but the current revision is {current}")]
    StaleRevision { expected: u64, current: u64 },
    #[error("the command ID was already used with different input")]
    CommandIdReused,
    #[error("the application is not accepting mutations while {0:?}")]
    SessionUnavailable(SessionPhase),
    #[error("workspace {0} does not exist")]
    UnknownWorkspace(WorkspaceId),
    #[error("folder {folder} does not belong to workspace {workspace}")]
    FolderNotInWorkspace {
        workspace: WorkspaceId,
        folder: FolderId,
    },
    #[error("workspace {0} already exists")]
    DuplicateWorkspace(WorkspaceId),
    #[error("folder {0} already exists")]
    DuplicateFolder(FolderId),
    #[error("a workspace must keep at least one saved folder")]
    CannotRemoveOnlyFolder,
    #[error("download {0} does not exist")]
    UnknownDownload(DownloadId),
    #[error("download {0} already exists")]
    DuplicateDownload(DownloadId),
    #[error("torrent duplicates active download {0}")]
    DuplicateTorrent(DownloadId),
    #[error("{action} is not available while the download is {lifecycle}")]
    InvalidTransition {
        action: &'static str,
        lifecycle: &'static str,
    },
    #[error("that action does not apply to this download source")]
    WrongSource,
    #[error("the replacement workspace is required when deleting the last workspace")]
    ReplacementWorkspaceRequired,
    #[error("{0}")]
    InvalidInput(String),
    #[error("the proposed transition violates an invariant: {0}")]
    Invariant(#[from] InvariantViolation),
}
