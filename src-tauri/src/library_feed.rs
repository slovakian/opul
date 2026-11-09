//! Pushes the library to each webview as it changes.
//!
//! A subscriber first receives the whole library, then one patch per published change carrying
//! only what differs from the last message: changed scalar fields, the workspace outlines when a
//! workspace or its download order changed, and the downloads whose projection changed. When
//! nothing changes, nothing is sent.

use opul_core::{
    MachineDownload, MachineFileOrganization, MachineFolder, MachineFormat, MachineGrouping,
    MachineLayout, MachineSessionState, MachineSnapshot, MachineSort, MachineWorkspace,
    MachineWorkspaceIcon, OpulStateMachine, WorkspaceAppearance,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{
    Arc, Mutex, PoisonError,
    atomic::{AtomicBool, Ordering},
};
use tauri::ipc::{Channel, InvokeResponseBody};

/// How long a feed sleeps before checking whether a newer subscription replaced it.
const IDLE_WAIT_MILLISECONDS: u32 = 30_000;

#[derive(Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FeedMessage<'a> {
    Snapshot(&'a MachineSnapshot),
    Patch(Patch<'a>),
}

/// The difference between two snapshots of the same boot. Absent fields are unchanged.
#[derive(Debug, PartialEq, Serialize)]
pub struct Patch<'a> {
    pub sequence: u64,
    pub revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<MachineSessionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_emoji: Option<&'a [String]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_notices: Option<&'a [String]>,
    /// Every workspace in order, present when any workspace, or the downloads it holds, changed.
    /// A download no outline lists has been removed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspaces: Option<Vec<WorkspaceOutline<'a>>>,
    /// Downloads that are new or whose projection changed.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub downloads: Vec<&'a MachineDownload>,
}

/// A workspace without its downloads, which travel separately and only when they change.
#[derive(Debug, PartialEq, Serialize)]
pub struct WorkspaceOutline<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub icon: &'a MachineWorkspaceIcon,
    pub appearance: &'a WorkspaceAppearance,
    pub folders: &'a [MachineFolder],
    pub default_format: MachineFormat,
    pub file_organization: MachineFileOrganization,
    pub layout: MachineLayout,
    pub sort: MachineSort,
    pub grouping: MachineGrouping,
    pub download_ids: Vec<&'a str>,
}

impl<'a> WorkspaceOutline<'a> {
    fn of(workspace: &'a MachineWorkspace) -> Self {
        Self {
            id: &workspace.id,
            name: &workspace.name,
            icon: &workspace.icon,
            appearance: &workspace.appearance,
            folders: &workspace.folders,
            default_format: workspace.default_format,
            file_organization: workspace.file_organization,
            layout: workspace.layout,
            sort: workspace.sort,
            grouping: workspace.grouping,
            download_ids: workspace
                .downloads
                .iter()
                .map(|download| download.id.as_str())
                .collect(),
        }
    }
}

/// What a subscriber that has seen `previous` needs in order to show `next`.
pub fn changes<'a>(previous: &MachineSnapshot, next: &'a MachineSnapshot) -> FeedMessage<'a> {
    if previous.boot_id != next.boot_id {
        return FeedMessage::Snapshot(next);
    }
    let outlines: Vec<_> = next.workspaces.iter().map(WorkspaceOutline::of).collect();
    let outlines_changed = previous.workspaces.len() != outlines.len()
        || previous
            .workspaces
            .iter()
            .zip(&outlines)
            .any(|(workspace, outline)| WorkspaceOutline::of(workspace) != *outline);
    let seen: HashMap<&str, &MachineDownload> = previous
        .workspaces
        .iter()
        .flat_map(|workspace| &workspace.downloads)
        .map(|download| (download.id.as_str(), download))
        .collect();
    let downloads = next
        .workspaces
        .iter()
        .flat_map(|workspace| &workspace.downloads)
        .filter(|download| {
            seen.get(download.id.as_str())
                .is_none_or(|before| *before != *download)
        })
        .collect();
    FeedMessage::Patch(Patch {
        sequence: next.sequence,
        revision: next.revision,
        session: (previous.session != next.session).then_some(next.session),
        recent_emoji: (previous.recent_emoji != next.recent_emoji)
            .then_some(next.recent_emoji.as_slice()),
        recovery_notices: (previous.recovery_notices != next.recovery_notices)
            .then_some(next.recovery_notices.as_slice()),
        workspaces: outlines_changed.then_some(outlines),
        downloads,
    })
}

/// The live feed of each webview, so a reloaded page replaces the feed of the page it replaced.
#[derive(Default)]
pub struct LibraryFeeds {
    active: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl LibraryFeeds {
    pub fn start(
        &self,
        webview: String,
        machine: Arc<OpulStateMachine>,
        channel: Channel,
    ) -> Result<(), String> {
        let stopped = Arc::new(AtomicBool::new(false));
        if let Some(replaced) = self
            .active
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(webview, Arc::clone(&stopped))
        {
            replaced.store(true, Ordering::Release);
        }
        std::thread::Builder::new()
            .name("opul-library-feed".into())
            .spawn(move || stream(&machine, &channel, &stopped))
            .map(drop)
            .map_err(|error| format!("Couldn't start the library feed: {error}"))
    }
}

fn stream(machine: &OpulStateMachine, channel: &Channel, stopped: &AtomicBool) {
    let mut sent = machine.snapshot();
    if send(channel, &FeedMessage::Snapshot(&sent)).is_err() {
        return;
    }
    while !stopped.load(Ordering::Acquire) {
        let next =
            machine.wait_for_snapshot(sent.boot_id.clone(), sent.sequence, IDLE_WAIT_MILLISECONDS);
        let unchanged = next.boot_id == sent.boot_id && next.sequence == sent.sequence;
        if unchanged || stopped.load(Ordering::Acquire) {
            continue;
        }
        if send(channel, &changes(&sent, &next)).is_err() {
            return;
        }
        sent = next;
    }
}

fn send(channel: &Channel, message: &FeedMessage<'_>) -> Result<(), String> {
    let json = serde_json::to_string(message).map_err(|error| error.to_string())?;
    channel
        .send(InvokeResponseBody::Json(json))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "library_feed_tests.rs"]
mod tests;
