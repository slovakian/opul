use super::*;

#[test]
fn removal_revokes_selection_before_stopping_and_preserves_preferences_across_restart() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    fixture.wait(&tools);
    let interpreter = tools.invocation().unwrap().interpreter;
    tools
        .execute(VideoToolsOperation::SetKeepUpToDate { enabled: false })
        .unwrap();
    tools
        .remove_with(true, || {
            assert!(tools.invocation().is_none());
            assert!(
                interpreter.exists(),
                "stop callback runs before files are deleted"
            );
            assert!(tools.execute(VideoToolsOperation::SetUp).is_err());
            Ok(())
        })
        .unwrap();
    assert!(!interpreter.exists());
    assert!(matches!(tools.snapshot().state, VideoToolsState::TurnedOff));
    tools
        .execute(VideoToolsOperation::Remove { turn_off: false })
        .unwrap();
    drop(tools);
    let tools = fixture.open();
    assert!(matches!(tools.snapshot().state, VideoToolsState::TurnedOff));
    assert!(!tools.snapshot().keep_up_to_date);
    let hits = fixture.hits.lock().unwrap().len();
    tools.execute(VideoToolsOperation::CheckForUpdates).unwrap();
    tools.execute(VideoToolsOperation::Update).unwrap();
    thread::sleep(Duration::from_millis(30));
    assert_eq!(fixture.hits.lock().unwrap().len(), hits);
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
}

#[test]
fn removal_cancels_setup_and_update_without_late_activation_or_channel_interference() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    tools
        .execute(VideoToolsOperation::Remove { turn_off: false })
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(tools.lock().stored.installed.is_none());
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    fixture.wait(&tools);
    let sibling = Fixture::new(GOOD);
    let nightly = sibling.open();
    nightly.execute(VideoToolsOperation::SetUp).unwrap();
    sibling.wait(&nightly);
    let retained = nightly.invocation().unwrap().interpreter;
    tools.execute(VideoToolsOperation::Update).unwrap();
    tools
        .execute(VideoToolsOperation::Remove { turn_off: false })
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(tools.lock().stored.installed.is_none());
    assert!(retained.exists());
    assert!(nightly.invocation().is_some());
}

#[test]
fn removal_failure_keeps_cleanup_actionable_and_retries_preserve_turn_off() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    fixture.wait(&tools);
    let interpreter = tools.invocation().unwrap().interpreter;
    assert!(
        tools
            .remove_with(true, || Err("writer did not stop".into()))
            .is_err()
    );
    assert!(interpreter.exists());
    assert!(matches!(
        tools.snapshot().state,
        VideoToolsState::NotWorking { .. }
    ));
    tools
        .execute(VideoToolsOperation::Remove { turn_off: false })
        .unwrap();
    assert!(!interpreter.exists());
    assert!(matches!(tools.snapshot().state, VideoToolsState::TurnedOff));
}

#[test]
fn removal_cancels_verified_update_waiting_for_an_active_download() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    fixture.wait(&tools);
    let usage = tools.acquire().unwrap();
    fixture.publish(1, "2026.11.01", &GOOD.replace("2026.10.26", "2026.11.01"));
    tools.execute(VideoToolsOperation::Update).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if matches!(
            tools.snapshot().state,
            VideoToolsState::Ready {
                update_flag: UpdateFlag::Updating { progress: 80, .. },
                ..
            }
        ) && tools.lock().active_users == 1
        {
            break;
        }
        assert!(Instant::now() < deadline, "{:?}", tools.snapshot());
        thread::sleep(Duration::from_millis(10));
    }
    assert!(usage.invocation.interpreter.exists());
    tools
        .remove_with(false, || {
            drop(usage);
            Ok(())
        })
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(tools.lock().stored.installed.is_none());
    assert!(
        !fixture
            .config
            .root
            .join("yt-dlp/2026.11.01/yt-dlp")
            .exists()
    );
    assert!(matches!(tools.snapshot().state, VideoToolsState::NotSetUp));
}
