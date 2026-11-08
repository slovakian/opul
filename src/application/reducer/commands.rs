use super::*;

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
        Command::SetWorkspaceAppearance {
            workspace,
            emoji,
            appearance,
        } => {
            let existing = state
                .workspace(*workspace)
                .ok_or(Rejection::UnknownWorkspace(*workspace))?;
            if emoji.trim().is_empty() || emoji.len() > 128 || emoji.chars().any(char::is_control) {
                return Err(Rejection::InvalidInput("choose a valid emoji".into()));
            }
            if !appearance.valid() {
                return Err(Rejection::InvalidInput(
                    "invalid workspace appearance".into(),
                ));
            }
            let icon = WorkspaceIcon::Emoji(emoji.clone());
            let mut saved = existing.appearance.clone();
            saved.preference = appearance.preference.clone();
            if saved.automatic_folder.is_none() && icon.renders_as_folder() {
                saved.automatic_folder = appearance
                    .automatic_folder
                    .clone()
                    .or_else(|| state.propose_folder_appearance(*workspace).automatic_folder);
            }
            Ok(simple(Event::WorkspaceAppearanceChanged {
                workspace: *workspace,
                emoji: emoji.clone(),
                appearance: saved,
            }))
        }
        Command::CreateWorkspace { workspace } => {
            let mut workspace = workspace.clone();
            state.assign_folder_appearance(&mut workspace);
            validate_new_workspace(state, &workspace)?;
            Ok(Decision {
                events: vec![Event::WorkspaceCreated(workspace)],
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
            let existing = state
                .workspace(*workspace)
                .ok_or(Rejection::UnknownWorkspace(*workspace))?;
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
                appearance: if icon.renders_as_folder() && existing.appearance.is_unassigned() {
                    state.propose_folder_appearance(*workspace)
                } else {
                    existing.appearance.clone()
                },
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
