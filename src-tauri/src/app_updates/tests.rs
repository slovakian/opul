use super::platform::{describe, detect_install_blocker, means_up_to_date};
use super::*;
use std::path::Path;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::Instant;
use tokio::sync::{Semaphore, oneshot};

type CheckResult = Result<Option<String>, String>;

struct FakeSource {
    checks: AtomicUsize,
    check_result: Mutex<CheckResult>,
    /// Checks wait for a permit, so tests can hold one in flight.
    check_permits: Semaphore,
    installs: Mutex<Vec<String>>,
    install_result: Mutex<Result<(), String>>,
}

impl FakeSource {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            checks: AtomicUsize::new(0),
            check_result: Mutex::new(Ok(None)),
            check_permits: Semaphore::new(1_000),
            installs: Mutex::new(Vec::new()),
            install_result: Mutex::new(Ok(())),
        })
    }

    fn publish(&self, result: CheckResult) {
        *self.check_result.lock().unwrap() = result;
    }

    fn checks(&self) -> usize {
        self.checks.load(Ordering::SeqCst)
    }

    fn installs(&self) -> Vec<String> {
        self.installs.lock().unwrap().clone()
    }
}

impl UpdateSource for FakeSource {
    fn check(&self) -> BoxFuture<'_, CheckResult> {
        Box::pin(async move {
            self.checks.fetch_add(1, Ordering::SeqCst);
            self.check_permits.acquire().await.unwrap().forget();
            self.check_result.lock().unwrap().clone()
        })
    }

    fn install(
        &self,
        version: String,
        mut progress: Progress,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            self.installs.lock().unwrap().push(version);
            progress(50, Some(100));
            progress(100, Some(100));
            self.install_result.lock().unwrap().clone()
        })
    }
}

#[derive(Default)]
struct FakeClock {
    now: AtomicU64,
    sleeps: Mutex<Vec<(Duration, oneshot::Sender<()>)>>,
}

impl Clock for FakeClock {
    fn unix_now(&self) -> u64 {
        self.now.load(Ordering::SeqCst)
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        let (wake, woken) = oneshot::channel();
        self.sleeps.lock().unwrap().push((duration, wake));
        Box::pin(async move {
            let _ = woken.await;
        })
    }
}

impl FakeClock {
    /// Waits for the scheduler to sleep, checks how long it asked for, then wakes it.
    fn elapse(&self, expected: Duration) {
        wait_for(|| !self.sleeps.lock().unwrap().is_empty());
        let (duration, wake) = self.sleeps.lock().unwrap().remove(0);
        assert_eq!(duration, expected);
        self.now.fetch_add(duration.as_secs(), Ordering::SeqCst);
        wake.send(()).unwrap();
    }
}

fn wait_for(condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting");
        std::thread::sleep(Duration::from_millis(5));
    }
}

struct Harness {
    updates: Arc<AppUpdates>,
    source: Arc<FakeSource>,
    clock: Arc<FakeClock>,
    published: Arc<Mutex<Vec<AppUpdatesSnapshot>>>,
    directory: tempfile::TempDir,
}

impl Harness {
    fn new(blocker: Option<InstallBlocker>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let source = FakeSource::new();
        let clock = Arc::new(FakeClock::default());
        let published = Arc::new(Mutex::new(Vec::new()));
        let updates = open(
            directory.path(),
            &source,
            &clock,
            blocker,
            Some(Arc::clone(&published)),
        );
        Self {
            updates,
            source,
            clock,
            published,
            directory,
        }
    }

    /// Opens a fresh instance over the same state file, like relaunching Opul.
    fn reopen(&mut self, blocker: Option<InstallBlocker>) {
        self.updates = open(
            self.directory.path(),
            &self.source,
            &self.clock,
            blocker,
            Some(Arc::clone(&self.published)),
        );
    }

    fn phase(&self) -> Phase {
        self.updates.snapshot().phase
    }

    fn settle(&self, expected: &Phase) {
        wait_for(|| &self.phase() == expected);
    }
}

fn open(
    directory: &Path,
    source: &Arc<FakeSource>,
    clock: &Arc<FakeClock>,
    install_blocker: Option<InstallBlocker>,
    published: Option<Arc<Mutex<Vec<AppUpdatesSnapshot>>>>,
) -> Arc<AppUpdates> {
    AppUpdates::open(
        AppUpdatesConfig {
            state_file: Some(directory.join("app-updates.json")),
            current_version: "0.3.0".into(),
            channel: ReleaseChannel::Stable,
            install_blocker,
        },
        Arc::clone(source) as Arc<dyn UpdateSource>,
        Arc::clone(clock) as Arc<dyn Clock>,
        Box::new(move |snapshot| {
            if let Some(published) = &published {
                published.lock().unwrap().push(snapshot.clone());
            }
        }),
    )
}

