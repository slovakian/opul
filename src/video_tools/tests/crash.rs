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
    drop(tools);
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
