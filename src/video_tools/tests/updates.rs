use super::*;

fn command(tools: &Arc<VideoTools>, kind: &str) {
    tools
        .execute(serde_json::from_value(serde_json::json!({"kind": kind})).unwrap())
        .unwrap();
}

fn selected(tools: &VideoTools) -> InstalledPair {
    match tools.snapshot().state {
        VideoToolsState::Ready { installed, .. } => installed,
        state => panic!("expected Ready, got {state:?}"),
    }
}

#[test]
fn manual_check_and_script_update_keep_the_runtime_and_previous_script() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let old = tools.invocation().unwrap();
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "check_for_updates");
    let snapshot = fixture.wait(&tools);
    assert!(
        matches!(
            snapshot.state,
            VideoToolsState::Ready {
                update_flag: UpdateFlag::Available { .. },
                last_checked: Some(_),
                ..
            }
        ),
        "{snapshot:?}"
    );
    assert_eq!(tools.invocation().unwrap(), old);
    assert_eq!(
        fixture.count("GET /yt/releases/download/2026.11.01/yt-dlp"),
        0
    );
    command(&tools, "update");
    let snapshot = fixture.wait(&tools);
    assert!(
        matches!(
            snapshot.state,
            VideoToolsState::Ready {
                update_flag: UpdateFlag::None,
                ..
            }
        ),
        "{snapshot:?}"
    );
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    assert_eq!(tools.invocation().unwrap().interpreter, old.interpreter);
    assert!(old.script.exists());
    assert_eq!(fixture.count("GET /runtime-1"), 0);
}

#[test]
fn runtime_only_then_both_parts_update_runtime_first_and_retain_until_success() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    fixture.publish(2, "2026.10.26", GOOD);
    command(&tools, "update");
    let snapshot = fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2, "{snapshot:?}");
    assert!(
        fixture
            .config
            .root
            .join("python-runtime/1/bin/python3")
            .exists()
    );
    assert_eq!(
        fixture.count("GET /yt/releases/download/2026.10.26/yt-dlp"),
        1
    );
    tools.begin_video_download().unwrap().finish(true).unwrap();
    assert!(!fixture.config.root.join("python-runtime/1").exists());
    fixture.publish(3, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 3);
    let hits = fixture.hits.lock().unwrap().clone();
    assert!(
        hits.iter().position(|p| p == "GET /runtime-3").unwrap()
            < hits
                .iter()
                .position(|p| p == "GET /yt/releases/download/2026.11.01/yt-dlp")
                .unwrap()
    );
    assert!(fixture.config.root.join("python-runtime/2").exists());
    tools.begin_video_download().unwrap().finish(false).unwrap();
    assert_eq!(selected(&tools).runtime, 2);
    assert_eq!(selected(&tools).yt_dlp, "2026.10.26");
    drop(tools);
    let reopened = fixture.open();
    assert_eq!(selected(&reopened).runtime, 2);
}

#[test]
fn active_download_defers_runtime_installation_and_activation_but_old_pair_runs() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let download = tools.begin_video_download().unwrap();
    let old = download.invocation.clone();
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "update");
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture.count("GET /runtime-2.sig") == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(tools.invocation().unwrap(), old);
    assert!(
        old.command()
            .arg("--version")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!fixture.config.root.join("python-runtime/2").exists());
    assert_eq!(
        fixture.count("GET /yt/releases/download/2026.11.01/yt-dlp"),
        0
    );
    drop(download);
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
}

