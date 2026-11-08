mod downloads;
pub(crate) use downloads::*;

use super::WorkspaceAppearance;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub(crate) struct $name(pub(crate) Uuid);

        impl $name {
            #[allow(dead_code)]
            pub(crate) fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }
    };
}

id_type!(WorkspaceId);
id_type!(FolderId);
id_type!(DownloadId);
id_type!(CommandId);
id_type!(EffectId);
id_type!(BootId);

pub(crate) const STATE_SCHEMA_VERSION: u32 = 2;
pub(crate) const RECENT_EMOJI_LIMIT: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct AppState {
    pub(crate) schema_version: u32,
    pub(crate) revision: u64,
    #[serde(default, skip_serializing_if = "VideoToolsAvailability::is_ready")]
    pub(crate) video_tools: VideoToolsAvailability,
    pub(crate) session: SessionPhase,
    pub(crate) workspaces: Vec<Workspace>,
    // Preserve checksums of checkpoints written before recent emoji existed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) recent_emoji: Vec<String>,
    #[serde(default)]
    pub(crate) structural_changes: PendingStructuralChanges,
    pub(crate) outbox: BTreeMap<EffectId, PendingEffect>,
    pub(crate) receipts: BTreeMap<CommandId, CommandReceipt>,
    pub(crate) recovery: Vec<RecoveryItem>,
    pub(crate) migration: Option<MigrationRecord>,
}

impl AppState {
    pub(crate) fn fresh(downloads_path: String) -> Self {
        let folder_id = FolderId::new();
        let mut state = Self {
            schema_version: STATE_SCHEMA_VERSION,
            revision: 0,
            video_tools: VideoToolsAvailability::Ready,
            session: SessionPhase::Booting,
            workspaces: vec![Workspace {
                id: WorkspaceId::new(),
                appearance: WorkspaceAppearance::default(),
                name: "Downloads".to_string(),
                icon: WorkspaceIcon::Emoji("📁".to_string()),
                folders: vec![SavedFolder {
                    id: folder_id,
                    name: "Downloads".to_string(),
                    path: downloads_path,
                }],
                default_folder: folder_id,
                default_format: MediaFormat::Best,
                file_organization: FileOrganization::Automatic,
                view: WorkspaceView::default(),
                downloads: Vec::new(),
            }],
            structural_changes: PendingStructuralChanges::default(),
            recent_emoji: Vec::new(),
            outbox: BTreeMap::new(),
            receipts: BTreeMap::new(),
            recovery: Vec::new(),
            migration: None,
        };
        state.migrate_workspace_appearances();
        state
    }

