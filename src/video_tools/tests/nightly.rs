use super::updates::{command, selected};
use super::*;

fn switch(enabled: bool) -> VideoToolsOperation {
    serde_json::from_value(serde_json::json!({
        "kind": "set_use_nightly", "enabled": enabled
    }))
    .unwrap()
}

#[test]
fn stable_defaults_to_stable_and_rejects_nightly_commands_without_network() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    let snapshot = serde_json::to_value(tools.snapshot()).unwrap();
    assert_eq!(snapshot["nightly_available"], false);
    assert_eq!(snapshot["use_nightly"], false);
    assert!(
        tools
            .execute(switch(true))
            .unwrap_err()
            .contains("Opul Nightly")
    );
    assert!(tools.execute(switch(false)).is_err());
    assert!(fixture.hits.lock().unwrap().is_empty());
}

fn nightly_fixture() -> Fixture {
    let mut fixture = Fixture::new(GOOD);
    fixture.config.nightly_available = true;
    fixture.publish_nightly(1, "2026.11.04.232659", GOOD);
    fixture
}

#[test]
fn nightly_defaults_to_stable_then_switches_immediately_and_can_downgrade_to_stable() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    assert!(tools.snapshot().nightly_available);
    assert!(!tools.snapshot().use_nightly);
    command(&tools, "set_up");
    fixture.wait(&tools);
    let stable = tools.invocation().unwrap();
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    let download = tools.acquire().unwrap();
    tools.execute(switch(true)).unwrap();
    assert!(tools.snapshot().use_nightly);
    let deadline = Instant::now() + Duration::from_secs(10);
    while fixture.count("GET /nightly/releases/download/2026.11.04.232659/yt-dlp") == 0 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(tools.invocation().unwrap(), stable);
    assert!(stable.script.exists());
    drop(download);
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Nightly);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.04.232659");
    let nightly = tools.invocation().unwrap();
    assert_ne!(nightly.script, stable.script);
    for asset in ["yt-dlp", "SHA2-256SUMS", "SHA2-256SUMS.sig"] {
        assert_eq!(
            fixture.count(&format!(
                "GET /nightly/releases/download/2026.11.04.232659/{asset}"
            )),
            1
        );
    }
    assert!(stable.script.exists());
    tools.execute(switch(false)).unwrap();
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Stable);
    assert_eq!(selected(&tools).yt_dlp, "2026.10.26");
    assert!(nightly.script.exists());
    assert!(!tools.snapshot().keep_up_to_date);
}

#[test]
fn switch_preference_persists_without_setup() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    tools.execute(switch(true)).unwrap();
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    assert!(fixture.hits.lock().unwrap().is_empty());
    fixture.close(tools);
    let tools = fixture.open();
    assert!(tools.snapshot().use_nightly);
    assert!(!tools.snapshot().keep_up_to_date);
    command(&tools, "set_up");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Nightly);
    assert_eq!(fixture.count("GET /yt/releases/latest"), 0);
}

#[test]
fn invalid_nightly_signature_keeps_ready_and_the_previous_verified_pair() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let previous = tools.invocation().unwrap();
    fixture.tamper("/nightly/releases/download/2026.11.04.232659/SHA2-256SUMS.sig");
    tools.execute(switch(true)).unwrap();
    let snapshot = fixture.wait(&tools);
    assert!(matches!(
        snapshot.state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Failed { .. },
            ..
        }
    ));
    assert_eq!(tools.invocation().unwrap(), previous);
    assert_eq!(
        fixture.count("GET /nightly/releases/download/2026.11.04.232659/SHA2-256SUMS.sig"),
        2
    );
    assert_eq!(
        fixture.count("GET /nightly/releases/download/2026.11.04.232659/yt-dlp"),
        0
    );
}

#[test]
fn failed_nightly_health_keeps_the_old_pair_until_a_new_runtime_passes() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let previous = tools.invocation().unwrap();
    let interpreter = std::fs::read_to_string(&previous.interpreter).unwrap();
    std::fs::write(
        &previous.interpreter,
        interpreter.replace("curl_cffi", "curl_cffi (unsupported)"),
    )
    .unwrap();
    tools.execute(switch(true)).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::WaitingForRuntime { .. },
            ..
        }
    ));
    assert_eq!(tools.invocation().unwrap(), previous);
    fixture.publish_nightly(2, "2026.11.04.232659", GOOD);
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Nightly);
}

#[test]
fn switching_during_setup_installs_the_selected_channel() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    command(&tools, "set_up");
    tools.execute(switch(true)).unwrap();
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Nightly);
}

#[test]
fn stable_refuses_a_nightly_pair_even_with_a_pending_switch_back_to_stable() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    tools.execute(switch(true)).unwrap();
    command(&tools, "set_up");
    fixture.wait(&tools);
    let active = tools.acquire().unwrap();
    tools.execute(switch(false)).unwrap();
    tools.shut_down();
    drop(active);
    fixture.close(tools);
    let mut config = fixture.config.clone();
    config.nightly_available = false;
    let error = VideoTools::open_with_sources(config, fixture.sources.clone()).unwrap_err();
    assert!(error.contains("Opul Nightly"));
}

#[test]
fn stable_and_nightly_schedules_check_only_their_own_sources() {
    let nightly = nightly_fixture();
    let stable = Fixture::new(GOOD);
    let nightly_tools = nightly.open();
    let stable_tools = stable.open();
    nightly_tools.execute(switch(true)).unwrap();
    for (fixture, tools) in [(&nightly, &nightly_tools), (&stable, &stable_tools)] {
        tools
            .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
            .unwrap();
        command(tools, "set_up");
        fixture.wait(tools);
    }
    let deadline = Instant::now() + Duration::from_secs(10);
    while nightly.network.scheduled.lock().unwrap().is_empty() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    nightly
        .network
        .clock
        .send_modify(|now| *now += Duration::from_secs(60));
    while nightly.count("GET /pointer") < 2 {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(2));
    }
    nightly.wait(&nightly_tools);
    assert_eq!(stable.count("GET /pointer"), 1);
    assert_eq!(nightly.count("GET /nightly/releases/latest"), 2);
    assert_eq!(nightly.count("GET /yt/releases/latest"), 0);
    assert_eq!(stable.count("GET /nightly/releases/latest"), 0);
}

#[test]
fn opul_nightly_accepts_the_runtime_for_its_base_app_version() {
    let mut fixture = nightly_fixture();
    fixture.config.opul_version = "0.2.2-nightly.20261108.1".into();
    let tools = fixture.open();
    command(&tools, "set_up");
    let snapshot = fixture.wait(&tools);
    assert!(
        matches!(snapshot.state, VideoToolsState::Ready { .. }),
        "{snapshot:?}"
    );
}
