// Tauri injects command arguments by value.
#![allow(clippy::needless_pass_by_value)]

use crate::{
    Desktop,
    preferences::{AggregateView, DisplayPreferences, Preferences},
};
use tauri::State;

#[tauri::command]
pub fn get_preferences(state: State<'_, Desktop>) -> Result<Preferences, String> {
    Ok(state.preferences.as_ref().map_err(Clone::clone)?.get())
}

#[tauri::command]
pub fn save_preferences(
    state: State<'_, Desktop>,
    preferences: Preferences,
) -> Result<Preferences, String> {
    let preferences = state
        .preferences
        .as_ref()
        .map_err(Clone::clone)?
        .save(preferences)?;
    if let Ok(machine) = &state.machine {
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
    state
        .preferences
        .as_ref()
        .map_err(Clone::clone)?
        .save_aggregate_view(view, preferences)
}
