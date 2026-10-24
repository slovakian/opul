use opul_core::{
    EngineConfig, MachineCommandReceipt, MachineDownloadCommand, MachineFileOrganization,
    MachineFolderRequest, MachineFormat, MachineGrouping, MachineLayout, MachineMediaItem,
    MachineMediaPolicy, MachineMediaRequest, MachineSnapshot, MachineSort, MachineTorrentFile,
    MachineTorrentRequest, MachineWorkspaceIcon, MachineWorkspaceIconKind, MachineWorkspaceRequest,
    MediaEngineConfig, MediaProbe, MediaToolStatus, OpulMachineConfig, OpulStateMachine,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tauri::webview::PageLoadEvent;
use tauri::State;
use tauri_plugin_log::{Target, TargetKind};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct SystemPaths {
    pub downloads_folder: String,
    pub support_folder: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EnginePreferences {
    pub max_download_bps: u64,
    pub max_upload_bps: u64,
    pub max_concurrent_media_downloads: u32,
}

pub struct AppState {
    pub machine: Arc<OpulStateMachine>,
    pub downloads_folder: String,
    pub support_folder: String,
    pub preferences: Arc<std::sync::RwLock<EnginePreferences>>,
}

#[tauri::command]
fn get_system_paths(state: State<AppState>) -> Result<SystemPaths, String> {
    Ok(SystemPaths {
        downloads_folder: state.downloads_folder.clone(),
        support_folder: state.support_folder.clone(),
    })
}

#[tauri::command]
fn get_snapshot(state: State<AppState>) -> Result<MachineSnapshot, String> {
    Ok(state.machine.snapshot())
}

#[tauri::command]
async fn wait_for_snapshot(
    state: State<'_, AppState>,
    after_boot_id: Option<String>,
    after_sequence: u64,
    timeout_ms: Option<u32>,
) -> Result<MachineSnapshot, String> {
    let timeout = timeout_ms.unwrap_or(15_000).min(30_000);
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || {
        machine.wait_for_snapshot(after_boot_id, after_sequence, timeout)
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn acknowledge_recovery(state: State<AppState>) -> Result<MachineCommandReceipt, String> {
    state
        .machine
        .acknowledge_recovery()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_workspace(
    state: State<AppState>,
    name: String,
    icon_kind: Option<String>,
    icon_value: Option<String>,
    folder_name: Option<String>,
    folder_path: Option<String>,
    default_format: Option<String>,
    file_organization: Option<String>,
) -> Result<MachineCommandReceipt, String> {
    let workspace_id = Uuid::new_v4().to_string();
    let folder_id = Uuid::new_v4().to_string();
    let path = folder_path.unwrap_or_else(|| state.downloads_folder.clone());
    let folder_name = folder_name.unwrap_or_else(|| {
        Path::new(&path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Downloads")
            .to_string()
    });

    let icon = MachineWorkspaceIcon {
        kind: match icon_kind.as_deref() {
            Some("emoji") => MachineWorkspaceIconKind::Emoji,
            _ => MachineWorkspaceIconKind::Symbol,
        },
        value: icon_value.unwrap_or_else(|| "sparkles".to_string()),
    };

    let format = parse_format(default_format.as_deref());
    let org = parse_org(file_organization.as_deref());

    let req = MachineWorkspaceRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        name,
        icon,
        folder_id,
        folder_name,
        folder_path: path,
        default_format: format,
        file_organization: org,
    };

    state.machine.create_workspace(req).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_workspace(
    state: State<AppState>,
    workspace_id: String,
    name: String,
    icon_kind: Option<String>,
    icon_value: Option<String>,
    file_organization: Option<String>,
) -> Result<MachineCommandReceipt, String> {
    let icon = MachineWorkspaceIcon {
        kind: match icon_kind.as_deref() {
            Some("emoji") => MachineWorkspaceIconKind::Emoji,
            _ => MachineWorkspaceIconKind::Symbol,
        },
        value: icon_value.unwrap_or_else(|| "folder".to_string()),
    };
    let org = parse_org(file_organization.as_deref());

    let req = MachineWorkspaceRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        name,
        icon,
        folder_id: String::new(),
        folder_name: String::new(),
        folder_path: String::new(),
        default_format: MachineFormat::Best,
        file_organization: org,
    };

    state.machine.update_workspace(req).map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_workspace(
    state: State<'_, AppState>,
    workspace_id: String,
    move_files_to_trash: bool,
) -> Result<MachineCommandReceipt, String> {
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || {
        machine.delete_workspace(
            Uuid::new_v4().to_string(),
            None,
            workspace_id,
            None,
            move_files_to_trash,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_folder(
    state: State<AppState>,
    workspace_id: String,
    name: String,
    path: String,
    make_default: bool,
) -> Result<MachineCommandReceipt, String> {
    let folder_id = Uuid::new_v4().to_string();
    let req = MachineFolderRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        folder_id,
        name,
        path,
    };

    state
        .machine
        .add_folder(req, make_default)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_default_folder(
    state: State<AppState>,
    workspace_id: String,
    folder_id: String,
) -> Result<MachineCommandReceipt, String> {
    let req = MachineFolderRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        folder_id,
        name: String::new(),
        path: String::new(),
    };

    state
        .machine
        .set_default_folder(req)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn remove_folder(
    state: State<AppState>,
    workspace_id: String,
    folder_id: String,
    move_files_to_trash: bool,
) -> Result<MachineCommandReceipt, String> {
    let req = MachineFolderRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        folder_id,
        name: String::new(),
        path: String::new(),
    };

    state
        .machine
        .remove_folder(req, move_files_to_trash)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_workspace_view(
    state: State<AppState>,
    workspace_id: String,
    layout: Option<String>,
    sort: Option<String>,
    grouping: Option<String>,
) -> Result<MachineCommandReceipt, String> {
    let m_layout = match layout.as_deref() {
        Some("table") => MachineLayout::Table,
        _ => MachineLayout::List,
    };
    let m_sort = match sort.as_deref() {
        Some("name") => MachineSort::Name,
        Some("state") => MachineSort::State,
        Some("source") => MachineSort::Source,
        _ => MachineSort::Order,
    };
    let m_group = match grouping.as_deref() {
        Some("state") => MachineGrouping::State,
        Some("folder") => MachineGrouping::Folder,
        Some("kind") => MachineGrouping::Kind,
        Some("source") => MachineGrouping::Source,
        _ => MachineGrouping::None,
    };

    state
        .machine
        .set_workspace_view(
            Uuid::new_v4().to_string(),
            None,
            workspace_id,
            m_layout,
            m_sort,
            m_group,
        )
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_default_format(
    state: State<AppState>,
    workspace_id: String,
    format: String,
) -> Result<MachineCommandReceipt, String> {
    let m_format = parse_format(Some(&format));
    state
        .machine
        .set_default_format(Uuid::new_v4().to_string(), None, workspace_id, m_format)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_torrent(
    state: State<'_, AppState>,
    workspace_id: String,
    folder_id: String,
    source: String,
    metainfo_bytes: Option<Vec<u8>>,
    seed_ratio_limit: Option<f64>,
) -> Result<MachineCommandReceipt, String> {
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || {
        let req = MachineTorrentRequest {
            request_id: Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id,
            folder_id,
            source,
            metainfo: metainfo_bytes,
            seed_ratio_limit,
        };
        machine.add_torrent(req)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn probe_media(state: State<'_, AppState>, url: String) -> Result<MediaProbe, String> {
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || machine.probe_media(url))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn add_media(
    state: State<AppState>,
    workspace_id: String,
    folder_id: String,
    format: Option<String>,
    items: Vec<MachineMediaItem>,
) -> Result<MachineCommandReceipt, String> {
    let m_format = parse_format(format.as_deref());
    let req = MachineMediaRequest {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id,
        folder_id,
        format: m_format,
        policy: MachineMediaPolicy {
            download_subtitles: false,
            subtitle_languages: vec!["en".to_string()],
            embed_subtitles: false,
            write_thumbnail: true,
            write_metadata: true,
            filename_template: "%(title)s [%(id)s].%(ext)s".to_string(),
            user_agent: None,
        },
        items,
    };

    state.machine.add_media(req).map_err(|e| e.to_string())
}

#[tauri::command]
fn pause_download(
    state: State<AppState>,
    download_id: String,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state.machine.pause_download(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
fn resume_download(
    state: State<AppState>,
    download_id: String,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state.machine.resume_download(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
fn retry_download(
    state: State<AppState>,
    download_id: String,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state.machine.retry_download(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
fn cancel_download(
    state: State<AppState>,
    download_id: String,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state.machine.cancel_download(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
fn stop_seeding(
    state: State<AppState>,
    download_id: String,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state.machine.stop_seeding(cmd).map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_downloads(
    state: State<'_, AppState>,
    download_ids: Vec<String>,
    move_files_to_trash: bool,
) -> Result<MachineCommandReceipt, String> {
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || {
        machine.remove_downloads(
            Uuid::new_v4().to_string(),
            None,
            download_ids,
            move_files_to_trash,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_seed_ratio(
    state: State<AppState>,
    download_id: String,
    ratio: Option<f64>,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state
        .machine
        .set_seed_ratio(cmd, ratio)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_torrent_files(
    state: State<AppState>,
    download_id: String,
    files: Vec<MachineTorrentFile>,
) -> Result<MachineCommandReceipt, String> {
    let cmd = MachineDownloadCommand {
        request_id: Uuid::new_v4().to_string(),
        expected_revision: None,
        download_id,
    };
    state
        .machine
        .set_torrent_files(cmd, files)
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn move_download(
    state: State<'_, AppState>,
    download_id: String,
    folder_id: String,
    output_path: String,
    content_folder: Option<String>,
) -> Result<MachineCommandReceipt, String> {
    let machine = Arc::clone(&state.machine);
    tokio::task::spawn_blocking(move || {
        let cmd = MachineDownloadCommand {
            request_id: Uuid::new_v4().to_string(),
            expected_revision: None,
            download_id,
        };
        machine.move_download(
            cmd,
            folder_id,
            output_path,
            content_folder,
        )
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn();
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        open::that(&path).map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn get_engine_preferences(state: State<AppState>) -> Result<EnginePreferences, String> {
    Ok(state.preferences.read().map(|p| p.clone()).unwrap_or(EnginePreferences {
        max_download_bps: 0,
        max_upload_bps: 0,
        max_concurrent_media_downloads: 3,
    }))
}

#[tauri::command]
fn get_media_tool_status(state: State<AppState>) -> Result<MediaToolStatus, String> {
    Ok(state.machine.media_tool_status())
}

#[tauri::command]
fn set_speed_limits(
    state: State<AppState>,
    download_bps: u64,
    upload_bps: u64,
) -> Result<(), String> {
    state.machine.set_speed_limits(download_bps, upload_bps);
    if let Ok(mut prefs) = state.preferences.write() {
        prefs.max_download_bps = download_bps;
        prefs.max_upload_bps = upload_bps;
    }
    Ok(())
}

#[tauri::command]
fn set_max_concurrent_media_downloads(
    state: State<AppState>,
    maximum: u32,
) -> Result<(), String> {
    state.machine.set_max_concurrent_media_downloads(maximum);
    if let Ok(mut prefs) = state.preferences.write() {
        prefs.max_concurrent_media_downloads = maximum;
    }
    Ok(())
}

fn parse_format(f: Option<&str>) -> MachineFormat {
    match f {
        Some("best_mp4") | Some("BestMp4") => MachineFormat::BestMp4,
        Some("p1080") | Some("P1080") => MachineFormat::P1080,
        Some("p720") | Some("P720") => MachineFormat::P720,
        Some("original") | Some("Original") => MachineFormat::Original,
        Some("audio_mp3") | Some("AudioMp3") => MachineFormat::AudioMp3,
        Some("audio_m4a") | Some("AudioM4a") => MachineFormat::AudioM4a,
        _ => MachineFormat::Best,
    }
}

fn parse_org(org: Option<&str>) -> MachineFileOrganization {
    match org {
        Some("always_create_folder") | Some("AlwaysCreateFolder") => {
            MachineFileOrganization::AlwaysCreateFolder
        }
        Some("keep_in_save_location") | Some("KeepInSaveLocation") => {
            MachineFileOrganization::KeepInSaveLocation
        }
        _ => MachineFileOrganization::Automatic,
    }
}

fn external_navigation_plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::<R>::new("external-navigation")
        .on_navigation(|webview, url| {
            let is_internal_host = matches!(
                url.host_str(),
                Some("localhost") | Some("127.0.0.1") | Some("tauri.localhost") | Some("::1")
            );

            let is_internal = url.scheme() == "tauri" || is_internal_host;

            if is_internal {
                return true;
            }

            let is_external_link = matches!(url.scheme(), "http" | "https" | "mailto" | "tel");

            if is_external_link {
                log::info!("opening external link in system browser: {}", url);
                let _ = webview.opener().open_url(url.as_str(), None::<&str>);
                return false;
            }

            true
        })
        .build()
}

fn handle_platform_effects(machine: Arc<OpulStateMachine>) {
    std::thread::Builder::new()
        .name("opul-platform-effect-handler".to_string())
        .spawn(move || loop {
            let effects = machine.platform_effects();
            for effect in effects {
                for target in &effect.targets {
                    let path = Path::new(&target.path);
                    if path.exists() {
                        let _ = trash::delete(path);
                    }
                }
                let _ = machine.complete_platform_effect(effect.effect_id, true);
            }
            std::thread::sleep(Duration::from_millis(200));
        })
        .ok();
}

fn start_snapshot_emitter<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    machine: Arc<OpulStateMachine>,
) {
    std::thread::Builder::new()
        .name("opul-snapshot-emitter".to_string())
        .spawn(move || {
            let initial = machine.snapshot();
            let mut boot = initial.boot_id;
            let mut sequence = initial.sequence;
            loop {
                let next = machine.wait_for_snapshot(boot.clone(), sequence, 30_000);
                if boot != next.boot_id || next.sequence != sequence {
                    boot = next.boot_id.clone();
                    sequence = next.sequence;
                    use tauri::Emitter;
                    if app.emit("opul://snapshot", &next).is_err() {
                        break;
                    }
                }
            }
        })
        .ok();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let support_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Opul");
    let downloads_dir = dirs::download_dir()
        .or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))
        .unwrap_or_else(|| PathBuf::from("."));

    fs::create_dir_all(&support_dir).ok();
    let torrent_session = support_dir.join("torrent-session");
    let media_session = support_dir.join("media-session");
    fs::create_dir_all(&torrent_session).ok();
    fs::create_dir_all(&media_session).ok();

    let config = OpulMachineConfig {
        application_support_folder: support_dir.to_string_lossy().to_string(),
        downloads_folder: downloads_dir.to_string_lossy().to_string(),
        torrent: EngineConfig {
            session_folder: torrent_session.to_string_lossy().to_string(),
            listen_port: 0,
            randomize_port: true,
            enable_dht: true,
            max_connections: 64,
            max_download_bps: 0,
            max_upload_bps: 0,
        },
        media: MediaEngineConfig {
            session_folder: media_session.to_string_lossy().to_string(),
            resources_folder: None,
            yt_dlp_path: None,
            ffmpeg_path: None,
            max_concurrent_downloads: 3,
            max_download_bps: 0,
        },
    };

    let machine = OpulStateMachine::open(config)
        .expect("failed to open Opul state machine");

    // Ensure at least one default workspace exists
    let snapshot = machine.snapshot();
    if snapshot.workspaces.is_empty() {
        let ws_id = Uuid::new_v4().to_string();
        let folder_id = Uuid::new_v4().to_string();
        let _ = machine.create_workspace(MachineWorkspaceRequest {
            request_id: Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id: ws_id,
            name: "Downloads".to_string(),
            icon: MachineWorkspaceIcon {
                kind: MachineWorkspaceIconKind::Symbol,
                value: "sparkles".to_string(),
            },
            folder_id,
            folder_name: "Downloads".to_string(),
            folder_path: downloads_dir.to_string_lossy().to_string(),
            default_format: MachineFormat::Best,
            file_organization: MachineFileOrganization::Automatic,
        });
    }

    handle_platform_effects(Arc::clone(&machine));

    let emitter_machine = Arc::clone(&machine);

    let app_state = AppState {
        machine,
        downloads_folder: downloads_dir.to_string_lossy().to_string(),
        support_folder: support_dir.to_string_lossy().to_string(),
        preferences: Arc::new(std::sync::RwLock::new(EnginePreferences {
            max_download_bps: 0,
            max_upload_bps: 0,
            max_concurrent_media_downloads: 3,
        })),
    };

    tauri::Builder::default()
        .setup(move |app| {
            start_snapshot_emitter(app.handle().clone(), emitter_machine);
            Ok(())
        })
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    Target::new(TargetKind::Stdout),
                    Target::new(TargetKind::LogDir { file_name: None }),
                    Target::new(TargetKind::Webview),
                ])
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(external_navigation_plugin())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_system_paths,
            get_snapshot,
            wait_for_snapshot,
            acknowledge_recovery,
            create_workspace,
            update_workspace,
            delete_workspace,
            add_folder,
            set_default_folder,
            remove_folder,
            set_workspace_view,
            set_default_format,
            add_torrent,
            probe_media,
            add_media,
            pause_download,
            resume_download,
            retry_download,
            cancel_download,
            stop_seeding,
            remove_downloads,
            set_seed_ratio,
            set_torrent_files,
            move_download,
            reveal_in_finder,
            get_media_tool_status,
            get_engine_preferences,
            set_speed_limits,
            set_max_concurrent_media_downloads
        ])
        .on_page_load(|webview, payload| {
            if webview.label() == "main" && matches!(payload.event(), PageLoadEvent::Finished) {
                log::info!("main webview finished loading");
                let _ = webview.window().show();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running opul application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tauri::Manager;

    fn test_app_state() -> AppState {
        std::thread::spawn(|| {
            let temp_dir = std::env::temp_dir().join(format!("opul-test-{}", Uuid::new_v4()));
            let support_dir = temp_dir.join("support");
            let downloads_dir = temp_dir.join("downloads");
            let torrent_session = support_dir.join("torrent-session");
            let media_session = support_dir.join("media-session");
            fs::create_dir_all(&torrent_session).ok();
            fs::create_dir_all(&media_session).ok();
            fs::create_dir_all(&downloads_dir).ok();

            let config = OpulMachineConfig {
                application_support_folder: support_dir.to_string_lossy().to_string(),
                downloads_folder: downloads_dir.to_string_lossy().to_string(),
                torrent: EngineConfig {
                    session_folder: torrent_session.to_string_lossy().to_string(),
                    listen_port: 0,
                    randomize_port: true,
                    enable_dht: false,
                    max_connections: 10,
                    max_download_bps: 0,
                    max_upload_bps: 0,
                },
                media: MediaEngineConfig {
                    session_folder: media_session.to_string_lossy().to_string(),
                    resources_folder: None,
                    yt_dlp_path: None,
                    ffmpeg_path: None,
                    max_concurrent_downloads: 1,
                    max_download_bps: 0,
                },
            };
            let machine = OpulStateMachine::open(config).unwrap();
            AppState {
                machine,
                downloads_folder: downloads_dir.to_string_lossy().to_string(),
                support_folder: support_dir.to_string_lossy().to_string(),
                preferences: Arc::new(std::sync::RwLock::new(EnginePreferences {
                    max_download_bps: 0,
                    max_upload_bps: 0,
                    max_concurrent_media_downloads: 3,
                })),
            }
        })
        .join()
        .unwrap()
    }

    #[test]
    fn test_snapshot_wait_blocks_main_thread_event_loop() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let app = tauri::test::mock_app();
            app.manage(test_app_state());
            let state = app.state::<AppState>();
            let current_snap = state.machine.snapshot();

            let start = Instant::now();
            let (tx, rx) = tokio::sync::oneshot::channel();

            // Task simulating a user click / UI event on the main thread
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let _ = tx.send(Instant::now());
            });

            // Calling wait_for_snapshot asynchronously off the main thread
            let _ = wait_for_snapshot(
                state,
                current_snap.boot_id,
                current_snap.sequence,
                Some(500),
            )
            .await;

            let ui_finished = rx.await.expect("UI click should have run");
            let ui_latency = ui_finished.duration_since(start);
            assert!(
                ui_latency < Duration::from_millis(100),
                "UI click was blocked by synchronous wait_for_snapshot! Latency was {:?}",
                ui_latency
            );
        });
    }

    #[tokio::test]
    async fn test_snapshot_emitter_pushes_state_changes() {
        use tauri::Listener;

        let state = test_app_state();
        let machine = Arc::clone(&state.machine);
        let app = tauri::test::mock_app();
        let handle = app.handle().clone();

        let (tx, rx) = tokio::sync::oneshot::channel();
        let tx_cell = Arc::new(std::sync::Mutex::new(Some(tx)));

        handle.listen("opul://snapshot", move |event| {
            if let Ok(snap) = serde_json::from_str::<MachineSnapshot>(event.payload()) {
                if snap.sequence > 0 {
                    if let Some(sender) = tx_cell.lock().unwrap().take() {
                        let _ = sender.send(snap);
                    }
                }
            }
        });

        start_snapshot_emitter(handle, Arc::clone(&machine));

        // Create a workspace on a background blocking task to trigger a state change
        let m = Arc::clone(&machine);
        tokio::task::spawn_blocking(move || {
            let _ = m.create_workspace(MachineWorkspaceRequest {
                request_id: Uuid::new_v4().to_string(),
                expected_revision: None,
                workspace_id: Uuid::new_v4().to_string(),
                name: "Test Workspace".to_string(),
                icon: MachineWorkspaceIcon {
                    kind: MachineWorkspaceIconKind::Symbol,
                    value: "sparkles".to_string(),
                },
                folder_id: Uuid::new_v4().to_string(),
                folder_name: "Downloads".to_string(),
                folder_path: "/tmp".to_string(),
                default_format: MachineFormat::Best,
                file_organization: MachineFileOrganization::Automatic,
            });
        });

        let received = tokio::time::timeout(Duration::from_secs(3), rx)
            .await
            .expect("snapshot event should be emitted within 3s")
            .expect("should receive valid snapshot");

        assert!(
            received.sequence > 0,
            "Received sequence should be greater than 0"
        );
        assert!(
            received.workspaces.iter().any(|w| w.name == "Test Workspace"),
            "Snapshot should contain newly created workspace"
        );
    }
}

