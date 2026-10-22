use super::domain::*;
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
    fn none() -> Self {
        Self {
            events: Vec::new(),
            effects: Vec::new(),
            created_downloads: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Event {
    WorkspaceCreated(Workspace),
    WorkspaceRenamed {
        workspace: WorkspaceId,
        name: String,
        icon: WorkspaceIcon,
        organization: FileOrganization,
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

pub(crate) fn decide(state: &AppState, envelope: &CommandEnvelope) -> Result<Decision, Rejection> {
    if let Some(receipt) = state.receipts.get(&envelope.id) {
        return if receipt.fingerprint == envelope.fingerprint {
            Ok(Decision::none())
        } else {
            Err(Rejection::CommandIdReused)
        };
    }
    if let Some(expected) = envelope.expected_revision
        && expected != state.revision
    {
        return Err(Rejection::StaleRevision {
            expected,
            current: state.revision,
        });
    }
    if !matches!(
        envelope.command,
        Command::AcknowledgeRecovery
            | Command::MarkReconciled
            | Command::BeginShutdown { .. }
            | Command::CompleteShutdown
    ) && !matches!(state.session, SessionPhase::Running)
    {
        return Err(Rejection::SessionUnavailable(state.session.clone()));
    }

    match &envelope.command {
        Command::CreateWorkspace { workspace } => {
            validate_new_workspace(state, workspace)?;
            Ok(Decision {
                events: vec![Event::WorkspaceCreated(workspace.clone())],
                effects: Vec::new(),
                created_downloads: Vec::new(),
            })
        }
        Command::RenameWorkspace {
            workspace,
            name,
            icon,
            organization,
        } => {
            if state.workspace(*workspace).is_none() {
                return Err(Rejection::UnknownWorkspace(*workspace));
            }
            if name.trim().is_empty() {
                return Err(Rejection::InvalidInput(
                    "a workspace name cannot be empty".to_string(),
                ));
            }
            Ok(simple(Event::WorkspaceRenamed {
                workspace: *workspace,
                name: name.trim().to_string(),
                icon: icon.clone(),
                organization: *organization,
            }))
        }
        Command::DeleteWorkspace {
            workspace,
            replacement,
            move_files_to_trash,
        } => decide_delete_workspace(state, *workspace, replacement.clone(), *move_files_to_trash),
        Command::AddFolder {
            workspace,
            folder,
            make_default,
        } => {
            let existing = state
                .workspace(*workspace)
                .ok_or(Rejection::UnknownWorkspace(*workspace))?;
            if state
                .workspaces
                .iter()
                .flat_map(|item| &item.folders)
                .any(|item| item.id == folder.id)
            {
                return Err(Rejection::DuplicateFolder(folder.id));
            }
            if folder.name.trim().is_empty() || folder.path.trim().is_empty() {
                return Err(Rejection::InvalidInput(
                    "a saved folder needs a name and path".to_string(),
                ));
            }
            if existing.folders.iter().any(|item| item.path == folder.path) {
                return Err(Rejection::InvalidInput(
                    "that path is already saved in this workspace".to_string(),
                ));
            }
            Ok(simple(Event::FolderAdded {
                workspace: *workspace,
                folder: folder.clone(),
                make_default: *make_default,
            }))
        }
        Command::SetDefaultFolder { workspace, folder } => {
            require_folder(state, *workspace, *folder)?;
            Ok(simple(Event::DefaultFolderChanged {
                workspace: *workspace,
                folder: *folder,
            }))
        }
        Command::RelinkFolder {
            workspace,
            folder,
            name,
            path,
        } => {
            require_folder(state, *workspace, *folder)?;
            if name.trim().is_empty() || path.trim().is_empty() {
                return Err(Rejection::InvalidInput(
                    "a saved folder needs a name and path".to_string(),
                ));
            }
            Ok(simple(Event::FolderRelinked {
                workspace: *workspace,
                folder: *folder,
                name: name.trim().to_string(),
                path: path.to_string(),
            }))
        }
        Command::RemoveFolder {
            workspace,
            folder,
            move_files_to_trash,
        } => {
            let existing = state
                .workspace(*workspace)
                .ok_or(Rejection::UnknownWorkspace(*workspace))?;
            require_folder(state, *workspace, *folder)?;
            if existing.folders.len() == 1 {
                return Err(Rejection::CannotRemoveOnlyFolder);
            }
            if state
                .structural_changes
                .folder_removals
                .iter()
                .any(|pending| pending.workspace == *workspace && pending.folder == *folder)
            {
                return Ok(Decision::none());
            }
            let downloads = existing
                .downloads
                .iter()
                .filter(|download| download.folder == *folder && !download.lifecycle.is_retired())
                .map(|download| download.id)
                .collect::<Vec<_>>();
            if downloads.is_empty() {
                Ok(simple(Event::FolderRemoved {
                    workspace: *workspace,
                    folder: *folder,
                }))
            } else {
                let mut decision = decide_remove(state, &downloads, *move_files_to_trash)?;
                decision.events.insert(
                    0,
                    Event::FolderRemovalRequested(PendingFolderRemoval {
                        workspace: *workspace,
                        folder: *folder,
                    }),
                );
                Ok(decision)
            }
        }
        Command::SetWorkspaceView { workspace, view } => {
            if state.workspace(*workspace).is_none() {
                return Err(Rejection::UnknownWorkspace(*workspace));
            }
            Ok(simple(Event::WorkspaceViewChanged {
                workspace: *workspace,
                view: view.clone(),
            }))
        }
        Command::SetDefaultFormat { workspace, format } => {
            if state.workspace(*workspace).is_none() {
                return Err(Rejection::UnknownWorkspace(*workspace));
            }
            Ok(simple(Event::DefaultFormatChanged {
                workspace: *workspace,
                format: *format,
            }))
        }
        Command::AddDownload {
            workspace,
            download,
        } => decide_add_download(state, *workspace, download.as_ref().clone()),
        Command::AddMediaItems {
            workspace,
            downloads,
        } => decide_add_media_items(state, *workspace, downloads),
        Command::Pause { download } => decide_pause(state, *download, HoldCause::Person),
        Command::Resume { download } => decide_resume(state, *download),
        Command::Retry { download } => decide_retry(state, *download),
        Command::Cancel { download } => decide_cancel(state, *download),
        Command::StopSeeding { download } => decide_stop_seeding(state, *download),
        Command::Remove {
            downloads,
            move_files_to_trash,
        } => decide_remove(state, downloads, *move_files_to_trash),
        Command::SetSeedRatio { download, limit } => {
            let (_, item) = state
                .locate_download(*download)
                .ok_or(Rejection::UnknownDownload(*download))?;
            if !matches!(item.source, DownloadSource::Torrent(_)) {
                return Err(Rejection::WrongSource);
            }
            if limit.is_some_and(|value| !value.is_finite() || value <= 0.0) {
                return Err(Rejection::InvalidInput(
                    "the seed ratio must be a positive finite number".to_string(),
                ));
            }
            let generation = item.control_generation + 1;
            Ok(Decision {
                events: vec![Event::SeedRatioChanged {
                    download: *download,
                    limit: *limit,
                    control_generation: generation,
                }],
                effects: vec![control_effect(item, generation)],
                created_downloads: Vec::new(),
            })
        }
        Command::SetTorrentFiles { download, files } => {
            let (_, item) = state
                .locate_download(*download)
                .ok_or(Rejection::UnknownDownload(*download))?;
            if !matches!(item.source, DownloadSource::Torrent(_)) {
                return Err(Rejection::WrongSource);
            }
            if matches!(
                item.lifecycle,
                DownloadLifecycle::Finished
                    | DownloadLifecycle::Failed { .. }
                    | DownloadLifecycle::Cancelled
                    | DownloadLifecycle::Retiring { .. }
            ) {
                return Err(invalid("change file selection", &item.lifecycle));
            }
            let generation = item.control_generation + 1;
            Ok(Decision {
                events: vec![Event::TorrentFilesChanged {
                    download: *download,
                    files: files.clone(),
                    control_generation: generation,
                }],
                effects: vec![control_effect(item, generation)],
                created_downloads: Vec::new(),
            })
        }
        Command::MoveDownload {
            download,
            folder,
            output_path,
            content_folder,
            identity,
        } => decide_move_download(
            state,
            *download,
            *folder,
            output_path,
            content_folder.as_deref(),
            identity.clone(),
        ),
        Command::AcknowledgeRecovery => {
            if !matches!(state.session, SessionPhase::RecoveryRequired) {
                return Err(Rejection::InvalidInput(
                    "there is no fenced recovery to acknowledge".to_string(),
                ));
            }
            Ok(Decision {
                events: vec![
                    Event::SessionChanged(SessionPhase::Reconciling),
                    Event::RecoveryAcknowledged,
                ],
                effects: Vec::new(),
                created_downloads: Vec::new(),
            })
        }
        Command::MarkReconciled => decide_reconciled(state),
        Command::BeginShutdown { boot } => decide_shutdown(state, *boot),
        Command::CompleteShutdown => {
            if state
                .workspaces
                .iter()
                .flat_map(|workspace| &workspace.downloads)
                .any(|download| {
                    matches!(
                        download.lifecycle,
                        DownloadLifecycle::Pausing {
                            cause: HoldCause::Shutdown,
                            ..
                        }
                    )
                })
            {
                return Err(Rejection::InvalidInput(
                    "shutdown cannot complete while a transfer is still pausing".to_string(),
                ));
            }
            Ok(simple(Event::SessionChanged(SessionPhase::Stopped)))
        }
    }
}

pub(crate) fn decide_fact(state: &AppState, fact: &Fact) -> Result<Decision, Rejection> {
    match fact {
        Fact::TransferStarted {
            download,
            attempt,
            control_generation,
            phase,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.control_generation != *control_generation {
                return Ok(Decision::none());
            }
            let lifecycle = match &item.lifecycle {
                DownloadLifecycle::Queued | DownloadLifecycle::Starting => {
                    DownloadLifecycle::Active(*phase)
                }
                DownloadLifecycle::Pausing { from, cause } => DownloadLifecycle::Pausing {
                    from: *from,
                    cause: *cause,
                },
                _ => return Ok(Decision::none()),
            };
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle,
                control_generation: *control_generation,
            }))
        }
        Fact::TransferAdvanced {
            download,
            attempt,
            phase,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if !phase_allowed(item.source_kind(), *phase) {
                return Ok(Decision::none());
            }
            if matches!(item.lifecycle, DownloadLifecycle::Active(current) if current == *phase) {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Active(*phase),
                control_generation: item.control_generation,
            }))
        }
        Fact::TransferPaused {
            download,
            attempt,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.control_generation != *control_generation {
                return Ok(Decision::none());
            }
            let DownloadLifecycle::Pausing { from, cause } = item.lifecycle else {
                return Ok(Decision::none());
            };
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Paused {
                    resume: from,
                    cause,
                },
                control_generation: *control_generation,
            }))
        }
        Fact::TransferFinished {
            download,
            attempt,
            output,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if matches!(item.lifecycle, DownloadLifecycle::Finished)
                && item.output.as_ref() == Some(output)
            {
                return Ok(Decision::none());
            }
            Ok(Decision {
                events: vec![
                    Event::DownloadOutputRecorded {
                        download: *download,
                        output: output.clone(),
                    },
                    Event::DownloadLifecycleChanged {
                        download: *download,
                        lifecycle: DownloadLifecycle::Finished,
                        control_generation: item.control_generation,
                    },
                ],
                effects: Vec::new(),
                created_downloads: Vec::new(),
            })
        }
        Fact::TransferFailed {
            download,
            attempt,
            message,
            retryable,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if matches!(
                &item.lifecycle,
                DownloadLifecycle::Failed {
                    message: current,
                    retryable: current_retryable,
                } if current == message && current_retryable == retryable
            ) {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Failed {
                    message: message.clone(),
                    retryable: *retryable,
                },
                control_generation: item.control_generation,
            }))
        }
        Fact::TorrentResolved {
            download,
            attempt,
            info_hash,
            metadata,
            files,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || !matches!(item.source, DownloadSource::Torrent(_)) {
                return Ok(Decision::none());
            }
            let normalized = info_hash.to_ascii_lowercase();
            if let DownloadSource::Torrent(torrent) = &item.source
                && torrent.info_hash.as_deref() == Some(normalized.as_str())
                && torrent.metadata.as_ref() == Some(metadata)
                && torrent.files == *files
            {
                return Ok(Decision::none());
            }
            if let Some((_, existing)) = state.workspaces.iter().find_map(|workspace| {
                workspace.downloads.iter().find_map(|candidate| {
                    (candidate.id != *download
                        && candidate
                            .active_info_hash()
                            .is_some_and(|hash| hash.eq_ignore_ascii_case(&normalized)))
                    .then_some((workspace.id, candidate))
                })
            }) {
                return Ok(simple(Event::DownloadLifecycleChanged {
                    download: *download,
                    lifecycle: DownloadLifecycle::Failed {
                        message: format!(
                            "This torrent already exists as download {}.",
                            existing.id
                        ),
                        retryable: false,
                    },
                    control_generation: item.control_generation,
                }));
            }
            Ok(simple(Event::TorrentResolved {
                download: *download,
                info_hash: normalized,
                metadata: metadata.clone(),
                files: files.clone(),
            }))
        }
        Fact::LeftoversFound {
            download,
            attempt,
            leftovers,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            let DownloadLifecycle::Retiring {
                move_files_to_trash: true,
                writer_stopped: false,
                leftovers: current,
                ..
            } = &item.lifecycle
            else {
                return Ok(Decision::none());
            };
            if item.attempt != *attempt || current == leftovers {
                return Ok(Decision::none());
            }
            Ok(simple(Event::RetirementLeftoversRecorded {
                download: *download,
                leftovers: leftovers.clone(),
            }))
        }
        Fact::WriterStopped {
            download,
            attempt,
            control_generation,
        } => retirement_progress(
            state,
            *download,
            *attempt,
            Some(*control_generation),
            Some(true),
            None,
            None,
        ),
        Fact::AdapterRecordAbsent { download, attempt } => {
            retirement_progress(state, *download, *attempt, None, None, Some(true), None)
        }
        Fact::FilesResolved {
            download,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            retirement_progress(
                state,
                *download,
                item.attempt,
                Some(*control_generation),
                None,
                None,
                Some(true),
            )
        }
        Fact::RetirementFinalized {
            download,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.control_generation != *control_generation
                || !matches!(
                    item.lifecycle,
                    DownloadLifecycle::Retiring {
                        writer_stopped: true,
                        adapter_absent: true,
                        files_resolved: true,
                        ..
                    }
                )
            {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadRetired {
                download: *download,
            }))
        }
    }
}

