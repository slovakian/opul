use super::*;
use ed25519_dalek::SigningKey;
use std::{
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

#[test]
fn consent_and_immediate_preferences_survive_restart() {
    let temp = tempfile::tempdir().unwrap();
    let config = VideoToolsConfig {
        nightly_available: false,
        root: temp.path().join("Tools"),
        opul_version: "0.2.2".into(),
    };
    let tools = VideoTools::open(config.clone()).unwrap();
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
    assert!(tools.invocation().is_none());
    assert!(tools.snapshot().keep_up_to_date);
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    tools.execute(VideoToolsOperation::TurnOff).unwrap();
    assert!(matches!(tools.snapshot().state, VideoToolsState::TurnedOff));
    drop(tools);
    let reopened = VideoTools::open(config).unwrap();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::TurnedOff
    ));
    assert!(!reopened.snapshot().keep_up_to_date);
    assert!(reopened.invocation().is_none());
}

mod fixture;
mod fixture_release;
mod network;
mod nightly;
use fixture::*;

const GOOD: &str = "[debug] yt-dlp version stable@2026.10.26 from fixture\n[debug] Python 3.13.18 (CPython)\n[debug] Request Handlers: urllib, curl_cffi\n[debug] JS runtimes: quickjs-2026-06-04";

#[test]
fn setup_verifies_activates_and_supplies_media_engine_without_restart() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    let mut snapshots = tools.subscribe();
    thread::sleep(Duration::from_millis(20));
    assert!(
        fixture.hits.lock().unwrap().is_empty(),
        "opening tools must not use the network"
    );
    assert!(tools.invocation().is_none());
    // The existing MediaEngine is opened before Setup, then reads the selected pair dynamically.
    let media = crate::MediaEngine::new(crate::MediaEngineConfig {
        session_folder: fixture
            ._temp
            .path()
            .join("media")
            .to_string_lossy()
            .into_owned(),
        resources_folder: None,
        video_tools: Arc::clone(&tools),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    assert!(
        media
            .probe_url("https://example.test/video".into())
            .is_err()
    );
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    let mut observed = Vec::new();
    while matches!(tools.snapshot().state, VideoToolsState::SettingUp { .. }) {
        if snapshots.has_changed().unwrap() {
            observed.push(snapshots.borrow_and_update().clone());
        }
        thread::sleep(Duration::from_millis(1));
    }
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
    assert!(snapshot.published_size.unwrap() > 0);
    assert!(observed.iter().any(|s| matches!(&s.state, VideoToolsState::SettingUp { steps } if steps.len() == 2 && steps[0].component == Component::Python && steps[1].component == Component::YtDlp)));
    let probe = media
        .probe_url("https://example.test/video".into())
        .unwrap();
    assert_eq!(probe.title, "Fixture video");
    let arguments = std::fs::read_to_string(fixture._temp.path().join("executed")).unwrap();
    let selected = tools.invocation().unwrap();
    assert!(arguments.contains(&format!("-I\n{}\n", selected.script.display())));
    assert!(arguments.contains(&format!(
        "--no-js-runtimes\n--js-runtimes\nquickjs:{}\n",
        selected.quickjs.display()
    )));
    assert!(arguments.contains("--impersonate\nchrome\n"));
    assert!(arguments.contains("--dump-single-json"));
    assert!(fixture.config.root.join("state.json").is_file());
    drop(media);
    drop(tools);
    let reopened = fixture.open();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::Ready { .. }
    ));
}

#[test]
fn tampered_inputs_are_retried_once_and_never_executed_or_selected() {
    for path in [
        "/pointer",
        "/pointer.sig",
        "/runtime-1/runtime",
        "/runtime-1/runtime.sig",
        "/yt/releases/download/2026.10.26/SHA2-256SUMS",
        "/yt/releases/download/2026.10.26/SHA2-256SUMS.sig",
        "/yt/releases/download/2026.10.26/yt-dlp",
    ] {
        let fixture = Fixture::new(GOOD);
        fixture.tamper(path);
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        let snapshot = fixture.wait(&tools);
        assert!(
            matches!(
                snapshot.state,
                VideoToolsState::NotWorking {
                    reason: FailureReason::Verification,
                    ..
                }
            ),
            "{path}: {snapshot:?}"
        );
        assert_eq!(fixture.count(&format!("GET {path}")), 2, "{path}");
        assert!(tools.invocation().is_none());
        assert!(!fixture._temp.path().join("executed").exists());
        assert!(!fixture.config.root.join("python-runtime/1").exists());
    }
}

#[test]
fn health_rejects_deprecated_python_unsupported_curl_missing_js_and_wrong_versions() {
    for output in [
        GOOD.replace(
            "3.13.18",
            "3.9.0\nDeprecated Feature: Support for Python 3.9 has been deprecated",
        ),
        GOOD.replace("urllib, curl_cffi", "urllib, curl_cffi (unsupported)"),
        GOOD.replace("quickjs-2026-06-04", "none"),
        GOOD.replace("stable@2026.10.26", "stable@2026.01.01"),
    ] {
        let fixture = Fixture::new(&output);
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        let snapshot = fixture.wait(&tools);
        assert!(
            matches!(
                snapshot.state,
                VideoToolsState::NotWorking {
                    reason: FailureReason::HealthCheck,
                    ..
                }
            ),
            "{snapshot:?}"
        );
        assert!(tools.invocation().is_none());
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture.config.root.join("state.json")).unwrap())
                .unwrap();
        assert!(stored["installed"].is_null());
    }
}

