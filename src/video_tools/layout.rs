//! Where each part of the video tools lives inside Tools/. Every path is built here, so
//! installing, selecting, pruning and recovering always agree on the layout.

use super::{InstalledPair, VideoToolsConfig, VideoToolsInvocation, YtDlpChannel};
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub(super) const RUNTIMES: &str = "python-runtime";
pub(super) const SCRIPTS: &str = "yt-dlp";
pub(super) const STATE: &str = "state.json";
pub(super) const LOG: &str = "updates.log";
pub(super) const LOCK: &str = "tools.lock";
const STAGING_PREFIX: &str = ".install-";
const STATE_DRAFT_PREFIX: &str = ".state-";

pub(super) fn runtime_name(runtime: u64) -> String {
    runtime.to_string()
}

pub(super) fn script_name(channel: YtDlpChannel, version: &str) -> String {
    match channel {
        YtDlpChannel::Stable => version.to_string(),
        YtDlpChannel::Nightly => format!("nightly-{version}"),
    }
}

pub(super) fn runtime_dir(root: &Path, runtime: u64) -> PathBuf {
    root.join(RUNTIMES).join(runtime_name(runtime))
}

pub(super) fn script_dir(root: &Path, channel: YtDlpChannel, version: &str) -> PathBuf {
    root.join(SCRIPTS).join(script_name(channel, version))
}

pub(super) fn staging_dir(root: &Path, id: Uuid) -> PathBuf {
    root.join(format!("{STAGING_PREFIX}{id}"))
}

pub(super) fn state_draft(root: &Path) -> PathBuf {
    root.join(format!("{STATE_DRAFT_PREFIX}{}.json", Uuid::new_v4()))
}

/// Leftovers of an interrupted install or state write. Never selected, always safe to delete.
pub(super) fn is_temporary(name: &str) -> bool {
    name.starts_with(STAGING_PREFIX)
        || (name.starts_with(STATE_DRAFT_PREFIX) && name.ends_with(".json"))
}

/// The commands inside a runtime directory and a script directory, wherever those live.
pub(super) fn invocation(runtime: &Path, script: &Path) -> VideoToolsInvocation {
    VideoToolsInvocation {
        interpreter: runtime.join("bin/python3"),
        quickjs: runtime.join("bin/qjs"),
        script: script_file(script),
    }
}

pub(super) fn script_file(script_dir: &Path) -> PathBuf {
    script_dir.join("yt-dlp")
}

pub(super) fn selected(config: &VideoToolsConfig, pair: &InstalledPair) -> VideoToolsInvocation {
    invocation(
        &runtime_dir(&config.root, pair.runtime),
        &script_dir(&config.root, pair.yt_dlp_channel, &pair.yt_dlp),
    )
}
