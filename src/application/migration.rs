use super::{
    ActivePhase, AppState, Download, DownloadId, DownloadLifecycle, DownloadSource,
    FileOrganization, FolderId, HoldCause, MediaDownload, MediaFormat, MediaOrigin,
    MigrationRecord, OutputArtifact, RecoveryItem, SavedFolder, SessionPhase, TorrentDownload,
    TorrentFilePriority, TorrentFileSelection, TorrentOrigin, Workspace, WorkspaceGrouping,
    WorkspaceIcon, WorkspaceLayout, WorkspaceSort, WorkspaceView,
};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub(crate) fn import_legacy_library(
    path: &Path,
    downloads_path: String,
) -> Result<AppState, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read legacy library {}: {error}", path.display()))?;
    let root: Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "could not decode legacy library {}: {error}",
            path.display()
        )
    })?;
    let workspace_values = root
        .get("workspaces")
        .and_then(Value::as_array)
        .or_else(|| root.as_array())
        .ok_or_else(|| "the legacy library does not contain a workspace array".to_string())?;

    let mut state = AppState::fresh(downloads_path.clone());
    state.workspaces.clear();
    state.session = SessionPhase::Booting;
    let mut seen_downloads = BTreeSet::new();
    let mut recovered = Vec::new();
    let mut imported_downloads = 0_u64;

    for value in workspace_values {
        let id = parse_uuid(value, "id").map(super::WorkspaceId)?;
        let name = string(value, "name").unwrap_or_else(|| "Workspace".to_string());
        let icon = parse_icon(value.get("icon"));
        let mut folders = value
            .get("savedFolders")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|folder| {
                let id = parse_uuid(folder, "id").ok().map(FolderId)?;
                let path = string(folder, "path")?;
                Some((
                    SavedFolder {
                        id,
                        name: string(folder, "name").unwrap_or_else(|| "Folder".to_string()),
                        path,
                    },
                    folder
                        .get("isDefault")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                ))
            })
            .collect::<Vec<_>>();
        if folders.is_empty() {
            let folder = SavedFolder {
                id: FolderId::new(),
                name: "Downloads".to_string(),
                path: downloads_path.clone(),
            };
            recovered.push(RecoveryItem {
                key: format!("workspace-{id}-missing-folders"),
                summary: format!("{name} had no usable saved folder."),
                detail: "A Downloads folder was added. Downloads with missing placement were preserved as recovery items.".to_string(),
            });
            folders.push((folder, true));
        }
        let default_folder = folders
            .iter()
            .find(|(_, is_default)| *is_default)
            .map(|(folder, _)| folder.id)
            .unwrap_or(folders[0].0.id);
        let folder_ids = folders
            .iter()
            .map(|(folder, _)| folder.id)
            .collect::<BTreeSet<_>>();

        let mut downloads = Vec::new();
        for download in value
            .get("downloads")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(parsed) =
                parse_download(download, &folder_ids, default_folder, &mut recovered)
            else {
                continue;
            };
            if !seen_downloads.insert(parsed.id) {
                recovered.push(RecoveryItem {
                    key: format!("duplicate-download-{}", parsed.id),
                    summary: "A duplicate legacy Download ID was quarantined.".to_string(),
                    detail: parsed.name,
                });
                continue;
            }
            imported_downloads += 1;
            downloads.push(parsed);
        }
        state.workspaces.push(Workspace {
            id,
            name,
            icon,
            appearance: super::WorkspaceAppearance::default(),
            folders: folders.into_iter().map(|(folder, _)| folder).collect(),
            default_folder,
            default_format: parse_format(value.get("defaultFormat")).unwrap_or(MediaFormat::Best),
            file_organization: parse_organization(value.get("fileOrganizationOverride"))
                .unwrap_or(FileOrganization::Automatic),
            view: parse_view(value.get("view")),
            downloads,
        });
    }

    if state.workspaces.is_empty() {
        state = AppState::fresh(downloads_path);
        recovered.push(RecoveryItem {
            key: "legacy-library-empty".to_string(),
            summary: "The legacy library had no usable workspace.".to_string(),
            detail: "A fresh ordinary Downloads workspace was created.".to_string(),
        });
    }
    state.recovery = recovered;
    state.migration = Some(MigrationRecord {
        source_fingerprint: hex::encode(Sha1::digest(&bytes)),
        imported_downloads,
        recovery_items: state.recovery.len() as u64,
    });
    state.migrate_workspace_appearances();
    state
        .validate()
        .map_err(|error| format!("legacy migration violated an invariant: {error}"))?;
    Ok(state)
}