pub(crate) fn evolve(
    state: &AppState,
    decision: &Decision,
) -> Result<AppState, InvariantViolation> {
    let mut next = state.clone();
    for event in &decision.events {
        apply(&mut next, event);
    }
    next.revision = next.revision.saturating_add(1);
    next.validate()?;
    Ok(next)
}

pub(crate) fn apply(state: &mut AppState, event: &Event) {
    match event {
        Event::WorkspaceCreated(workspace) => state.workspaces.push(workspace.clone()),
        Event::WorkspaceRenamed {
            workspace,
            name,
            icon,
            organization,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.name = name.clone();
                item.icon = icon.clone();
                item.file_organization = *organization;
            }
        }
        Event::WorkspaceDeleted {
            workspace,
            replacement,
        } => {
            state.workspaces.retain(|item| item.id != *workspace);
            state
                .structural_changes
                .workspace_deletions
                .retain(|pending| pending.workspace != *workspace);
            state
                .structural_changes
                .folder_removals
                .retain(|pending| pending.workspace != *workspace);
            if let Some(replacement) = replacement {
                state.workspaces.push(replacement.clone());
            }
        }
        Event::FolderAdded {
            workspace,
            folder,
            make_default,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.folders.push(folder.clone());
                if *make_default {
                    item.default_folder = folder.id;
                }
            }
        }
        Event::DefaultFolderChanged { workspace, folder } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.default_folder = *folder;
            }
        }
        Event::FolderRelinked {
            workspace,
            folder,
            name,
            path,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                let old_path = item
                    .folders
                    .iter()
                    .find(|saved| saved.id == *folder)
                    .map(|saved| saved.path.clone());
                if let Some(old_path) = old_path {
                    for download in &mut item.downloads {
                        if download.folder != *folder {
                            continue;
                        }
                        if let Some(output) = &mut download.output {
                            if let Some(rebased) = rebase_path(&output.path, &old_path, path) {
                                output.path = rebased;
                            }
                            if let Some(content) = &output.content_folder
                                && let Some(rebased) = rebase_path(content, &old_path, path)
                            {
                                output.content_folder = Some(rebased);
                            }
                            // Moving the artifact invalidates the previous filesystem identity.
                            output.identity = None;
                        }
                    }
                    if let Some(saved) = item.folders.iter_mut().find(|saved| saved.id == *folder) {
                        saved.name = name.clone();
                        saved.path = path.clone();
                    }
                }
            }
        }
        Event::FolderRemoved { workspace, folder } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.folders.retain(|saved| saved.id != *folder);
                if item.default_folder == *folder {
                    item.default_folder = item.folders[0].id;
                }
            }
            state
                .structural_changes
                .folder_removals
                .retain(|pending| pending.workspace != *workspace || pending.folder != *folder);
        }
        Event::WorkspaceDeletionRequested(pending) => {
            state
                .structural_changes
                .workspace_deletions
                .push(pending.clone());
        }
        Event::FolderRemovalRequested(pending) => {
            state.structural_changes.folder_removals.push(*pending);
        }
        Event::WorkspaceViewChanged { workspace, view } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.view = view.clone();
            }
        }
        Event::DefaultFormatChanged { workspace, format } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.default_format = *format;
            }
        }
        Event::DownloadAdded {
            workspace,
            download,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.downloads.push(download.as_ref().clone());
            }
        }
        Event::DownloadLifecycleChanged {
            download,
            lifecycle,
            control_generation,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.lifecycle = lifecycle.clone();
                item.control_generation = *control_generation;
            }
        }
        Event::DownloadRetried {
            download,
            attempt,
            control_generation,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.attempt = *attempt;
                item.control_generation = *control_generation;
                item.lifecycle = DownloadLifecycle::Queued;
                item.output = None;
            }
        }
        Event::DownloadsBeganRetiring {
            downloads,
            move_files_to_trash,
        } => {
            for id in downloads {
                if let Some((_, item)) = state.locate_download_mut(*id) {
                    let previous =
                        std::mem::replace(&mut item.lifecycle, DownloadLifecycle::Cancelled);
                    item.control_generation += 1;
                    item.lifecycle = DownloadLifecycle::Retiring {
                        previous: Box::new(previous),
                        move_files_to_trash: *move_files_to_trash,
                        writer_stopped: false,
                        adapter_absent: false,
                        files_resolved: !*move_files_to_trash || item.verified_output().is_none(),
                        leftovers: Vec::new(),
                    };
                }
            }
        }
        Event::RetirementLeftoversRecorded {
            download,
            leftovers: recorded,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download)
                && let DownloadLifecycle::Retiring { leftovers, .. } = &mut item.lifecycle
            {
                *leftovers = recorded.clone();
                let resolved = !item.has_trash_targets();
                if let DownloadLifecycle::Retiring { files_resolved, .. } = &mut item.lifecycle {
                    *files_resolved = resolved;
                }
            }
        }
        Event::RetirementProgressed {
            download,
            writer_stopped,
            adapter_absent,
            files_resolved,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download)
                && let DownloadLifecycle::Retiring {
                    writer_stopped: stopped,
                    adapter_absent: absent,
                    files_resolved: resolved,
                    ..
                } = &mut item.lifecycle
            {
                if let Some(value) = writer_stopped {
                    *stopped = *value;
                }
                if let Some(value) = adapter_absent {
                    *absent = *value;
                }
                if let Some(value) = files_resolved {
                    *resolved = *value;
                }
            }
        }
        Event::DownloadRetired { download } => {
            for workspace in &mut state.workspaces {
                workspace.downloads.retain(|item| item.id != *download);
            }
            finalize_structural_changes(state);
        }
        Event::DownloadMoved {
            download,
            folder,
            output_path,
            content_folder,
            identity,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.folder = *folder;
                if let Some(output) = &mut item.output {
                    output.path = output_path.clone();
                    output.content_folder = content_folder.clone();
                    // The previous identity described the old location; keep only what the
                    // shell measured at the new one.
                    output.identity = identity.clone();
                }
            }
        }
        Event::SeedRatioChanged {
            download,
            limit,
            control_generation,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.control_generation = *control_generation;
                if let DownloadSource::Torrent(torrent) = &mut item.source {
                    torrent.seed_ratio_limit = *limit;
                }
            }
        }
        Event::TorrentFilesChanged {
            download,
            files,
            control_generation,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.control_generation = *control_generation;
                if let DownloadSource::Torrent(torrent) = &mut item.source {
                    torrent.files = files.clone();
                }
            }
        }
        Event::TorrentResolved {
            download,
            info_hash,
            metadata,
            files,
        } => {
            if let Some((_, item)) = state.locate_download_mut(*download)
                && let DownloadSource::Torrent(torrent) = &mut item.source
            {
                torrent.info_hash = Some(info_hash.clone());
                torrent.metadata = Some(metadata.clone());
                torrent.files = files.clone();
            }
        }
        Event::DownloadOutputRecorded { download, output } => {
            if let Some((_, item)) = state.locate_download_mut(*download) {
                item.output = Some(output.clone());
            }
        }
        Event::SessionChanged(session) => state.session = session.clone(),
        Event::RecoveryAcknowledged => state.recovery.clear(),
    }
}

