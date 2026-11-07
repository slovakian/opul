//! Opt-in packaged-app integration check. The harness supplies consent and isolated data paths.
use crate::Desktop;
use opul_core::video_tools::{VideoToolsOperation, VideoToolsState};
use opul_core::{MachineDownloadState, MachineFormat, MachineMediaItem, OpulStateMachine};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tauri::Manager;

pub fn start(app: &tauri::AppHandle) {
    let (Some(url), Some(report)) = (
        std::env::var_os("OPUL_SMOKE_VIDEO_URL"),
        std::env::var_os("OPUL_SMOKE_VIDEO_REPORT"),
    ) else {
        return;
    };
    let machine = app.state::<Desktop>().machine();
    let app = app.clone();
    std::thread::spawn(move || {
        let result =
            machine.and_then(|machine| download(&machine, url.to_string_lossy().into_owned()));
        let result = match result {
            Ok(message) => serde_json::json!({ "ok": true, "evidence": message }),
            Err(error) => serde_json::json!({ "ok": false, "error": error }),
        };
        let _ = std::fs::write(
            report,
            serde_json::to_vec_pretty(&result).unwrap_or_default(),
        );
        // Closing the final window exercises the ordinary ExitRequested shutdown path.
        let deadline = Instant::now() + Duration::from_secs(30);
        while std::env::var_os("OPUL_SMOKE_READY_FILE")
            .is_some_and(|path| !std::path::Path::new(&path).exists())
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(100));
        }
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.close();
        }
    });
}

fn download(machine: &Arc<OpulStateMachine>, url: String) -> Result<String, String> {
    if std::env::var_os("OPUL_YT_DLP").is_some() {
        return Err("Packaged smoke must use Opul's managed tools, without OPUL_YT_DLP.".into());
    }
    let tools = machine
        .video_tools()
        .ok_or("Video tools module unavailable")?;
    if std::env::var_os("OPUL_SMOKE_VIDEO_RESTART").is_some() {
        if !matches!(tools.snapshot().state, VideoToolsState::Ready { .. }) {
            return Err("Re-setup did not survive restart.".into());
        }
        let snapshot = machine.snapshot();
        let finished = snapshot
            .workspaces
            .iter()
            .flat_map(|w| &w.downloads)
            .any(|d| {
                d.state == MachineDownloadState::Finished
                    && d.output_path.as_ref().is_some_and(|p| {
                        std::fs::metadata(p).is_ok_and(|metadata| metadata.len() >= 100_000)
                    })
            });
        if !finished {
            return Err("Completed output did not survive restart.".into());
        }
        return Ok(format!(
            "Restart preserved Ready and completed output.\n{}",
            tools.diagnostics()
        ));
    }
    if !matches!(tools.snapshot().state, VideoToolsState::NotSetUp) {
        return Err("Packaged journey requires a fresh data directory.".into());
    }
    let snapshot = machine.snapshot();
    let workspace = snapshot.workspaces.first().ok_or("No smoke workspace")?;
    let folder = workspace.folders.first().ok_or("No smoke folder")?;
    let receipt = machine
        .add_video_page(super::commands::media_request(
            workspace.id.clone(),
            folder.id.clone(),
            MachineFormat::Original,
            vec![MachineMediaItem {
                url: url.clone(),
                page_url: url,
                title: "Permitted-content smoke".into(),
                thumbnail_url: None,
                extractor: None,
                playlist_index: None,
            }],
        ))
        .map_err(|e| e.to_string())?;
    let id = receipt
        .created_download_ids
        .first()
        .ok_or("No smoke download created")?;
    let waiting = machine
        .snapshot()
        .workspaces
        .iter()
        .flat_map(|w| &w.downloads)
        .any(|d| &d.id == id && d.state == MachineDownloadState::WaitingForVideoTools);
    if !waiting || tools.invocation().is_some() {
        return Err("Video did not wait for explicit setup consent.".into());
    }
    tools.execute(VideoToolsOperation::SetUp)?;
    wait_ready(&tools)?;
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        let snapshot = machine.snapshot();
        let item = snapshot
            .workspaces
            .iter()
            .flat_map(|w| &w.downloads)
            .find(|d| &d.id == id)
            .ok_or("Smoke download disappeared")?;
        match item.state {
            MachineDownloadState::Finished => {
                let output = item
                    .output_path
                    .as_ref()
                    .ok_or("Smoke download has no output")?;
                let metadata = std::fs::metadata(output).map_err(|e| e.to_string())?;
                if metadata.len() < 100_000 {
                    return Err("Smoke output is unexpectedly small.".into());
                }
                let output_bytes = metadata.len();
                machine
                    .remove_video_tools(true)
                    .map_err(|e| e.to_string())?;
                if !matches!(tools.snapshot().state, VideoToolsState::TurnedOff)
                    || tools.invocation().is_some()
                    || std::fs::metadata(output).map_err(|e| e.to_string())?.len() != output_bytes
                {
                    return Err("Removal did not turn tools off and preserve output.".into());
                }
                tools.execute(VideoToolsOperation::SetUp)?;
                wait_ready(&tools)?;
                return Ok(format!(
                    "Waiting before consent, verified setup, completion, removal to Turned off, and re-setup passed.\n{}\nOutput: {output}, {output_bytes} bytes\nBuilt in: {:?}",
                    tools.diagnostics(),
                    machine.built_in_tool_versions()
                ));
            }
            MachineDownloadState::Failed => {
                return Err(item
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Smoke media download failed.".into()));
            }
            _ if Instant::now() >= deadline => {
                return Err("Packaged video download timed out.".into());
            }
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

fn wait_ready(tools: &opul_core::video_tools::VideoTools) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        match tools.snapshot().state {
            VideoToolsState::Ready { .. } => return Ok(()),
            VideoToolsState::NotWorking { details, .. } => return Err(details),
            _ if Instant::now() >= deadline => return Err("Packaged setup timed out.".into()),
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}