fn parse_download(
    value: &Value,
    folder_ids: &BTreeSet<FolderId>,
    default_folder: FolderId,
    recovery: &mut Vec<RecoveryItem>,
) -> Option<Download> {
    let id = parse_uuid(value, "id").ok().map(DownloadId)?;
    let name = string(value, "name").unwrap_or_else(|| "Download".to_string());
    let requested_folder = parse_uuid(value, "folderID").ok().map(FolderId);
    let folder = requested_folder
        .filter(|folder| folder_ids.contains(folder))
        .unwrap_or_else(|| {
            recovery.push(RecoveryItem {
                key: format!("download-{id}-missing-folder"),
                summary: format!("{name} referred to a missing folder."),
                detail: "It was attached to the workspace default during migration.".to_string(),
            });
            default_folder
        });
    let source_name = string(value, "source").unwrap_or_else(|| "fileURL".to_string());
    let address = string(value, "address").unwrap_or_default();
    let source = if source_name == "torrent" {
        let origin = if address.to_ascii_lowercase().starts_with("magnet:") {
            TorrentOrigin::Magnet {
                uri: address.clone(),
            }
        } else if address.to_ascii_lowercase().starts_with("http://")
            || address.to_ascii_lowercase().starts_with("https://")
        {
            TorrentOrigin::RemoteMetainfo {
                url: address.clone(),
            }
        } else {
            TorrentOrigin::Metainfo {
                blob: format!("legacy-path:{address}"),
                original_name: std::path::Path::new(&address)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("download.torrent")
                    .to_string(),
            }
        };
        let files = value
            .get("files")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|file| {
                Some(TorrentFileSelection {
                    index: file.get("fileIndex")?.as_u64()? as u32,
                    path: string(file, "path")?,
                    size: file.get("size").and_then(Value::as_u64).unwrap_or(0),
                    priority: match string(file, "priority").as_deref() {
                        Some("skip") => TorrentFilePriority::Skip,
                        Some("low") => TorrentFilePriority::Low,
                        Some("high") => TorrentFilePriority::High,
                        _ => TorrentFilePriority::Normal,
                    },
                })
            })
            .collect();
        DownloadSource::Torrent(TorrentDownload {
            origin,
            info_hash: string(value, "infoHash").map(|hash| hash.to_ascii_lowercase()),
            metadata: None,
            seed_ratio_limit: value.get("seedRatioLimit").and_then(Value::as_f64),
            files,
        })
    } else {
        let page_url = string(value, "pageURL");
        let origin = if source_name == "guided" {
            MediaOrigin::Guided {
                page_url: page_url.clone().unwrap_or_else(|| address.clone()),
                url: address.clone(),
                discovery_id: format!("legacy-{id}"),
            }
        } else if let Some(page_url) = page_url {
            MediaOrigin::Page {
                page_url,
                item_url: address.clone(),
                playlist_index: value
                    .get("playlistIndex")
                    .and_then(Value::as_u64)
                    .map(|index| index as u32),
            }
        } else {
            MediaOrigin::Direct {
                url: address.clone(),
                referer: None,
            }
        };
        DownloadSource::Media(MediaDownload {
            origin,
            format: parse_format(value.get("format")).unwrap_or(MediaFormat::Original),
            policy: super::MediaPolicy::default(),
            legacy_binding: string(value, "transferID"),
            thumbnail_url: string(value, "thumbnailURL"),
            extractor: string(value, "extractor"),
        })
    };
    let lifecycle = parse_lifecycle(value, &source);
    let output_path = string(value, "outputPath");
    let content_folder = string(value, "contentFolder");
    Some(Download {
        id,
        folder,
        name,
        attempt: 0,
        control_generation: 0,
        source,
        lifecycle,
        output: output_path.map(|path| OutputArtifact {
            path,
            content_folder,
            identity: None,
        }),
    })
}

