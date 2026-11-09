use super::*;
use crate::application::projection::capabilities;

fn torrent(state: &AppState, lifecycle: DownloadLifecycle) -> Download {
    Download {
        id: DownloadId::new(),
        folder: state.workspaces[0].default_folder,
        name: "ubuntu.iso".to_string(),
        attempt: 0,
        control_generation: 3,
        source: DownloadSource::Torrent(TorrentDownload {
            origin: TorrentOrigin::Magnet {
                uri: "magnet:?xt=urn:btih:abc".to_string(),
            },
            info_hash: Some("abc".to_string()),
            metadata: None,
            seed_ratio_limit: None,
            files: Vec::new(),
            complete: false,
            reached_seed_ratio: false,
        }),
        lifecycle,
        output: None,
    }
}

fn with(state: &AppState, download: &Download) -> AppState {
    let mut next = state.clone();
    next.workspaces[0].downloads.push(download.clone());
    next
}

fn item(state: &AppState, id: DownloadId) -> &Download {
    state.locate_download(id).expect("download exists").1
}

fn torrent_of(download: &Download) -> &TorrentDownload {
    let DownloadSource::Torrent(torrent) = &download.source else {
        panic!("not a torrent");
    };
    torrent
}

fn advanced(id: DownloadId, phase: ActivePhase) -> Fact {
    Fact::TransferAdvanced {
        download: id,
        attempt: 0,
        phase,
    }
}

/// A torrent that finished downloading and now seeds.
fn seeding(limit: Option<f64>) -> (AppState, DownloadId) {
    let base = running_state();
    let mut download = torrent(&base, DownloadLifecycle::Active(ActivePhase::Fetching));
    if let DownloadSource::Torrent(torrent) = &mut download.source {
        torrent.seed_ratio_limit = limit;
    }
    let id = download.id;
    let (state, _) = apply_facts(
        &with(&base, &download),
        &[advanced(id, ActivePhase::Seeding)],
    );
    (state, id)
}

#[test]
fn seeding_marks_a_torrent_complete_and_leaves_only_the_seeding_switch() {
    let (state, id) = seeding(None);
    let download = item(&state, id);
    assert!(torrent_of(download).complete);
    assert_eq!(
        download.lifecycle,
        DownloadLifecycle::Active(ActivePhase::Seeding)
    );
    let caps = capabilities(&state, download);
    assert!(!caps.can_pause);
    assert!(!caps.can_resume);
    assert!(!caps.can_cancel);
    assert!(caps.can_stop_seeding);
    assert!(!caps.can_start_seeding);
}

#[test]
fn a_complete_torrent_cannot_be_paused_or_cancelled() {
    let (state, id) = seeding(None);
    for command in [
        Command::Pause { download: id },
        Command::Cancel { download: id },
    ] {
        assert!(matches!(
            decide(&state, &self::command(command)),
            Err(Rejection::InvalidTransition { .. })
        ));
    }
}

#[test]
fn stop_seeding_can_be_undone_and_rechecks_without_downloading() {
    let (state, id) = seeding(None);
    let stopped = run(&state, Command::StopSeeding { download: id });
    assert_eq!(item(&stopped, id).lifecycle, DownloadLifecycle::Finished);
    assert!(capabilities(&stopped, item(&stopped, id)).can_start_seeding);

    // A late "seeding" report from before the stop doesn't undo it.
    let (stale, _) = apply_facts(&stopped, &[advanced(id, ActivePhase::Seeding)]);
    assert_eq!(item(&stale, id).lifecycle, DownloadLifecycle::Finished);

    let decision = decide(&stopped, &command(Command::StartSeeding { download: id })).unwrap();
    assert!(matches!(
        decision.effects.as_slice(),
        [EffectPlan {
            kind: EffectKind::EnsureTransfer { .. },
            ..
        }]
    ));
    let starting = evolve(&stopped, &decision).unwrap();
    let generation = item(&starting, id).control_generation;
    assert!(item(&starting, id).checking_files());
    assert!(capabilities(&starting, item(&starting, id)).can_stop_seeding);

    // Reattaching, the engine's old "stopped" report and its recheck don't change anything.
    let (checking, _) = apply_facts(
        &starting,
        &[
            Fact::TransferStarted {
                download: id,
                attempt: 0,
                control_generation: generation,
                phase: ActivePhase::Fetching,
            },
            Fact::TransferFinished {
                download: id,
                attempt: 0,
                output: partial("/tmp/Downloads/ubuntu.iso"),
            },
            advanced(id, ActivePhase::Verifying),
        ],
    );
    assert_eq!(item(&checking, id).lifecycle, DownloadLifecycle::Starting);
    assert!(torrent_of(item(&checking, id)).complete);

    let (seeding_again, _) = apply_facts(&checking, &[advanced(id, ActivePhase::Seeding)]);
    assert_eq!(
        item(&seeding_again, id).lifecycle,
        DownloadLifecycle::Active(ActivePhase::Seeding)
    );
}

