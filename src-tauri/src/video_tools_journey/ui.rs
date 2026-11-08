use super::*;
use std::sync::mpsc;
use tauri::{Emitter, Listener, Manager};

fn perform(
    app: &tauri::AppHandle,
    mut request: Value,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    let request_id = uuid::Uuid::new_v4().to_string();
    request["request_id"] = json!(request_id);
    let (sender, receiver) = mpsc::channel();
    let listener = app.listen("opul-acceptance-ui", move |event| {
        if let Ok(report) = serde_json::from_str::<Value>(event.payload())
            && report["request_id"] == request_id
        {
            let _ = sender.send(report);
        }
    });
    let script = include_str!("ui.js").replace("__REQUEST__", &request.to_string());
    let result = (|| {
        let window = app.get_webview_window("main").ok_or("No main window")?;
        window.set_focus().map_err(|e| e.to_string())?;
        window.eval(&script).map_err(|e| e.to_string())?;
        let report = receiver
            .recv_timeout(Duration::from_secs(40))
            .map_err(|e| format!("UI request {request}: {e}"))?;
        if report["ok"] != true {
            return Err(report.to_string());
        }
        evidence.push(report);
        Ok(())
    })();
    app.unlisten(listener);
    result
}

pub(super) fn click(
    app: &tauri::AppHandle,
    label: &str,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    perform(
        app,
        json!({"action": "click", "target": label, "prefix": false}),
        evidence,
    )
}

pub(super) fn download(app: &tauri::AppHandle, evidence: &mut Vec<Value>) -> Result<(), String> {
    perform(
        app,
        json!({"action": "click", "target": "Download video tools (", "prefix": true}),
        evidence,
    )
}

pub(super) fn text(
    app: &tauri::AppHandle,
    value: &str,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    perform(app, json!({"action": "text", "target": value}), evidence)
}

pub(super) fn dismissed(
    app: &tauri::AppHandle,
    title: &str,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    perform(
        app,
        json!({"action": "dismissed", "target": title}),
        evidence,
    )
}

pub(super) fn open_setup(app: &tauri::AppHandle) -> Result<(), String> {
    app.emit("set-up-video-tools", ())
        .map_err(|e| e.to_string())
}

pub(super) fn open_tools(app: &tauri::AppHandle, evidence: &mut Vec<Value>) -> Result<(), String> {
    app.emit("settings", ()).map_err(|e| e.to_string())?;
    click(app, "Tools", evidence)
}

pub(super) fn keep_updated(
    app: &tauri::AppHandle,
    enabled: bool,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    perform(
        app,
        json!({"action": "switch", "target": "keep-video-tools-up-to-date", "enabled": enabled}),
        evidence,
    )
}
