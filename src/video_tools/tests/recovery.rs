use super::*;

#[test]
fn interrupted_payload_resumes_from_the_received_offset() {
    let fixture = Fixture::new(GOOD);
    let size = {
        let mut routes = fixture.routes.lock().unwrap();
        let response = routes.get_mut("/runtime").unwrap();
        response.interruptions = 1;
        response.bytes.len()
    };
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    let snapshot = fixture.wait(&tools);
    assert!(
        matches!(snapshot.state, VideoToolsState::Ready { .. }),
        "{snapshot:?}"
    );
    assert_eq!(
        *fixture.ranges.lock().unwrap(),
        vec![("/runtime".into(), (size / 2) as u64)]
    );
    assert_eq!(fixture.count("GET /runtime"), 2);
}

#[test]
fn startup_prunes_unreferenced_versions_and_abandoned_state_writes() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    drop(tools);
    for path in ["python-runtime/999", "yt-dlp/orphan", ".install-crash"] {
        std::fs::create_dir_all(fixture.config.root.join(path)).unwrap();
    }
    std::fs::write(fixture.config.root.join(".state-crash.json"), b"incomplete").unwrap();
    let reopened = fixture.open();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::Ready { .. }
    ));
    for path in [
        "python-runtime/999",
        "yt-dlp/orphan",
        ".install-crash",
        ".state-crash.json",
    ] {
        assert!(!fixture.config.root.join(path).exists(), "{path}");
    }
}

#[test]
fn retries_exhaust_after_two_ten_and_thirty_seconds_and_retry_recovers() {
    for path in [
        "/pointer",
        "/yt/releases/latest",
        "/runtime",
        "/yt/releases/download/2026.10.26/yt-dlp",
    ] {
        let fixture = Fixture::new(GOOD);
        fixture
            .routes
            .lock()
            .unwrap()
            .get_mut(path)
            .unwrap()
            .failures = 4;
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        let snapshot = fixture.wait(&tools);
        assert!(
            matches!(
                snapshot.state,
                VideoToolsState::NotWorking {
                    reason: FailureReason::Download,
                    ..
                }
            ),
            "{path}: {snapshot:?}"
        );
        assert_eq!(fixture.count(&format!("GET {path}")), 4);
        assert_eq!(
            *fixture.network.sleeps.lock().unwrap(),
            vec![
                Duration::from_secs(2),
                Duration::from_secs(10),
                Duration::from_secs(30)
            ]
        );
        assert!(tools.invocation().is_none());
        assert!(!fixture._temp.path().join("executed").exists());
        tools.execute(VideoToolsOperation::Retry).unwrap();
        assert!(matches!(
            fixture.wait(&tools).state,
            VideoToolsState::Ready { .. }
        ));
    }
}

#[test]
fn interrupted_metadata_and_script_resume_and_servers_ignoring_range_restart_safely() {
    for path in ["/pointer", "/yt/releases/download/2026.10.26/yt-dlp"] {
        for ignore_range in [false, true] {
            let fixture = Fixture::new(GOOD);
            let size = {
                let mut routes = fixture.routes.lock().unwrap();
                let response = routes.get_mut(path).unwrap();
                response.interruptions = 1;
                response.ignore_range = ignore_range;
                response.bytes.len()
            };
            let tools = fixture.open();
            tools.execute(VideoToolsOperation::SetUp).unwrap();
            let snapshot = fixture.wait(&tools);
            assert!(
                matches!(snapshot.state, VideoToolsState::Ready { .. }),
                "{snapshot:?}"
            );
            assert_eq!(
                *fixture.ranges.lock().unwrap(),
                vec![(path.into(), (size / 2) as u64)]
            );
            assert_eq!(fixture.count(&format!("GET {path}")), 2);
        }
    }
}

fn wait_offline(tools: &VideoTools) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if matches!(tools.snapshot().state, VideoToolsState::SettingUp { steps } if steps.iter().any(|step| step.status == StepStatus::WaitingForInternet))
        {
            return;
        }
        assert!(Instant::now() < deadline, "{:?}", tools.snapshot());
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn offline_before_setup_uses_no_requests_or_retries_and_reachability_resumes_it() {
    let fixture = Fixture::new(GOOD);
    fixture.network.set_online(false);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    wait_offline(&tools);
    assert!(fixture.hits.lock().unwrap().is_empty());
    assert!(fixture.network.sleeps.lock().unwrap().is_empty());
    assert!(tools.invocation().is_none());
    fixture.network.set_online(true);
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    assert!(fixture.network.sleeps.lock().unwrap().is_empty());
}

