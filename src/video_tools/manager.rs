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
    pub installed: Option<InstalledPair>,
    #[serde(default)]
    pub previous_runtime: Option<u64>,
    #[serde(default)]
    pub previous_yt_dlp: Option<String>,
    #[serde(default)]
    pub rollback_pair: Option<InstalledPair>,
    #[serde(default)]
    pub last_checked: Option<u64>,
    #[serde(default)]
    pub compatibility_hold: Option<(String, u64)>,
    #[serde(default)]
    pub skipped_runtime: Option<u64>,
    #[serde(default)]
    pub skipped_yt_dlp: Option<String>,
    pub keep_up_to_date: bool,
    pub turned_off: bool,
}

impl Default for StoredState {
    fn default() -> Self {
        Self {
            installed: None,
            previous_runtime: None,
            previous_yt_dlp: None,
            rollback_pair: None,
            last_checked: None,
            compatibility_hold: None,
            skipped_runtime: None,
            skipped_yt_dlp: None,
            keep_up_to_date: true,
            turned_off: false,
        }
    }
}

pub(super) struct LiveState {
    pub stored: StoredState,
    pub snapshot: VideoToolsSnapshot,
    pub job: Option<(Uuid, AbortHandle)>,
    pub candidate: Option<(u64, String)>,
    pub active_downloads: usize,
}

