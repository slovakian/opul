use super::layout;
use super::*;
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use tokio::runtime::Runtime;
use tokio::sync::watch;
use tokio::task::AbortHandle;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct StoredState {
    #[serde(default)]
    pub yt_dlp_channel: YtDlpChannel,
    pub installed: Option<InstalledPair>,
    /// The script directory kept until the next successful update replaces it.
    #[serde(default)]
    pub previous_yt_dlp: Option<String>,
    /// The full pair restored if a new runtime fails before its first successful Download.
    #[serde(default)]
    pub rollback_pair: Option<InstalledPair>,
    #[serde(default)]
    pub last_checked: Option<u64>,
    #[serde(default)]
    pub compatibility_hold: Option<super::updates::CompatibilityHold>,
    #[serde(default)]
    pub skipped_runtime: Option<u64>,
    #[serde(default)]
    pub skipped_yt_dlp: Option<String>,
    #[serde(default)]
    pub available_update: Option<UpdateNotice>,
    #[serde(default)]
    pub notified_updates: Vec<String>,
    pub keep_up_to_date: bool,
    pub turned_off: bool,
}

impl Default for StoredState {
    fn default() -> Self {
        Self {
            yt_dlp_channel: YtDlpChannel::Stable,
            installed: None,
            previous_yt_dlp: None,
            rollback_pair: None,
            last_checked: None,
            compatibility_hold: None,
            skipped_runtime: None,
            skipped_yt_dlp: None,
            available_update: None,
            notified_updates: Vec::new(),
            keep_up_to_date: true,
            turned_off: false,
        }
    }
}

impl StoredState {
    /// The state when nothing is installed and nothing is running.
    pub(super) fn resting_state(&self) -> VideoToolsState {
        if self.turned_off {
            VideoToolsState::TurnedOff
        } else {
            VideoToolsState::NotSetUp
        }
    }

    /// The update flag a Ready pair shows when no check or update is running.
    pub(super) fn update_flag(&self) -> UpdateFlag {
        if let Some(hold) = &self.compatibility_hold {
            UpdateFlag::WaitingForRuntime {
                version: hold.yt_dlp.clone(),
            }
        } else if let Some(notice) = &self.available_update {
            UpdateFlag::Available {
                version: notice.version.clone(),
            }
        } else {
            UpdateFlag::None
        }
    }

    /// What survives when the installed tools are replaced or removed: the person's choices.
    pub(super) fn preferences_only(&self, turned_off: bool) -> Self {
        Self {
            yt_dlp_channel: self.yt_dlp_channel,
            keep_up_to_date: self.keep_up_to_date,
            notified_updates: self.notified_updates.clone(),
            turned_off,
            ..Self::default()
        }
    }

    /// Select the retained pair again. Returns the pair that was replaced.
    pub(super) fn roll_back(&mut self) -> Option<InstalledPair> {
        let previous = self.rollback_pair.take()?;
        self.installed.replace(previous)
    }

    fn uses_nightly(&self) -> bool {
        self.yt_dlp_channel == YtDlpChannel::Nightly
            || self
                .installed
                .iter()
                .chain(self.rollback_pair.iter())
                .any(|pair| pair.yt_dlp_channel == YtDlpChannel::Nightly)
    }
}

pub(super) struct LiveState {
    pub stored: StoredState,
    pub snapshot: VideoToolsSnapshot,
    pub job: Option<(Uuid, AbortHandle)>,
    /// The runtime and yt-dlp versions the last setup or update resolved, for diagnostics.
    pub candidate: Option<(u64, String)>,
    pub active_users: usize,
    pub schedule: Option<AbortHandle>,
    pub automatic_update: bool,
    pub removing: bool,
}

impl LiveState {
    /// Change the stored state durably. Memory changes only after state.json does.
    pub(super) fn commit(
        &mut self,
        config: &VideoToolsConfig,
        change: impl FnOnce(&mut StoredState),
    ) -> Result<(), String> {
        let mut stored = self.stored.clone();
        change(&mut stored);
        persist(config, &stored)?;
        self.stored = stored;
        Ok(())
    }