fn available(version: &str) -> Phase {
    Phase::Available {
        version: version.into(),
    }
}

fn ready(version: &str) -> Phase {
    Phase::Ready {
        version: version.into(),
    }
}

#[test]
fn defaults_check_but_do_not_install_and_survive_older_state_files() {
    let mut harness = Harness::new(None);
    let snapshot = harness.updates.snapshot();
    assert!(snapshot.check_automatically);
    assert!(!snapshot.install_automatically);
    assert_eq!(snapshot.phase, Phase::Idle);
    assert_eq!(snapshot.channel, "stable");
    assert_eq!(snapshot.current_version, "0.3.0");
    fs::write(
        harness.directory.path().join("app-updates.json"),
        br#"{"last_checked":5}"#,
    )
    .unwrap();
    harness.reopen(None);
    let snapshot = harness.updates.snapshot();
    assert!(snapshot.check_automatically);
    assert!(!snapshot.install_automatically);
    assert_eq!(snapshot.last_checked, Some(5));
}

#[test]
fn scheduler_checks_a_minute_after_launch_then_daily_and_respects_the_switch() {
    let harness = Harness::new(None);
    harness.updates.start_schedule();
    assert_eq!(harness.source.checks(), 0);
    harness.clock.elapse(FIRST_CHECK_DELAY);
    wait_for(|| harness.source.checks() == 1);
    harness.settle(&Phase::UpToDate);
    assert_eq!(
        harness.updates.snapshot().last_checked,
        Some(FIRST_CHECK_DELAY.as_secs())
    );
    harness.clock.elapse(CHECK_INTERVAL);
    wait_for(|| harness.source.checks() == 2);
    harness.updates.configure(false, true).unwrap();
    harness.clock.elapse(CHECK_INTERVAL);
    // The schedule re-arms without touching the network while checks are off.
    harness.clock.elapse(CHECK_INTERVAL);
    assert_eq!(harness.source.checks(), 2);
}

#[test]
fn manual_checks_are_single_flight() {
    let harness = Harness::new(None);
    harness.source.check_permits.forget_permits(1_000);
    assert_eq!(harness.updates.check(Origin::Manual).phase, Phase::Checking);
    assert_eq!(harness.updates.check(Origin::Manual).phase, Phase::Checking);
    wait_for(|| harness.source.checks() == 1);
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.source.check_permits.add_permits(1);
    harness.settle(&available("0.4.0"));
    assert_eq!(harness.source.checks(), 1);
    assert_eq!(
        harness.updates.snapshot().release_notes_url.as_deref(),
        Some("https://github.com/slovakian/opul/releases/tag/v0.4.0")
    );
}

#[test]
fn scheduled_failures_are_silent_and_manual_failures_explain() {
    let harness = Harness::new(None);
    harness.updates.check(Origin::Manual);
    harness.settle(&Phase::UpToDate);
    harness.source.publish(Err("offline".into()));
    harness.updates.check(Origin::Scheduled);
    wait_for(|| harness.source.checks() == 2);
    harness.settle(&Phase::UpToDate);
    harness.updates.check(Origin::Manual);
    harness.settle(&Phase::CheckFailed {
        message: "offline".into(),
    });
}

#[test]
fn notices_are_claimed_once_per_version_across_restarts() {
    let mut harness = Harness::new(None);
    assert_eq!(harness.updates.take_notice().unwrap(), None);
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.updates.check(Origin::Scheduled);
    harness.settle(&available("0.4.0"));
    assert_eq!(harness.source.installs(), Vec::<String>::new());
    assert_eq!(
        harness.updates.take_notice().unwrap().as_deref(),
        Some("0.4.0")
    );
    assert_eq!(harness.updates.take_notice().unwrap(), None);
    harness.reopen(None);
    harness.updates.check(Origin::Scheduled);
    harness.settle(&available("0.4.0"));
    assert_eq!(harness.updates.take_notice().unwrap(), None);
    harness.source.publish(Ok(Some("0.4.1".into())));
    harness.updates.check(Origin::Scheduled);
    harness.settle(&available("0.4.1"));
    assert_eq!(
        harness.updates.take_notice().unwrap().as_deref(),
        Some("0.4.1")
    );
}

