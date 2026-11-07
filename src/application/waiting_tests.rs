use super::*;

fn command(command: Command) -> CommandEnvelope {
    CommandEnvelope {
        id: CommandId::new(),
        fingerprint: uuid::Uuid::new_v4().to_string(),
        expected_revision: None,
        command,
    }
}

fn page(state: &AppState) -> Download {
    Download {
        id: DownloadId::new(),
        folder: state.workspaces[0].default_folder,
        name: "https://example.com/playlist".into(),
        attempt: 0,
        control_generation: 0,
        source: DownloadSource::Media(MediaDownload {
            origin: MediaOrigin::Page {
                page_url: "https://example.com/playlist".into(),
                item_url: "https://example.com/playlist".into(),
                playlist_index: None,
            },
            format: MediaFormat::AudioM4a,
            policy: MediaPolicy::default(),
            legacy_binding: None,
            thumbnail_url: None,
            extractor: None,
        }),
        lifecycle: DownloadLifecycle::Queued,
        output: None,
    }
}

#[test]
fn unavailable_video_tools_hold_pages_without_dispatching_an_effect() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    state.video_tools = VideoToolsAvailability::SettingUp;
    let download = page(&state);
    let decision = decide(
        &state,
        &command(Command::AddMediaItems {
            workspace: state.workspaces[0].id,
            downloads: vec![download.clone()],
        }),
    )
    .unwrap();
    assert!(
        decision.effects.is_empty(),
        "a waiting page must not reach the media adapter"
    );
    let next = evolve(&state, &decision).unwrap();
    let saved = &next.workspaces[0].downloads[0];
    assert_eq!(saved.id, download.id);
    assert_eq!(saved.folder, download.folder);
    assert_eq!(saved.source, download.source);
}

fn apply_fact(state: &AppState, fact: Fact) -> AppState {
    evolve(state, &decide_fact(state, &fact).unwrap()).unwrap()
}

#[test]
fn ready_queues_waiting_pages_once_and_direct_files_still_start() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    state.video_tools = VideoToolsAvailability::SettingUp;
    let download = page(&state);
    let id = download.id;
    let mut direct = download.clone();
    direct.id = DownloadId::new();
    if let DownloadSource::Media(media) = &mut direct.source {
        media.origin = MediaOrigin::Direct {
            url: "https://example.com/file.zip".into(),
            referer: None,
        };
    }
    let decision = decide(
        &state,
        &command(Command::AddMediaItems {
            workspace: state.workspaces[0].id,
            downloads: vec![download, direct],
        }),
    )
    .unwrap();
    assert_eq!(decision.effects.len(), 1);
    let waiting = evolve(&state, &decision).unwrap();
    assert_eq!(
        waiting.workspaces[0].downloads[0].lifecycle,
        DownloadLifecycle::WaitingForVideoTools
    );
    let ready = apply_fact(
        &waiting,
        Fact::VideoToolsChanged(VideoToolsAvailability::Ready),
    );
    assert_eq!(
        ready.locate_download(id).unwrap().1.lifecycle,
        DownloadLifecycle::Queued
    );
    assert_eq!(
        decide_fact(
            &waiting,
            &Fact::VideoToolsChanged(VideoToolsAvailability::Ready)
        )
        .unwrap()
        .effects
        .len(),
        1
    );
    let repeated = decide_fact(
        &ready,
        &Fact::VideoToolsChanged(VideoToolsAvailability::Ready),
    )
    .unwrap();
    assert!(repeated.effects.is_empty());
    assert!(repeated.events.is_empty());
}

#[test]
fn retry_in_turned_off_projects_explicit_setup_and_failure_projects_retry() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    state.video_tools = VideoToolsAvailability::TurnedOff;
    let mut download = page(&state);
    download.lifecycle = DownloadLifecycle::Failed {
        message: "tools missing".into(),
        retryable: true,
    };
    let id = download.id;
    state.workspaces[0].downloads.push(download);
    let decision = decide(&state, &command(Command::Retry { download: id })).unwrap();
    assert!(decision.effects.is_empty());
    let waiting = evolve(&state, &decision).unwrap();
    let projected = project(&waiting, &RuntimeOverlay::default());
    let item = &projected.workspaces[0].downloads[0];
    assert_eq!(item.waiting_label.as_deref(), Some("Needs video tools"));
    assert!(item.capabilities.can_set_up_video_tools);
    assert!(!item.capabilities.can_retry);
    let failed = apply_fact(
        &waiting,
        Fact::VideoToolsChanged(VideoToolsAvailability::NotWorking),
    );
    let projected = project(&failed, &RuntimeOverlay::default());
    let item = &projected.workspaces[0].downloads[0];
    assert_eq!(
        item.waiting_label.as_deref(),
        Some("Video tools aren't working")
    );
    assert!(item.capabilities.can_retry_video_tools);
    assert!(item.capabilities.can_cancel);
}