    pub(crate) fn validate(&self) -> Result<(), InvariantViolation> {
        if self.schema_version != STATE_SCHEMA_VERSION {
            return Err(InvariantViolation::UnsupportedSchema(self.schema_version));
        }
        if self.workspaces.is_empty() {
            return Err(InvariantViolation::NoWorkspace);
        }
        if self.recent_emoji.len() > RECENT_EMOJI_LIMIT
            || self.recent_emoji.iter().collect::<BTreeSet<_>>().len() != self.recent_emoji.len()
            || self.recent_emoji.iter().any(|emoji| {
                emoji.trim().is_empty() || emoji.len() > 128 || emoji.chars().any(char::is_control)
            })
        {
            return Err(InvariantViolation::InvalidRecentEmoji);
        }

        let mut workspace_ids = BTreeSet::new();
        let mut folder_ids = BTreeSet::new();
        let mut download_ids = BTreeSet::new();
        let mut active_hashes = BTreeMap::<String, DownloadId>::new();
        for workspace in &self.workspaces {
            if !workspace_ids.insert(workspace.id) {
                return Err(InvariantViolation::DuplicateWorkspace(workspace.id));
            }
            if workspace.name.trim().is_empty() {
                return Err(InvariantViolation::EmptyWorkspaceName(workspace.id));
            }
            if !workspace.appearance.valid() {
                return Err(InvariantViolation::InvalidWorkspaceAppearance(workspace.id));
            }
            if workspace.folders.is_empty() {
                return Err(InvariantViolation::NoFolder(workspace.id));
            }
            if !workspace
                .folders
                .iter()
                .any(|folder| folder.id == workspace.default_folder)
            {
                return Err(InvariantViolation::MissingDefaultFolder(workspace.id));
            }
            let local_folders = workspace
                .folders
                .iter()
                .map(|folder| folder.id)
                .collect::<BTreeSet<_>>();
            if local_folders.len() != workspace.folders.len() {
                return Err(InvariantViolation::DuplicateFolder(workspace.id));
            }
            for folder in &workspace.folders {
                if !folder_ids.insert(folder.id) {
                    return Err(InvariantViolation::FolderSharedAcrossWorkspaces(folder.id));
                }
                if folder.name.trim().is_empty() || folder.path.trim().is_empty() {
                    return Err(InvariantViolation::InvalidFolder(folder.id));
                }
            }
            for download in &workspace.downloads {
                if !download_ids.insert(download.id) {
                    return Err(InvariantViolation::DuplicateDownload(download.id));
                }
                if !local_folders.contains(&download.folder) {
                    return Err(InvariantViolation::FolderOutsideWorkspace {
                        download: download.id,
                        folder: download.folder,
                    });
                }
                if download.name.trim().is_empty() {
                    return Err(InvariantViolation::EmptyDownloadName(download.id));
                }
                if !download.lifecycle.valid_for(download.source_kind()) {
                    return Err(InvariantViolation::LifecycleDoesNotMatchSource(download.id));
                }
                if let Some(hash) = download.active_info_hash() {
                    let normalized = hash.to_ascii_lowercase();
                    if let Some(existing) = active_hashes.insert(normalized, download.id) {
                        return Err(InvariantViolation::DuplicateActiveInfoHash {
                            first: existing,
                            second: download.id,
                        });
                    }
                }
            }
        }

        for effect in self.outbox.values() {
            if effect.id.0.is_nil() {
                return Err(InvariantViolation::NilEffectId);
            }
            if let Some(target) = effect.target()
                && !download_ids.contains(&target)
                && !matches!(effect.kind, EffectKind::FinalizeRetirement { .. })
            {
                return Err(InvariantViolation::EffectTargetsUnknownDownload(target));
            }
        }
        let mut pending_workspaces = BTreeSet::new();
        for pending in &self.structural_changes.workspace_deletions {
            if !pending_workspaces.insert(pending.workspace)
                || self.workspace(pending.workspace).is_none()
            {
                return Err(InvariantViolation::InvalidPendingStructure);
            }
            if let Some(replacement) = &pending.replacement {
                let replacement_folders = replacement
                    .folders
                    .iter()
                    .map(|folder| folder.id)
                    .collect::<BTreeSet<_>>();
                if !replacement.appearance.valid()
                    || replacement.name.trim().is_empty()
                    || replacement.folders.is_empty()
                    || replacement_folders.len() != replacement.folders.len()
                    || !replacement_folders.contains(&replacement.default_folder)
                    || !replacement.downloads.is_empty()
                    || workspace_ids.contains(&replacement.id)
                    || replacement_folders
                        .iter()
                        .any(|folder| folder_ids.contains(folder))
                {
                    return Err(InvariantViolation::InvalidPendingStructure);
                }
            }
        }
        let mut pending_folders = BTreeSet::new();
        for pending in &self.structural_changes.folder_removals {
            if !pending_folders.insert((pending.workspace, pending.folder))
                || self.workspace(pending.workspace).is_none_or(|workspace| {
                    !workspace
                        .folders
                        .iter()
                        .any(|folder| folder.id == pending.folder)
                })
            {
                return Err(InvariantViolation::InvalidPendingStructure);
            }
        }
        Ok(())
    }

