use crate::Desktop;
use tauri::State;

#[tauri::command]
pub fn video_tools_details(state: State<'_, Desktop>) -> Result<String, String> {
    Ok(state
        .machine()?
        .video_tools()
        .ok_or("Video tools module is unavailable.")?
        .diagnostics())
}

#[tauri::command]
pub fn tools_snapshot(state: State<'_, Desktop>) -> Result<serde_json::Value, String> {
    let machine = state.machine()?;
    let tools = machine
        .video_tools()
        .ok_or("Video tools module is unavailable.")?;
    Ok(
        serde_json::json!({ "video": tools.snapshot(), "built_in": machine.built_in_tool_versions() }),
    )
}

#[tauri::command]
pub async fn video_tools_execute(
    state: State<'_, Desktop>,
    operation: opul_core::video_tools::VideoToolsOperation,
) -> Result<opul_core::video_tools::VideoToolsSnapshot, String> {
    let tools = state
        .machine()?
        .video_tools()
        .ok_or("Video tools module is unavailable.")?;
    tauri::async_runtime::spawn_blocking(move || tools.execute(operation))
        .await
        .map_err(|error| error.to_string())?
}
