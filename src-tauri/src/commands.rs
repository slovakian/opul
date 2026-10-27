// Tauri injects command arguments by value.
#![allow(clippy::needless_pass_by_value)]

use crate::{Desktop, preferences::Preferences};
use opul_core::{
    MachineCommandReceipt, MachineDownloadCommand, MachineFolderRequest, MachineFormat,
    MachineGrouping, MachineLayout, MachineMediaItem, MachineMediaPolicy, MachineMediaRequest,
    MachineSessionState, MachineSnapshot, MachineSort, MachineTorrentFile, MachineTorrentRequest,
    MachineWorkspaceIcon, MachineWorkspaceIconKind, MachineWorkspaceRequest, OpulStateMachine,
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DesktopCommand {
    SetWorkspaceAppearance {
        request: opul_core::MachineWorkspaceAppearanceRequest,
    },
    CreateWorkspace {
        request: MachineWorkspaceRequest,
    },
    UpdateWorkspace {
        request: MachineWorkspaceRequest,
    },
    DeleteWorkspace {
        workspace_id: String,
    },
    AddFolder {
        request: MachineFolderRequest,
    },
    DefaultFolder {
        request: MachineFolderRequest,
    },
    RemoveFolder {
        request: MachineFolderRequest,
    },
    View {
        workspace_id: String,
        layout: MachineLayout,
        sort: MachineSort,
        grouping: MachineGrouping,
    },
    Format {
        workspace_id: String,
        format: MachineFormat,
    },
    AddUrl {
        workspace_id: String,
        folder_id: String,
        url: String,
        mode: ImportMode,
        format: MachineFormat,
    },
    AddTorrentFile {
        workspace_id: String,
        folder_id: String,
        path: String,
    },
    Download {
        download_id: String,
        action: DownloadAction,
    },
    RemoveDownloads {
        download_ids: Vec<String>,
    },
    TorrentFiles {
        download_id: String,
        files: Vec<MachineTorrentFile>,
    },
    SeedRatio {
        download_id: String,
        limit: Option<f64>,
    },
    AcknowledgeRecovery,
}

#[derive(Deserialize)]
pub enum ImportMode {
    Direct,
    Torrent,
    Page,
}

#[derive(Deserialize)]
pub enum DownloadAction {
    Pause,
    Resume,
    Retry,
    Cancel,
    StopSeeding,
}

fn request_id() -> String {
    Uuid::new_v4().to_string()
}

impl DesktopCommand {
    fn execute(self, machine: &OpulStateMachine) -> Result<MachineCommandReceipt, String> {
        let result = match self {
            Self::SetWorkspaceAppearance { request } => machine.set_workspace_appearance(request),
            Self::CreateWorkspace { mut request } => {
                normalize_emoji(&mut request);
                machine.create_workspace(request)
            }
            Self::UpdateWorkspace { mut request } => {
                normalize_emoji(&mut request);
                machine.update_workspace(request)
            }
            Self::DeleteWorkspace { workspace_id } => {
                machine.delete_workspace(request_id(), None, workspace_id, None, false)
            }
            Self::AddFolder { request } => machine.add_folder(request, false),
            Self::DefaultFolder { request } => machine.set_default_folder(request),
            Self::RemoveFolder { request } => machine.remove_folder(request, false),
            Self::View {
                workspace_id,
                layout,
                sort,
                grouping,
            } => {
                machine.set_workspace_view(request_id(), None, workspace_id, layout, sort, grouping)
            }
            Self::Format {
                workspace_id,
                format,
            } => machine.set_default_format(request_id(), None, workspace_id, format),
            Self::AddUrl {
                workspace_id,
                folder_id,
                url,
                mode,
                format,
            } => {
                validate_url(&url, matches!(mode, ImportMode::Torrent))?;
                match mode {
                    ImportMode::Torrent => machine.add_torrent(MachineTorrentRequest {
                        request_id: request_id(),
                        expected_revision: None,
                        workspace_id,
                        folder_id,
                        source: url,
                        metainfo: None,
                        seed_ratio_limit: None,
                    }),
                    ImportMode::Direct => machine.add_direct_media(media_request(
                        workspace_id,
                        folder_id,
                        MachineFormat::Original,
                        vec![MachineMediaItem {
                            title: filename_from_url(&url),
                            page_url: url.clone(),
                            url,
                            thumbnail_url: None,
                            extractor: None,
                            playlist_index: None,
                        }],
                    )),
                    ImportMode::Page => {
                        let probe = machine
                            .probe_media(url)
                            .map_err(|error| error.to_string())?;
                        let items = probe
                            .items
                            .into_iter()
                            .map(|item| MachineMediaItem {
                                url: item.webpage_url,
                                title: item.title,
                                page_url: probe.webpage_url.clone(),
                                thumbnail_url: item.thumbnail_url,
                                extractor: item.extractor,
                                playlist_index: item.playlist_index,
                            })
                            .collect();
                        machine.add_media(media_request(workspace_id, folder_id, format, items))
                    }
                }
            }
            Self::AddTorrentFile {
                workspace_id,
                folder_id,
                path,
            } => {
                let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
                if !metadata.is_file()
                    || metadata.len() > 16 * 1024 * 1024
                    || !path.to_ascii_lowercase().ends_with(".torrent")
                {
                    return Err("Choose a .torrent file smaller than 16 MB.".into());
                }
                let bytes = std::fs::read(&path).map_err(|error| error.to_string())?;
                machine.add_torrent(MachineTorrentRequest {
                    request_id: request_id(),
                    expected_revision: None,
                    workspace_id,
                    folder_id,
                    source: path,
                    metainfo: Some(bytes),
                    seed_ratio_limit: None,
                })
            }
            Self::Download {
                download_id,
                action,
            } => {
                let request = MachineDownloadCommand {
                    request_id: request_id(),
                    expected_revision: None,
                    download_id,
                };
                match action {
                    DownloadAction::Pause => machine.pause_download(request),
                    DownloadAction::Resume => machine.resume_download(request),
                    DownloadAction::Retry => machine.retry_download(request),
                    DownloadAction::Cancel => machine.cancel_download(request),
                    DownloadAction::StopSeeding => machine.stop_seeding(request),
                }
            }
            Self::RemoveDownloads { download_ids } => {
                machine.remove_downloads(request_id(), None, download_ids, false)
            }
            Self::TorrentFiles { download_id, files } => machine.set_torrent_files(
                MachineDownloadCommand {
                    request_id: request_id(),
                    expected_revision: None,
                    download_id,
                },
                files,
            ),
            Self::SeedRatio { download_id, limit } => {
                if limit.is_some_and(|value| !value.is_finite() || value < 0.0) {
                    return Err("Seed ratio must be zero or greater.".into());
                }
                machine.set_seed_ratio(
                    MachineDownloadCommand {
                        request_id: request_id(),
                        expected_revision: None,
                        download_id,
                    },
                    limit,
                )
            }
            Self::AcknowledgeRecovery => machine.acknowledge_recovery(),
        };
        result.map_err(|error| error.to_string())
    }
}

fn normalize_emoji(request: &mut MachineWorkspaceRequest) {
    if request.icon.kind != MachineWorkspaceIconKind::Emoji || request.icon.value.trim().is_empty()
    {
        request.icon = MachineWorkspaceIcon {
            kind: MachineWorkspaceIconKind::Emoji,
            value: "📁".into(),
        };
    }
}

fn validate_url(value: &str, torrent: bool) -> Result<(), String> {
    let url = url::Url::parse(value)
        .map_err(|_| "Enter a complete URL, including https://.".to_string())?;
    if matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
        || torrent && url.scheme() == "magnet"
    {
        Ok(())
    } else {
        Err("Use an HTTP, HTTPS, or magnet link.".into())
    }
}

fn filename_from_url(value: &str) -> String {
    url::Url::parse(value)
        .ok()
        .and_then(|url| {
            url.path_segments()?
                .next_back()
                .filter(|name| !name.is_empty())
                .map(str::to_string)
        })
        .unwrap_or_else(|| "Download".into())
}

fn media_request(
    workspace_id: String,
    folder_id: String,
    format: MachineFormat,
    items: Vec<MachineMediaItem>,
) -> MachineMediaRequest {
    MachineMediaRequest {
        request_id: request_id(),
        expected_revision: None,
        workspace_id,
        folder_id,
        format,
        items,
        policy: MachineMediaPolicy {
            download_subtitles: false,
            subtitle_languages: vec![],
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: "%(title)s.%(ext)s".into(),
            user_agent: None,
        },
    }
}

#[tauri::command]
pub fn snapshot(state: State<'_, Desktop>) -> Result<MachineSnapshot, String> {
    Ok(state.machine()?.snapshot())
}

#[tauri::command]
pub fn workspace_draft(
    state: State<'_, Desktop>,
) -> Result<opul_core::MachineWorkspaceDraft, String> {
    state
        .machine()?
        .workspace_draft()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn workspace_appearance_draft(
    state: State<'_, Desktop>,
    workspace_id: String,
) -> Result<opul_core::WorkspaceAppearance, String> {
    state
        .machine()?
        .workspace_appearance_draft(workspace_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn frontend_ready(
    window: tauri::WebviewWindow,
    state: State<'_, Desktop>,
) -> Result<(), String> {
    window.show().map_err(|error| error.to_string())?;
    if let Some(path) = std::env::var_os("OPUL_SMOKE_READY_FILE") {
        let snapshot = state.machine()?.snapshot();
        if snapshot.session != MachineSessionState::Running {
            return Err(format!("Smoke engine is {:?}", snapshot.session));
        }
        let bytes = serde_json::to_vec(&snapshot).map_err(|error| error.to_string())?;
        std::fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub async fn execute(
    state: State<'_, Desktop>,
    command: DesktopCommand,
) -> Result<MachineCommandReceipt, String> {
    let machine = Arc::clone(state.machine()?);
    tauri::async_runtime::spawn_blocking(move || command.execute(&machine))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub fn get_preferences(state: State<'_, Desktop>) -> Result<Preferences, String> {
    Ok(state.preferences.as_ref().map_err(Clone::clone)?.get())
}

#[tauri::command]
pub fn save_preferences(state: State<'_, Desktop>, preferences: Preferences) -> Result<(), String> {
    state
        .preferences
        .as_ref()
        .map_err(Clone::clone)?
        .save(preferences.clone())?;
    if let Ok(machine) = &state.machine {
        machine.set_speed_limits(
            u64::from(preferences.max_download_kbps) * 1024,
            u64::from(preferences.max_upload_kbps) * 1024,
        );
        machine.set_max_concurrent_media_downloads(preferences.concurrent_downloads);
    }
    Ok(())
}

#[tauri::command]
pub fn tool_status(state: State<'_, Desktop>) -> Result<serde_json::Value, String> {
    let status = state.machine()?.media_tool_status();
    Ok(
        serde_json::json!({ "ready": status.ready, "message": status.message,
        "yt_dlp_path": status.yt_dlp_path, "yt_dlp_version": status.yt_dlp_version,
        "ffmpeg_path": status.ffmpeg_path, "ffmpeg_version": status.ffmpeg_version }),
    )
}

#[tauri::command]
pub fn reveal_download(
    app: AppHandle,
    state: State<'_, Desktop>,
    download_id: String,
) -> Result<(), String> {
    let snapshot = state.machine()?.snapshot();
    let download = snapshot
        .workspaces
        .iter()
        .flat_map(|workspace| &workspace.downloads)
        .find(|download| download.id == download_id)
        .ok_or("Download no longer exists.")?;
    let path = download
        .output_path
        .as_ref()
        .ok_or("This download has no output yet.")?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opul_core::{
        EngineConfig, MachineDownloadState, MachineSource, MediaEngineConfig, OpulMachineConfig,
    };
    use std::io::{Read, Write};
    use std::time::{Duration, Instant};

    fn test_machine(root: &std::path::Path) -> Arc<OpulStateMachine> {
        OpulStateMachine::open(OpulMachineConfig {
            application_support_folder: root.join("state").to_string_lossy().into_owned(),
            downloads_folder: root.join("downloads").to_string_lossy().into_owned(),
            torrent: EngineConfig {
                session_folder: root
                    .join("state/torrent-session")
                    .to_string_lossy()
                    .into_owned(),
                listen_port: 0,
                randomize_port: true,
                enable_dht: false,
                max_connections: 32,
                max_download_bps: 0,
                max_upload_bps: 0,
            },
            media: MediaEngineConfig {
                session_folder: root
                    .join("state/media-session")
                    .to_string_lossy()
                    .into_owned(),
                resources_folder: None,
                yt_dlp_path: Some("/not-installed".into()),
                ffmpeg_path: Some("/not-installed".into()),
                max_concurrent_downloads: 1,
                max_download_bps: 0,
            },
        })
        .unwrap()
    }

    #[test]
    fn desktop_command_downloads_a_real_file_without_tools_and_reopens_its_library() {
        let root = std::env::temp_dir().join(format!("opul-bridge-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("downloads")).unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        listener.set_nonblocking(true).unwrap();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        stream.set_nonblocking(false).unwrap();
                        stream
                            .set_read_timeout(Some(Duration::from_secs(3)))
                            .unwrap();
                        let mut request = [0_u8; 4096];
                        assert!(stream.read(&mut request).unwrap() > 0);
                        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 15\r\nConnection: close\r\n\r\nopul-test-bytes").unwrap();
                        break;
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::WouldBlock
                            && Instant::now() < deadline =>
                    {
                        std::thread::sleep(Duration::from_millis(10))
                    }
                    Err(error) => panic!("HTTP fixture failed: {error}"),
                }
            }
        });
        let machine = test_machine(&root);
        let initial = machine.snapshot();
        assert_eq!(initial.workspaces[0].icon.value, "📁");
        let workspace_id = initial.workspaces[0].id.clone();
        let folder_id = initial.workspaces[0].folders[0].id.clone();
        let receipt = DesktopCommand::AddUrl {
            workspace_id: workspace_id.clone(),
            folder_id,
            url: format!("http://{address}/fixture.bin"),
            mode: ImportMode::Direct,
            format: MachineFormat::Original,
        }
        .execute(&machine)
        .unwrap();
        let id = receipt.created_download_ids[0].clone();
        let deadline = Instant::now() + Duration::from_secs(10);
        let finished = loop {
            let snapshot = machine.snapshot();
            let download = snapshot.workspaces[0]
                .downloads
                .iter()
                .find(|download| download.id == id)
                .unwrap();
            if download.state == MachineDownloadState::Finished {
                break download.clone();
            }
            assert_ne!(
                download.state,
                MachineDownloadState::Failed,
                "{:?}",
                download.error_message
            );
            assert!(
                Instant::now() < deadline,
                "Download timed out in {:?}",
                download.state
            );
            std::thread::sleep(Duration::from_millis(25));
        };
        let output = finished.output_path.unwrap();
        assert_eq!(std::fs::read(&output).unwrap(), b"opul-test-bytes");
        assert_eq!(finished.source, MachineSource::Direct);
        let transfer_checkpoint =
            std::fs::read(root.join("state/media-session/media-downloads.json"))
                .expect("The transfer must have a durable adapter checkpoint");
        let checkpoint: serde_json::Value = serde_json::from_slice(&transfer_checkpoint).unwrap();
        assert!(checkpoint.to_string().contains(&id));
        let wire = serde_json::to_value(machine.snapshot()).unwrap();
        assert_eq!(wire["workspaces"][0]["downloads"][0]["state"], "Finished");
        machine.begin_shutdown().unwrap();
        drop(machine);
        let machine = test_machine(&root);
        assert_eq!(machine.snapshot().workspaces[0].downloads[0].id, id);
        DesktopCommand::RemoveDownloads {
            download_ids: vec![id],
        }
        .execute(&machine)
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !machine.snapshot().workspaces[0].downloads.is_empty() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(25));
        }
        assert!(
            std::path::Path::new(&output).is_file(),
            "Removing history must preserve downloaded files"
        );
        machine.begin_shutdown().unwrap();
        drop(machine);
        server.join().unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn import_urls_reject_non_download_schemes() {
        assert!(validate_url("https://example.com/file.zip", false).is_ok());
        assert!(validate_url("magnet:?xt=urn:btih:abcd", true).is_ok());
        for url in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "example.com",
            "magnet:?xt=a",
        ] {
            assert!(validate_url(url, false).is_err());
        }
    }
}
