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
fn switch_preference_persists_without_setup_and_is_isolated_from_stable() {
    let fixture = nightly_fixture();
    let stable_fixture = Fixture::new(GOOD);
    let stable = stable_fixture.open();
    let tools = fixture.open();
    tools.execute(switch(true)).unwrap();
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    assert!(fixture.hits.lock().unwrap().is_empty());
    drop(tools);
    let tools = fixture.open();
    assert!(tools.snapshot().use_nightly);
    assert!(!tools.snapshot().keep_up_to_date);
    assert!(!stable.snapshot().use_nightly);
    assert!(stable.snapshot().keep_up_to_date);
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
fn switching_during_setup_cancels_the_old_job_and_installs_the_selected_channel() {
    let fixture = nightly_fixture();
    let tools = fixture.open();
    command(&tools, "set_up");
    tools.execute(switch(true)).unwrap();
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp_channel, YtDlpChannel::Nightly);
}