#[test]
fn start_seeding_after_the_ratio_limit_clears_the_limit() {
    let (state, id) = seeding(Some(1.0));
    let (stopped, _) = apply_facts(
        &state,
        &[Fact::TransferFinished {
            download: id,
            attempt: 0,
            output: partial("/tmp/Downloads/ubuntu.iso"),
        }],
    );
    assert_eq!(item(&stopped, id).lifecycle, DownloadLifecycle::Finished);
    assert!(torrent_of(item(&stopped, id)).reached_seed_ratio);

    let started = run(&stopped, Command::StartSeeding { download: id });
    let torrent = torrent_of(item(&started, id));
    assert_eq!(torrent.seed_ratio_limit, None);
    assert!(!torrent.reached_seed_ratio);
}

#[test]
fn start_seeding_after_stopping_by_hand_keeps_the_limit() {
    let (state, id) = seeding(Some(2.0));
    let stopped = run(&state, Command::StopSeeding { download: id });
    let started = run(&stopped, Command::StartSeeding { download: id });
    assert_eq!(torrent_of(item(&started, id)).seed_ratio_limit, Some(2.0));
}

#[test]
fn downloading_again_makes_a_torrent_incomplete() {
    let (state, id) = seeding(None);
    // More files were selected, so the engine fetches again.
    let (fetching, _) = apply_facts(&state, &[advanced(id, ActivePhase::Fetching)]);
    let download = item(&fetching, id);
    assert!(!torrent_of(download).complete);
    assert_eq!(
        download.lifecycle,
        DownloadLifecycle::Active(ActivePhase::Fetching)
    );
    assert!(capabilities(&fetching, download).can_pause);
}

#[test]
fn a_recheck_that_finds_missing_files_downloads_them() {
    let (state, id) = seeding(None);
    let stopped = run(&state, Command::StopSeeding { download: id });
    let starting = run(&stopped, Command::StartSeeding { download: id });
    let (fetching, _) = apply_facts(&starting, &[advanced(id, ActivePhase::Fetching)]);
    assert!(!torrent_of(item(&fetching, id)).complete);
    assert_eq!(
        item(&fetching, id).lifecycle,
        DownloadLifecycle::Active(ActivePhase::Fetching)
    );
}

#[test]
fn an_incomplete_torrent_rechecking_reads_as_downloading() {
    let base = running_state();
    let download = torrent(&base, DownloadLifecycle::Starting);
    let id = download.id;
    let (state, _) = apply_facts(
        &with(&base, &download),
        &[advanced(id, ActivePhase::Verifying)],
    );
    assert_eq!(
        item(&state, id).lifecycle,
        DownloadLifecycle::Active(ActivePhase::Fetching)
    );
}

#[test]
fn a_seeding_torrent_records_where_its_files_are() {
    let (state, id) = seeding(None);
    let output = partial("/tmp/Downloads/ubuntu.iso");
    let (found, _) = apply_facts(
        &state,
        &[Fact::OutputFound {
            download: id,
            attempt: 0,
            output: output.clone(),
        }],
    );
    let download = item(&found, id);
    assert_eq!(download.output, Some(output));
    assert_eq!(
        download.lifecycle,
        DownloadLifecycle::Active(ActivePhase::Seeding)
    );
    assert!(capabilities(&found, download).can_reveal_output);
}

#[test]
fn older_checkpoints_load_paused_seeds_as_not_seeding() {
    let base = running_state();
    let paused = torrent(
        &base,
        DownloadLifecycle::Paused {
            resume: ActivePhase::Seeding,
            cause: HoldCause::Person,
        },
    );
    let seeding = torrent(&base, DownloadLifecycle::Active(ActivePhase::Seeding));
    let finished = torrent(&base, DownloadLifecycle::Finished);
    let downloading = torrent(&base, DownloadLifecycle::Active(ActivePhase::Fetching));
    let mut state = base.clone();
    state.workspaces[0]
        .downloads
        .extend([&paused, &seeding, &finished, &downloading].map(Clone::clone));

    assert!(state.migrate_torrent_completion());
    assert_eq!(
        item(&state, paused.id).lifecycle,
        DownloadLifecycle::Finished
    );
    for id in [paused.id, seeding.id, finished.id] {
        assert!(torrent_of(item(&state, id)).complete);
    }
    assert!(!torrent_of(item(&state, downloading.id)).complete);
    assert!(!state.migrate_torrent_completion());
}