#[test]
fn playlist_expands_at_its_position_with_identity_destination_format_and_local_deduplication() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    let mut pending = page(&state);
    if let DownloadSource::Media(media) = &mut pending.source {
        media.origin = MediaOrigin::PendingPage {
            url: "https://example.com/playlist".into(),
        };
    }
    let id = pending.id;
    let mut existing = page(&state);
    existing.name = "Already here".into();
    let mut other_workspace = state.workspaces[0].clone();
    other_workspace.id = WorkspaceId::new();
    other_workspace.folders[0].id = FolderId::new();
    other_workspace.default_folder = other_workspace.folders[0].id;
    let mut elsewhere = existing.clone();
    elsewhere.id = DownloadId::new();
    elsewhere.folder = other_workspace.default_folder;
    other_workspace.downloads.push(elsewhere);
    state.workspaces.push(other_workspace);
    state.workspaces[0].downloads = vec![existing.clone(), pending.clone()];
    let mut first = page(&state);
    first.name = "First unique".into();
    if let DownloadSource::Media(media) = &mut first.source {
        media.origin = MediaOrigin::Page {
            page_url: "https://example.com/playlist".into(),
            item_url: "https://example.com/1".into(),
            playlist_index: Some(2),
        };
        media.format = MediaFormat::Best;
    }
    let mut second = first.clone();
    second.id = DownloadId::new();
    second.name = "Second unique".into();
    if let DownloadSource::Media(media) = &mut second.source {
        media.origin = MediaOrigin::Page {
            page_url: "https://example.com/playlist".into(),
            item_url: "https://example.com/2".into(),
            playlist_index: Some(3),
        };
    }
    let fact = Fact::MediaPageResolved {
        download: id,
        attempt: 0,
        control_generation: 0,
        items: vec![existing, first, second],
    };
    let next = apply_fact(&state, fact.clone());
    let items = &next.workspaces[0].downloads;
    assert_eq!(items.len(), 3);
    assert_eq!(items[1].id, id);
    assert_eq!(items[1].name, "First unique");
    assert_eq!(items[2].name, "Second unique");
    assert_eq!(items[1].folder, pending.folder);
    assert!(
        matches!(&items[1].source, DownloadSource::Media(media) if media.format == MediaFormat::AudioM4a)
    );
    assert_eq!(decide_fact(&state, &fact).unwrap().effects.len(), 2);
    assert!(decide_fact(&next, &fact).unwrap().events.is_empty());
    // An identical URL remains acceptable in another Workspace.
    let second_workspace = next.workspaces[1].id;
    let mut duplicate = items[1].clone();
    duplicate.id = DownloadId::new();
    duplicate.folder = next.workspaces[1].default_folder;
    let decision = decide(
        &next,
        &command(Command::AddMediaItems {
            workspace: second_workspace,
            downloads: vec![duplicate],
        }),
    )
    .unwrap();
    assert_eq!(decision.created_downloads.len(), 1);
}

#[test]
fn actor_checkpoint_keeps_waiting_page_then_dispatches_ready_after_restart() {
    let directory = tempfile::tempdir().unwrap();
    let actor = ApplicationActor::open(directory.path().into(), "/tmp/Films".into()).unwrap();
    actor
        .observe(Fact::VideoToolsChanged(VideoToolsAvailability::SettingUp))
        .unwrap();
    actor.dispatch(command(Command::MarkReconciled)).unwrap();
    let projection = actor.snapshot();
    let state = AppState::fresh("/tmp/Unused".into());
    let mut download = page(&state);
    download.folder = projection.workspaces[0].default_folder;
    if let DownloadSource::Media(media) = &mut download.source {
        media.origin = MediaOrigin::PendingPage {
            url: "https://example.com/playlist".into(),
        };
    }
    let workspace = projection.workspaces[0].id;
    let receipt = actor
        .dispatch(command(Command::AddMediaItems {
            workspace,
            downloads: vec![download.clone()],
        }))
        .unwrap();
    assert_eq!(receipt.created_downloads, vec![download.id]);
    assert!(actor.claim_effect().unwrap().is_none());
    drop(actor);
    let actor = ApplicationActor::open(directory.path().into(), "/tmp/Other".into()).unwrap();
    actor.dispatch(command(Command::MarkReconciled)).unwrap();
    let saved = &actor.snapshot().workspaces[0].downloads[0];
    assert_eq!(saved.id, download.id);
    assert_eq!(saved.folder, download.folder);
    assert_eq!(saved.source_data, download.source);
    assert_eq!(saved.lifecycle, DownloadLifecycle::WaitingForVideoTools);
    assert!(actor.claim_effect().unwrap().is_none());
    actor
        .observe(Fact::VideoToolsChanged(VideoToolsAvailability::Ready))
        .unwrap();
    let effect = actor.claim_effect().unwrap().unwrap();
    assert_eq!(
        effect.kind,
        EffectKind::EnsureTransfer {
            download: download.id
        }
    );
    actor
        .observe(Fact::VideoToolsChanged(VideoToolsAvailability::Ready))
        .unwrap();
    assert!(actor.claim_effect().unwrap().is_none());
}

#[test]
fn ready_during_reconciliation_waits_for_the_session_fence() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Reconciling;
    state.video_tools = VideoToolsAvailability::SettingUp;
    let mut download = page(&state);
    download.lifecycle = DownloadLifecycle::WaitingForVideoTools;
    state.workspaces[0].downloads.push(download.clone());
    let ready = apply_fact(
        &state,
        Fact::VideoToolsChanged(VideoToolsAvailability::Ready),
    );
    assert!(
        decide_fact(
            &state,
            &Fact::VideoToolsChanged(VideoToolsAvailability::Ready)
        )
        .unwrap()
        .effects
        .is_empty()
    );
    assert_eq!(
        ready.workspaces[0].downloads[0].lifecycle,
        DownloadLifecycle::WaitingForVideoTools
    );
    let decision = decide(&ready, &command(Command::MarkReconciled)).unwrap();
    assert_eq!(decision.effects.len(), 1);
    assert_eq!(
        decision.effects[0].kind,
        EffectKind::EnsureTransfer {
            download: download.id
        }
    );
}
