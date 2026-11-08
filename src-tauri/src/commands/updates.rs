// Tauri injects command arguments by value.
#![allow(clippy::needless_pass_by_value)]

use crate::Desktop;
use crate::app_updates::{AppUpdates, AppUpdatesSnapshot, Origin};
use std::sync::Arc;
use tauri::{AppHandle, State};

type Updates<'a> = State<'a, Arc<AppUpdates>>;

#[tauri::command]
pub fn app_updates_snapshot(updates: Updates<'_>) -> AppUpdatesSnapshot {
    updates.snapshot()
}

/// Starts a check and returns at once; the result arrives as an `app-updates` event.
#[tauri::command]
pub fn app_updates_check(updates: Updates<'_>) -> AppUpdatesSnapshot {
    updates.check(Origin::Manual)
}

#[tauri::command]
pub fn app_updates_download(updates: Updates<'_>) -> Result<AppUpdatesSnapshot, String> {
    updates.download(false)
}

#[tauri::command]
pub fn app_updates_configure(
    updates: Updates<'_>,
    check_automatically: bool,
    install_automatically: bool,
) -> Result<AppUpdatesSnapshot, String> {
    updates.configure(check_automatically, install_automatically)
}

#[tauri::command]
pub fn app_updates_take_notice(updates: Updates<'_>) -> Result<Option<String>, String> {
    updates.take_notice()
}

/// Opul never restarts on its own. This runs only when someone chooses Restart, and lets
/// the download engine checkpoint before the new version launches.
#[tauri::command]
pub fn app_updates_restart(
    app: AppHandle,
    updates: Updates<'_>,
    desktop: State<'_, Desktop>,
) -> Result<(), String> {
    if updates.ready_version().is_none() {
        return Err("There's no update waiting to finish.".into());
    }
    let machine = desktop.machine();
    std::thread::spawn(move || {
        if let Ok(machine) = machine {
            let _ = machine.begin_shutdown();
        }
        app.request_restart();
    });
    Ok(())
}