fn rebase_path(value: &str, old_base: &str, new_base: &str) -> Option<String> {
    let relative = std::path::Path::new(value)
        .strip_prefix(std::path::Path::new(old_base))
        .ok()?;
    Some(
        std::path::Path::new(new_base)
            .join(relative)
            .to_string_lossy()
            .into_owned(),
    )
}

fn finalize_structural_changes(state: &mut AppState) {
    let ready_folders = state
        .structural_changes
        .folder_removals
        .iter()
        .copied()
        .filter(|pending| {
            state.workspace(pending.workspace).is_some_and(|workspace| {
                !workspace
                    .downloads
                    .iter()
                    .any(|download| download.folder == pending.folder)
            })
        })
        .collect::<Vec<_>>();
    for pending in &ready_folders {
        if let Some(workspace) = state.workspace_mut(pending.workspace) {
            workspace
                .folders
                .retain(|folder| folder.id != pending.folder);
            if workspace.default_folder == pending.folder
                && let Some(first) = workspace.folders.first()
            {
                workspace.default_folder = first.id;
            }
        }
    }
    state
        .structural_changes
        .folder_removals
        .retain(|pending| !ready_folders.contains(pending));

    let ready_workspaces = state
        .structural_changes
        .workspace_deletions
        .iter()
        .filter(|pending| {
            state
                .workspace(pending.workspace)
                .is_some_and(|workspace| workspace.downloads.is_empty())
        })
        .cloned()
        .collect::<Vec<_>>();
    for pending in &ready_workspaces {
        state
            .workspaces
            .retain(|workspace| workspace.id != pending.workspace);
        state
            .structural_changes
            .folder_removals
            .retain(|folder| folder.workspace != pending.workspace);
        if let Some(replacement) = &pending.replacement {
            state.workspaces.push(replacement.clone());
        }
    }
    state
        .structural_changes
        .workspace_deletions
        .retain(|pending| !ready_workspaces.contains(pending));
}

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

