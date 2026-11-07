use super::*;

pub(super) fn validate_new_workspace(
    state: &AppState,
    workspace: &Workspace,
) -> Result<(), Rejection> {
    if !workspace.appearance.valid() {
        return Err(Rejection::InvalidInput(
            "invalid workspace appearance".to_string(),
        ));
    }
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

pub(super) fn decide_delete_workspace(
    state: &AppState,
    workspace: WorkspaceId,
    mut replacement: Option<Workspace>,
    move_files_to_trash: bool,
) -> Result<Decision, Rejection> {
    if let Some(replacement) = &mut replacement {
        state.assign_folder_appearance(replacement);
    }
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

pub(super) fn require_folder(
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