    pub(crate) fn record_recent_emoji(&mut self, emoji: &str) {
        self.recent_emoji.retain(|saved| saved != emoji);
        self.recent_emoji.insert(0, emoji.to_string());
        self.recent_emoji.truncate(RECENT_EMOJI_LIMIT);
    }

    pub(crate) fn workspace(&self, id: WorkspaceId) -> Option<&Workspace> {
        self.workspaces.iter().find(|workspace| workspace.id == id)
    }

    pub(crate) fn workspace_mut(&mut self, id: WorkspaceId) -> Option<&mut Workspace> {
        self.workspaces
            .iter_mut()
            .find(|workspace| workspace.id == id)
    }

    pub(crate) fn locate_download(&self, id: DownloadId) -> Option<(WorkspaceId, &Download)> {
        self.workspaces.iter().find_map(|workspace| {
            workspace
                .downloads
                .iter()
                .find(|download| download.id == id)
                .map(|download| (workspace.id, download))
        })
    }

    pub(crate) fn locate_download_mut(
        &mut self,
        id: DownloadId,
    ) -> Option<(WorkspaceId, &mut Download)> {
        self.workspaces.iter_mut().find_map(|workspace| {
            workspace
                .downloads
                .iter_mut()
                .find(|download| download.id == id)
                .map(|download| (workspace.id, download))
        })
    }