fn simple(event: Event) -> Decision {
    Decision {
        events: vec![event],
        effects: Vec::new(),
        created_downloads: Vec::new(),
    }
}

fn validate_new_workspace(state: &AppState, workspace: &Workspace) -> Result<(), Rejection> {
    if state.workspace(workspace.id).is_some() {
        return Err(Rejection::DuplicateWorkspace(workspace.id));
    }
    if workspace.name.trim().is_empty()
        || workspace.folders.is_empty()
        || !workspace
            .folders
            .iter()
            .any(|folder| folder.id == workspace.default_folder)
    {
        return Err(Rejection::InvalidInput(
            "a workspace needs a name, at least one folder, and a valid default".to_string(),
        ));
    }
    if !workspace.downloads.is_empty() {
        return Err(Rejection::InvalidInput(
            "a new workspace cannot arrive with downloads".to_string(),
        ));
    }
    let folder_ids = workspace
        .folders
        .iter()
        .map(|folder| folder.id)
        .collect::<std::collections::BTreeSet<_>>();
    if folder_ids.len() != workspace.folders.len()
        || workspace
            .folders
            .iter()
            .any(|folder| folder.name.trim().is_empty() || folder.path.trim().is_empty())
        || state
            .workspaces
            .iter()
            .flat_map(|existing| &existing.folders)
            .any(|existing| folder_ids.contains(&existing.id))
    {
        return Err(Rejection::InvalidInput(
            "a new workspace has an invalid or already-used folder identity".to_string(),
        ));
    }
    Ok(())
}

fn decide_delete_workspace(
    state: &AppState,
    workspace: WorkspaceId,
    replacement: Option<Workspace>,
    move_files_to_trash: bool,
) -> Result<Decision, Rejection> {
    let existing = state
        .workspace(workspace)
        .ok_or(Rejection::UnknownWorkspace(workspace))?;
    if state
        .structural_changes
        .workspace_deletions
        .iter()
        .any(|pending| pending.workspace == workspace)
    {
        return Ok(Decision::none());
    }
    if state.workspaces.len() == 1 {
        let Some(replacement) = replacement.as_ref() else {
            return Err(Rejection::ReplacementWorkspaceRequired);
        };
        validate_new_workspace(state, replacement)?;
    } else if replacement.is_some() {
        return Err(Rejection::InvalidInput(
            "only deletion of the last workspace accepts a replacement".to_string(),
        ));
    }
    let downloads = existing
        .downloads
        .iter()
        .filter(|download| !download.lifecycle.is_retired())
        .map(|download| download.id)
        .collect::<Vec<_>>();
    if downloads.is_empty() {
        return Ok(simple(Event::WorkspaceDeleted {
            workspace,
            replacement,
        }));
    }
    let mut decision = decide_remove(state, &downloads, move_files_to_trash)?;
    decision.events.insert(
        0,
        Event::WorkspaceDeletionRequested(PendingWorkspaceDeletion {
            workspace,
            replacement,
        }),
    );
    Ok(decision)
}

fn require_folder(
    state: &AppState,
    workspace: WorkspaceId,
    folder: FolderId,
) -> Result<(), Rejection> {
    let workspace_item = state
        .workspace(workspace)
        .ok_or(Rejection::UnknownWorkspace(workspace))?;
    if !workspace_item.folders.iter().any(|item| item.id == folder) {
        return Err(Rejection::FolderNotInWorkspace { workspace, folder });
    }
    Ok(())
}

fn decide_add_download(
    state: &AppState,
    workspace: WorkspaceId,
    download: Download,
) -> Result<Decision, Rejection> {
    require_folder(state, workspace, download.folder)?;
    if state.locate_download(download.id).is_some() {
        return Err(Rejection::DuplicateDownload(download.id));
    }
    if !matches!(download.lifecycle, DownloadLifecycle::Queued)
        || download.attempt != 0
        || download.control_generation != 0
    {
        return Err(Rejection::InvalidInput(
            "a new download must begin queued at attempt and generation zero".to_string(),
        ));
    }
    if let Some(hash) = download.active_info_hash()
        && let Some((_, existing)) = state.workspaces.iter().find_map(|workspace| {
            workspace.downloads.iter().find_map(|candidate| {
                candidate
                    .active_info_hash()
                    .is_some_and(|candidate_hash| candidate_hash.eq_ignore_ascii_case(hash))
                    .then_some((workspace.id, candidate))
            })
        })
    {
        return Err(Rejection::DuplicateTorrent(existing.id));
    }
    let id = download.id;
    Ok(Decision {
        events: vec![Event::DownloadAdded {
            workspace,
            download: Box::new(download.clone()),
        }],
        effects: vec![EffectPlan {
            attempt: Some(download.attempt),
            control_generation: Some(download.control_generation),
            kind: EffectKind::EnsureTransfer { download: id },
        }],
        created_downloads: vec![id],
    })
}

