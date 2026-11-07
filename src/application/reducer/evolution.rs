use super::*;

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
        Event::VideoToolsChanged(availability) => state.video_tools = *availability,
        Event::MediaPageExpanded {
            workspace,
            download,
            items,
        } => {
            if let Some(workspace) = state.workspace_mut(*workspace)
                && let Some(index) = workspace
                    .downloads
                    .iter()
                    .position(|item| item.id == *download)
            {
                workspace.downloads.splice(index..=index, items.clone());
            }
        }
        Event::WorkspaceAppearanceChanged {
            workspace,
            emoji,
            appearance,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.icon = WorkspaceIcon::Emoji(emoji.clone());
                item.appearance = appearance.clone();
            }
            state.record_recent_emoji(emoji);
        }
        Event::WorkspaceCreated(workspace) => {
            state.workspaces.push(workspace.clone());
            if let WorkspaceIcon::Emoji(emoji) = &workspace.icon {
                state.record_recent_emoji(emoji);
            }
        }
        Event::WorkspaceRenamed {
            workspace,
            name,
            icon,
            organization,
            appearance,
        } => {
            if let Some(item) = state.workspace_mut(*workspace) {
                item.name = name.clone();
                item.icon = icon.clone();
                item.file_organization = *organization;
                item.appearance = appearance.clone();
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

pub(super) fn rebase_path(value: &str, old_base: &str, new_base: &str) -> Option<String> {
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

pub(super) fn finalize_structural_changes(state: &mut AppState) {
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