    pub(crate) fn remember_receipt(&mut self, receipt: CommandReceipt) {
        self.receipts.insert(receipt.command_id, receipt);
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingStructuralChanges {
    pub(crate) workspace_deletions: Vec<PendingWorkspaceDeletion>,
    pub(crate) folder_removals: Vec<PendingFolderRemoval>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingWorkspaceDeletion {
    pub(crate) workspace: WorkspaceId,
    pub(crate) replacement: Option<Workspace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct PendingFolderRemoval {
    pub(crate) workspace: WorkspaceId,
    pub(crate) folder: FolderId,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum VideoToolsAvailability {
    NotSetUp,
    TurnedOff,
    SettingUp,
    #[default]
    Ready,
    NotWorking,
}

impl VideoToolsAvailability {
    fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum SessionPhase {
    Booting,
    Reconciling,
    Running,
    Quiescing { boot: BootId },
    Stopped,
    RecoveryRequired,
    ReadOnlyDurabilityFault,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Workspace {
    pub(crate) id: WorkspaceId,
    pub(crate) name: String,
    // Omitting the empty default preserves checksums of checkpoints written before appearances.
    #[serde(default, skip_serializing_if = "WorkspaceAppearance::is_unassigned")]
    pub(crate) appearance: WorkspaceAppearance,
    pub(crate) icon: WorkspaceIcon,
    pub(crate) folders: Vec<SavedFolder>,
    pub(crate) default_folder: FolderId,
    pub(crate) default_format: MediaFormat,
    pub(crate) file_organization: FileOrganization,
    pub(crate) view: WorkspaceView,
    pub(crate) downloads: Vec<Download>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum WorkspaceIcon {
    Symbol(String),
    Emoji(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SavedFolder {
    pub(crate) id: FolderId,
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MediaFormat {
    Best,
    BestMp4,
    P1080,
    P720,
    Original,
    AudioMp3,
    AudioM4a,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum FileOrganization {
    Automatic,
    AlwaysCreateFolder,
    KeepInSaveLocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum WorkspaceLayout {
    List,
    Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum WorkspaceSort {
    Order,
    Name,
    State,
    Source,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum WorkspaceGrouping {
    None,
    State,
    Folder,
    Kind,
    Source,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct WorkspaceView {
    pub(crate) layout: WorkspaceLayout,
    pub(crate) sort: WorkspaceSort,
    pub(crate) grouping: WorkspaceGrouping,
}

impl Default for WorkspaceView {
    fn default() -> Self {
        Self {
            layout: WorkspaceLayout::List,
            sort: WorkspaceSort::Order,
            grouping: WorkspaceGrouping::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PendingEffect {
    pub(crate) id: EffectId,
    pub(crate) created_revision: u64,
    pub(crate) attempt: Option<u32>,
    pub(crate) control_generation: Option<u64>,
    pub(crate) status: EffectStatus,
    pub(crate) kind: EffectKind,
}

impl Eq for PendingEffect {}

impl PendingEffect {
    pub(crate) fn target(&self) -> Option<DownloadId> {
        self.kind.target()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum EffectStatus {
    Pending,
    InFlight,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum EffectKind {
    EnsureTransfer {
        download: DownloadId,
    },
    ConvergeControl {
        download: DownloadId,
    },
    RemoveTransfer {
        download: DownloadId,
    },
    MoveOutputToTrash {
        download: DownloadId,
        artifact: Option<OutputArtifact>,
        #[serde(default)]
        leftovers: Vec<OutputArtifact>,
    },
    FinalizeRetirement {
        download: DownloadId,
    },
    StopAllTransfers,
}

impl Eq for EffectKind {}

impl EffectKind {
    pub(crate) fn target(&self) -> Option<DownloadId> {
        match self {
            Self::EnsureTransfer { download }
            | Self::ConvergeControl { download }
            | Self::RemoveTransfer { download }
            | Self::MoveOutputToTrash { download, .. }
            | Self::FinalizeRetirement { download } => Some(*download),
            Self::StopAllTransfers => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct CommandReceipt {
    pub(crate) command_id: CommandId,
    pub(crate) fingerprint: String,
    pub(crate) revision: u64,
    pub(crate) created_downloads: Vec<DownloadId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecoveryItem {
    pub(crate) key: String,
    pub(crate) summary: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MigrationRecord {
    pub(crate) source_fingerprint: String,
    pub(crate) imported_downloads: u64,
    pub(crate) recovery_items: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum InvariantViolation {
    #[error("recent emoji must be valid, unique, and bounded")]
    InvalidRecentEmoji,
    #[error("workspace {0} has an invalid appearance")]
    InvalidWorkspaceAppearance(WorkspaceId),
    #[error("unsupported application-state schema {0}")]
    UnsupportedSchema(u32),
    #[error("the application must contain at least one workspace")]
    NoWorkspace,
    #[error("workspace {0} occurs more than once")]
    DuplicateWorkspace(WorkspaceId),
    #[error("workspace {0} has an empty name")]
    EmptyWorkspaceName(WorkspaceId),
    #[error("workspace {0} has no saved folder")]
    NoFolder(WorkspaceId),
    #[error("workspace {0} does not contain its default folder")]
    MissingDefaultFolder(WorkspaceId),
    #[error("workspace {0} contains a duplicate folder")]
    DuplicateFolder(WorkspaceId),
    #[error("folder {0} is shared by multiple workspaces")]
    FolderSharedAcrossWorkspaces(FolderId),
    #[error("folder {0} has an empty name or path")]
    InvalidFolder(FolderId),
    #[error("download {0} occurs more than once")]
    DuplicateDownload(DownloadId),
    #[error("download {download} refers to folder {folder} outside its workspace")]
    FolderOutsideWorkspace {
        download: DownloadId,
        folder: FolderId,
    },
    #[error("download {0} has an empty name")]
    EmptyDownloadName(DownloadId),
    #[error("download {0} has a lifecycle that its source cannot enter")]
    LifecycleDoesNotMatchSource(DownloadId),
    #[error("downloads {first} and {second} share an active torrent info hash")]
    DuplicateActiveInfoHash {
        first: DownloadId,
        second: DownloadId,
    },
    #[error("an outbox effect has a nil identity")]
    NilEffectId,
    #[error("an outbox effect targets unknown download {0}")]
    EffectTargetsUnknownDownload(DownloadId),
    #[error("a pending Workspace or Saved Folder change is invalid")]
    InvalidPendingStructure,
}