pub struct VideoTools {
    pub(super) config: VideoToolsConfig,
    pub(super) sources: super::install::Sources,
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
        Self::open_with_sources(config, super::install::Sources::production())
    }

    pub fn open_with_environment(
        config: VideoToolsConfig,
        environment: Arc<dyn NetworkEnvironment>,
    ) -> Result<Arc<Self>, String> {
        let mut sources = super::install::Sources::production();
        sources.environment = environment;
        Self::open_with_sources(config, sources)
    }

    pub(super) fn open_with_sources(
        config: VideoToolsConfig,
        sources: super::install::Sources,
    ) -> Result<Arc<Self>, String> {
        fs::create_dir_all(&config.root).map_err(|e| e.to_string())?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(config.root.join("tools.lock"))
            .map_err(|e| e.to_string())?;
        lock.try_lock_exclusive()
            .map_err(|_| "The video tools directory is already in use.".to_string())?;
        super::recovery::clean_temporary(&config.root)?;
        let state_path = config.root.join("state.json");
        let (mut stored, load_error) = match fs::read(state_path) {
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
        // If state.json is damaged, preserve version directories for diagnosis and repair.
        if load_error.is_none() {
            super::recovery::prune(&config.root, &stored)?;
        }
        let state = if let Some(details) = load_error {
            VideoToolsState::NotWorking {
                reason: FailureReason::StoppedWorking,
                details,
            }
        } else if stored.turned_off {
            VideoToolsState::TurnedOff
        } else if let Some(pair) = stored.installed.clone() {
            match super::verification::validate_installed(&config, &pair) {
                Ok(()) => VideoToolsState::Ready {
                    installed: pair.clone(),
                    last_checked: stored.last_checked,
                    update_flag: stored.compatibility_hold.as_ref().map_or(
                        UpdateFlag::None,
                        |(version, _)| UpdateFlag::WaitingForRuntime {
                            version: version.clone(),
                        },
                    ),
                },
                Err(details) => {
                    if let Some(previous) = stored.rollback_pair.clone()
                        && super::verification::validate_installed(&config, &previous).is_ok()
                    {
                        stored.installed = Some(previous.clone());
                        stored.previous_runtime = None;
                        stored.rollback_pair = None;
                        persist(&config, &stored)?;
                        super::recovery::prune(&config.root, &stored)?;
                        if let Ok(mut file) = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(config.root.join("updates.log"))
                        {
                            let _ = writeln!(
                                file,
                                "Rolled back Python runtime {} at startup: {details}",
                                pair.runtime
                            );
                        }
                        let mut state = ready(previous);
                        if let VideoToolsState::Ready {
                            last_checked,
                            update_flag,
                            ..
                        } = &mut state
                        {
                            *last_checked = stored.last_checked;
                            *update_flag = UpdateFlag::Failed {
                                version: format!("Python runtime {}", pair.runtime),
                            };
                        }
                        state
                    } else {
                        VideoToolsState::NotWorking {
                            reason: FailureReason::StoppedWorking,
                            details,
                        }
                    }
                }
            }
        } else {
            VideoToolsState::NotSetUp
        };
        let snapshot = VideoToolsSnapshot {
            sequence: 0,
            state,
            keep_up_to_date: stored.keep_up_to_date,
            published_size: None,
        };
        let (publisher, _) = watch::channel(snapshot.clone());
        Ok(Arc::new(Self {
            config,
            sources,
            live: Mutex::new(LiveState {
                stored,
                snapshot,
                job: None,
                candidate: None,
                active_downloads: 0,
            }),
            publisher,
            idle: tokio::sync::Notify::new(),
            _lock: lock,
        }))
    }

    pub(super) fn lock(&self) -> MutexGuard<'_, LiveState> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn diagnostics(&self) -> String {
        let live = self.lock();
        format!(
            "Video tools: {:?}\nInstalled: {:?}\nCandidate runtime / yt-dlp: {:?}\nChip: {}\nOpul: {}",
            live.snapshot.state,
            live.stored.installed,
            live.candidate,
            std::env::consts::ARCH,
            self.config.opul_version
        )
    }

    pub fn snapshot(&self) -> VideoToolsSnapshot {
        self.lock().snapshot.clone()
    }

    /// A watch receiver starts with the current snapshot and coalesces slow subscribers.
    pub fn subscribe(&self) -> watch::Receiver<VideoToolsSnapshot> {
        self.publisher.subscribe()
    }

    pub fn invocation(&self) -> Option<VideoToolsInvocation> {
        match &self.lock().snapshot.state {
            VideoToolsState::Ready { installed, .. } => Some(invocation(&self.config, installed)),
            _ => None,
        }
    }

    pub fn execute(
        self: &Arc<Self>,
        operation: VideoToolsOperation,
    ) -> Result<VideoToolsSnapshot, String> {
        let mut live = self.lock();
        match operation {
            VideoToolsOperation::SetKeepUpToDate { enabled } => {
                let mut stored = live.stored.clone();
                stored.keep_up_to_date = enabled;
                persist(&self.config, &stored)?;
                live.stored = stored;
                live.snapshot.keep_up_to_date = enabled;
            }
            VideoToolsOperation::TurnOff => {
                if matches!(live.snapshot.state, VideoToolsState::Ready { .. }) {
                    return Err(
                        "Installed video tools must be removed before turning them off.".into(),
                    );
                }
                let mut stored = live.stored.clone();
                stored.turned_off = true;
                persist(&self.config, &stored)?;
                cancel(&mut live);
                live.stored = stored;
                live.snapshot.state = VideoToolsState::TurnedOff;
            }
            VideoToolsOperation::CheckForUpdates | VideoToolsOperation::Update => {
                if live.job.is_some()
                    || !matches!(live.snapshot.state, VideoToolsState::Ready { .. })
                {
                    return Ok(live.snapshot.clone());
                }
                let install = operation == VideoToolsOperation::Update;
                live.candidate = None;
                let id = Uuid::new_v4();
                let tools = Arc::clone(self);
                let task = runtime().spawn(async move {
                    tools.run_update(id, install).await;
                });
                live.job = Some((id, task.abort_handle()));
            }
            VideoToolsOperation::Cancel => {
                cancel(&mut live);
                if let VideoToolsState::Ready { update_flag, .. } = &mut live.snapshot.state {
                    *update_flag = UpdateFlag::None;
                } else if matches!(live.snapshot.state, VideoToolsState::SettingUp { .. }) {
                    live.snapshot.state = if live.stored.turned_off {
                        VideoToolsState::TurnedOff
                    } else {
                        VideoToolsState::NotSetUp
                    };
                }
            }
            VideoToolsOperation::CancelSetup => {
                if matches!(live.snapshot.state, VideoToolsState::SettingUp { .. })
                    && live.job.is_some()
                {
                    cancel(&mut live);
                    live.snapshot.state = if live.stored.turned_off {
                        VideoToolsState::TurnedOff
                    } else {
                        VideoToolsState::NotSetUp
                    };
                }
            }
            VideoToolsOperation::SetUp | VideoToolsOperation::Retry => {
                if live.job.is_some()
                    || matches!(live.snapshot.state, VideoToolsState::Ready { .. })
                {
                    return Ok(live.snapshot.clone());
                }
                let mut stored = live.stored.clone();
                stored.turned_off = false;
                persist(&self.config, &stored)?;
                live.stored = stored;
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
            }
            VideoToolsOperation::RefreshPublishedSize => {
                let tools = Arc::clone(self);
                // Explicit metadata request from Settings. Never fetch executable payloads before consent.
                runtime().spawn(async move {
                    if let Ok(size) = tools.published_size().await {
                        let mut live = tools.lock();
                        live.snapshot.published_size = Some(size);
                        tools.publish(&mut live);
                    }
                });
                return Ok(live.snapshot.clone());
            }
        }
        self.publish(&mut live);
        Ok(live.snapshot.clone())
    }

    pub(super) fn publish(&self, live: &mut LiveState) {
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
        if !is_current(&live, id) {
            return;
        }
        if let VideoToolsState::Ready {
            update_flag: UpdateFlag::Updating { progress, .. },
            ..
        } = &mut live.snapshot.state
        {
            *progress = match status {
                StepStatus::Done => 90,
                StepStatus::Installing => 80,
                StepStatus::Verifying => 70,
                _ => total.filter(|size| *size > 0).map_or(0, |size| {
                    (downloaded.saturating_mul(60) / size).min(60) as u8
                }),
            };
            self.publish(&mut live);
        }
        if let VideoToolsState::SettingUp { steps } = &mut live.snapshot.state {
            let step = steps
                .iter_mut()
                .find(|step| step.component == component)
                .expect("both setup steps exist");
            step.status = status;
            step.downloaded_bytes = downloaded;
            step.total_bytes = total;
            self.publish(&mut live);
        }
    }
}

