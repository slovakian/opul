//! The tools Opul's media queue runs: the yt-dlp it downloads once the person agrees, the
//! FFmpeg, ffprobe and QuickJS driver bundled in `Contents/Helpers`, and the copies a person
//! chooses in Settings. Opul never searches the system for installed tools.
//!
//! Every process asks for its tools when it starts, so a change applies to the next process
//! without a relaunch, and a running download keeps the yt-dlp it started with. See
//! docs/adr/0025-opul-ships-its-own-media-tools.md.

use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::persistence::{load_state, save_state};

const LATEST_RELEASE_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/latest";
const DOWNLOAD_URL: &str = "https://github.com/yt-dlp/yt-dlp/releases/download";
const ARCHIVE: &str = "yt-dlp_macos.zip";
const EXECUTABLE: &str = "yt-dlp_macos";
const CHECKSUMS: &str = "SHA2-256SUMS";
const SIGNATURE: &str = "SHA2-256SUMS.sig";
const CURRENT: &str = "current";

/// yt-dlp's release signing key, copied from `public.key` in the yt-dlp repository.
/// Primary key fingerprint: AC0CBBE6848D6A873464AF4E57CF65933B5A7581.
const YT_DLP_PUBLIC_KEY: &str = include_str!("tools/yt-dlp-public.key");

const UPDATE_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
const BUSY_UPDATE_DELAY: Duration = Duration::from_secs(5 * 60);
const RETRY_DELAYS: [Duration; 3] = [
    Duration::from_secs(30),
    Duration::from_secs(2 * 60),
    Duration::from_secs(10 * 60),
];
const LONGEST_RETRY_DELAY: Duration = Duration::from_secs(60 * 60);
/// The first run of a freshly extracted yt-dlp waits for the system's malware scan.
const VERSION_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SMALL_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum YtDlpState {
    NotInstalled,
    Installed {
        version: String,
        last_checked_ms: Option<u64>,
    },
    /// The last download failed and no copy is installed.
    Failed {
        reason: String,
    },
    /// A yt-dlp the person chose. The version is unknown until Opul has run it once.
    Custom {
        path: String,
        version: Option<String>,
    },
    CustomBroken {
        path: String,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, uniffi::Enum)]
pub enum YtDlpActivity {
    Idle,
    Checking,
    Downloading { progress: f64 },
    WaitingToRetry { seconds: u64 },
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct BundledToolStatus {
    pub path: Option<String>,
    pub version: Option<String>,
    pub custom: bool,
    pub problem: Option<String>,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct MediaToolStatus {
    /// Increases whenever anything below changes.
    pub generation: u64,
    pub yt_dlp: YtDlpState,
    pub activity: YtDlpActivity,
    /// yt-dlp can start a process now.
    pub yt_dlp_ready: bool,
    /// yt-dlp is being fetched for the first time, so video links wait for it.
    pub yt_dlp_pending: bool,
    /// The last update failed. The verified copy that was already installed stays in use.
    pub update_error: Option<String>,
    pub download_bytes: Option<u64>,
    pub auto_update: bool,
    pub ffmpeg: BundledToolStatus,
    pub ffprobe: BundledToolStatus,
    pub quickjs: BundledToolStatus,
    pub message: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, uniffi::Record)]
pub struct MediaToolSettings {
    pub custom_yt_dlp_path: Option<String>,
    pub custom_ffmpeg_path: Option<String>,
    pub auto_update_yt_dlp: bool,
}

impl MediaToolSettings {
    fn custom_yt_dlp(&self) -> Option<PathBuf> {
        non_blank_path(self.custom_yt_dlp_path.as_deref())
    }

    fn custom_ffmpeg(&self) -> Option<PathBuf> {
        non_blank_path(self.custom_ffmpeg_path.as_deref())
    }
}

fn non_blank_path(value: Option<&str>) -> Option<PathBuf> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ToolError {
    /// Worth retrying later: the network or GitHub was unavailable.
    Network(String),
    /// A checksum or signature did not match. Never retried silently.
    Verification(String),
    /// The downloaded yt-dlp did not run.
    Broken(String),
    Storage(String),
}

impl ToolError {
    fn message(&self) -> String {
        match self {
            ToolError::Network(reason) => format!("Couldn't reach GitHub: {reason}"),
            ToolError::Verification(reason) => {
                format!("The yt-dlp download didn't pass verification: {reason}")
            }
            ToolError::Broken(reason) => format!("The downloaded yt-dlp didn't run: {reason}"),
            ToolError::Storage(reason) => format!("Couldn't store yt-dlp: {reason}"),
        }
    }
}

/// Where yt-dlp releases come from. Tests replace GitHub with a local fake.
pub(crate) trait ReleaseSource: Send + Sync {
    fn latest_version(&self) -> Result<String, ToolError>;
    fn asset_size(&self, version: &str, asset: &str) -> Result<u64, ToolError>;
    fn download(
        &self,
        version: &str,
        asset: &str,
        limit: u64,
        destination: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), ToolError>;
}

/// A tool set resolved for one process. Holding it keeps that yt-dlp version on disk.
pub(crate) struct ProcessTools {
    pub(crate) yt_dlp: PathBuf,
    pub(crate) ffmpeg_location: Option<PathBuf>,
    pub(crate) quickjs: Option<PathBuf>,
    _lease: Option<VersionLease>,
}

impl ProcessTools {
    /// A yt-dlp command that only ever uses Opul's QuickJS driver for JavaScript, and finds
    /// ffprobe next to the FFmpeg it is given.
    pub(crate) fn yt_dlp_command(&self) -> Command {
        let mut command = Command::new(&self.yt_dlp);
        command.args(yt_dlp_tool_arguments(
            self.quickjs.as_deref(),
            self.ffmpeg_location.as_deref(),
        ));
        command
    }

