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
    });
}

fn download(machine: &Arc<OpulStateMachine>, url: String) -> Result<String, String> {
    if std::env::var_os("OPUL_YT_DLP").is_some() {
        return Err("Packaged smoke must use Opul's managed tools, without OPUL_YT_DLP.".into());
    }
    let tools = machine
        .video_tools()
        .ok_or("Video tools module unavailable")?;
    tools.execute(VideoToolsOperation::SetUp)?;
    let deadline = Instant::now() + Duration::from_secs(300);
    loop {
        match tools.snapshot().state {
            VideoToolsState::Ready { .. } => break,
            VideoToolsState::NotWorking { details, .. } => return Err(details),
            _ if Instant::now() >= deadline => return Err("Packaged setup timed out.".into()),
            _ => std::thread::sleep(Duration::from_millis(100)),
        }
    }
    let probe = machine
        .probe_media(url.clone())
        .map_err(|e| e.to_string())?;
    let snapshot = machine.snapshot();
    let workspace = snapshot.workspaces.first().ok_or("No smoke workspace")?;
    let folder = workspace.folders.first().ok_or("No smoke folder")?;
    let item = probe.items.first().ok_or("No media in smoke probe")?;
    let receipt = machine
        .add_media(super::commands::media_request(
            workspace.id.clone(),
            folder.id.clone(),
            MachineFormat::Original,
            vec![MachineMediaItem {
                url: item.webpage_url.clone(),
                page_url: url,
                title: "Permitted-content smoke".into(),
                thumbnail_url: None,
                extractor: item.extractor.clone(),
                playlist_index: None,
            }],
        ))
        .map_err(|e| e.to_string())?;
    let id = receipt
        .created_download_ids
        .first()
        .ok_or("No smoke download created")?;
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
                return Ok(format!(
                    "{}\nOutput: {output}, {} bytes\nBuilt in: {:?}",
                    tools.diagnostics(),
                    metadata.len(),
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
