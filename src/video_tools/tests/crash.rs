use super::*;
use std::process::{Command, Stdio};

#[test]
fn crash_before_activation_preserves_committed_state_and_restart_cleans_candidates() {
    const ROOT: &str = "OPUL_FIXTURE_CRASH_TOOLS_ROOT";
    const MARKER: &str = "OPUL_FIXTURE_CRASH_MARKER";
    if let Some(root) = std::env::var_os(ROOT) {
        let mut fixture = Fixture::new(GOOD);
        fixture.config.root = root.into();
        fixture
            .routes
            .lock()
            .unwrap()
            .get_mut("/yt/releases/download/2026.10.26/yt-dlp")
            .unwrap()
            .delay = Duration::from_secs(60);
        let tools = fixture.open();
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while fixture.count("GET /yt/releases/download/2026.10.26/yt-dlp") == 0 {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(2));
        }
        std::fs::write(
            std::env::var_os(MARKER).unwrap(),
            b"candidate staged, state not activated",
        )
        .unwrap();
        loop {
            thread::park();
        }
    }
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    let committed = std::fs::read(fixture.config.root.join("state.json")).unwrap();
    fixture.close(tools);
    let marker = fixture._temp.path().join("crash-marker");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "video_tools::tests::crash::crash_before_activation_preserves_committed_state_and_restart_cleans_candidates", "--nocapture"])
        .env(ROOT, &fixture.config.root)
        .env(MARKER, &marker)
        .env("TMPDIR", fixture._temp.path())
        .stdout(Stdio::null()).stderr(Stdio::inherit()).spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !marker.exists() {
        if Instant::now() >= deadline || child.try_wait().unwrap().is_some() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("child did not reach the pre-activation crash point");
        }
        thread::sleep(Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    assert_eq!(
        std::fs::read(fixture.config.root.join("state.json")).unwrap(),
        committed
    );
    assert!(
        std::fs::read_dir(&fixture.config.root)
            .unwrap()
            .any(|entry| entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-"))
    );
    let reopened = fixture.open();
    assert!(matches!(
        reopened.snapshot().state,
        VideoToolsState::NotSetUp
    ));
    assert!(reopened.invocation().is_none());
    assert!(!reopened.snapshot().keep_up_to_date);
    assert!(
        std::fs::read_dir(&fixture.config.root)
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-"))
    );
    assert!(fixture.hits.lock().unwrap().is_empty());
    reopened.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&reopened).state,
        VideoToolsState::Ready { .. }
    ));
    let selected = reopened.invocation().unwrap();
    let committed = std::fs::read(fixture.config.root.join("state.json")).unwrap();
    reopened.execute(VideoToolsOperation::CancelSetup).unwrap();
    assert_eq!(reopened.invocation(), Some(selected));
    assert_eq!(
        std::fs::read(fixture.config.root.join("state.json")).unwrap(),
        committed
    );
}

#[test]
fn update_crash_never_exposes_a_mixed_pair() {
    const ROOT: &str = "OPUL_FIXTURE_UPDATE_CRASH_ROOT";
    const MARKER: &str = "OPUL_FIXTURE_UPDATE_CRASH_MARKER";
    if let Some(root) = std::env::var_os(ROOT) {
        let mut fixture = Fixture::new(GOOD);
        fixture.config.root = root.into();
        let tools = fixture.open();
        fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
        let path = "/yt/releases/download/2026.11.01/yt-dlp";
        fixture.routes.lock().unwrap().get_mut(path).unwrap().delay = Duration::from_secs(60);
        tools.execute(VideoToolsOperation::Update).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while fixture.count(&format!("GET {path}")) == 0 {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(2));
        }
        std::fs::write(
            std::env::var_os(MARKER).unwrap(),
            b"runtime unpacked; script pending",
        )
        .unwrap();
        loop {
            thread::park();
        }
    }
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    fixture.wait(&tools);
    let old = tools.invocation().unwrap();
    fixture.close(tools);
    let marker = fixture._temp.path().join("update-crash-marker");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "video_tools::tests::crash::update_crash_never_exposes_a_mixed_pair",
            "--nocapture",
        ])
        .env(ROOT, &fixture.config.root)
        .env(MARKER, &marker)
        .env("TMPDIR", fixture._temp.path())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(15);
    while !marker.exists() {
        if Instant::now() >= deadline || child.try_wait().unwrap().is_some() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("update child did not reach the pre-activation crash point");
        }
        thread::sleep(Duration::from_millis(5));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let reopened = fixture.open();
    assert_eq!(reopened.invocation(), Some(old));
    assert!(!fixture.config.root.join("python-runtime/2").exists());
    assert!(!fixture.config.root.join("yt-dlp/2026.11.01").exists());
    assert!(
        std::fs::read_dir(&fixture.config.root)
            .unwrap()
            .all(|entry| !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(".install-"))
    );
}