#[test]
fn offline_during_backoff_preserves_the_retry_budget_and_resume_offset() {
    let fixture = Fixture::new(GOOD);
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut("/runtime")
        .unwrap()
        .interruptions = 4;
    fixture
        .network
        .disconnect_on_retry
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    wait_offline(&tools);
    assert_eq!(fixture.count("GET /runtime"), 1);
    assert_eq!(
        *fixture.network.sleeps.lock().unwrap(),
        vec![Duration::from_secs(2)]
    );
    let offset = match tools.snapshot().state {
        VideoToolsState::SettingUp { steps } => steps[0].downloaded_bytes,
        state => panic!("{state:?}"),
    };
    assert!(offset > 0);
    fixture.network.set_online(true);
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    assert_eq!(fixture.count("GET /runtime"), 5);
    assert_eq!(
        fixture.ranges.lock().unwrap()[0],
        ("/runtime".into(), offset)
    );
    assert_eq!(
        *fixture.network.sleeps.lock().unwrap(),
        vec![
            Duration::from_secs(2),
            Duration::from_secs(2),
            Duration::from_secs(10),
            Duration::from_secs(30)
        ]
    );
}

#[test]
fn cancelling_offline_setup_releases_the_root_and_restart_never_resumes_without_consent() {
    let fixture = Fixture::new(GOOD);
    fixture.network.set_online(false);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    wait_offline(&tools);
    let committed = std::fs::read(fixture.config.root.join("state.json")).unwrap();
    tools.execute(VideoToolsOperation::CancelSetup).unwrap();
    let weak = Arc::downgrade(&tools);
    drop(tools);
    let deadline = Instant::now() + Duration::from_secs(5);
    while weak.strong_count() > 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    fixture.network.set_online(true);
    let reopened = fixture.open();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::NotSetUp
    ));
    assert_eq!(
        std::fs::read(fixture.config.root.join("state.json")).unwrap(),
        committed
    );
    assert!(fixture.hits.lock().unwrap().is_empty());
    assert!(std::fs::read_dir(&fixture.config.root).unwrap().all(|e| {
        !e.unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".install-")
    }));
}

#[test]
fn transient_corruption_is_discarded_and_a_fresh_verified_copy_is_activated() {
    for path in [
        "/pointer.sig",
        "/runtime",
        "/runtime.sig",
        "/yt/releases/download/2026.10.26/SHA2-256SUMS.sig",
        "/yt/releases/download/2026.10.26/yt-dlp",
    ] {
        let fixture = Fixture::new(GOOD);
        fixture
            .routes
            .lock()
            .unwrap()
            .get_mut(path)
            .unwrap()
            .corruptions = 1;
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        assert!(
            matches!(fixture.wait(&tools).state, VideoToolsState::Ready { .. }),
            "{path}: {:?}",
            tools.snapshot()
        );
        assert_eq!(fixture.count(&format!("GET {path}")), 2);
        assert!(
            fixture.ranges.lock().unwrap().is_empty(),
            "verification retry must discard the payload"
        );
    }
}

#[test]
fn permanent_http_errors_do_not_retry() {
    let fixture = Fixture::new(GOOD);
    {
        let mut routes = fixture.routes.lock().unwrap();
        let response = routes.get_mut("/runtime").unwrap();
        response.failures = 10;
        response.failure_status = 404;
    }
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::NotWorking {
            reason: FailureReason::Download,
            ..
        }
    ));
    assert_eq!(fixture.count("GET /runtime"), 1);
    assert!(fixture.network.sleeps.lock().unwrap().is_empty());
    let details = tools.diagnostics();
    assert!(details.contains("HTTP 404"));
    assert!(details.contains("2026.10.26"));
    assert!(details.contains("Some((1,"));
    assert!(details.contains(std::env::consts::ARCH));
}