#[test]
fn incompatible_script_holds_across_restart_until_a_new_runtime_appears() {
    for bad in [
        GOOD.replace("3.13.18", "3.9.0\nPython deprecation warning"),
        GOOD.replace("curl_cffi", "curl_cffi (unsupported)"),
        GOOD.replace("quickjs-2026-06-04", "none"),
        GOOD.replace("stable@2026.10.26", "stable@wrong"),
    ] {
        let fixture = Fixture::new(GOOD);
        let tools = fixture.open();
        command(&tools, "set_up");
        fixture.wait(&tools);
        // The installed runtime now rejects new scripts but can still run the selected script.
        let interpreter = tools.invocation().unwrap().interpreter;
        let original = std::fs::read_to_string(&interpreter).unwrap();
        let bad = bad.replace("2026.10.26", "2026.11.01");
        let conditional = format!(
            "case \"$(cat \"$2\")\" in 2026.11.01) printf '%s\\n' 2026.11.01; cat >&2 <<'BAD'\n{bad}\nBAD\nexit 0 ;; esac\n"
        );
        std::fs::write(
            &interpreter,
            original.replacen("case", &format!("{conditional}case"), 1),
        )
        .unwrap();
        fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
        command(&tools, "update");
        let snapshot = fixture.wait(&tools);
        assert!(
            matches!(
                snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::WaitingForRuntime { .. },
                    ..
                }
            ),
            "{snapshot:?}"
        );
        let old = tools.invocation().unwrap();
        drop(tools);
        let tools = fixture.open();
        assert!(matches!(
            tools.snapshot().state,
            VideoToolsState::Ready {
                update_flag: UpdateFlag::WaitingForRuntime { .. },
                ..
            }
        ));
        command(&tools, "update");
        fixture.wait(&tools);
        assert_eq!(
            fixture.count("GET /yt/releases/download/2026.11.01/yt-dlp"),
            1
        );
        assert_eq!(tools.invocation().unwrap(), old);
        fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
        command(&tools, "update");
        let snapshot = fixture.wait(&tools);
        assert_eq!(selected(&tools).runtime, 2, "{snapshot:?}");
        assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    }
}

#[test]
fn second_integrity_failure_persists_skip_and_logs_until_a_newer_version() {
    for path in ["yt-dlp", "SHA2-256SUMS.sig"] {
        let fixture = Fixture::new(GOOD);
        let tools = fixture.open();
        command(&tools, "set_up");
        fixture.wait(&tools);
        let old = tools.invocation().unwrap();
        fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
        let path = format!("/yt/releases/download/2026.11.01/{path}");
        fixture.tamper(&path);
        command(&tools, "update");
        let snapshot = fixture.wait(&tools);
        assert!(
            matches!(
                snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Failed { .. },
                    ..
                }
            ),
            "{snapshot:?}"
        );
        assert_eq!(fixture.count(&format!("GET {path}")), 2);
        assert_eq!(tools.invocation().unwrap(), old);
        assert!(
            std::fs::read_to_string(fixture.config.root.join("updates.log"))
                .unwrap()
                .contains("checksum")
        );
        drop(tools);
        let tools = fixture.open();
        command(&tools, "update");
        fixture.wait(&tools);
        assert_eq!(fixture.count(&format!("GET {path}")), 2);
        fixture.publish(1, "2026.11.02", &GOOD.replace("2026.10.26", "2026.11.02"));
        command(&tools, "update");
        fixture.wait(&tools);
        assert_eq!(selected(&tools).yt_dlp, "2026.11.02");
    }
}

#[test]
fn cancellation_and_activation_failure_leave_committed_pair_usable() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let old = tools.invocation().unwrap();
    let committed = std::fs::read(fixture.config.root.join("state.json")).unwrap();
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut("/runtime-2")
        .unwrap()
        .delay = Duration::from_millis(200);
    command(&tools, "update");
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture.count("GET /runtime-2") == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    tools.cancel().unwrap();
    thread::sleep(Duration::from_millis(250));
    assert_eq!(tools.invocation().unwrap(), old);
    assert!(!fixture.config.root.join("python-runtime/2").exists());
    let before: serde_json::Value = serde_json::from_slice(&committed).unwrap();
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.config.root.join("state.json")).unwrap())
            .unwrap();
    assert_eq!(before["installed"], after["installed"]);
    // Runtime placement succeeds, script placement fails, selection must remain the full old pair.
    let blocked = fixture.config.root.join("yt-dlp/2026.11.01");
    std::fs::write(&blocked, "blocks installation").unwrap();
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(tools.invocation().unwrap(), old);
    drop(tools);
    let tools = fixture.open();
    assert_eq!(tools.invocation().unwrap(), old);
    assert!(!fixture.config.root.join("python-runtime/2").exists());
}

#[test]
fn restart_rolls_back_an_unusable_runtime_before_its_first_success() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let old = tools.invocation().unwrap();
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    std::fs::remove_file(tools.invocation().unwrap().interpreter).unwrap();
    drop(tools);
    let tools = fixture.open();
    assert_eq!(tools.invocation(), Some(old));
}

