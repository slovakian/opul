use super::updates::{command, selected};
use super::*;

#[test]
fn newer_app_runtime_does_not_block_a_compatible_script_update() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    fixture.minimum_opul_version("9.0.0");
    command(&tools, "check_for_updates");
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Available { .. },
            ..
        }
    ));
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 1);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    assert_eq!(fixture.count("GET /runtime-2"), 0);
}

#[test]
fn unpack_filesystem_failure_retries_without_skipping_a_verified_runtime() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let usage = tools.acquire().unwrap();
    fixture.publish(2, "2026.10.26", GOOD);
    command(&tools, "update");
    let deadline = Instant::now() + Duration::from_secs(5);
    while fixture.count("GET /runtime-2.sig") == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let staging = std::fs::read_dir(&fixture.config.root)
        .unwrap()
        .find_map(|entry| {
            let entry = entry.unwrap();
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".install-")
                .then(|| entry.path())
        })
        .unwrap();
    std::fs::write(staging.join("runtime"), "blocks unpack directory").unwrap();
    drop(usage);
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Failed { .. },
            ..
        }
    ));
    assert_eq!(selected(&tools).runtime, 1);
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    assert_eq!(fixture.count("GET /runtime-2"), 2);
}

#[test]
fn active_probe_retains_its_pair_and_never_counts_as_download_success() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let interpreter = tools.invocation().unwrap().interpreter;
    let started = fixture._temp.path().join("probe-started");
    let release = fixture._temp.path().join("release-probe");
    let original = std::fs::read_to_string(&interpreter).unwrap();
    let behavior = format!(
        "case \"$*\" in *--dump-single-json*) touch '{}'; while [ ! -f '{}' ]; do sleep 0.02; done ;; esac\n",
        started.display(),
        release.display()
    );
    std::fs::write(
        &interpreter,
        original.replacen("case", &format!("{behavior}case"), 1),
    )
    .unwrap();
    let media = crate::MediaEngine::new(crate::MediaEngineConfig {
        session_folder: fixture
            ._temp
            .path()
            .join("media")
            .to_string_lossy()
            .into_owned(),
        resources_folder: None,
        video_tools: Some(Arc::clone(&tools)),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    let probe = thread::spawn(move || media.probe_url("https://example.test/cc0".into()).unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !started.exists() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "update");
    while fixture.count("GET /runtime-2.sig") == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(selected(&tools).runtime, 1);
    assert!(interpreter.exists());
    std::fs::write(&release, "continue").unwrap();
    assert_eq!(probe.join().unwrap().title, "Fixture video");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    // Dropping a successful probe does not clear the new runtime's probation.
    drop(tools.acquire().unwrap());
    assert!(interpreter.exists());
    tools.acquire().unwrap().finish_download(true).unwrap();
    assert!(!interpreter.exists());
}

#[test]
fn app_upgrade_rechecks_a_hold_when_the_signed_runtime_pointer_is_unchanged() {
    let mut fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let interpreter = tools.invocation().unwrap().interpreter;
    let original = std::fs::read_to_string(&interpreter).unwrap();
    std::fs::write(
        &interpreter,
        original.replacen(
            "case",
            "case \"$(cat \"$2\")\" in 2026.11.01) exit 1 ;; esac\ncase",
            1,
        ),
    )
    .unwrap();
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    fixture.minimum_opul_version("0.3.0");
    command(&tools, "update");
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::WaitingForRuntime { .. },
            ..
        }
    ));
    assert_eq!(fixture.count("GET /runtime-2"), 0);
    drop(tools);
    fixture.config.opul_version = "0.3.0".into();
    let tools = fixture.open();
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
}

#[test]
fn twice_rejected_runtime_is_skipped_across_restart_until_a_newer_runtime() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    fixture.publish(2, "2026.10.26", GOOD);
    fixture.tamper("/runtime-2.sig");
    command(&tools, "update");
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Failed { .. },
            ..
        }
    ));
    assert_eq!(fixture.count("GET /runtime-2"), 2);
    drop(tools);
    let tools = fixture.open();
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(fixture.count("GET /runtime-2"), 2);
    assert_eq!(selected(&tools).runtime, 1);
    fixture.publish(3, "2026.10.26", GOOD);
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 3);
}
