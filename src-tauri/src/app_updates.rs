//! Opul's own updates. Rust owns checking, consent claims, download and install; the
//! frontend only renders snapshots and asks for actions.

mod platform;

pub use platform::{SystemClock, TauriSource, detect_install_blocker, folder_is_writable};

use crate::channel::ReleaseChannel;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
pub type Progress = Box<dyn FnMut(u64, Option<u64>) + Send>;

/// Mirrors the video-tools cadence: shortly after launch, then daily.
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(60);
pub const CHECK_INTERVAL: Duration = Duration::from_secs(86_400);

/// Where releases come from. `check` reports `Ok(None)` when nothing newer is published,
/// including when the channel has no manifest yet.
pub trait UpdateSource: Send + Sync + 'static {
    fn check(&self) -> BoxFuture<'_, Result<Option<String>, String>>;
    /// Downloads, verifies and installs the version the last check found.
    fn install(&self, version: String, progress: Progress) -> BoxFuture<'_, Result<(), String>>;
}

pub trait Clock: Send + Sync + 'static {
    fn unix_now(&self) -> u64;
    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()>;
}

/// Why this copy of Opul can check for updates but can't replace itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallBlocker {
    Development,
    Translocated,
    DiskImage,
}

impl InstallBlocker {
    fn message(self) -> &'static str {
        match self {
            Self::Development => "Development builds can't install updates.",
            Self::Translocated | Self::DiskImage => {
                "Move Opul to your Applications folder to install updates."
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Phase {
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
    },
    Downloading {
        version: String,
        automatic: bool,
        downloaded: u64,
        total: Option<u64>,
    },
    /// Installed in place. The new version runs after the next launch.
    Ready {
        version: String,
    },
    CheckFailed {
        message: String,
    },
    DownloadFailed {
        version: String,
        automatic: bool,
        message: String,
    },
}

impl Phase {
    fn version(&self) -> Option<&str> {
        match self {
            Self::Available { version }
            | Self::Downloading { version, .. }
            | Self::Ready { version }
            | Self::DownloadFailed { version, .. } => Some(version),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppUpdatesSnapshot {
    pub sequence: u64,
    pub current_version: String,
    pub channel: &'static str,
    pub phase: Phase,
    pub last_checked: Option<u64>,
    pub check_automatically: bool,
    pub install_automatically: bool,
    pub install_blocker: Option<InstallBlocker>,
    pub release_notes_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Stored {
    check_automatically: bool,
    install_automatically: bool,
    last_checked: Option<u64>,
    /// The newest version a consent notification was claimed for.
    notified_version: Option<String>,
}

impl Default for Stored {
    fn default() -> Self {
        Self {
            check_automatically: true,
            install_automatically: false,
            last_checked: None,
            notified_version: None,
        }
    }
}

impl Stored {
    fn installs_automatically(&self) -> bool {
        self.check_automatically && self.install_automatically
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Scheduled,
    Manual,
}

pub struct AppUpdatesConfig {
    /// `None` keeps choices in memory only, when the data directory is unavailable.
    pub state_file: Option<PathBuf>,
    pub current_version: String,
    pub channel: ReleaseChannel,
    pub install_blocker: Option<InstallBlocker>,
}

struct Live {
    stored: Stored,
    phase: Phase,
    sequence: u64,
}

type Notify = Box<dyn Fn(&AppUpdatesSnapshot) + Send + Sync>;

pub struct AppUpdates {
    config: AppUpdatesConfig,
    source: Arc<dyn UpdateSource>,
    clock: Arc<dyn Clock>,
    notify: Notify,
    live: Mutex<Live>,
}

impl AppUpdates {
    pub fn open(
        config: AppUpdatesConfig,
        source: Arc<dyn UpdateSource>,
        clock: Arc<dyn Clock>,
        notify: Notify,
    ) -> Arc<Self> {
        // Unreadable state falls back to defaults: at worst a notification repeats.
        let stored = config
            .state_file
            .as_ref()
            .and_then(|path| fs::read(path).ok())
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Arc::new(Self {
            config,
            source,
            clock,
            notify,
            live: Mutex::new(Live {
                stored,
                phase: Phase::Idle,
                sequence: 0,
            }),
        })
    }

    fn lock(&self) -> MutexGuard<'_, Live> {
        self.live
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn project(&self, live: &Live) -> AppUpdatesSnapshot {
        AppUpdatesSnapshot {
            sequence: live.sequence,
            current_version: self.config.current_version.clone(),
            channel: self.config.channel.id(),
            phase: live.phase.clone(),
            last_checked: live.stored.last_checked,
            check_automatically: live.stored.check_automatically,
            install_automatically: live.stored.install_automatically,
            install_blocker: self.config.install_blocker,
            release_notes_url: live
                .phase
                .version()
                .map(|version| self.config.channel.release_notes_url(version)),
        }
    }

    pub fn snapshot(&self) -> AppUpdatesSnapshot {
        self.project(&self.lock())
    }

    /// Applies a change, then publishes the new snapshot outside the lock.
    fn change<R>(&self, apply: impl FnOnce(&mut Live) -> R) -> (R, AppUpdatesSnapshot) {
        let (result, snapshot) = {
            let mut live = self.lock();
            let result = apply(&mut live);
            live.sequence += 1;
            (result, self.project(&live))
        };
        (self.notify)(&snapshot);
        (result, snapshot)
    }

    fn persist(&self, stored: &Stored) -> Result<(), String> {
        let Some(path) = &self.config.state_file else {
            return Ok(());
        };
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(stored).map_err(|error| error.to_string())?;
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())?;
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    }

    /// Updates the stored choices, keeping memory and disk in step.
    fn commit(&self, live: &mut Live, edit: impl FnOnce(&mut Stored)) -> Result<(), String> {
        let mut next = live.stored.clone();
        edit(&mut next);
        self.persist(&next)?;
        live.stored = next;
        Ok(())
    }

    pub fn start_schedule(self: &Arc<Self>) {
        let weak = Arc::downgrade(self);
        let clock = Arc::clone(&self.clock);
        tauri::async_runtime::spawn(async move {
            let mut delay = FIRST_CHECK_DELAY;
            loop {
                clock.sleep(delay).await;
                delay = CHECK_INTERVAL;
                let Some(updates) = weak.upgrade() else {
                    break;
                };
                if updates.lock().stored.check_automatically {
                    updates.check(Origin::Scheduled);
                }
            }
        });
    }

    /// Starts a check unless one is running, an install is underway or one is waiting for
    /// restart. Scheduled failures are silent; manual ones show their message.
    pub fn check(self: &Arc<Self>, origin: Origin) -> AppUpdatesSnapshot {
        let (previous, snapshot) = self.change(|live| {
            if matches!(
                live.phase,
                Phase::Checking | Phase::Downloading { .. } | Phase::Ready { .. }
            ) {
                return None;
            }
            Some(std::mem::replace(&mut live.phase, Phase::Checking))
        });
        let Some(previous) = previous else {
            return snapshot;
        };
        let updates = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let result = updates.source.check().await;
            updates.finish_check(result, previous, origin);
        });
        snapshot
    }

    fn finish_check(
        self: &Arc<Self>,
        result: Result<Option<String>, String>,
        previous: Phase,
        origin: Origin,
    ) {
        let now = self.clock.unix_now();
        let (install, _) = self.change(|live| {
            let found = match result {
                Ok(found) => found,
                Err(message) => {
                    live.phase = match origin {
                        Origin::Scheduled => previous,
                        Origin::Manual => Phase::CheckFailed { message },
                    };
                    return false;
                }
            };
            // A failed write only costs the "Last checked" line; the result still stands.
            let _ = self.commit(live, |stored| stored.last_checked = Some(now));
            live.phase = match found {
                Some(version) => Phase::Available { version },
                None => Phase::UpToDate,
            };
            matches!(live.phase, Phase::Available { .. })
                && live.stored.installs_automatically()
                && self.config.install_blocker.is_none()
        });
        if install {
            let _ = self.download(true);
        }
    }

    /// Downloads and installs the available version. `automatic` installs stay silent.
    pub fn download(self: &Arc<Self>, automatic: bool) -> Result<AppUpdatesSnapshot, String> {
        if let Some(blocker) = self.config.install_blocker {
            return Err(blocker.message().into());
        }
        let (version, snapshot) = self.change(|live| {
            let version = match &live.phase {
                Phase::Available { version } | Phase::DownloadFailed { version, .. } => {
                    version.clone()
                }
                _ => return None,
            };
            live.phase = Phase::Downloading {
                version: version.clone(),
                automatic,
                downloaded: 0,
                total: None,
            };
            Some(version)
        });
        let Some(version) = version else {
            return match snapshot.phase {
                Phase::Downloading { .. } | Phase::Ready { .. } => Ok(snapshot),
                _ => Err("Check for updates first.".into()),
            };
        };
        let updates = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            let progress = updates.progress_reporter();
            let result = updates.source.install(version.clone(), progress).await;
            updates.change(|live| {
                live.phase = match result {
                    Ok(()) => Phase::Ready { version },
                    Err(message) => Phase::DownloadFailed {
                        version,
                        automatic,
                        message,
                    },
                };
            });
        });
        Ok(snapshot)
    }

    /// Publishes progress at most once per percent, or per mebibyte when the size is unknown.
    fn progress_reporter(self: &Arc<Self>) -> Progress {
        const UNKNOWN_SIZE_STEP: u64 = 1024 * 1024;
        let weak = Arc::downgrade(self);
        let mut last_step = None;
        Box::new(move |downloaded, total| {
            let step = match total {
                Some(total) if total > 0 => downloaded.saturating_mul(100) / total,
                _ => downloaded / UNKNOWN_SIZE_STEP,
            };
            if last_step == Some(step) {
                return;
            }
            last_step = Some(step);
            let Some(updates) = weak.upgrade() else {
                return;
            };
            updates.change(|live| {
                if let Phase::Downloading {
                    downloaded: current,
                    total: known,
                    ..
                } = &mut live.phase
                {
                    *current = downloaded;
                    *known = total;
                }
            });
        })
    }

    /// Saves both switches. Turning on automatic installs with an update waiting starts it.
    pub fn configure(
        self: &Arc<Self>,
        check_automatically: bool,
        install_automatically: bool,
    ) -> Result<AppUpdatesSnapshot, String> {
        let (result, snapshot) = self.change(|live| {
            self.commit(live, |stored| {
                stored.check_automatically = check_automatically;
                stored.install_automatically = install_automatically;
            })?;
            Ok::<_, String>(
                live.stored.installs_automatically()
                    && self.config.install_blocker.is_none()
                    && matches!(live.phase, Phase::Available { .. }),
            )
        });
        if result? {
            return self.download(true);
        }
        Ok(snapshot)
    }

    /// Atomically claims the consent notification for the available version. Each version
    /// notifies at most once, across restarts; a newer version notifies again.
    pub fn take_notice(&self) -> Result<Option<String>, String> {
        let mut live = self.lock();
        let Phase::Available { version } = &live.phase else {
            return Ok(None);
        };
        if live.stored.installs_automatically() || self.config.install_blocker.is_some() {
            return Ok(None);
        }
        let version = version.clone();
        if !is_newer(&version, live.stored.notified_version.as_deref()) {
            return Ok(None);
        }
        self.commit(&mut live, |stored| {
            stored.notified_version = Some(version.clone());
        })?;
        Ok(Some(version))
    }

    pub fn ready_version(&self) -> Option<String> {
        match &self.lock().phase {
            Phase::Ready { version } => Some(version.clone()),
            _ => None,
        }
    }
}

fn is_newer(version: &str, notified: Option<&str>) -> bool {
    let Some(notified) = notified else {
        return true;
    };
    match (
        semver::Version::parse(version),
        semver::Version::parse(notified),
    ) {
        (Ok(version), Ok(notified)) => version > notified,
        _ => version != notified,
    }
}

#[cfg(test)]
mod tests;