    pub(crate) fn has_ffmpeg(&self) -> bool {
        self.ffmpeg_location.is_some()
    }
}

pub(crate) fn yt_dlp_tool_arguments(
    quickjs: Option<&Path>,
    ffmpeg_location: Option<&Path>,
) -> Vec<OsString> {
    let mut arguments = vec![OsString::from("--no-js-runtimes")];
    if let Some(quickjs) = quickjs {
        let mut runtime = OsString::from("quickjs:");
        runtime.push(quickjs);
        arguments.push("--js-runtimes".into());
        arguments.push(runtime);
    }
    if let Some(location) = ffmpeg_location {
        arguments.push("--ffmpeg-location".into());
        arguments.push(location.into());
    }
    arguments
}

struct VersionLease {
    tools: Arc<MediaTools>,
    version: String,
}

impl Drop for VersionLease {
    fn drop(&mut self) {
        let mut state = self.tools.lock();
        if let Some(count) = state.leases.get_mut(&self.version) {
            *count -= 1;
            if *count == 0 {
                state.leases.remove(&self.version);
                // An update or removal may have left this version behind for this process.
                self.tools.remove_unused_versions(&state);
            }
        }
        drop(state);
        self.tools.changed.notify_all();
    }
}

/// Why yt-dlp cannot start a process right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unavailable {
    /// Not installed, and nothing is being downloaded.
    Missing,
    /// Opul is getting yt-dlp. Video links wait for it.
    Pending,
    /// The download the person agreed to failed for good.
    Failed(String),
    CustomBroken {
        path: String,
    },
    FfmpegBroken {
        path: String,
    },
}

