#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_updates;
mod channel;
mod commands;
mod desktop;
mod library_feed;
mod preferences;
mod video_tools_journey;
mod video_tools_smoke;

use desktop::Desktop;
use opul_core::{EngineConfig, MediaEngineConfig, OpulMachineConfig, OpulStateMachine};
use preferences::Preferences;
use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tauri::{Emitter, Manager};

fn open_machine(
    app: &tauri::AppHandle,
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
    let machine = OpulStateMachine::open(OpulMachineConfig {
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
            video_tools: opul_core::video_tools::VideoTools::open(
                opul_core::video_tools::VideoToolsConfig {
                    nightly_available: current_channel(app) == channel::ReleaseChannel::Nightly,
                    root: support.join("Tools"),
                    opul_version: app.package_info().version.to_string(),
                },
            )?,
            max_concurrent_downloads: preferences.concurrent_downloads,
            max_download_bps: u64::from(preferences.max_download_kbps) * 1024,
        },
    })
    .map_err(|error| error.to_string())?;
    let mut snapshots = machine.video_tools().subscribe();
    let events = app.clone();
    tauri::async_runtime::spawn(async move {
        while snapshots.changed().await.is_ok() {
            let snapshot = snapshots.borrow_and_update().clone();
            let _ = events.emit("video-tools", &snapshot);
        }
    });
    Ok(machine)
}

fn current_channel(app: &tauri::AppHandle) -> channel::ReleaseChannel {
    let env_override = std::env::var("OPUL_CHANNEL").ok();
    let product_name = app
        .config()
        .product_name
        .as_deref()
        .unwrap_or(&app.package_info().name);
    channel::ReleaseChannel::detect(
        &app.config().identifier,
        product_name,
        env_override.as_deref(),
    )
}

fn data_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let channel = current_channel(app);
    let base = app.path().data_dir().map_err(|error| error.to_string())?;
    Ok(channel::resolve_data_directory(
        channel,
        &base,
        std::env::var_os("OPUL_DATA_DIR").as_deref(),
    ))
}

/// The app, File, Edit and Help menus. Each item's id is emitted to the frontend.
#[cfg(target_os = "macos")]
fn install_menu(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
    let channel = current_channel(app.handle());
    let app_name = channel.app_name();
    let about_title = format!("About {app_name}");
    let settings = MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
    let check_for_updates = MenuItem::with_id(
        app,
        "check-for-updates",
        "Check for Updates…",
        true,
        None::<&str>,
    )?;
    let add = MenuItem::with_id(
        app,
        "new-download",
        "New Download…",
        true,
        Some("CmdOrCtrl+N"),
    )?;
    let app_menu = Submenu::with_items(
        app,
        app_name,
        true,
        &[
            &PredefinedMenuItem::about(app, Some(&about_title), None)?,
            &check_for_updates,
            &PredefinedMenuItem::separator(app)?,
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
    let setup_tools = MenuItem::with_id(
        app,
        "set-up-video-tools",
        "Set Up Video Tools…",
        true,
        None::<&str>,
    )?;
    let help = Submenu::with_items(app, "Help", true, &[&setup_tools])?;
    app.set_menu(Menu::with_items(app, &[&app_menu, &file, &edit, &help])?)?;
    Ok(())
}

/// Opens Opul's own updater and publishes every change as an `app-updates` event.
fn open_app_updates(app: &tauri::AppHandle) -> Arc<app_updates::AppUpdates> {
    let install_blocker = std::env::current_exe().map_or(
        Some(app_updates::InstallBlocker::Development),
        |executable| {
            app_updates::detect_install_blocker(&executable, app_updates::folder_is_writable)
        },
    );
    let events = app.clone();
    let updates = app_updates::AppUpdates::open(
        app_updates::AppUpdatesConfig {
            state_file: data_directory(app)
                .ok()
                .map(|directory| directory.join("app-updates.json")),
            current_version: app.package_info().version.to_string(),
            channel: current_channel(app),
            install_blocker,
        },
        Arc::new(app_updates::TauriSource::new(app.clone())),
        Arc::new(app_updates::SystemClock),
        Box::new(move |snapshot| {
            let _ = events.emit("app-updates", snapshot);
        }),
    );
    updates.start_schedule();
    updates
}

fn main() {
    let shutting_down = Arc::new(AtomicBool::new(false));
    let application = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        // The public key comes only from the bundled configuration, never from the environment.
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            app.manage(library_feed::LibraryFeeds::default());
            app.manage(Desktop::open(data_directory(app.handle()), |preferences| {
                open_machine(app.handle(), preferences)
            }));
            video_tools_smoke::start(app.handle());
            app.manage(open_app_updates(app.handle()));
            #[cfg(target_os = "macos")]
            install_menu(app)?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            let _ = app.emit(event.id().as_ref(), ());
        })
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::subscribe_library,
            commands::workspace_draft,
            commands::workspace_appearance_draft,
            commands::frontend_ready,
            commands::execute,
            commands::preferences::get_preferences,
            commands::preferences::mark_video_tools_setup_shown,
            commands::preferences::save_preferences,
            commands::preferences::save_aggregate_view,
            commands::preferences::save_workspace_expansion,
            commands::tools::tools_snapshot,
            commands::tools::video_tools_details,
            commands::tools::video_tools_execute,
            commands::tools::video_tools_remove,
            commands::tools::video_tools_take_update_notice,
            commands::reveal_download,
            commands::updates::app_updates_snapshot,
            commands::updates::app_updates_check,
            commands::updates::app_updates_download,
            commands::updates::app_updates_configure,
            commands::updates::app_updates_take_notice,
            commands::updates::app_updates_restart,
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
                let machine = app.state::<Desktop>().machine();
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