fn decide_add_media_items(
    state: &AppState,
    workspace: WorkspaceId,
    downloads: &[Download],
) -> Result<Decision, Rejection> {
    let existing = state
        .workspace(workspace)
        .ok_or(Rejection::UnknownWorkspace(workspace))?;
    let mut keys = existing
        .downloads
        .iter()
        .map(|download| download.dedupe_key().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let mut ids = state
        .workspaces
        .iter()
        .flat_map(|workspace| workspace.downloads.iter().map(|download| download.id))
        .collect::<std::collections::BTreeSet<_>>();
    let mut decision = Decision::none();
    for download in downloads {
        if !matches!(download.source, DownloadSource::Media(_)) {
            return Err(Rejection::WrongSource);
        }
        require_folder(state, workspace, download.folder)?;
        if !matches!(download.lifecycle, DownloadLifecycle::Queued)
            || download.attempt != 0
            || download.control_generation != 0
        {
            return Err(Rejection::InvalidInput(
                "a new media download must begin queued at attempt and generation zero".to_string(),
            ));
        }
        if !ids.insert(download.id) {
            return Err(Rejection::DuplicateDownload(download.id));
        }
        // ADR 0006: keep playlist order and silently skip entries already in this Workspace.
        if !keys.insert(download.dedupe_key().to_string()) {
            continue;
        }
        decision.events.push(Event::DownloadAdded {
            workspace,
            download: Box::new(download.clone()),
        });
        decision.effects.push(EffectPlan {
            attempt: Some(0),
            control_generation: Some(0),
            kind: EffectKind::EnsureTransfer {
                download: download.id,
            },
        });
        decision.created_downloads.push(download.id);
    }
    Ok(decision)
}

fn decide_pause(state: &AppState, id: DownloadId, cause: HoldCause) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if matches!(
        item.lifecycle,
        DownloadLifecycle::Paused {
            cause: existing,
            ..
        } if existing == cause
    ) {
        return Ok(Decision::none());
    }
    let phase = match item.lifecycle {
        DownloadLifecycle::Queued | DownloadLifecycle::Starting => ActivePhase::Fetching,
        DownloadLifecycle::Active(phase) => phase,
        DownloadLifecycle::Pausing {
            cause: existing, ..
        } if existing == cause => {
            return Ok(Decision::none());
        }
        _ => return Err(invalid("pause", &item.lifecycle)),
    };
    let generation = item.control_generation + 1;
    Ok(Decision {
        events: vec![Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Pausing { from: phase, cause },
            control_generation: generation,
        }],
        effects: vec![control_effect(item, generation)],
        created_downloads: Vec::new(),
    })
}

fn decide_resume(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    let DownloadLifecycle::Paused { .. } = item.lifecycle else {
        return Err(invalid("resume", &item.lifecycle));
    };
    let generation = item.control_generation + 1;
    Ok(Decision {
        events: vec![Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Starting,
            control_generation: generation,
        }],
        effects: vec![EffectPlan {
            attempt: Some(item.attempt),
            control_generation: Some(generation),
            kind: EffectKind::EnsureTransfer { download: id },
        }],
        created_downloads: Vec::new(),
    })
}

fn decide_retry(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if !matches!(
        item.lifecycle,
        DownloadLifecycle::Failed {
            retryable: true,
            ..
        }
    ) {
        return Err(invalid("retry", &item.lifecycle));
    }
    let attempt = item.attempt + 1;
    let generation = item.control_generation + 1;
    Ok(Decision {
        events: vec![Event::DownloadRetried {
            download: id,
            attempt,
            control_generation: generation,
        }],
        effects: vec![EffectPlan {
            attempt: Some(attempt),
            control_generation: Some(generation),
            kind: EffectKind::EnsureTransfer { download: id },
        }],
        created_downloads: Vec::new(),
    })
}

fn decide_cancel(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if matches!(item.lifecycle, DownloadLifecycle::Cancelled) {
        return Ok(Decision::none());
    }
    if matches!(
        item.lifecycle,
        DownloadLifecycle::Finished | DownloadLifecycle::Retiring { .. }
    ) {
        return Err(invalid("cancel", &item.lifecycle));
    }
    let generation = item.control_generation + 1;
    Ok(Decision {
        events: vec![Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Cancelled,
            control_generation: generation,
        }],
        effects: vec![control_effect(item, generation)],
        created_downloads: Vec::new(),
    })
}

fn decide_stop_seeding(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if !matches!(item.source, DownloadSource::Torrent(_)) {
        return Err(Rejection::WrongSource);
    }
    if !matches!(
        item.lifecycle,
        DownloadLifecycle::Active(ActivePhase::Seeding)
    ) {
        return Err(invalid("stop seeding", &item.lifecycle));
    }
    let generation = item.control_generation + 1;
    Ok(Decision {
        events: vec![Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Finished,
            control_generation: generation,
        }],
        effects: vec![control_effect(item, generation)],
        created_downloads: Vec::new(),
    })
}

fn decide_remove(
    state: &AppState,
    ids: &[DownloadId],
    move_files_to_trash: bool,
) -> Result<Decision, Rejection> {
    let mut effects = Vec::new();
    let ids = ids
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if ids.is_empty() {
        return Ok(Decision::none());
    }
    for id in &ids {
        let (_, item) = state
            .locate_download(*id)
            .ok_or(Rejection::UnknownDownload(*id))?;
        if item.lifecycle.is_retired() {
            continue;
        }
        let generation = item.control_generation + 1;
        effects.push(EffectPlan {
            attempt: Some(item.attempt),
            control_generation: Some(generation),
            kind: EffectKind::RemoveTransfer { download: *id },
        });
    }
    Ok(Decision {
        events: vec![Event::DownloadsBeganRetiring {
            downloads: ids.into_iter().collect(),
            move_files_to_trash,
        }],
        effects,
        created_downloads: Vec::new(),
    })
}

fn decide_reconciled(state: &AppState) -> Result<Decision, Rejection> {
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
                | DownloadLifecycle::Finished
                | DownloadLifecycle::Failed { .. }
                | DownloadLifecycle::Cancelled => {}
            }
        }
    }
    Ok(decision)
}

