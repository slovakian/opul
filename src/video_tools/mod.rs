//! Opul owns a verified Python runtime and the official bare yt-dlp script as one selected pair.
//!
//! Call `execute` to change tools, read `snapshot`, or `subscribe` to pushed snapshots. Only
//! `invocation` in Ready supplies an interpreter/script/QuickJS command to MediaEngine.
//! Setup requires an explicit command, including after TurnedOff. Preferences commit immediately.
//! Installers stage both parts, verify each project's signature before its checksum, unpack,
//! health-check the pair, then atomically replace state.json. A failed candidate is never selected.
//! Ready schedules a check after one minute and every 24 hours; failures are quiet. One persisted
//! preference governs both components. take_update_notice durably claims each consent notification.
//! Manual checks and updates keep Ready and its selected pair usable. MediaEngine acquires a
//! VideoToolsUse at execution and reports completion; installation waits for idle, and the first
//! successful Download releases the retained runtime. Failure before that success restores the
//! full previous pair. Compatibility holds and integrity skips survive restart. See docs/video-tools.md for pairing, storage and the sibling-ticket handoff.

mod install;
mod manager;
mod nightly;
mod recovery;
mod removal;
mod schedule;
mod transport;
mod updates;
mod verification;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

pub use manager::VideoTools;
pub use transport::{NetworkEnvironment, NetworkFuture};
pub use updates::VideoToolsUse;

#[derive(Debug, Clone)]
pub struct VideoToolsConfig {
    /// Already resolved by the shell for Stable/Nightly and OPUL_DATA_DIR. Ends in Tools/.
    pub root: PathBuf,
    pub opul_version: String,
    pub built_in_tools: PathBuf,
    pub nightly_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VideoToolsOperation {
    SetUp,
    Remove { turn_off: bool },
    CancelSetup,
    Cancel,
    CheckForUpdates,
    Update,
    Retry,
    TurnOff,
    SetKeepUpToDate { enabled: bool },
    SetUseNightly { enabled: bool },
    RefreshPublishedSize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VideoToolsSnapshot {
    pub sequence: u64,
    pub state: VideoToolsState,
    pub keep_up_to_date: bool,
    pub nightly_available: bool,
    pub use_nightly: bool,
    pub update_available: bool,
    pub published_size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VideoToolsState {
    NotSetUp,
    TurnedOff,
    SettingUp {
        steps: Vec<SetupStep>,
    },
    Ready {
        installed: InstalledPair,
        last_checked: Option<u64>,
        update_flag: UpdateFlag,
    },
    NotWorking {
        reason: FailureReason,
        details: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UpdateFlag {
    None,
    Available { version: String },
    Updating { version: String, progress: u8 },
    Failed { version: String },
    WaitingForRuntime { version: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FailureReason {
    Download,
    Verification,
    HealthCheck,
    StoppedWorking,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SetupStep {
    pub component: Component,
    pub status: StepStatus,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Component {
    Python,
    YtDlp,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    WaitingForInternet,
    Downloading,
    Verifying,
    Installing,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateNotice {
    pub identity: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledPair {
    pub runtime: u64,
    pub python_version: String,
    pub yt_dlp: String,
    #[serde(default)]
    pub yt_dlp_channel: YtDlpChannel,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum YtDlpChannel {
    #[default]
    Stable,
    Nightly,
}

impl YtDlpChannel {
    fn directory(self, version: &str) -> String {
        match self {
            Self::Stable => version.to_string(),
            Self::Nightly => format!("nightly-{version}"),
        }
    }
}

/// A selected, health-checked pair. Paths come from the module, never user preferences or PATH.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoToolsInvocation {
    pub interpreter: PathBuf,
    pub script: PathBuf,
    pub quickjs: PathBuf,
    pub built_in_tools: PathBuf,
}

impl VideoToolsInvocation {
    pub fn command(&self) -> Command {
        let mut command = Command::new(&self.interpreter);
        command
            .arg("-I")
            .arg(&self.script)
            .args([
                "--ignore-config",
                "--no-plugin-dirs",
                "--no-cache-dir",
                "--no-update",
                "--impersonate",
                "chrome",
                "--no-js-runtimes",
                "--js-runtimes",
            ])
            .arg(format!("quickjs:{}", self.quickjs.display()))
            .arg("--ffmpeg-location")
            .arg(&self.built_in_tools);
        command
    }
}

#[cfg(test)]
mod tests;
