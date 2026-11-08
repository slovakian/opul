//! Opt-in acceptance against the packaged app, real tools, HTTP and BitTorrent.
mod lifecycle;
mod transfers;
mod ui;
mod updates;

use opul_core::video_tools::{VideoTools, VideoToolsState};
use opul_core::{
    MachineDownload, MachineDownloadState, MachineFormat, MachineMediaItem, OpulStateMachine,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

#[derive(Deserialize)]
pub struct Config {
    mode: String,
    base_url: String,
    gate: PathBuf,
    expected_runtime: u64,
}

pub fn run(
    app: &tauri::AppHandle,
    machine: &Arc<OpulStateMachine>,
    path: PathBuf,
) -> Result<Value, String> {
    until(
        || {
            Ok(std::env::var_os("OPUL_SMOKE_READY_FILE")
                .is_some_and(|path| std::path::Path::new(&path).exists()))
        },
        30,
    )?;
    let config: Config = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let tools = machine.video_tools().ok_or("No managed tools")?;
    let mut evidence = Vec::new();
    match config.mode.as_str() {
        "offer" => lifecycle::offer(app, machine, &tools, &config, &mut evidence)?,
        "setup" => lifecycle::setup(app, machine, &tools, &config, &mut evidence)?,
        "manual" => updates::manual(app, machine, &tools, &config, &mut evidence)?,
        "automatic" => updates::automatic(app, machine, &tools, &config, &mut evidence)?,
        "restart" => {
            wait_runtime(&tools, config.expected_runtime)?;
            transfers::independent(machine, &config, "restart", &mut evidence)?;
            evidence.push(json!({"restart": "Ready and independent transfers verified"}));
        }
        _ => return Err("Unknown acceptance mode".into()),
    }
    Ok(json!({"ok": true, "mode": config.mode, "tools": tools.snapshot(), "evidence": evidence}))
}

fn until(mut condition: impl FnMut() -> Result<bool, String>, seconds: u64) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(seconds);
    while !condition()? {
        if Instant::now() >= deadline {
            return Err("Packaged acceptance condition timed out".into());
        }
        thread::sleep(Duration::from_millis(50));
    }
    Ok(())
}

fn item(machine: &OpulStateMachine, id: &str) -> Result<MachineDownload, String> {
    machine
        .snapshot()
        .workspaces
        .into_iter()
        .flat_map(|w| w.downloads)
        .find(|d| d.id == id)
        .ok_or_else(|| format!("Download {id} disappeared"))
}

fn wait_finished(machine: &OpulStateMachine, id: &str) -> Result<MachineDownload, String> {
    until(
        || {
            let download = item(machine, id)?;
            if download.state == MachineDownloadState::Failed {
                return Err(download
                    .error_message
                    .unwrap_or_else(|| "Download failed".into()));
            }
            Ok(download.state == MachineDownloadState::Finished)
        },
        180,
    )?;
    item(machine, id)
}

fn runtime(tools: &VideoTools) -> Option<u64> {
    match tools.snapshot().state {
        VideoToolsState::Ready { installed, .. } => Some(installed.runtime),
        _ => None,
    }
}

fn wait_runtime(tools: &VideoTools, expected: u64) -> Result<(), String> {
    until(
        || {
            if let VideoToolsState::NotWorking { details, .. } = tools.snapshot().state {
                return Err(details);
            }
            Ok(runtime(tools) == Some(expected))
        },
        300,
    )
}

fn add_video(machine: &OpulStateMachine, url: String, pending: bool) -> Result<String, String> {
    let snapshot = machine.snapshot();
    let workspace = &snapshot.workspaces[0];
    let request = crate::commands::media_request(
        workspace.id.clone(),
        workspace.folders[0].id.clone(),
        MachineFormat::Original,
        vec![MachineMediaItem {
            url: url.clone(),
            page_url: url,
            title: "Permitted acceptance fixture".into(),
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
        }],
    );
    let receipt = if pending {
        machine.add_video_page(request)
    } else {
        machine.add_media(request)
    }
    .map_err(|e| e.to_string())?;
    receipt
        .created_download_ids
        .first()
        .cloned()
        .ok_or_else(|| "No Download created".into())
}
