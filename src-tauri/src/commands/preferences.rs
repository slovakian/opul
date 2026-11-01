// Tauri injects command arguments by value.
#![allow(clippy::needless_pass_by_value)]

use crate::{
    Desktop,
    preferences::{AggregateView, DisplayPreferences, Preferences},
};
use std::collections::BTreeMap;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn save_workspace_expansion(
    state: State<'_, Desktop>,
    choices: BTreeMap<String, Option<bool>>,
) -> Result<BTreeMap<String, bool>, String> {
    state.preferences()?.save_workspace_expansion(choices)
}

#[tauri::command]
pub fn get_preferences(app: AppHandle, state: State<'_, Desktop>) -> Result<Preferences, String> {
    state.load_preferences(crate::data_directory(&app), |preferences| {
        crate::open_machine(&app, preferences)
    })
}

#[tauri::command]
pub fn save_preferences(
    state: State<'_, Desktop>,
    preferences: Preferences,
) -> Result<Preferences, String> {
    let preferences = state.preferences()?.save(preferences)?;
    if let Ok(machine) = state.machine() {
        machine.set_speed_limits(
            u64::from(preferences.max_download_kbps) * 1024,
            u64::from(preferences.max_upload_kbps) * 1024,
        );
        machine.set_max_concurrent_media_downloads(preferences.concurrent_downloads);
    }
    Ok(preferences)
}

#[tauri::command]
pub fn save_aggregate_view(
    state: State<'_, Desktop>,
    view: AggregateView,
    preferences: DisplayPreferences,
) -> Result<Preferences, String> {
    state.preferences()?.save_aggregate_view(view, preferences)
}