fn decide_shutdown(state: &AppState, boot: BootId) -> Result<Decision, Rejection> {
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

fn retirement_progress(
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

fn trash_effect(item: &Download) -> EffectKind {
    EffectKind::MoveOutputToTrash {
        download: item.id,
        artifact: item.verified_output().cloned(),
        leftovers: item.uncovered_leftovers(),
    }
}

fn control_effect(item: &Download, generation: u64) -> EffectPlan {
    EffectPlan {
        attempt: Some(item.attempt),
        control_generation: Some(generation),
        kind: EffectKind::ConvergeControl { download: item.id },
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

/// The shell moves the files first; this records where they now live. Only a finished
/// download qualifies, because no writer or engine still holds its paths.
fn decide_move_download(
    state: &AppState,
    id: DownloadId,
    folder: FolderId,
    output_path: &str,
    content_folder: Option<&str>,
    identity: Option<OutputIdentity>,
) -> Result<Decision, Rejection> {
    let (workspace, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    require_folder(state, workspace, folder)?;
    if !matches!(item.lifecycle, DownloadLifecycle::Finished) || item.output.is_none() {
        return Err(invalid("move", &item.lifecycle));
    }
    if item.folder == folder {
        return Ok(Decision::none());
    }
    let target = state
        .workspace(workspace)
        .and_then(|workspace| workspace.folders.iter().find(|saved| saved.id == folder))
        .map(|saved| std::path::Path::new(&saved.path))
        .ok_or(Rejection::FolderNotInWorkspace { workspace, folder })?;
    let inside_target = |path: &str| {
        let path = std::path::Path::new(path);
        path != target && path.starts_with(target)
    };
    if !inside_target(output_path) || content_folder.is_some_and(|content| !inside_target(content))
    {
        return Err(Rejection::InvalidInput(
            "a moved download must live inside its new saved folder".to_string(),
        ));
    }
    Ok(simple(Event::DownloadMoved {
        download: id,
        folder,
        output_path: output_path.to_string(),
        content_folder: content_folder.map(str::to_string),
        identity,
    }))
}

fn invalid(action: &'static str, lifecycle: &DownloadLifecycle) -> Rejection {
    Rejection::InvalidTransition {
        action,
        lifecycle: lifecycle_name(lifecycle),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn running_state() -> AppState {
        let mut state = AppState::fresh("/tmp/Downloads".to_string());
        state.session = SessionPhase::Running;
        state
    }

    fn media_download(state: &AppState) -> Download {
        Download {
            id: DownloadId::new(),
            folder: state.workspaces[0].default_folder,
            name: "Film".to_string(),
            attempt: 0,
            control_generation: 0,
            source: DownloadSource::Media(MediaDownload {
                origin: MediaOrigin::Page {
                    page_url: "https://example.com/watch".to_string(),
                    item_url: "https://example.com/watch?v=1".to_string(),
                    playlist_index: None,
                },
                format: MediaFormat::Best,
                policy: MediaPolicy::default(),
                legacy_binding: None,
                thumbnail_url: None,
                extractor: None,
            }),
            lifecycle: DownloadLifecycle::Queued,
            output: None,
        }
    }

    fn command(command: Command) -> CommandEnvelope {
        CommandEnvelope {
            id: CommandId::new(),
            fingerprint: Uuid::new_v4().to_string(),
            expected_revision: None,
            command,
        }
    }

    fn run(state: &AppState, command: Command) -> AppState {
        let decision = decide(state, &self::command(command)).unwrap();
        evolve(state, &decision).unwrap()
    }

    #[test]
    fn new_download_is_committed_with_a_stable_effect_identity_target() {
        let state = running_state();
        let download = media_download(&state);
        let decision = decide(
            &state,
            &command(Command::AddDownload {
                workspace: state.workspaces[0].id,
                download: Box::new(download.clone()),
            }),
        )
        .unwrap();
        assert_eq!(decision.created_downloads, vec![download.id]);
        assert_eq!(
            decision.effects[0].kind,
            EffectKind::EnsureTransfer {
                download: download.id
            }
        );
        let next = evolve(&state, &decision).unwrap();
        assert_eq!(next.locate_download(download.id).unwrap().1.id, download.id);
    }

    #[test]
    fn pause_is_pending_until_the_matching_generation_acknowledges() {
        let state = running_state();
        let download = media_download(&state);
        let state = run(
            &state,
            Command::AddDownload {
                workspace: state.workspaces[0].id,
                download: Box::new(download.clone()),
            },
        );
        let state = evolve(
            &state,
            &decide(
                &state,
                &command(Command::Pause {
                    download: download.id,
                }),
            )
            .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            state.locate_download(download.id).unwrap().1.lifecycle,
            DownloadLifecycle::Pausing {
                cause: HoldCause::Person,
                ..
            }
        ));
        let stale = decide_fact(
            &state,
            &Fact::TransferPaused {
                download: download.id,
                attempt: 0,
                control_generation: 0,
            },
        )
        .unwrap();
        assert!(stale.events.is_empty());
        let matching = decide_fact(
            &state,
            &Fact::TransferPaused {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
        )
        .unwrap();
        let paused = evolve(&state, &matching).unwrap();
        assert!(matches!(
            paused.locate_download(download.id).unwrap().1.lifecycle,
            DownloadLifecycle::Paused {
                cause: HoldCause::Person,
                ..
            }
        ));
    }

    #[test]
    fn obsolete_pause_effect_is_ineligible_after_resume() {
        let state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        };
        download.control_generation = 4;
        let workspace = state.workspaces[0].id;
        let mut state = state;
        state.workspaces[0].downloads.push(download.clone());
        let decision = decide(
            &state,
            &command(Command::Resume {
                download: download.id,
            }),
        )
        .unwrap();
        let next = evolve(&state, &decision).unwrap();
        let stale = PendingEffect {
            id: EffectId::new(),
            created_revision: state.revision,
            attempt: Some(0),
            control_generation: Some(4),
            status: EffectStatus::Pending,
            kind: EffectKind::ConvergeControl {
                download: download.id,
            },
        };
        assert!(!effect_is_eligible(&next, &stale));
        assert_eq!(next.workspace(workspace).unwrap().downloads.len(), 1);
    }

    #[test]
    fn backup_recovery_cannot_be_unfenced_by_mark_reconciled() {
        let mut state = running_state();
        state.session = SessionPhase::RecoveryRequired;
        let result = decide(&state, &command(Command::MarkReconciled));
        assert!(matches!(result, Err(Rejection::InvalidInput(_))));
        let effect = PendingEffect {
            id: EffectId::new(),
            created_revision: state.revision,
            attempt: None,
            control_generation: None,
            status: EffectStatus::Pending,
            kind: EffectKind::StopAllTransfers,
        };
        assert!(!effect_is_eligible(&state, &effect));
        let acknowledged = run(&state, Command::AcknowledgeRecovery);
        assert_eq!(acknowledged.session, SessionPhase::Reconciling);
    }

    #[test]
    fn reconciliation_reissues_queued_and_shutdown_interrupted_intent() {
        let mut state = running_state();
        state.session = SessionPhase::Reconciling;
        let queued = media_download(&state);
        let mut interrupted = media_download(&state);
        interrupted.id = DownloadId::new();
        interrupted.control_generation = 3;
        interrupted.lifecycle = DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Shutdown,
        };
        state.workspaces[0].downloads = vec![queued.clone(), interrupted.clone()];

        let decision = decide(&state, &command(Command::MarkReconciled)).unwrap();
        assert!(
            decision
                .events
                .contains(&Event::SessionChanged(SessionPhase::Running))
        );
        assert!(decision.effects.iter().any(|effect| {
            effect.control_generation == Some(0)
                && effect.kind
                    == EffectKind::EnsureTransfer {
                        download: queued.id,
                    }
        }));
        assert!(decision.effects.iter().any(|effect| {
            effect.control_generation == Some(4)
                && effect.kind
                    == EffectKind::EnsureTransfer {
                        download: interrupted.id,
                    }
        }));
        let next = evolve(&state, &decision).unwrap();
        assert!(matches!(
            next.locate_download(interrupted.id).unwrap().1.lifecycle,
            DownloadLifecycle::Starting
        ));
    }

    #[test]
    fn duplicate_torrent_hash_is_rejected_globally() {
        let mut state = running_state();
        let folder = state.workspaces[0].default_folder;
        let first = Download {
            id: DownloadId::new(),
            folder,
            name: "One".to_string(),
            attempt: 0,
            control_generation: 0,
            source: DownloadSource::Torrent(TorrentDownload {
                origin: TorrentOrigin::Magnet {
                    uri: "magnet:?xt=urn:btih:abc".to_string(),
                },
                info_hash: Some("abc".to_string()),
                metadata: None,
                seed_ratio_limit: Some(1.0),
                files: Vec::new(),
            }),
            lifecycle: DownloadLifecycle::Queued,
            output: None,
        };
        state.workspaces[0].downloads.push(first.clone());
        let mut second = first.clone();
        second.id = DownloadId::new();
        let result = decide(
            &state,
            &command(Command::AddDownload {
                workspace: state.workspaces[0].id,
                download: Box::new(second),
            }),
        );
        assert_eq!(result.unwrap_err(), Rejection::DuplicateTorrent(first.id));
    }

    #[test]
    fn finished_download_moves_into_another_saved_folder() {
        let mut state = running_state();
        let workspace = state.workspaces[0].id;
        let target = SavedFolder {
            id: FolderId::new(),
            name: "Films".to_string(),
            path: "/tmp/Films".to_string(),
        };
        state = run(
            &state,
            Command::AddFolder {
                workspace,
                folder: target.clone(),
                make_default: false,
            },
        );
        let mut download = media_download(&state);
        download.output = Some(OutputArtifact {
            path: "/tmp/Downloads/film.mp4".to_string(),
            content_folder: None,
            identity: None,
        });
        state.workspaces[0].downloads.push(download.clone());

        let move_to = |output_path: &str| Command::MoveDownload {
            download: download.id,
            folder: target.id,
            output_path: output_path.to_string(),
            content_folder: None,
            identity: None,
        };
        assert!(matches!(
            decide(&state, &command(move_to("/tmp/Films/film.mp4"))),
            Err(Rejection::InvalidTransition { action: "move", .. })
        ));

        state.workspaces[0].downloads[0].lifecycle = DownloadLifecycle::Finished;
        assert!(matches!(
            decide(&state, &command(move_to("/tmp/Elsewhere/film.mp4"))),
            Err(Rejection::InvalidInput(_))
        ));

        let moved = run(&state, move_to("/tmp/Films/film.mp4"));
        let (_, item) = moved.locate_download(download.id).unwrap();
        assert_eq!(item.folder, target.id);
        assert_eq!(item.output.as_ref().unwrap().path, "/tmp/Films/film.mp4");

        let identity = OutputIdentity {
            byte_length: 42,
            modified_at_millis: 7,
            file_identifier: Some("inode-9".to_string()),
        };
        let measured = run(
            &state,
            Command::MoveDownload {
                download: download.id,
                folder: target.id,
                output_path: "/tmp/Films/film.mp4".to_string(),
                content_folder: None,
                identity: Some(identity.clone()),
            },
        );
        let (_, item) = measured.locate_download(download.id).unwrap();
        assert_eq!(item.verified_output().unwrap().identity, Some(identity));
    }

    #[test]
    fn retirement_keeps_evidence_until_every_dependency_acknowledges() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
        download.output = Some(OutputArtifact {
            path: "/tmp/film.mp4".to_string(),
            content_folder: None,
            identity: Some(OutputIdentity {
                byte_length: 42,
                modified_at_millis: 7,
                file_identifier: Some("inode-9".to_string()),
            }),
        });
        state.workspaces[0].downloads.push(download.clone());
        let retiring = run(
            &state,
            Command::Remove {
                downloads: vec![download.id],
                move_files_to_trash: true,
            },
        );
        assert!(retiring.locate_download(download.id).is_some());
        let writer = decide_fact(
            &retiring,
            &Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
        )
        .unwrap();
        let after_writer = evolve(&retiring, &writer).unwrap();
        assert!(after_writer.locate_download(download.id).is_some());
        let absent = decide_fact(
            &after_writer,
            &Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        )
        .unwrap();
        assert!(matches!(
            absent.effects.first().map(|effect| &effect.kind),
            Some(EffectKind::MoveOutputToTrash { .. })
        ));
    }

    fn apply_facts(state: &AppState, facts: &[Fact]) -> (AppState, Vec<EffectKind>) {
        let mut next = state.clone();
        let mut effects = Vec::new();
        for fact in facts {
            let decision = decide_fact(&next, fact).unwrap();
            effects.extend(decision.effects.iter().map(|effect| effect.kind.clone()));
            next = evolve(&next, &decision).unwrap();
        }
        (next, effects)
    }

    fn partial(path: &str) -> OutputArtifact {
        OutputArtifact {
            path: path.to_string(),
            content_folder: None,
            identity: Some(OutputIdentity {
                byte_length: 10,
                modified_at_millis: 1,
                file_identifier: Some(format!("inode-{path}")),
            }),
        }
    }

    #[test]
    fn every_unfinished_state_can_trash_its_partial_files() {
        let lifecycles = [
            DownloadLifecycle::Queued,
            DownloadLifecycle::Starting,
            DownloadLifecycle::Active(ActivePhase::Fetching),
            DownloadLifecycle::Active(ActivePhase::Processing),
            DownloadLifecycle::Pausing {
                from: ActivePhase::Fetching,
                cause: HoldCause::Person,
            },
            DownloadLifecycle::Paused {
                resume: ActivePhase::Fetching,
                cause: HoldCause::Person,
            },
            DownloadLifecycle::Failed {
                message: "network".to_string(),
                retryable: true,
            },
            DownloadLifecycle::Cancelled,
        ];
        for lifecycle in lifecycles {
            let mut state = running_state();
            let mut download = media_download(&state);
            download.lifecycle = lifecycle.clone();
            state.workspaces[0].downloads.push(download.clone());
            let retiring = run(
                &state,
                Command::Remove {
                    downloads: vec![download.id],
                    move_files_to_trash: true,
                },
            );
            let leftovers = vec![partial("/tmp/Downloads/.film.part")];
            let (stopped, effects) = apply_facts(
                &retiring,
                &[
                    Fact::LeftoversFound {
                        download: download.id,
                        attempt: 0,
                        leftovers: leftovers.clone(),
                    },
                    Fact::WriterStopped {
                        download: download.id,
                        attempt: 0,
                        control_generation: 1,
                    },
                    Fact::AdapterRecordAbsent {
                        download: download.id,
                        attempt: 0,
                    },
                ],
            );
            assert_eq!(
                effects,
                vec![EffectKind::MoveOutputToTrash {
                    download: download.id,
                    artifact: None,
                    leftovers,
                }],
                "{lifecycle:?} should trash its partial files"
            );
            let (resolved, effects) = apply_facts(
                &stopped,
                &[Fact::FilesResolved {
                    download: download.id,
                    control_generation: 1,
                }],
            );
            assert_eq!(
                effects,
                vec![EffectKind::FinalizeRetirement {
                    download: download.id
                }]
            );
            let (retired, _) = apply_facts(
                &resolved,
                &[Fact::RetirementFinalized {
                    download: download.id,
                    control_generation: 1,
                }],
            );
            assert!(retired.locate_download(download.id).is_none());
        }
    }

    #[test]
    fn unverified_output_is_trashed_only_on_the_adapters_evidence() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Finished;
        download.output = Some(OutputArtifact {
            path: "/tmp/Downloads/film.mp4".to_string(),
            content_folder: None,
            identity: None,
        });
        state.workspaces[0].downloads.push(download.clone());
        let remove = Command::Remove {
            downloads: vec![download.id],
            move_files_to_trash: true,
        };
        assert!(decide(&state, &command(remove.clone())).is_ok());
        let retiring = run(&state, remove);

        // The adapter still owned the file, so it is trashed with the identity it has now.
        let leftovers = vec![partial("/tmp/Downloads/film.mp4")];
        let (_, effects) = apply_facts(
            &retiring,
            &[
                Fact::LeftoversFound {
                    download: download.id,
                    attempt: 0,
                    leftovers: leftovers.clone(),
                },
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::MoveOutputToTrash {
                download: download.id,
                artifact: None,
                leftovers,
            }]
        );

        // Without that evidence the file stays, and the removal still finishes.
        let (_, effects) = apply_facts(
            &retiring,
            &[
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::FinalizeRetirement {
                download: download.id
            }]
        );
    }

    #[test]
    fn leftovers_inside_the_verified_output_are_not_trashed_twice() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Finished;
        let output = OutputArtifact {
            path: "/tmp/Downloads/Film/film.mp4".to_string(),
            content_folder: Some("/tmp/Downloads/Film".to_string()),
            identity: Some(OutputIdentity {
                byte_length: 42,
                modified_at_millis: 7,
                file_identifier: Some("inode-9".to_string()),
            }),
        };
        download.output = Some(output.clone());
        state.workspaces[0].downloads.push(download.clone());
        let retiring = run(
            &state,
            Command::Remove {
                downloads: vec![download.id],
                move_files_to_trash: true,
            },
        );
        let (_, effects) = apply_facts(
            &retiring,
            &[
                Fact::LeftoversFound {
                    download: download.id,
                    attempt: 0,
                    leftovers: vec![
                        partial("/tmp/Downloads/Film"),
                        partial("/tmp/Downloads/.film.f137.mp4.part"),
                    ],
                },
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::MoveOutputToTrash {
                download: download.id,
                artifact: Some(output),
                leftovers: vec![partial("/tmp/Downloads/.film.f137.mp4.part")],
            }]
        );
    }

    #[test]
    fn removing_from_the_list_ignores_leftovers() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        };
        state.workspaces[0].downloads.push(download.clone());
        let retiring = run(
            &state,
            Command::Remove {
                downloads: vec![download.id],
                move_files_to_trash: false,
            },
        );
        let (_, effects) = apply_facts(
            &retiring,
            &[
                Fact::LeftoversFound {
                    download: download.id,
                    attempt: 0,
                    leftovers: vec![partial("/tmp/Downloads/.film.part")],
                },
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::FinalizeRetirement {
                download: download.id
            }]
        );
    }

    #[test]
    fn deleting_a_workspace_can_trash_its_downloads_files() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        };
        state.workspaces[0].downloads.push(download.clone());
        let mut remaining = state.workspaces[0].clone();
        remaining.id = WorkspaceId::new();
        remaining.default_folder = FolderId::new();
        remaining.folders[0].id = remaining.default_folder;
        remaining.downloads.clear();
        state.workspaces.push(remaining.clone());
        let deleting = run(
            &state,
            Command::DeleteWorkspace {
                workspace: state.workspaces[0].id,
                replacement: None,
                move_files_to_trash: true,
            },
        );
        let leftovers = vec![partial("/tmp/Downloads/.film.part")];
        let (stopped, effects) = apply_facts(
            &deleting,
            &[
                Fact::LeftoversFound {
                    download: download.id,
                    attempt: 0,
                    leftovers: leftovers.clone(),
                },
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::MoveOutputToTrash {
                download: download.id,
                artifact: None,
                leftovers,
            }]
        );
        let (finished, _) = apply_facts(
            &stopped,
            &[
                Fact::FilesResolved {
                    download: download.id,
                    control_generation: 1,
                },
                Fact::RetirementFinalized {
                    download: download.id,
                    control_generation: 1,
                },
            ],
        );
        assert_eq!(finished.workspaces, vec![remaining]);
        assert!(finished.structural_changes.workspace_deletions.is_empty());
    }

    #[test]
    fn workspace_deletion_intent_survives_until_retirement_finishes() {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
        state.workspaces[0].downloads.push(download.clone());
        let replacement_folder = FolderId::new();
        let replacement = Workspace {
            id: WorkspaceId::new(),
            name: "Downloads".to_string(),
            icon: WorkspaceIcon::Symbol("arrow.down.circle".to_string()),
            folders: vec![SavedFolder {
                id: replacement_folder,
                name: "Downloads".to_string(),
                path: "/tmp/NewDownloads".to_string(),
            }],
            default_folder: replacement_folder,
            default_format: MediaFormat::Best,
            file_organization: FileOrganization::Automatic,
            view: WorkspaceView::default(),
            downloads: Vec::new(),
        };
        let deleting = run(
            &state,
            Command::DeleteWorkspace {
                workspace: state.workspaces[0].id,
                replacement: Some(replacement.clone()),
                move_files_to_trash: false,
            },
        );
        assert_eq!(deleting.structural_changes.workspace_deletions.len(), 1);
        assert!(deleting.locate_download(download.id).is_some());

        let writer = decide_fact(
            &deleting,
            &Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
        )
        .unwrap();
        let after_writer = evolve(&deleting, &writer).unwrap();
        let absent = decide_fact(
            &after_writer,
            &Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        )
        .unwrap();
        let ready = evolve(&after_writer, &absent).unwrap();
        let retired = decide_fact(
            &ready,
            &Fact::RetirementFinalized {
                download: download.id,
                control_generation: 1,
            },
        )
        .unwrap();
        let finished = evolve(&ready, &retired).unwrap();
        assert_eq!(finished.workspaces, vec![replacement]);
        assert!(finished.structural_changes.workspace_deletions.is_empty());
    }

    #[test]
    fn state_rejects_a_folder_from_another_workspace() {
        let mut state = running_state();
        let foreign = FolderId::new();
        state.workspaces.push(Workspace {
            id: WorkspaceId::new(),
            name: "Other".to_string(),
            icon: WorkspaceIcon::Emoji("🎬".to_string()),
            folders: vec![SavedFolder {
                id: foreign,
                name: "Other".to_string(),
                path: "/tmp/Other".to_string(),
            }],
            default_folder: foreign,
            default_format: MediaFormat::Best,
            file_organization: FileOrganization::Automatic,
            view: WorkspaceView::default(),
            downloads: Vec::new(),
        });
        let mut download = media_download(&state);
        download.folder = foreign;
        state.workspaces[0].downloads.push(download);
        assert!(matches!(
            state.validate(),
            Err(InvariantViolation::FolderOutsideWorkspace { .. })
        ));
    }
}