impl Unavailable {
    pub(crate) fn message(&self) -> String {
        match self {
            Unavailable::Missing => {
                "Video downloads need yt-dlp. Download it from Help ▸ Welcome to Opul or in Settings ▸ Media."
                    .to_string()
            }
            Unavailable::Pending => "Getting video tools…".to_string(),
            Unavailable::Failed(reason) => format!("Opul couldn't get yt-dlp. {reason}"),
            Unavailable::CustomBroken { path } => format!("Your yt-dlp at {path} isn't working."),
            Unavailable::FfmpegBroken { path } => format!("Your FFmpeg at {path} isn't working."),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Activity {
    Idle,
    Checking,
    Downloading { received: u64, total: Option<u64> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Failure {
    error: ToolError,
    /// Whether a copy was installed when it happened, which makes it an update failure.
    during_update: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Job {
    Install { manual: bool },
    Size,
    Probe,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct StoredRecord {
    /// The person agreed to download yt-dlp and has not removed it since.
    #[serde(default)]
    wanted: bool,
    #[serde(default)]
    previous: Option<String>,
    #[serde(default)]
    last_checked_ms: Option<u64>,
    /// Releases that failed `--version` here. Opul never installs them again.
    #[serde(default)]
    broken_versions: Vec<String>,
    /// Releases whose checksum or signature did not match. Only a manual check retries them.
    #[serde(default)]
    rejected_versions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileIdentity {
    length: u64,
    modified: i64,
    inode: u64,
}

impl FileIdentity {
    const MISSING: Self = Self {
        length: 0,
        modified: 0,
        inode: 0,
    };

    fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            length: metadata.len(),
            modified: metadata.mtime(),
            inode: metadata.ino(),
        })
    }
}

#[derive(Debug, Clone, Default)]
struct BundledVersions {
    ffmpeg: Option<String>,
    ffprobe: Option<String>,
    quickjs: Option<String>,
}

struct ToolState {
    settings: MediaToolSettings,
    record: StoredRecord,
    activity: Activity,
    failure: Option<Failure>,
    retry: Option<(Instant, usize)>,
    requests: Vec<Job>,
    next_update_check: Option<Instant>,
    download_bytes: Option<u64>,
    leases: HashMap<String, usize>,
    custom_checks: HashMap<PathBuf, (FileIdentity, Result<String, String>)>,
    generation: u64,
    started: bool,
    shutdown: bool,
}

pub(crate) struct MediaTools {
    root: PathBuf,
    helpers: Option<PathBuf>,
    source: Box<dyn ReleaseSource>,
    signing_key: SignedPublicKey,
    state: Mutex<ToolState>,
    changed: Condvar,
    bundled: OnceLock<BundledVersions>,
}

impl MediaTools {
    pub(crate) fn new(
        tools_folder: &Path,
        helpers: Option<PathBuf>,
        settings: MediaToolSettings,
    ) -> Arc<Self> {
        let key = SignedPublicKey::from_string(YT_DLP_PUBLIC_KEY)
            .expect("the bundled yt-dlp public key parses")
            .0;
        Self::with_source(
            tools_folder,
            helpers,
            settings,
            Box::new(GitHubReleases),
            key,
        )
    }

    pub(crate) fn with_source(
        tools_folder: &Path,
        helpers: Option<PathBuf>,
        settings: MediaToolSettings,
        source: Box<dyn ReleaseSource>,
        signing_key: SignedPublicKey,
    ) -> Arc<Self> {
        let root = tools_folder.join("yt-dlp");
        let record = load_state(&root.join("state.json"), "yt-dlp state", |_| Ok(()))
            .map(|loaded| loaded.items)
            .unwrap_or_default();
        Arc::new(Self {
            root,
            helpers,
            source,
            signing_key,
            state: Mutex::new(ToolState {
                settings,
                record,
                activity: Activity::Idle,
                failure: None,
                retry: None,
                requests: Vec::new(),
                next_update_check: None,
                download_bytes: None,
                leases: HashMap::new(),
                custom_checks: HashMap::new(),
                generation: 1,
                started: false,
                shutdown: false,
            }),
            changed: Condvar::new(),
            bundled: OnceLock::new(),
        })
    }

    /// Starts the background work: resuming a download the person agreed to, checking a
    /// chosen yt-dlp, and automatic update checks when they are on.
    pub(crate) fn start(self: &Arc<Self>) {
        {
            let mut state = self.lock();
            if state.started {
                return;
            }
            state.started = true;
            state.requests.push(Job::Probe);
            if state.settings.custom_yt_dlp().is_none() {
                if self.current_version().is_some() {
                    if state.settings.auto_update_yt_dlp {
                        state.next_update_check = Some(Instant::now());
                    }
                } else if state.record.wanted {
                    state.requests.push(Job::Install { manual: false });
                }
            }
        }
        let tools = Arc::clone(self);
        let _ = thread::Builder::new()
            .name("opul-media-tools".to_string())
            .spawn(move || tools.run_scheduler());
    }

    pub(crate) fn shutdown(&self) {
        self.lock().shutdown = true;
        self.changed.notify_all();
    }

    pub(crate) fn apply_settings(&self, settings: MediaToolSettings) {
        let mut state = self.lock();
        if state.settings == settings {
            return;
        }
        let was_auto = state.settings.auto_update_yt_dlp;
        state.settings = settings;
        if !state.settings.auto_update_yt_dlp {
            state.next_update_check = None;
        } else if !was_auto && self.current_version().is_some() {
            state.next_update_check = Some(Instant::now());
        }
        state.requests.push(Job::Probe);
        self.touch(&mut state);
    }

    /// The person agreed to download yt-dlp, or asked to check for a newer one.
    pub(crate) fn request_install(&self) {
        let mut state = self.lock();
        state.record.wanted = true;
        state.retry = None;
        if !state.requests.contains(&Job::Install { manual: true }) {
            state.requests.push(Job::Install { manual: true });
        }
        self.save_record(&state);
        self.touch(&mut state);
    }

    pub(crate) fn request_download_size(&self) {
        let mut state = self.lock();
        if state.download_bytes.is_none() && !state.requests.contains(&Job::Size) {
            state.requests.push(Job::Size);
            self.touch(&mut state);
        }
    }

    /// Deletes Opul's copy of yt-dlp. A version a running process uses stays on disk until
    /// that process ends.
    pub(crate) fn remove(&self) {
        let mut state = self.lock();
        state.record = StoredRecord {
            broken_versions: std::mem::take(&mut state.record.broken_versions),
            rejected_versions: std::mem::take(&mut state.record.rejected_versions),
            ..StoredRecord::default()
        };
        state.failure = None;
        state.retry = None;
        state.next_update_check = None;
        state
            .requests
            .retain(|job| !matches!(job, Job::Install { .. }));
        let _ = fs::remove_file(self.root.join(CURRENT));
        self.remove_unused_versions(&state);
        self.save_record(&state);
        self.touch(&mut state);
    }

    /// Runs `--version` on a yt-dlp the person picked, before Opul saves its path.
    pub(crate) fn verify_yt_dlp(path: &Path) -> Result<String, String> {
        run_version(path, &["--version"])
    }

    pub(crate) fn verify_ffmpeg(path: &Path) -> Result<String, String> {
        run_version(path, &["-hide_banner", "-version"]).map(|line| ffmpeg_version(&line))
    }

    pub(crate) fn status(&self) -> MediaToolStatus {
        let state = self.lock();
        self.status_locked(&state)
    }

    pub(crate) fn wait_for_status(
        &self,
        after_generation: u64,
        timeout: Duration,
    ) -> MediaToolStatus {
        let state = self.lock();
        let (state, _) = self
            .changed
            .wait_timeout_while(state, timeout, |state| {
                state.generation <= after_generation && !state.shutdown
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.status_locked(&state)
    }

    /// The tools for a process that starts now. The returned value keeps its yt-dlp version on
    /// disk until it is dropped.
    pub(crate) fn resolve(self: &Arc<Self>) -> Result<ProcessTools, Unavailable> {
        let settings = self.lock().settings.clone();
        let ffmpeg_location = match settings.custom_ffmpeg() {
            Some(path) => {
                if !is_executable(&path) {
                    return Err(Unavailable::FfmpegBroken {
                        path: path.to_string_lossy().into_owned(),
                    });
                }
                Some(ffmpeg_location(&path))
            }
            None => self.bundled_tool("ffmpeg").map(|_| {
                self.helpers
                    .clone()
                    .expect("a bundled tool lives in the helpers folder")
            }),
        };
        let quickjs = self.bundled_tool("qjs");
        if let Some(path) = settings.custom_yt_dlp() {
            return match self.check_custom(&path) {
                Ok(_) => Ok(ProcessTools {
                    yt_dlp: path,
                    ffmpeg_location,
                    quickjs,
                    _lease: None,
                }),
                Err(_) => Err(Unavailable::CustomBroken {
                    path: path.to_string_lossy().into_owned(),
                }),
            };
        }
        let mut state = self.lock();
        let Some(version) = self.current_version() else {
            return Err(self.unavailable_locked(&state));
        };
        *state.leases.entry(version.clone()).or_default() += 1;
        drop(state);
        Ok(ProcessTools {
            yt_dlp: self.root.join(&version).join(EXECUTABLE),
            ffmpeg_location,
            quickjs,
            _lease: Some(VersionLease {
                tools: Arc::clone(self),
                version,
            }),
        })
    }

    /// Whether an FFmpeg is configured: the person's own, or the bundled one.
    pub(crate) fn has_ffmpeg(&self) -> bool {
        let custom = self.lock().settings.custom_ffmpeg();
        custom.is_some() || self.bundled_tool("ffmpeg").is_some()
    }

    /// Whether a video item may be queued: yt-dlp is usable now or is on its way.
    pub(crate) fn can_start_video(&self) -> bool {
        matches!(self.unavailable(), None | Some(Unavailable::Pending))
    }

    /// Whether a video item should keep waiting for yt-dlp instead of failing.
    pub(crate) fn is_pending(&self) -> bool {
        let state = self.lock();
        state.settings.custom_yt_dlp().is_none()
            && self.current_version().is_none()
            && self.pending_locked(&state)
    }

    pub(crate) fn unavailable(&self) -> Option<Unavailable> {
        let state = self.lock();
        if let Some(path) = state.settings.custom_yt_dlp() {
            return match state.custom_checks.get(&path) {
                Some((_, Err(_))) => Some(Unavailable::CustomBroken {
                    path: path.to_string_lossy().into_owned(),
                }),
                _ => None,
            };
        }
        if self.current_version().is_some() {
            return None;
        }
        Some(self.unavailable_locked(&state))
    }

    fn unavailable_locked(&self, state: &ToolState) -> Unavailable {
        if self.pending_locked(state) {
            Unavailable::Pending
        } else if let Some(failure) = &state.failure {
            Unavailable::Failed(failure.error.message())
        } else {
            Unavailable::Missing
        }
    }

    fn pending_locked(&self, state: &ToolState) -> bool {
        state.record.wanted
            && (state.activity != Activity::Idle
                || state.retry.is_some()
                || state
                    .requests
                    .iter()
                    .any(|job| matches!(job, Job::Install { .. })))
    }

    fn status_locked(&self, state: &ToolState) -> MediaToolStatus {
        let current = self.current_version();
        let custom = state.settings.custom_yt_dlp();
        let yt_dlp = if let Some(path) = &custom {
            let display = path.to_string_lossy().into_owned();
            match state.custom_checks.get(path) {
                Some((_, Err(reason))) => YtDlpState::CustomBroken {
                    path: display,
                    reason: reason.clone(),
                },
                Some((_, Ok(version))) => YtDlpState::Custom {
                    path: display,
                    version: Some(version.clone()),
                },
                None => YtDlpState::Custom {
                    path: display,
                    version: None,
                },
            }
        } else if let Some(version) = &current {
            YtDlpState::Installed {
                version: version.clone(),
                last_checked_ms: state.record.last_checked_ms,
            }
        } else if let Some(failure) = state
            .failure
            .as_ref()
            .filter(|failure| !failure.during_update)
        {
            YtDlpState::Failed {
                reason: failure.error.message(),
            }
        } else {
            YtDlpState::NotInstalled
        };
        let activity = match state.activity {
            Activity::Idle => match state.retry {
                Some((at, _)) if state.record.wanted => YtDlpActivity::WaitingToRetry {
                    seconds: at.saturating_duration_since(Instant::now()).as_secs(),
                },
                _ => YtDlpActivity::Idle,
            },
            Activity::Checking => YtDlpActivity::Checking,
            Activity::Downloading { received, total } => YtDlpActivity::Downloading {
                progress: total
                    .filter(|total| *total > 0)
                    .map(|total| (received as f64 / total as f64).clamp(0.0, 1.0))
                    .unwrap_or(0.0),
            },
        };
        let yt_dlp_ready = matches!(
            yt_dlp,
            YtDlpState::Installed { .. } | YtDlpState::Custom { .. }
        );
        let yt_dlp_pending = custom.is_none() && current.is_none() && self.pending_locked(state);
        let update_error = state
            .failure
            .as_ref()
            .filter(|failure| failure.during_update && current.is_some() && custom.is_none())
            .map(|failure| failure.error.message());
        let bundled = self.bundled.get().cloned().unwrap_or_default();
        let custom_ffmpeg = state.settings.custom_ffmpeg();
        let ffmpeg = match &custom_ffmpeg {
            Some(path) => BundledToolStatus {
                path: Some(path.to_string_lossy().into_owned()),
                version: state
                    .custom_checks
                    .get(path)
                    .and_then(|(_, result)| result.as_ref().ok().cloned()),
                custom: true,
                problem: match state.custom_checks.get(path) {
                    Some((_, Err(reason))) => Some(reason.clone()),
                    _ => None,
                },
            },
            None => self.bundled_status("ffmpeg", bundled.ffmpeg.clone()),
        };
        let ffprobe = match &custom_ffmpeg {
            Some(path) => {
                let ffprobe = path.with_file_name("ffprobe");
                let present = is_executable(&ffprobe);
                BundledToolStatus {
                    path: present.then(|| ffprobe.to_string_lossy().into_owned()),
                    version: None,
                    custom: true,
                    problem: (!present).then(|| {
                        "There's no ffprobe next to your FFmpeg, so MP3 and M4A downloads and thumbnails may fail."
                            .to_string()
                    }),
                }
            }
            None => self.bundled_status("ffprobe", bundled.ffprobe.clone()),
        };
        let quickjs = self.bundled_status("qjs", bundled.quickjs.clone());
        let message = if let Some(path) = custom.as_ref().filter(|_| !yt_dlp_ready) {
            format!("Your yt-dlp at {} isn't working.", path.display())
        } else if yt_dlp_ready && ffmpeg.problem.is_none() && ffmpeg.path.is_some() {
            "Video downloads are ready.".to_string()
        } else if yt_dlp_ready {
            "Video downloads are ready, but FFmpeg isn't available, so formats can't be merged or converted."
                .to_string()
        } else {
            self.unavailable_locked(state).message()
        };
        MediaToolStatus {
            generation: state.generation,
            yt_dlp,
            activity,
            yt_dlp_ready,
            yt_dlp_pending,
            update_error,
            download_bytes: state.download_bytes,
            auto_update: state.settings.auto_update_yt_dlp,
            ffmpeg,
            ffprobe,
            quickjs,
            message,
        }
    }

    fn bundled_status(&self, name: &str, version: Option<String>) -> BundledToolStatus {
        let path = self.bundled_tool(name);
        BundledToolStatus {
            problem: path
                .is_none()
                .then(|| format!("{name} isn't in this copy of Opul.")),
            path: path.map(|path| path.to_string_lossy().into_owned()),
            version,
            custom: false,
        }
    }

    fn bundled_tool(&self, name: &str) -> Option<PathBuf> {
        self.helpers
            .as_ref()
            .map(|helpers| helpers.join(name))
            .filter(|path| is_executable(path))
    }

    /// The version `current` points at, if that version is still on disk.
    fn current_version(&self) -> Option<String> {
        let target = fs::read_link(self.root.join(CURRENT)).ok()?;
        let version = target.to_str()?.to_string();
        (is_version(&version) && is_executable(&self.root.join(&version).join(EXECUTABLE)))
            .then_some(version)
    }

    fn check_custom(&self, path: &Path) -> Result<String, String> {
        let identity = FileIdentity::of(path);
        if let Some(identity) = &identity {
            let state = self.lock();
            if let Some((checked, result)) = state.custom_checks.get(path)
                && checked == identity
            {
                return result.clone();
            }
        }
        let result = match &identity {
            Some(_) => run_version(path, &["--version"]),
            None => Err("the file is missing".to_string()),
        };
        let mut state = self.lock();
        state.custom_checks.insert(
            path.to_path_buf(),
            (identity.unwrap_or(FileIdentity::MISSING), result.clone()),
        );
        self.touch(&mut state);
        result
    }

    fn lock(&self) -> MutexGuard<'_, ToolState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn touch(&self, state: &mut ToolState) {
        state.generation += 1;
        self.changed.notify_all();
    }

    fn save_record(&self, state: &ToolState) {
        if fs::create_dir_all(&self.root).is_ok() {
            let _ = fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700));
            if let Err(error) = save_state(&self.root.join("state.json"), &state.record, |_| Ok(()))
            {
                eprintln!("Warning: could not save yt-dlp state: {error}");
            }
        }
    }

    fn run_scheduler(self: Arc<Self>) {
        loop {
            let job = {
                let mut state = self.lock();
                loop {
                    if state.shutdown {
                        return;
                    }
                    let now = Instant::now();
                    if let Some(job) = self.next_job(&mut state, now) {
                        break job;
                    }
                    let wait = self.next_wake(&state, now);
                    state = self
                        .changed
                        .wait_timeout(state, wait)
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .0;
                }
            };
            self.run_job(job);
        }
    }

    /// The next piece of background work that is due, if any. Automatic checks only happen
    /// while automatic updates are on and Opul manages yt-dlp itself.
    fn next_job(&self, state: &mut ToolState, now: Instant) -> Option<Job> {
        if !state.requests.is_empty() {
            return Some(state.requests.remove(0));
        }
        if state.settings.custom_yt_dlp().is_some() {
            return None;
        }
        if let Some((at, _)) = state.retry
            && at <= now
            && state.record.wanted
        {
            return Some(Job::Install { manual: false });
        }
        if state.settings.auto_update_yt_dlp
            && state.retry.is_none()
            && state.next_update_check.is_some_and(|at| at <= now)
        {
            if !state.leases.is_empty() {
                state.next_update_check = Some(now + BUSY_UPDATE_DELAY);
                return None;
            }
            return Some(Job::Install { manual: false });
        }
        None
    }

    fn next_wake(&self, state: &ToolState, now: Instant) -> Duration {
        let mut wake = now + Duration::from_secs(60 * 60);
        if let Some((at, _)) = state.retry {
            wake = wake.min(at);
        }
        if state.settings.auto_update_yt_dlp
            && let Some(at) = state.next_update_check
        {
            wake = wake.min(at);
        }
        wake.saturating_duration_since(now)
            .max(Duration::from_millis(10))
    }

    fn run_job(self: &Arc<Self>, job: Job) {
        match job {
            Job::Install { manual } => {
                let _ = self.install(manual);
            }
            Job::Size => {
                if let Some(version) = self.current_or_latest_version()
                    && let Ok(size) = self.source.asset_size(&version, ARCHIVE)
                {
                    let mut state = self.lock();
                    state.download_bytes = Some(size);
                    self.touch(&mut state);
                }
            }
            Job::Probe => {
                let _ = self.bundled.get_or_init(|| self.probe_bundled());
                let settings = self.lock().settings.clone();
                if let Some(path) = settings.custom_yt_dlp() {
                    let _ = self.check_custom(&path);
                }
                if let Some(path) = settings.custom_ffmpeg() {
                    self.check_custom_ffmpeg(&path);
                }
                let mut state = self.lock();
                self.touch(&mut state);
            }
        }
    }

    fn current_or_latest_version(&self) -> Option<String> {
        self.source.latest_version().ok()
    }

    fn probe_bundled(&self) -> BundledVersions {
        let version_of = |name: &str, arguments: &[&str]| {
            self.bundled_tool(name)
                .and_then(|path| run_version(&path, arguments).ok())
        };
        BundledVersions {
            ffmpeg: version_of("ffmpeg", &["-hide_banner", "-version"])
                .map(|line| ffmpeg_version(&line)),
            ffprobe: version_of("ffprobe", &["-hide_banner", "-version"])
                .map(|line| ffmpeg_version(&line)),
            quickjs: self
                .bundled_tool("qjs")
                .and_then(|path| quickjs_version(&path)),
        }
    }

    fn check_custom_ffmpeg(&self, path: &Path) {
        let identity = FileIdentity::of(path).unwrap_or(FileIdentity::MISSING);
        let result = Self::verify_ffmpeg(path);
        let mut state = self.lock();
        state
            .custom_checks
            .insert(path.to_path_buf(), (identity, result));
        self.touch(&mut state);
    }

    /// Checks GitHub for the latest stable yt-dlp and installs it when it is newer than the
    /// installed copy. Runs on the scheduler thread, or directly in tests.
    pub(crate) fn install(self: &Arc<Self>, manual: bool) -> Result<(), ToolError> {
        let during_update = self.current_version().is_some();
        {
            let mut state = self.lock();
            if state.settings.custom_yt_dlp().is_some() && !manual {
                return Ok(());
            }
            state.activity = Activity::Checking;
            self.touch(&mut state);
        }
        let result = self.check_and_install(manual);
        let mut state = self.lock();
        state.activity = Activity::Idle;
        let now = Instant::now();
        match &result {
            Ok(()) => {
                state.failure = None;
                state.retry = None;
                state.record.last_checked_ms = Some(now_millis());
            }
            Err(error @ ToolError::Network(_)) => {
                let attempts = state.retry.map(|(_, attempts)| attempts).unwrap_or(0);
                let delay = RETRY_DELAYS
                    .get(attempts)
                    .copied()
                    .unwrap_or(LONGEST_RETRY_DELAY);
                state.retry = Some((now + delay, attempts + 1));
                state.failure = Some(Failure {
                    error: error.clone(),
                    during_update,
                });
            }
            Err(error) => {
                state.retry = None;
                state.failure = Some(Failure {
                    error: error.clone(),
                    during_update,
                });
                state.record.last_checked_ms = Some(now_millis());
            }
        }
        if state.settings.auto_update_yt_dlp && self.current_version().is_some() {
            state.next_update_check = Some(now + UPDATE_INTERVAL);
        }
        self.save_record(&state);
        self.touch(&mut state);
        result
    }

    fn check_and_install(self: &Arc<Self>, manual: bool) -> Result<(), ToolError> {
        let version = self.source.latest_version()?;
        if !is_version(&version) {
            return Err(ToolError::Verification(format!(
                "GitHub named an unexpected release, {version:?}"
            )));
        }
        let current = self.current_version();
        if current.as_deref() == Some(version.as_str()) {
            return Ok(());
        }
        {
            let state = self.lock();
            if state.record.broken_versions.contains(&version) {
                return Ok(());
            }
            if !manual && state.record.rejected_versions.contains(&version) {
                return Ok(());
            }
        }
        if let Some(current) = &current
            && compare_versions(&version, current).is_lt()
        {
            return Ok(());
        }

        fs::create_dir_all(&self.root).map_err(storage_error)?;
        fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
            .map_err(storage_error)?;
        let staging = self.root.join(format!(".download-{}", Uuid::new_v4()));
        fs::create_dir(&staging).map_err(storage_error)?;
        let result = self.download_and_verify(&version, &staging);
        if let Err(error) = &result {
            let _ = fs::remove_dir_all(&staging);
            let mut state = self.lock();
            let list = match error {
                ToolError::Verification(_) => Some(&mut state.record.rejected_versions),
                ToolError::Broken(_) => Some(&mut state.record.broken_versions),
                _ => None,
            };
            if let Some(list) = list
                && !list.contains(&version)
            {
                list.push(version.clone());
            }
            self.save_record(&state);
            return result;
        }
        let _ = fs::remove_dir_all(&staging);

        let mut state = self.lock();
        let next = self.root.join(format!("{CURRENT}.next"));
        let _ = fs::remove_file(&next);
        std::os::unix::fs::symlink(&version, &next).map_err(storage_error)?;
        fs::rename(&next, self.root.join(CURRENT)).map_err(storage_error)?;
        state.record.previous = current;
        state
            .record
            .rejected_versions
            .retain(|rejected| rejected != &version);
        self.remove_unused_versions(&state);
        self.save_record(&state);
        Ok(())
    }

    fn download_and_verify(&self, version: &str, staging: &Path) -> Result<(), ToolError> {
        let mut checksums = Vec::new();
        self.source.download(
            version,
            CHECKSUMS,
            MAX_SMALL_FILE_BYTES,
            &mut checksums,
            &mut |_, _| {},
        )?;
        let mut signature = Vec::new();
        self.source.download(
            version,
            SIGNATURE,
            MAX_SMALL_FILE_BYTES,
            &mut signature,
            &mut |_, _| {},
        )?;
        verify_signature(&self.signing_key, &checksums, &signature)?;
        let expected = expected_checksum(&checksums, ARCHIVE)?;

        let archive_path = staging.join(ARCHIVE);
        let mut archive = HashingWriter {
            file: File::create(&archive_path).map_err(storage_error)?,
            hasher: Sha256::new(),
        };
        let mut last_reported = 0;
        self.source.download(
            version,
            ARCHIVE,
            MAX_ARCHIVE_BYTES,
            &mut archive,
            &mut |received, total| {
                if received < last_reported + 256 * 1024 && Some(received) != total {
                    return;
                }
                last_reported = received;
                let mut state = self.lock();
                state.activity = Activity::Downloading { received, total };
                if let Some(total) = total {
                    state.download_bytes = Some(total);
                }
                self.touch(&mut state);
            },
        )?;
        archive.file.flush().map_err(storage_error)?;
        let actual = hex::encode(archive.hasher.finalize());
        if actual != expected {
            return Err(ToolError::Verification(format!(
                "{ARCHIVE} has SHA-256 {actual}, but {CHECKSUMS} lists {expected}"
            )));
        }

        let extracted = staging.join("extracted");
        let output = Command::new("/usr/bin/ditto")
            .arg("-x")
            .arg("-k")
            .arg(&archive_path)
            .arg(&extracted)
            .stdin(Stdio::null())
            .output()
            .map_err(storage_error)?;
        if !output.status.success() {
            return Err(ToolError::Storage(format!(
                "could not extract {ARCHIVE}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        let executable = extracted.join(EXECUTABLE);
        let metadata = fs::symlink_metadata(&executable)
            .map_err(|_| ToolError::Broken(format!("{ARCHIVE} does not contain {EXECUTABLE}")))?;
        if !metadata.is_file() {
            return Err(ToolError::Broken(format!(
                "{EXECUTABLE} is not a regular file"
            )));
        }
        run_version(&executable, &["--version"]).map_err(ToolError::Broken)?;

        let destination = self.root.join(version);
        if destination.exists() {
            fs::remove_dir_all(&destination).map_err(storage_error)?;
        }
        fs::rename(&extracted, &destination).map_err(storage_error)?;
        fs::set_permissions(&destination, fs::Permissions::from_mode(0o700))
            .map_err(storage_error)?;
        Ok(())
    }

    /// Keeps the current and previous versions and any version a process is using.
    fn remove_unused_versions(&self, state: &ToolState) {
        let current = self.current_version();
        let Ok(entries) = fs::read_dir(&self.root) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let keep = current.as_deref() == Some(name.as_str())
                || (current.is_some() && state.record.previous.as_deref() == Some(name.as_str()))
                || state.leases.contains_key(&name);
            if is_version(&name) && !keep {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

struct HashingWriter {
    file: File,
    hasher: Sha256,
}

impl Write for HashingWriter {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        let written = self.file.write(buffer)?;
        self.hasher.update(&buffer[..written]);
        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn verify_signature(
    key: &SignedPublicKey,
    checksums: &[u8],
    signature: &[u8],
) -> Result<(), ToolError> {
    let signature = DetachedSignature::from_bytes(signature)
        .map_err(|error| ToolError::Verification(format!("{SIGNATURE} is unreadable: {error}")))?;
    let verified = signature.verify(key, checksums).is_ok()
        || key
            .public_subkeys
            .iter()
            .any(|subkey| signature.verify(subkey, checksums).is_ok());
    if verified {
        Ok(())
    } else {
        Err(ToolError::Verification(format!(
            "{SIGNATURE} was not made by yt-dlp's release key"
        )))
    }
}

fn expected_checksum(checksums: &[u8], name: &str) -> Result<String, ToolError> {
    let text = std::str::from_utf8(checksums)
        .map_err(|_| ToolError::Verification(format!("{CHECKSUMS} is not text")))?;
    text.lines()
        .filter_map(|line| {
            let (hash, file) = line.split_once(char::is_whitespace)?;
            Some((hash, file.trim().trim_start_matches('*')))
        })
        .find(|(hash, file)| {
            *file == name && hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        })
        .map(|(hash, _)| hash.to_ascii_lowercase())
        .ok_or_else(|| ToolError::Verification(format!("{CHECKSUMS} does not list {name}")))
}

/// yt-dlp's stable releases are named like 2026.10.21, sometimes with a fourth number.
fn is_version(value: &str) -> bool {
    let parts = value.split('.').collect::<Vec<_>>();
    (3..=4).contains(&parts.len())
        && parts[0].len() == 4
        && parts[1].len() == 2
        && parts[2].len() == 2
        && parts.iter().all(|part| {
            !part.is_empty() && part.len() <= 6 && part.bytes().all(|b| b.is_ascii_digit())
        })
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    let parse = |value: &str| {
        value
            .split('.')
            .map(|part| part.parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    parse(left).cmp(&parse(right))
}

/// yt-dlp looks for ffprobe next to the FFmpeg it is given. A folder only works when the
/// file is called ffmpeg, so another name is passed as the file itself.
fn ffmpeg_location(path: &Path) -> PathBuf {
    match (path.file_name(), path.parent()) {
        (Some(name), Some(folder)) if name == "ffmpeg" && !folder.as_os_str().is_empty() => {
            folder.to_path_buf()
        }
        _ => path.to_path_buf(),
    }
}

fn ffmpeg_version(line: &str) -> String {
    line.split_whitespace()
        .skip_while(|word| *word != "version")
        .nth(1)
        .unwrap_or(line)
        .to_string()
}

fn quickjs_version(path: &Path) -> Option<String> {
    // Like stock qjs, the driver prints its version for --help and exits with 1.
    let output =
        run_with_timeout(Command::new(path).arg("--help"), Duration::from_secs(10)).ok()?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()?
        .strip_prefix("QuickJS version ")
        .map(|version| version.trim().to_string())
}

fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

fn run_version(path: &Path, arguments: &[&str]) -> Result<String, String> {
    let output = run_with_timeout(Command::new(path).args(arguments), VERSION_TIMEOUT)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = stderr
            .lines()
            .rev()
            .find(|line| !line.trim().is_empty())
            .unwrap_or("no output");
        return Err(format!(
            "it exited with {} ({})",
            output.status,
            detail.trim()
        ));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "it printed no version".to_string())
}

fn run_with_timeout(command: &mut Command, timeout: Duration) -> Result<Output, String> {
    let child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("it could not start ({error})"))?;
    let pid = child.id();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(child.wait_with_output());
    });
    match receiver.recv_timeout(timeout) {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(error)) => Err(format!("it could not run ({error})")),
        Err(_) => {
            let _ = Command::new("/bin/kill")
                .arg("-KILL")
                .arg(pid.to_string())
                .status();
            Err(format!(
                "it did not finish within {} seconds",
                timeout.as_secs()
            ))
        }
    }
}

fn storage_error(error: std::io::Error) -> ToolError {
    ToolError::Storage(error.to_string())
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// yt-dlp's GitHub releases. The latest tag comes from the redirect of `releases/latest`
/// rather than the API, which is rate limited, and every file comes from that one tag.
struct GitHubReleases;

impl GitHubReleases {
    fn runtime() -> Result<tokio::runtime::Runtime, ToolError> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| ToolError::Network(error.to_string()))
    }

    fn client(redirects: bool) -> Result<reqwest::Client, ToolError> {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(20))
            .read_timeout(Duration::from_secs(60))
            .redirect(if redirects {
                reqwest::redirect::Policy::limited(10)
            } else {
                reqwest::redirect::Policy::none()
            })
            .user_agent(concat!("Opul/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| ToolError::Network(error.to_string()))
    }

    fn asset_url(version: &str, asset: &str) -> String {
        format!("{DOWNLOAD_URL}/{version}/{asset}")
    }
}

fn network_error(error: reqwest::Error) -> ToolError {
    ToolError::Network(error.to_string())
}

impl ReleaseSource for GitHubReleases {
    fn latest_version(&self) -> Result<String, ToolError> {
        Self::runtime()?.block_on(async {
            let response = Self::client(false)?
                .head(LATEST_RELEASE_URL)
                .send()
                .await
                .map_err(network_error)?;
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|value| value.to_str().ok())
                .ok_or_else(|| {
                    ToolError::Network(format!(
                        "GitHub answered {} without naming the latest release",
                        response.status()
                    ))
                })?;
            location
                .trim_end_matches('/')
                .rsplit_once("/releases/tag/")
                .map(|(_, tag)| tag.to_string())
                .ok_or_else(|| ToolError::Network(format!("unexpected release address {location}")))
        })
    }

    fn asset_size(&self, version: &str, asset: &str) -> Result<u64, ToolError> {
        Self::runtime()?.block_on(async {
            let response = Self::client(true)?
                .head(Self::asset_url(version, asset))
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(network_error)?;
            response
                .content_length()
                .filter(|length| *length > 0)
                .ok_or_else(|| ToolError::Network("GitHub did not say how large it is".to_string()))
        })
    }

    fn download(
        &self,
        version: &str,
        asset: &str,
        limit: u64,
        destination: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), ToolError> {
        Self::runtime()?.block_on(async {
            let mut response = Self::client(true)?
                .get(Self::asset_url(version, asset))
                .send()
                .await
                .and_then(reqwest::Response::error_for_status)
                .map_err(network_error)?;
            let total = response.content_length();
            if total.is_some_and(|total| total > limit) {
                return Err(ToolError::Verification(format!(
                    "{asset} is unexpectedly large"
                )));
            }
            let mut received = 0u64;
            while let Some(chunk) = response.chunk().await.map_err(network_error)? {
                received += chunk.len() as u64;
                if received > limit {
                    return Err(ToolError::Verification(format!(
                        "{asset} is unexpectedly large"
                    )));
                }
                destination.write_all(&chunk).map_err(storage_error)?;
                progress(received, total);
            }
            if total.is_some_and(|total| total != received) {
                return Err(ToolError::Network(format!(
                    "the download of {asset} ended early"
                )));
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests;