    pub(super) fn set_update_flag(&mut self, flag: UpdateFlag) {
        if let VideoToolsState::Ready { update_flag, .. } = &mut self.snapshot.state {
            *update_flag = flag;
        }
    }

    pub(super) fn stop_job(&mut self) {
        if let Some((_, task)) = self.job.take() {
            task.abort();
        }
    }

    pub(super) fn is_current(&self, id: Uuid) -> bool {
        self.job.as_ref().is_some_and(|(current, _)| *current == id)
    }
}

pub struct VideoTools {
    pub(super) config: VideoToolsConfig,
    pub(super) sources: super::sources::Sources,
    pub(super) live: Mutex<LiveState>,
    pub(super) publisher: watch::Sender<VideoToolsSnapshot>,
    pub(super) idle: tokio::sync::Notify,
    _lock: File,
}

impl std::fmt::Debug for VideoTools {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoTools")
            .field("root", &self.config.root)
            .finish_non_exhaustive()
    }
}

pub(super) fn runtime() -> &'static Runtime {
    static RUNTIME: OnceLock<Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("video-tools runtime")
    })
}

impl VideoTools {
    pub fn open(config: VideoToolsConfig) -> Result<Arc<Self>, String> {
        Self::open_with_sources(config, super::sources::Sources::production())
    }