#[test]
fn startup_keeps_only_selected_and_explicitly_retained_versions() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    let selected = tools.invocation().unwrap();
    let committed = std::fs::read(fixture.config.root.join("state.json")).unwrap();
    // A leftover candidate that never reached state.json cannot replace the committed pair.
    for path in [
        "python-runtime/2",
        "python-runtime/3",
        "yt-dlp/previous",
        "yt-dlp/candidate",
    ] {
        std::fs::create_dir_all(fixture.config.root.join(path)).unwrap();
    }
    let mut state: serde_json::Value = serde_json::from_slice(&committed).unwrap();
    state["previous_runtime"] = 2.into();
    state["previous_yt_dlp"] = "previous".into();
    std::fs::write(
        fixture.config.root.join("state.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    drop(tools);
    let reopened = fixture.open();
    assert_eq!(reopened.invocation(), Some(selected));
    for path in [
        "python-runtime/1",
        "python-runtime/2",
        "yt-dlp/2026.10.26",
        "yt-dlp/previous",
    ] {
        assert!(fixture.config.root.join(path).is_dir(), "{path}");
    }
    for path in ["python-runtime/3", "yt-dlp/candidate"] {
        assert!(!fixture.config.root.join(path).exists(), "{path}");
    }
    assert_eq!(fixture.count("GET /runtime"), 1);
}

#[test]
fn startup_rejects_missing_unexecutable_and_os_rejected_tools() {
    use std::os::unix::fs::PermissionsExt;
    for damage in ["script", "permissions", "blocked"] {
        let fixture = Fixture::new(GOOD);
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        assert!(matches!(
            fixture.wait(&tools).state,
            VideoToolsState::Ready { .. }
        ));
        let selected = tools.invocation().unwrap();
        match damage {
            "script" => std::fs::remove_file(selected.script).unwrap(),
            "permissions" => std::fs::set_permissions(
                selected.interpreter,
                std::fs::Permissions::from_mode(0o644),
            )
            .unwrap(),
            _ => std::fs::write(
                selected.interpreter,
                b"#!/bin/sh\nprintf 'blocked by OS' >&2\nexit 126\n",
            )
            .unwrap(),
        }
        drop(tools);
        let reopened = fixture.open();
        assert!(
            matches!(
                reopened.snapshot().state,
                VideoToolsState::NotWorking {
                    reason: FailureReason::StoppedWorking,
                    ..
                }
            ),
            "{damage}"
        );
        assert!(reopened.invocation().is_none());
        assert!(reopened.diagnostics().contains("2026.10.26"));
        reopened.execute(VideoToolsOperation::Retry).unwrap();
        assert!(matches!(
            fixture.wait(&reopened).state,
            VideoToolsState::Ready { .. }
        ));
    }
}

#[test]
fn invalid_resume_ranges_fail_closed() {
    for range in ["bytes 0-10/20", "bytes 300-200/600", "items 300-599/600"] {
        let fixture = Fixture::new(GOOD);
        {
            let mut routes = fixture.routes.lock().unwrap();
            let response = routes.get_mut("/runtime").unwrap();
            response.interruptions = 1;
            response.range_header = Some(range.into());
        }
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        assert!(matches!(
            fixture.wait(&tools).state,
            VideoToolsState::NotWorking {
                reason: FailureReason::Download,
                ..
            }
        ));
        assert!(tools.invocation().is_none());
        assert!(!fixture._temp.path().join("executed").exists());
        assert_eq!(fixture.count("GET /runtime"), 2);
    }
}

#[test]
fn authentic_runtime_with_the_wrong_signed_checksum_is_rejected_before_execution() {
    use ed25519_dalek::Signer;
    let fixture = Fixture::new(GOOD);
    {
        let mut routes = fixture.routes.lock().unwrap();
        let mut pointer: serde_json::Value =
            serde_json::from_slice(&routes["/pointer"].bytes).unwrap();
        for architecture in ["arm64", "x64"] {
            pointer["architectures"][architecture]["sha256"] = "0".repeat(64).into();
        }
        let bytes = serde_json::to_vec(&pointer).unwrap();
        routes.get_mut("/pointer.sig").unwrap().bytes = SigningKey::from_bytes(&[9; 32])
            .sign(&bytes)
            .to_bytes()
            .to_vec();
        routes.get_mut("/pointer").unwrap().bytes = bytes;
    }
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::NotWorking {
            reason: FailureReason::Verification,
            ..
        }
    ));
    assert_eq!(fixture.count("GET /runtime"), 2);
    assert!(tools.diagnostics().contains("SHA-256"));
    assert!(!fixture._temp.path().join("executed").exists());
}

#[test]
fn optional_size_lookup_offline_keeps_its_fallback_and_releases_the_tools_root() {
    let fixture = Fixture::new(GOOD);
    fixture.network.set_online(false);
    let tools = fixture.open();
    tools
        .execute(VideoToolsOperation::RefreshPublishedSize)
        .unwrap();
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
    assert!(tools.snapshot().published_size.is_none());
    let weak = Arc::downgrade(&tools);
    drop(tools);
    let deadline = Instant::now() + Duration::from_secs(5);
    while weak.strong_count() > 0 {
        assert!(
            Instant::now() < deadline,
            "optional metadata lookup kept the Tools root locked"
        );
        thread::sleep(Duration::from_millis(2));
    }
    let reopened = fixture.open();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::NotSetUp
    ));
    assert!(fixture.hits.lock().unwrap().is_empty());
}