fn parse_lifecycle(value: &Value, source: &DownloadSource) -> DownloadLifecycle {
    match string(value, "state").as_deref() {
        Some("fetching") => DownloadLifecycle::Active(ActivePhase::Fetching),
        Some("processing") if matches!(source, DownloadSource::Media(_)) => {
            DownloadLifecycle::Active(ActivePhase::Processing)
        }
        Some("paused") => DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        },
        Some("seeding") if matches!(source, DownloadSource::Torrent(_)) => {
            DownloadLifecycle::Active(ActivePhase::Seeding)
        }
        Some("finished") => DownloadLifecycle::Finished,
        Some("failed") => DownloadLifecycle::Failed {
            message: string(value, "errorMessage")
                .unwrap_or_else(|| "The legacy transfer failed.".to_string()),
            retryable: true,
        },
        Some("cancelled") => DownloadLifecycle::Cancelled,
        _ => DownloadLifecycle::Queued,
    }
}

fn parse_icon(value: Option<&Value>) -> WorkspaceIcon {
    let Some(value) = value else {
        return WorkspaceIcon::Symbol("arrow.down.circle".to_string());
    };
    if let Some(symbol) = value.get("symbol") {
        return WorkspaceIcon::Symbol(
            symbol
                .as_str()
                .map(str::to_string)
                .or_else(|| symbol.get("_0").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| "arrow.down.circle".to_string()),
        );
    }
    if let Some(emoji) = value.get("emoji") {
        return WorkspaceIcon::Emoji(
            emoji
                .as_str()
                .map(str::to_string)
                .or_else(|| emoji.get("_0").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| "⬇️".to_string()),
        );
    }
    WorkspaceIcon::Symbol("arrow.down.circle".to_string())
}

fn parse_format(value: Option<&Value>) -> Option<MediaFormat> {
    match value?.as_str()? {
        "best" => Some(MediaFormat::Best),
        "bestMp4" => Some(MediaFormat::BestMp4),
        "p1080" => Some(MediaFormat::P1080),
        "p720" => Some(MediaFormat::P720),
        "original" => Some(MediaFormat::Original),
        "audioMp3" => Some(MediaFormat::AudioMp3),
        "audioM4a" => Some(MediaFormat::AudioM4a),
        _ => None,
    }
}

fn parse_organization(value: Option<&Value>) -> Option<FileOrganization> {
    match value?.as_str()? {
        "automatic" => Some(FileOrganization::Automatic),
        "alwaysCreateFolder" => Some(FileOrganization::AlwaysCreateFolder),
        "keepInSaveLocation" => Some(FileOrganization::KeepInSaveLocation),
        _ => None,
    }
}

fn parse_view(value: Option<&Value>) -> WorkspaceView {
    let value = value.unwrap_or(&Value::Null);
    WorkspaceView {
        layout: match string(value, "layout").as_deref() {
            Some("table") => WorkspaceLayout::Table,
            _ => WorkspaceLayout::List,
        },
        sort: match string(value, "sort").as_deref() {
            Some("name") => WorkspaceSort::Name,
            Some("state") => WorkspaceSort::State,
            Some("source") => WorkspaceSort::Source,
            _ => WorkspaceSort::Order,
        },
        grouping: match string(value, "grouping").as_deref() {
            Some("state") => WorkspaceGrouping::State,
            Some("folder") => WorkspaceGrouping::Folder,
            Some("kind") => WorkspaceGrouping::Kind,
            Some("source") => WorkspaceGrouping::Source,
            _ => WorkspaceGrouping::None,
        },
    }
}