#[test]
fn cancellation_cannot_activate_and_settings_setup_reenables_turned_off_tools() {
    let fixture = Fixture::new(GOOD);
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut("/runtime-1/runtime")
        .unwrap()
        .delay = Duration::from_millis(200);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::TurnOff).unwrap();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while fixture.count("GET /runtime-1/runtime") == 0 {
        assert!(Instant::now() < deadline, "{:?}", tools.snapshot());
        thread::sleep(Duration::from_millis(2));
    }
    tools.execute(VideoToolsOperation::CancelSetup).unwrap();
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
    thread::sleep(Duration::from_millis(230));
    assert!(tools.invocation().is_none());
    tools.execute(VideoToolsOperation::Retry).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
}

#[test]
fn minimum_app_version_and_restart_validation_protect_the_selected_pair() {
    let mut fixture = Fixture::new(GOOD);
    fixture.config.opul_version = "0.2.1".into();
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::NotWorking { .. }
    ));
    assert_eq!(fixture.count("GET /runtime-1/runtime"), 0);
    drop(tools);
    fixture.config.opul_version = "0.2.2".into();
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    std::fs::remove_file(tools.invocation().unwrap().quickjs).unwrap();
    drop(tools);
    let broken = fixture.open();
    assert!(matches!(
        broken.snapshot().state,
        VideoToolsState::NotWorking {
            reason: FailureReason::StoppedWorking,
            ..
        }
    ));
    assert!(broken.invocation().is_none());
}

#[test]
fn metadata_size_does_not_download_executables_or_grant_consent() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools
        .execute(VideoToolsOperation::RefreshPublishedSize)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while tools.snapshot().published_size.is_none() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
    assert_eq!(fixture.count("GET /runtime-1/runtime"), 0);
    assert_eq!(
        fixture.count("GET /yt/releases/download/2026.10.26/yt-dlp"),
        0
    );
    assert_eq!(
        fixture.count("HEAD /yt/releases/download/2026.10.26/yt-dlp"),
        1
    );
}

#[test]
fn activation_is_atomic_and_cancelled_staging_is_not_selected_after_restart() {
    let fixture = Fixture::new(GOOD);
    let script_path = "/yt/releases/download/2026.10.26/yt-dlp";
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut(script_path)
        .unwrap()
        .delay = Duration::from_millis(200);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while fixture.count(&format!("GET {script_path}")) == 0 {
        assert!(Instant::now() < deadline, "{:?}", tools.snapshot());
        thread::sleep(Duration::from_millis(2));
    }
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    let stored: serde_json::Value =
        serde_json::from_slice(&std::fs::read(fixture.config.root.join("state.json")).unwrap())
            .unwrap();
    assert!(stored["installed"].is_null());
    assert_eq!(stored["keep_up_to_date"], false);
    assert!(tools.invocation().is_none());
    assert!(!fixture.config.root.join("python-runtime/1").exists());
    tools.execute(VideoToolsOperation::CancelSetup).unwrap();
    let weak = Arc::downgrade(&tools);
    drop(tools);
    while weak.strong_count() > 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    let stale = fixture.config.root.join(".install-crash");
    std::fs::create_dir(&stale).unwrap();
    std::fs::write(stale.join("unverified"), b"partial archive").unwrap();
    let reopened = fixture.open();
    assert!(!stale.exists());
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::NotSetUp
    ));
    assert!(!reopened.snapshot().keep_up_to_date);
    reopened.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&reopened).state,
        VideoToolsState::Ready { .. }
    ));
    assert!(!reopened.snapshot().keep_up_to_date);
    let selected = reopened.invocation().unwrap();
    assert!(selected.interpreter.is_file());
    assert!(selected.script.is_file());
    assert!(!selected.interpreter.to_string_lossy().contains(".install-"));
}

#[test]
fn signatures_from_other_keys_are_rejected() {
    let mut fixture = Fixture::new(GOOD);
    fixture.sources.opul_key = SigningKey::from_bytes(&[10; 32]).verifying_key().to_bytes();
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::NotWorking {
            reason: FailureReason::Verification,
            ..
        }
    ));
    assert!(!fixture._temp.path().join("executed").exists());
}

#[test]
fn activation_failure_preserves_the_committed_state_and_retry_selects_a_complete_pair() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    let state_path = fixture.config.root.join("state.json");
    let committed = std::fs::read(&state_path).unwrap();
    // Force a real filesystem error after runtime placement, before script placement/state commit.
    let blocked = fixture.config.root.join("yt-dlp");
    std::fs::write(&blocked, b"fixture blocks the script destination").unwrap();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    let snapshot = fixture.wait(&tools);
    assert!(
        matches!(snapshot.state, VideoToolsState::NotWorking { .. }),
        "{snapshot:?}"
    );
    assert!(
        fixture
            .config
            .root
            .join("python-runtime/1/bin/python3")
            .is_file()
    );
    assert_eq!(std::fs::read(&state_path).unwrap(), committed);
    assert!(tools.invocation().is_none());
    std::fs::remove_file(blocked).unwrap();
    tools.execute(VideoToolsOperation::Retry).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    let selected = tools.invocation().unwrap();
    assert!(
        selected.interpreter.is_file() && selected.script.is_file() && selected.quickjs.is_file()
    );
    assert!(!tools.snapshot().keep_up_to_date);
}

mod recovery;

mod crash;

mod waiting_downloads;

mod updates;

mod update_regressions;

mod removal;

mod schedule;
