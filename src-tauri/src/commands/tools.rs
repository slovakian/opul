use crate::Desktop;
use opul_core::BuiltInToolVersions;
use opul_core::video_tools::{UpdateNotice, VideoToolsOperation, VideoToolsSnapshot};
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
pub struct ToolsSnapshot {
    video: VideoToolsSnapshot,
    built_in: BuiltInToolVersions,
}

#[tauri::command]
pub fn video_tools_details(state: State<'_, Desktop>) -> Result<String, String> {
    Ok(state.machine()?.video_tools().diagnostics())
}

#[tauri::command]
pub fn tools_snapshot(state: State<'_, Desktop>) -> Result<ToolsSnapshot, String> {
    let machine = state.machine()?;
    Ok(ToolsSnapshot {
        video: machine.video_tools().snapshot(),
        built_in: machine.built_in_tool_versions(),
    })
}

#[tauri::command]
pub async fn video_tools_execute(
    state: State<'_, Desktop>,
    operation: VideoToolsOperation,
) -> Result<VideoToolsSnapshot, String> {
    let tools = state.machine()?.video_tools().clone();
    tauri::async_runtime::spawn_blocking(move || tools.execute(operation))
        .await
        .map_err(|error| error.to_string())?
}

/// Removal goes through the machine, which holds video downloads before stopping them.
#[tauri::command]
pub async fn video_tools_remove(
    state: State<'_, Desktop>,
    turn_off: bool,
) -> Result<VideoToolsSnapshot, String> {
    let machine = state.machine()?;
    tauri::async_runtime::spawn_blocking(move || machine.remove_video_tools(turn_off))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn video_tools_take_update_notice(
    state: State<'_, Desktop>,
) -> Result<Option<UpdateNotice>, String> {
    state.machine()?.video_tools().take_update_notice()
}