fn parse_uuid(value: &Value, key: &str) -> Result<Uuid, String> {
    let raw = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("missing {key}"))?;
    Uuid::parse_str(raw).map_err(|error| format!("invalid {key}: {error}"))
}

fn string(value: &Value, key: &str) -> Option<String> {
    value.get(key).and_then(Value::as_str).map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn legacy_missing_and_symbol_icons_receive_folder_treatments_without_changing_data() {
        let directory =
            std::env::temp_dir().join(format!("opul-legacy-appearance-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("library.json");
        let icons = [
            serde_json::Value::Null,
            serde_json::json!({"symbol": "film"}),
            serde_json::json!({"emoji": ""}),
            serde_json::json!({"emoji": "🎬"}),
        ];
        let workspaces = icons.iter().enumerate().map(|(index, icon)| serde_json::json!({
            "id": Uuid::from_u128(index as u128 + 1), "name": format!("Workspace {index}"), "icon": icon,
            "savedFolders": [{"id": Uuid::from_u128(index as u128 + 101), "name": "Saved", "path": "/tmp/Legacy", "isDefault": true}],
            "defaultFormat": "bestMp4", "downloads": [],
        })).collect::<Vec<_>>();
        fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({"workspaces": workspaces})).unwrap(),
        )
        .unwrap();
        let state = import_legacy_library(&path, "/tmp/Unused".into()).unwrap();
        for (index, workspace) in state.workspaces.iter().enumerate() {
            assert_eq!(workspace.id.0, Uuid::from_u128(index as u128 + 1));
            assert_eq!(
                workspace.folders[0].id.0,
                Uuid::from_u128(index as u128 + 101)
            );
            assert_eq!(workspace.folders[0].path, "/tmp/Legacy");
            assert_eq!(workspace.default_format, MediaFormat::BestMp4);
            assert_eq!(workspace.appearance.is_unassigned(), index == 3);
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn migration_preserves_ids_and_decoder_omissions() {
        let workspace = Uuid::new_v4();
        let folder = Uuid::new_v4();
        let download = Uuid::new_v4();
        let directory = std::env::temp_dir().join(format!("opul-migration-{}", Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join("library.json");
        let json = serde_json::json!({
            "schemaVersion": 1,
            "workspaces": [{
                "id": workspace,
                "name": "Film",
                "icon": { "symbol": { "_0": "film" } },
                "savedFolders": [{
                    "id": folder,
                    "name": "Film",
                    "path": "/tmp/Film",
                    "isDefault": true
                }],
                "defaultFormat": "bestMp4",
                "fileOrganizationOverride": "alwaysCreateFolder",
                "view": { "layout": "grid", "sort": "name", "grouping": "kind" },
                "downloads": [{
                    "id": download,
                    "name": "Example",
                    "address": "http://example.com/film.mp4",
                    "state": "finished",
                    "source": "fileURL",
                    "kind": "video",
                    "format": "original",
                    "folderID": folder,
                    "outputPath": "/tmp/Film/film.mp4",
                    "contentFolder": "/tmp/Film/Example"
                }]
            }]
        });
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes())
            .unwrap();
        let state = import_legacy_library(&path, "/tmp/Downloads".to_string()).unwrap();
        assert_eq!(state.workspaces[0].id.0, workspace);
        assert_eq!(state.workspaces[0].downloads[0].id.0, download);
        assert_eq!(
            state.workspaces[0].file_organization,
            FileOrganization::AlwaysCreateFolder
        );
        assert_eq!(
            state.workspaces[0].downloads[0]
                .output
                .as_ref()
                .unwrap()
                .content_folder
                .as_deref(),
            Some("/tmp/Film/Example")
        );
        assert_eq!(state.migration.as_ref().unwrap().imported_downloads, 1);
        fs::remove_dir_all(directory).unwrap();
    }
}
