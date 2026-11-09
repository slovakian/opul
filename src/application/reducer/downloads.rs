use super::*;

pub(super) fn decide_add_download(
    state: &AppState,
    workspace: WorkspaceId,
    mut download: Download,
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
    hold_for_tools(state, &mut download);
    let id = download.id;
    Ok(Decision {
        events: vec![Event::DownloadAdded {
            workspace,
            download: Box::new(download.clone()),
        }],
        effects: start_effect(&download).into_iter().collect(),
        created_downloads: vec![id],
    })
}

pub(super) fn decide_add_media_items(
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
        let mut download = download.clone();
        hold_for_tools(state, &mut download);
        decision.events.push(Event::DownloadAdded {
            workspace,
            download: Box::new(download.clone()),
        });
        decision.effects.extend(start_effect(&download));
        decision.created_downloads.push(download.id);
    }
    Ok(decision)
}

pub(super) fn decide_pause(
    state: &AppState,
    id: DownloadId,
    cause: HoldCause,
) -> Result<Decision, Rejection> {
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
    // A complete torrent seeds or doesn't; Stop seeding and Start seeding control it.
    if item.torrent_complete() {
        return Err(invalid("pause", &item.lifecycle));
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
    if matches!(&item.source, DownloadSource::Media(media) if matches!(media.origin, MediaOrigin::PendingPage { .. }))
    {
        return Ok(simple(Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Paused {
                resume: phase,
                cause,
            },
            control_generation: generation,
        }));
    }
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

pub(super) fn decide_resume(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    let DownloadLifecycle::Paused { .. } = item.lifecycle else {
        return Err(invalid("resume", &item.lifecycle));
    };
    if item.torrent_complete() {
        return Err(invalid("resume", &item.lifecycle));
    }
    if must_wait_for_tools(state, item) {
        let mut decision = Decision::none();
        push_waiting(item, &mut decision);
        return Ok(decision);
    }
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

pub(super) fn decide_retry(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
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
    if must_wait_for_tools(state, item) {
        let mut decision = Decision::none();
        push_waiting(item, &mut decision);
        return Ok(decision);
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

pub(super) fn decide_cancel(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if matches!(item.lifecycle, DownloadLifecycle::Cancelled) {
        return Ok(Decision::none());
    }
    // Once a torrent is complete there is nothing left to cancel; removing it is the way out.
    if item.torrent_complete()
        || matches!(
            item.lifecycle,
            DownloadLifecycle::Finished | DownloadLifecycle::Retiring { .. }
        )
    {
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

pub(super) fn decide_stop_seeding(state: &AppState, id: DownloadId) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    if !matches!(item.source, DownloadSource::Torrent(_)) {
        return Err(Rejection::WrongSource);
    }
    if !matches!(
        item.lifecycle,
        DownloadLifecycle::Active(ActivePhase::Seeding)
    ) && !item.checking_files()
    {
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

/// Seeding is a switch: a torrent that stopped, by hand or at its ratio limit, can seed
/// again. It rechecks its files where they are now, then seeds. Starting again after the
/// ratio limit stopped it clears that limit, so it seeds until stopped or given a new one.
pub(super) fn decide_start_seeding(
    state: &AppState,
    id: DownloadId,
) -> Result<Decision, Rejection> {
    let (_, item) = state
        .locate_download(id)
        .ok_or(Rejection::UnknownDownload(id))?;
    let DownloadSource::Torrent(torrent) = &item.source else {
        return Err(Rejection::WrongSource);
    };
    if !matches!(item.lifecycle, DownloadLifecycle::Finished) {
        return Err(invalid("start seeding", &item.lifecycle));
    }
    let mut generation = item.control_generation;
    let mut events = Vec::new();
    if torrent.reached_seed_ratio {
        generation += 1;
        events.push(Event::SeedRatioChanged {
            download: id,
            limit: None,
            control_generation: generation,
        });
    }
    if !torrent.complete {
        events.push(Event::TorrentCompletionChanged {
            download: id,
            complete: true,
        });
    }
    generation += 1;
    events.push(Event::DownloadLifecycleChanged {
        download: id,
        lifecycle: DownloadLifecycle::Starting,
        control_generation: generation,
    });
    Ok(Decision {
        events,
        effects: vec![EffectPlan {
            attempt: Some(item.attempt),
            control_generation: Some(generation),
            kind: EffectKind::EnsureTransfer { download: id },
        }],
        created_downloads: Vec::new(),
    })
}

pub(super) fn decide_remove(
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

/// The shell moves the files first; this records where they now live. Only a finished
/// download qualifies, because no writer or engine still holds its paths.
pub(super) fn decide_move_download(
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