#[test]
fn automatic_installs_stay_silent_and_wait_for_a_restart() {
    let harness = Harness::new(None);
    harness.updates.configure(true, true).unwrap();
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.updates.check(Origin::Scheduled);
    harness.settle(&ready("0.4.0"));
    assert_eq!(harness.source.installs(), ["0.4.0"]);
    assert_eq!(harness.updates.take_notice().unwrap(), None);
    assert_eq!(harness.updates.ready_version().as_deref(), Some("0.4.0"));
    let published = harness.published.lock().unwrap().clone();
    assert!(published.iter().any(|snapshot| snapshot.phase
        == Phase::Downloading {
            version: "0.4.0".into(),
            automatic: true,
            downloaded: 50,
            total: Some(100),
        }));
    // A ready update is not checked again or reinstalled.
    harness.updates.check(Origin::Manual);
    assert_eq!(harness.source.checks(), 1);
}

#[test]
fn turning_on_automatic_installs_starts_a_waiting_update() {
    let harness = Harness::new(None);
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.updates.check(Origin::Manual);
    harness.settle(&available("0.4.0"));
    // Install is nested under Check: with checks off it stays manual.
    harness.updates.configure(false, true).unwrap();
    assert_eq!(harness.phase(), available("0.4.0"));
    harness.updates.configure(true, true).unwrap();
    harness.settle(&ready("0.4.0"));
}

#[test]
fn failed_downloads_can_be_retried() {
    let harness = Harness::new(None);
    assert!(harness.updates.download(false).is_err());
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.updates.check(Origin::Manual);
    harness.settle(&available("0.4.0"));
    *harness.source.install_result.lock().unwrap() = Err("bad signature".into());
    harness.updates.download(false).unwrap();
    harness.settle(&Phase::DownloadFailed {
        version: "0.4.0".into(),
        automatic: false,
        message: "bad signature".into(),
    });
    *harness.source.install_result.lock().unwrap() = Ok(());
    harness.updates.download(false).unwrap();
    harness.settle(&ready("0.4.0"));
}

#[test]
fn blocked_copies_check_but_never_install_or_notify() {
    let harness = Harness::new(Some(InstallBlocker::DiskImage));
    harness.updates.configure(true, true).unwrap();
    harness.source.publish(Ok(Some("0.4.0".into())));
    harness.updates.check(Origin::Manual);
    harness.settle(&available("0.4.0"));
    assert_eq!(harness.updates.take_notice().unwrap(), None);
    assert_eq!(
        harness.updates.download(false).unwrap_err(),
        "Move Opul to your Applications folder to install updates."
    );
    assert_eq!(harness.source.installs(), Vec::<String>::new());
    assert_eq!(
        harness.updates.snapshot().install_blocker,
        Some(InstallBlocker::DiskImage)
    );
}

#[test]
fn a_missing_manifest_means_up_to_date() {
    assert!(means_up_to_date(
        &tauri_plugin_updater::Error::ReleaseNotFound
    ));
    let network = tauri_plugin_updater::Error::Network("timed out".into());
    assert!(!means_up_to_date(&network));
    assert!(describe(&network).starts_with("Couldn't reach the update server"));
}

#[test]
fn install_blockers_follow_where_opul_runs_from() {
    let writable = |_: &Path| true;
    let read_only = |_: &Path| false;
    let detect =
        |path: &str, probe: &dyn Fn(&Path) -> bool| detect_install_blocker(Path::new(path), probe);
    assert_eq!(
        detect("/Users/me/opul/target/debug/Opul", &writable),
        Some(InstallBlocker::Development)
    );
    assert_eq!(
        detect("/Applications/Opul.app/Contents/MacOS/Opul", &read_only),
        None
    );
    assert_eq!(
        detect(
            "/private/var/folders/x/AppTranslocation/ABC/d/Opul.app/Contents/MacOS/Opul",
            &writable
        ),
        Some(InstallBlocker::Translocated)
    );
    assert_eq!(
        detect("/Volumes/Opul/Opul.app/Contents/MacOS/Opul", &read_only),
        Some(InstallBlocker::DiskImage)
    );
    assert_eq!(
        detect(
            "/Volumes/External/Apps/Opul.app/Contents/MacOS/Opul",
            &writable
        ),
        None
    );
}