    pub(super) fn open_with_sources(
        config: VideoToolsConfig,
        sources: super::sources::Sources,
    ) -> Result<Arc<Self>, String> {
        fs::create_dir_all(&config.root).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(config.root.join(layout::LOCK))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive()
            .map_err(|_| "The video tools directory is already in use.".to_string())?;
        super::recovery::clean_temporary(&config.root)?;
        let (mut stored, load_error) = match fs::read(config.root.join(layout::STATE)) {
            Ok(bytes) => match serde_json::from_slice::<StoredState>(&bytes) {
                Ok(stored) => (stored, None),
                Err(error) => (
                    StoredState::default(),
                    Some(format!("Could not read Tools/state.json: {error}")),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (StoredState::default(), None)
            }
            Err(error) => return Err(error.to_string()),
        };
        if !config.nightly_available && stored.uses_nightly() {
            return Err(
                "Nightly video tools state requires Opul Nightly and its own data directory."
                    .into(),
            );
        }
        // If state.json is damaged, preserve version directories for diagnosis and repair.
        if load_error.is_none() {
            super::recovery::prune(&config.root, &stored)?;
        }
        let state = match (load_error, stored.installed.clone()) {
            (Some(details), _) => VideoToolsState::NotWorking {
                reason: FailureReason::StoppedWorking,
                details,
            },
            _ if stored.turned_off => VideoToolsState::TurnedOff,
            (None, None) => VideoToolsState::NotSetUp,
            (None, Some(pair)) => Self::startup_state(&config, &mut stored, pair)?,
        };
        let snapshot = VideoToolsSnapshot {
            sequence: 0,
            update_available: false,
            state,
            keep_up_to_date: stored.keep_up_to_date,
            nightly_available: config.nightly_available,
            use_nightly: stored.yt_dlp_channel == YtDlpChannel::Nightly,
            published_size: None,
        };
        let (publisher, _) = watch::channel(snapshot.clone());
        let tools = Arc::new(Self {
            config,
            sources,
            live: Mutex::new(LiveState {
                stored,
                snapshot,
                job: None,
                candidate: None,
                active_users: 0,
                schedule: None,
                automatic_update: false,
                removing: false,
            }),
            publisher,
            idle: tokio::sync::Notify::new(),
            _lock: lock,
        });
        tools.start_schedule();
        {
            let mut live = tools.lock();
            tools.publish(&mut live);
            if live
                .stored
                .installed
                .as_ref()
                .is_some_and(|pair| pair.yt_dlp_channel != live.stored.yt_dlp_channel)
            {
                tools.start_update(&mut live, super::updates::UpdateMode::Manual);
            }
        }
        Ok(tools)
    }

    /// Ready with the installed pair, the retained pair if the installed one stopped working,
    /// or NotWorking.
    fn startup_state(
        config: &VideoToolsConfig,
        stored: &mut StoredState,
        pair: InstalledPair,
    ) -> Result<VideoToolsState, String> {
        let details = match super::verification::validate_installed(config, &pair) {
            Ok(()) => {
                return Ok(VideoToolsState::Ready {
                    installed: pair,
                    last_checked: stored.last_checked,
                    update_flag: stored.update_flag(),
                });
            }
            Err(details) => details,
        };
        let retained = stored
            .rollback_pair
            .clone()
            .filter(|previous| super::verification::validate_installed(config, previous).is_ok());
        let Some(previous) = retained else {
            return Ok(VideoToolsState::NotWorking {
                reason: FailureReason::StoppedWorking,
                details,
            });
        };
        let mut rolled_back = stored.clone();
        rolled_back.roll_back();
        persist(config, &rolled_back)?;
        *stored = rolled_back;
        super::recovery::prune(&config.root, stored)?;
        append_log(
            config,
            &format!(
                "Rolled back {} at startup: {details}",
                runtime_label(pair.runtime)
            ),
        );
        Ok(VideoToolsState::Ready {
            installed: previous,
            last_checked: stored.last_checked,
            update_flag: UpdateFlag::Failed {
                version: runtime_label(pair.runtime),
            },
        })
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, LiveState> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn diagnostics(&self) -> String {
        let live = self.lock();
        let candidate = live.candidate.as_ref().map_or_else(
            || "none".to_string(),
            |(runtime, yt_dlp)| format!("{}, yt-dlp {yt_dlp}", runtime_label(*runtime)),
        );
        format!(
            "Video tools: {:?}\nInstalled: {:?}\nCandidate: {candidate}\nChip: {}\nOpul: {}",
            live.snapshot.state,
            live.stored.installed,
            std::env::consts::ARCH,
            self.config.opul_version
        )
    }

    pub fn snapshot(&self) -> VideoToolsSnapshot {
        self.lock().snapshot.clone()
    }

    pub fn subscribe(&self) -> watch::Receiver<VideoToolsSnapshot> {
        self.publisher.subscribe()
    }

    pub fn invocation(&self) -> Option<VideoToolsInvocation> {
        match &self.lock().snapshot.state {
            VideoToolsState::Ready { installed, .. } => {
                Some(layout::selected(&self.config, installed))
            }
            _ => None,
        }
    }

    pub fn execute(
        self: &Arc<Self>,
        operation: VideoToolsOperation,
    ) -> Result<VideoToolsSnapshot, String> {
        let mut live = self.lock();
        if live.removing {
            return Err("Video tools are being removed.".into());
        }
        // Publish even after a failure: some changes may already be in memory.
        let result = (|| -> Result<(), String> {
            match operation {
                VideoToolsOperation::SetUseNightly { enabled } => {
                    self.set_use_nightly(&mut live, enabled)?;
                }
                VideoToolsOperation::SetKeepUpToDate { enabled } => {
                    self.set_keep_up_to_date(&mut live, enabled)?;
                }
                VideoToolsOperation::TurnOff => {
                    if matches!(live.snapshot.state, VideoToolsState::Ready { .. }) {
                        Err("Installed video tools must be removed before turning them off.")?;
                    }
                    live.commit(&self.config, |stored| stored.turned_off = true)?;
                    live.stop_job();
                    live.snapshot.state = VideoToolsState::TurnedOff;
                }
                VideoToolsOperation::CheckForUpdates => {
                    self.start_update(&mut live, super::updates::UpdateMode::Check);
                }
                VideoToolsOperation::Update => {
                    self.start_update(&mut live, super::updates::UpdateMode::Manual);
                }
                VideoToolsOperation::CancelSetup => {
                    if matches!(live.snapshot.state, VideoToolsState::SettingUp { .. }) {
                        live.stop_job();
                        live.snapshot.state = live.stored.resting_state();
                    }
                }
                VideoToolsOperation::SetUp | VideoToolsOperation::Retry => {
                    self.start_setup(&mut live)?;
                }
                VideoToolsOperation::RefreshPublishedSize => {
                    let tools = Arc::clone(self);
                    // Metadata only. Never fetch executable payloads before consent.
                    runtime().spawn(async move {
                        if let Ok(size) = tools.published_size().await {
                            let mut live = tools.lock();
                            live.snapshot.published_size = Some(size);
                            tools.publish(&mut live);
                        }
                    });
                }
            }
            Ok(())
        })();
        self.publish(&mut live);
        result.map(|()| live.snapshot.clone())
    }

    pub(super) fn start_setup(self: &Arc<Self>, live: &mut LiveState) -> Result<(), String> {
        if live.job.is_some() || matches!(live.snapshot.state, VideoToolsState::Ready { .. }) {
            return Ok(());
        }
        live.commit(&self.config, |stored| stored.turned_off = false)?;
        live.snapshot.state = VideoToolsState::SettingUp {
            steps: [Component::Python, Component::YtDlp]
                .into_iter()
                .map(|component| SetupStep {
                    component,
                    status: StepStatus::Pending,
                    downloaded_bytes: 0,
                    total_bytes: None,
                })
                .collect(),
        };
        live.candidate = None;
        let id = Uuid::new_v4();
        let tools = Arc::clone(self);
        let task = runtime().spawn(async move {
            tools.install(id).await;
        });
        live.job = Some((id, task.abort_handle()));
        Ok(())
    }

    /// Stop scheduled checks and any running setup or update. The selected pair stays selected.
    pub fn shut_down(&self) {
        let mut live = self.lock();
        if let Some(schedule) = live.schedule.take() {
            schedule.abort();
        }
        live.stop_job();
        if matches!(live.snapshot.state, VideoToolsState::SettingUp { .. }) {
            live.snapshot.state = live.stored.resting_state();
        }
        let flag = live.stored.update_flag();
        live.set_update_flag(flag);
        self.publish(&mut live);
    }

    /// Every snapshot change goes out through here, with its derived fields filled in.
    pub(super) fn publish(&self, live: &mut LiveState) {
        live.snapshot.update_available = live.stored.available_update.is_some()
            && matches!(live.snapshot.state, VideoToolsState::Ready { .. })
            && !matches!(
                live.snapshot.state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Updating { .. },
                    ..
                }
            );
        live.snapshot.sequence += 1;
        self.publisher.send_replace(live.snapshot.clone());
    }

    pub(super) fn progress(
        &self,
        id: Uuid,
        component: Component,
        status: StepStatus,
        downloaded: u64,
        total: Option<u64>,
    ) {
        let mut live = self.lock();
        if !live.is_current(id) {
            return;
        }
        match &mut live.snapshot.state {
            VideoToolsState::Ready {
                update_flag: UpdateFlag::Updating { progress, .. },
                ..
            } => {
                *progress = match status {
                    StepStatus::Done => 90,
                    StepStatus::Installing => 80,
                    StepStatus::Verifying => 70,
                    _ => total.filter(|size| *size > 0).map_or(0, |size| {
                        (downloaded.saturating_mul(60) / size).min(60) as u8
                    }),
                };
            }
            VideoToolsState::SettingUp { steps } => {
                let step = steps
                    .iter_mut()
                    .find(|step| step.component == component)
                    .expect("both setup steps exist");
                step.status = status;
                step.downloaded_bytes = downloaded;
                step.total_bytes = total;
            }
            _ => return,
        }
        self.publish(&mut live);
    }

    pub(super) fn log_failure(&self, details: &str) {
        append_log(&self.config, details);
    }
}

/// How an update that changes only the Python runtime is named to people.
pub(super) fn runtime_label(runtime: u64) -> String {
    format!("Python runtime {runtime}")
}

fn append_log(config: &VideoToolsConfig, line: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(config.root.join(layout::LOG))
    {
        let _ = writeln!(file, "{line}");
    }
}

pub(super) fn ready(installed: InstalledPair) -> VideoToolsState {
    VideoToolsState::Ready {
        installed,
        last_checked: None,
        update_flag: UpdateFlag::None,
    }
}

fn persist(config: &VideoToolsConfig, stored: &StoredState) -> Result<(), String> {
    let path = layout::state_draft(&config.root);
    let result: Result<(), String> = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(stored).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&path, config.root.join(layout::STATE)).map_err(|e| e.to_string())?;
        File::open(&config.root)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