#[test]
fn transient_update_failure_retries_and_old_scripts_rotate_only_on_script_updates() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let first = tools.invocation().unwrap().script;
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    let path = "/yt/releases/download/2026.11.01/yt-dlp";
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut(path)
        .unwrap()
        .failures = 4;
    command(&tools, "update");
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Failed { .. },
            ..
        }
    ));
    assert_eq!(selected(&tools).yt_dlp, "2026.10.26");
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    let second = tools.invocation().unwrap().script;
    assert!(first.exists());
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "update");
    fixture.wait(&tools);
    assert!(first.exists() && second.exists());
    fixture.publish(2, "2026.11.02", &GOOD.replace("2026.10.26", "2026.11.02"));
    command(&tools, "update");
    fixture.wait(&tools);
    assert!(!first.exists());
    assert!(second.exists());
    // A later script update must preserve the full rollback pair until runtime probation ends.
    fixture.publish(2, "2026.11.03", &GOOD.replace("2026.10.26", "2026.11.03"));
    command(&tools, "update");
    fixture.wait(&tools);
    assert!(second.exists());
    let download = tools.begin_video_download().unwrap();
    download.finish(false).unwrap();
    assert_eq!(selected(&tools).runtime, 1);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    assert!(tools.invocation().unwrap().script.exists());
}

#[test]
fn media_execution_reports_runtime_success_and_failure_and_defers_activation() {
    use std::os::unix::fs::PermissionsExt;
    for success in [true, false] {
        let fixture = Fixture::new(GOOD);
        let tools = fixture.open();
        command(&tools, "set_up");
        fixture.wait(&tools);
        fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
        command(&tools, "update");
        fixture.wait(&tools);
        let interpreter = tools.invocation().unwrap().interpreter;
        let started = fixture._temp.path().join("download-started");
        let release = fixture._temp.path().join("release-download");
        let original = std::fs::read_to_string(&interpreter).unwrap();
        let behavior = format!(
            "*) touch '{}'; while [ ! -f '{}' ]; do sleep 0.02; done; exit {} ;;",
            started.display(),
            release.display(),
            if success { 0 } else { 1 }
        );
        std::fs::write(&interpreter, original.replace("*) exit 0 ;;", &behavior)).unwrap();
        let resources = fixture._temp.path().join("resources");
        std::fs::create_dir_all(resources.join("media-tools")).unwrap();
        for tool in ["ffmpeg", "ffprobe"] {
            let path = resources.join("media-tools").join(tool);
            std::fs::write(&path, "#!/bin/sh\necho 'ffmpeg version fixture'\n").unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let media = crate::MediaEngine::new(crate::MediaEngineConfig {
            session_folder: fixture
                ._temp
                .path()
                .join("media")
                .to_string_lossy()
                .into_owned(),
            resources_folder: Some(resources.to_string_lossy().into_owned()),
            video_tools: Some(Arc::clone(&tools)),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        })
        .unwrap();
        media
            .add_download(
                crate::MediaProbeItem {
                    webpage_url: "https://example.test/cc0".into(),
                    title: "CC0 fixture".into(),
                    thumbnail_url: None,
                    extractor: None,
                    duration_seconds: None,
                    playlist_index: None,
                },
                fixture
                    ._temp
                    .path()
                    .join("downloads")
                    .to_string_lossy()
                    .into_owned(),
                crate::MediaDownloadOptions::default(),
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !started.exists() {
            assert!(Instant::now() < deadline, "{:?}", media.snapshots());
            thread::sleep(Duration::from_millis(5));
        }
        fixture.publish(2, "2026.11.02", &GOOD.replace("2026.10.26", "2026.11.02"));
        command(&tools, "update");
        while fixture.count("GET /yt/releases/download/2026.11.02/yt-dlp") == 0 {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
        assert!(fixture.config.root.join("python-runtime/1").exists());
        std::fs::write(&release, "continue").unwrap();
        fixture.wait(&tools);
        while fixture
            .config
            .root
            .join(if success {
                "python-runtime/1"
            } else {
                "python-runtime/2"
            })
            .exists()
        {
            assert!(Instant::now() < deadline, "{:?}", tools.snapshot());
            thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(selected(&tools).runtime, if success { 2 } else { 1 });
    }
}
