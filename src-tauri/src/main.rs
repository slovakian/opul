#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod preferences;

use opul_core::{EngineConfig, MediaEngineConfig, OpulMachineConfig, OpulStateMachine};
use preferences::{PreferenceStore, Preferences};
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};

struct Desktop {
    machine: Result<Arc<OpulStateMachine>, String>,
    preferences: Result<PreferenceStore, String>,
}

impl Desktop {
    fn machine(&self) -> Result<&Arc<OpulStateMachine>, String> {
        self.machine.as_ref().map_err(Clone::clone)
    }
}

fn open_machine(
    app: &tauri::App,
    preferences: &Preferences,
) -> Result<Arc<OpulStateMachine>, String> {
    let support = data_directory(app)?;
    let downloads = std::env::var_os("OPUL_DOWNLOAD_DIR")
        .map(PathBuf::from)
        .map_or_else(
            || app.path().download_dir().map_err(|error| error.to_string()),
            Ok,
        )?;
    std::fs::create_dir_all(&downloads).map_err(|error| error.to_string())?;
    OpulStateMachine::open(OpulMachineConfig {
        application_support_folder: support.to_string_lossy().into_owned(),
        downloads_folder: downloads.to_string_lossy().into_owned(),
        torrent: EngineConfig {
            session_folder: support
                .join("torrent-session")
                .to_string_lossy()
                .into_owned(),
            listen_port: preferences.listen_port,
            randomize_port: preferences.randomize_port,
            enable_dht: preferences.enable_dht,
            max_connections: preferences.max_connections,
            max_download_bps: u64::from(preferences.max_download_kbps) * 1024,
            max_upload_bps: u64::from(preferences.max_upload_kbps) * 1024,
        },
        media: MediaEngineConfig {
            session_folder: support.join("media-session").to_string_lossy().into_owned(),
            resources_folder: app
                .path()
                .resource_dir()
                .ok()
                .map(|path| path.to_string_lossy().into_owned()),
            yt_dlp_path: preferences.yt_dlp_path.clone(),
            ffmpeg_path: preferences.ffmpeg_path.clone(),
            max_concurrent_downloads: preferences.concurrent_downloads,
            max_download_bps: u64::from(preferences.max_download_kbps) * 1024,
        },
    })
    .map_err(|error| error.to_string())
}

fn data_directory(app: &tauri::App) -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("OPUL_DATA_DIR") {
        return Ok(PathBuf::from(path));
    }
    // Match the Swift application's directory so core checkpoints and legacy libraries reopen.
    app.path()
        .data_dir()
        .map(|path| path.join("Opul"))
        .map_err(|error| error.to_string())
}

fn main() {
    let shutting_down = Arc::new(AtomicBool::new(false));
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let preferences = data_directory(app).and_then(|directory| {
                std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
                PreferenceStore::open(&directory)
            });
            let machine = match &preferences {
                Ok(store) => open_machine(app, &store.get()),
                Err(error) => Err(error.clone()),
            };
            app.manage(Desktop {
                machine,
                preferences,
            });
            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
                let settings =
                    MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
                let add = MenuItem::with_id(
                    app,
                    "new-download",
                    "New Download…",
                    true,
                    Some("CmdOrCtrl+N"),
                )?;
                let app_menu = Submenu::with_items(
                    app,
                    "Opul",
                    true,
                    &[
                        &PredefinedMenuItem::about(app, Some("About Opul"), None)?,
                        &settings,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::hide(app, None)?,
                        &PredefinedMenuItem::quit(app, None)?,
                    ],
                )?;
                let file = Submenu::with_items(
                    app,
                    "File",
                    true,
                    &[&add, &PredefinedMenuItem::close_window(app, None)?],
                )?;
                let edit = Submenu::with_items(
                    app,
                    "Edit",
                    true,
                    &[
                        &PredefinedMenuItem::undo(app, None)?,
                        &PredefinedMenuItem::redo(app, None)?,
                        &PredefinedMenuItem::separator(app)?,
                        &PredefinedMenuItem::cut(app, None)?,
                        &PredefinedMenuItem::copy(app, None)?,
                        &PredefinedMenuItem::paste(app, None)?,
                        &PredefinedMenuItem::select_all(app, None)?,
                    ],
                )?;
                app.set_menu(Menu::with_items(app, &[&app_menu, &file, &edit])?)?;
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit(event.id().as_ref(), ());
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::workspace_draft,
            commands::workspace_appearance_draft,
            commands::frontend_ready,
            commands::execute,
            commands::preferences::get_preferences,
            commands::preferences::save_preferences,
            commands::preferences::save_aggregate_view,
            commands::tool_status,
            commands::reveal_download
        ])
        .build(tauri::generate_context!())
        .expect("Could not start Opul");

    application.run(move |app, event| {
        if let tauri::RunEvent::ExitRequested {
            api, code: None, ..
        } = event
        {
            api.prevent_exit();
            if !shutting_down.swap(true, Ordering::SeqCst) {
                let app = app.clone();
                let machine = app.state::<Desktop>().machine.clone();
                std::thread::spawn(move || {
                    if let Ok(machine) = machine {
                        let _ = machine.begin_shutdown();
                    }
                    app.exit(0);
                });
            }
        }
    });
}
