use super::updates::{command, selected};
use super::*;

#[track_caller]
fn until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        assert!(Instant::now() < deadline, "scheduled work did not finish");
        thread::sleep(Duration::from_millis(2));
    }
}

fn advance(fixture: &Fixture, seconds: u64) {
    fixture
        .network
        .clock
        .send_modify(|now| *now += Duration::from_secs(seconds));
}

#[test]
fn automatic_checks_start_after_a_minute_and_repeat_daily() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    until(|| !fixture.network.scheduled.lock().unwrap().is_empty());
    advance(&fixture, 59);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.count("GET /pointer"), 1);
    advance(&fixture, 1);
    until(|| selected(&tools).runtime == 2);
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    until(|| fixture.network.scheduled.lock().unwrap().len() >= 2);
    let checks = fixture.count("GET /pointer");
    advance(&fixture, 86_399);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.count("GET /pointer"), checks);
    advance(&fixture, 1);
    until(|| fixture.count("GET /pointer") > checks);
    fixture.wait(&tools);
}

#[test]
fn consent_checks_never_install_and_notices_survive_restart() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    until(|| !fixture.network.scheduled.lock().unwrap().is_empty());
    advance(&fixture, 60);
    until(|| fixture.count("GET /pointer") == 2);
    fixture.wait(&tools);
    assert_eq!(selected(&tools).yt_dlp, "2026.10.26");
    assert_eq!(
        fixture.count("GET /yt/releases/download/2026.11.01/yt-dlp"),
        0
    );
    let notice = tools.take_update_notice().unwrap().unwrap();
    assert_eq!(notice.version, "2026.11.01");
    assert!(tools.take_update_notice().unwrap().is_none());
    command(&tools, "check_for_updates");
    fixture.wait(&tools);
    assert!(tools.take_update_notice().unwrap().is_none());
    drop(tools);
    let tools = fixture.open();
    assert!(!tools.snapshot().keep_up_to_date);
    assert!(matches!(
        tools.snapshot().state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Available { .. },
            ..
        }
    ));
    assert!(tools.take_update_notice().unwrap().is_none());
    fixture.publish(2, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    command(&tools, "check_for_updates");
    fixture.wait(&tools);
    let runtime_notice = tools.take_update_notice().unwrap().unwrap();
    assert_ne!(runtime_notice.identity, notice.identity);
    command(&tools, "update");
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    assert!(tools.take_update_notice().unwrap().is_none());
}

#[test]
fn ineligible_states_do_not_schedule_or_request_updates() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    for turned_off in [false, true] {
        if turned_off {
            command(&tools, "turn_off");
        }
        advance(&fixture, 86_460);
        thread::sleep(Duration::from_millis(20));
        assert!(fixture.hits.lock().unwrap().is_empty());
        assert!(fixture.network.scheduled.lock().unwrap().is_empty());
    }
    fixture.tamper("/runtime-1/runtime.sig");
    command(&tools, "set_up");
    fixture.wait(&tools);
    let hits = fixture.hits.lock().unwrap().len();
    advance(&fixture, 86_400);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.hits.lock().unwrap().len(), hits);
    assert!(fixture.network.scheduled.lock().unwrap().is_empty());
}

#[test]
fn check_failures_are_silent_and_wait_for_the_next_day() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    let before = fixture.wait(&tools).state;
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut("/pointer")
        .unwrap()
        .failure_status = 404;
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut("/pointer")
        .unwrap()
        .failures = 1;
    until(|| !fixture.network.scheduled.lock().unwrap().is_empty());
    advance(&fixture, 60);
    until(|| fixture.count("GET /pointer") == 2);
    assert_eq!(fixture.wait(&tools).state, before);
    assert!(tools.take_update_notice().unwrap().is_none());
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    advance(&fixture, 86_399);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.count("GET /pointer"), 2);
    advance(&fixture, 1);
    until(|| selected(&tools).yt_dlp == "2026.11.01");
    fixture.wait(&tools);
}

#[test]
fn automatic_install_failure_retries_daily_and_preference_revokes_pending_install() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    let script = "/yt/releases/download/2026.11.01/yt-dlp";
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut(script)
        .unwrap()
        .failure_status = 404;
    fixture
        .routes
        .lock()
        .unwrap()
        .get_mut(script)
        .unwrap()
        .failures = 1;
    until(|| !fixture.network.scheduled.lock().unwrap().is_empty());
    advance(&fixture, 60);
    until(|| fixture.count(&format!("GET {script}")) == 1);
    let failed = fixture.wait(&tools);
    assert!(matches!(
        failed.state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Failed { .. },
            ..
        }
    ));
    assert!(failed.update_available);
    advance(&fixture, 86_400);
    until(|| selected(&tools).yt_dlp == "2026.11.01");
    fixture.wait(&tools);
    let usage = tools.acquire().unwrap();
    fixture.publish(2, "2026.11.02", &GOOD.replace("2026.10.26", "2026.11.02"));
    advance(&fixture, 86_400);
    until(|| {
        matches!(
            tools.snapshot().state,
            VideoToolsState::Ready {
                update_flag: UpdateFlag::Updating { .. },
                ..
            }
        )
    });
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    drop(usage);
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 1);
    assert_eq!(selected(&tools).yt_dlp, "2026.11.01");
    assert!(tools.take_update_notice().unwrap().is_some());
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: true })
        .unwrap();
    fixture.wait(&tools);
    assert_eq!(selected(&tools).runtime, 2);
    assert!(tools.take_update_notice().unwrap().is_none());
}

#[test]
fn removing_tools_and_shutdown_cancel_scheduled_work() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    command(&tools, "set_up");
    fixture.wait(&tools);
    until(|| !fixture.network.scheduled.lock().unwrap().is_empty());
    tools.remove_with(true, || Ok(())).unwrap();
    let requests = fixture.hits.lock().unwrap().len();
    advance(&fixture, 86_460);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.hits.lock().unwrap().len(), requests);
    assert!(matches!(tools.snapshot().state, VideoToolsState::TurnedOff));
    command(&tools, "set_up");
    fixture.wait(&tools);
    tools.shut_down();
    let requests = fixture.hits.lock().unwrap().len();
    advance(&fixture, 172_800);
    thread::sleep(Duration::from_millis(20));
    assert_eq!(fixture.hits.lock().unwrap().len(), requests);
}
