// Tauri injects command arguments by value.
#![allow(clippy::needless_pass_by_value)]

pub mod preferences;
pub mod tools;

use crate::Desktop;
use opul_core::{
    MachineCommandReceipt, MachineDownloadCommand, MachineFolderRequest, MachineFormat,
    MachineGrouping, MachineLayout, MachineMediaItem, MachineMediaPolicy, MachineMediaRequest,
    MachineSessionState, MachineSnapshot, MachineSort, MachineTorrentFile, MachineTorrentRequest,
    MachineWorkspaceIcon, MachineWorkspaceIconKind, MachineWorkspaceRequest, OpulStateMachine,
};
use serde::Deserialize;
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
    StartSeeding,
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
                    ImportMode::Page => machine.add_video_page(media_request(
                        workspace_id,
                        folder_id,
                        format,
                        vec![MachineMediaItem {
                            title: url.clone(),
                            page_url: url.clone(),
                            url,
                            thumbnail_url: None,
                            extractor: None,
                            playlist_index: None,
                        }],
                    )),
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
                    DownloadAction::StartSeeding => machine.start_seeding(request),
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

pub(super) fn media_request(
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
        let tools = state.machine()?.built_in_tool_versions();
        if tools.ffmpeg.is_none() || tools.ffprobe.is_none() {
            return Err("Smoke built-in media tools are unavailable".into());
        }
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
    let machine = state.machine()?;
    tauri::async_runtime::spawn_blocking(move || command.execute(&machine))
        .await
        .map_err(|error| error.to_string())?
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
mod tests;