fn cancel(live: &mut LiveState) {
    if let Some((_, task)) = live.job.take() {
        task.abort();
    }
}

pub(super) fn is_current(live: &LiveState, id: Uuid) -> bool {
    live.job.as_ref().is_some_and(|(current, _)| *current == id)
}

pub(super) fn ready(installed: InstalledPair) -> VideoToolsState {
    VideoToolsState::Ready {
        installed,
        last_checked: None,
        update_flag: UpdateFlag::None,
    }
}

pub(super) fn invocation(
    config: &VideoToolsConfig,
    installed: &InstalledPair,
) -> VideoToolsInvocation {
    let runtime = config
        .root
        .join("python-runtime")
        .join(installed.runtime.to_string());
    VideoToolsInvocation {
        interpreter: runtime.join("bin/python3"),
        quickjs: runtime.join("bin/qjs"),
        script: config
            .root
            .join("yt-dlp")
            .join(&installed.yt_dlp)
            .join("yt-dlp"),
        built_in_tools: config.built_in_tools.clone(),
    }
}

pub(super) fn persist(config: &VideoToolsConfig, stored: &StoredState) -> Result<(), String> {
    let path = config.root.join(format!(".state-{}.json", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&serde_json::to_vec_pretty(stored).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        fs::rename(&path, config.root.join("state.json")).map_err(|e| e.to_string())?;
        File::open(&config.root)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(path);
    }
    result
}
