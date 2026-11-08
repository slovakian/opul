use super::*;

pub(super) fn icon_from_view(icon: MachineWorkspaceIcon) -> WorkspaceIcon {
    match icon.kind {
        MachineWorkspaceIconKind::Symbol => WorkspaceIcon::Symbol(icon.value),
        MachineWorkspaceIconKind::Emoji => WorkspaceIcon::Emoji(icon.value),
    }
}

pub(super) fn format_from_view(format: MachineFormat) -> MediaFormat {
    match format {
        MachineFormat::Best => MediaFormat::Best,
        MachineFormat::BestMp4 => MediaFormat::BestMp4,
        MachineFormat::P1080 => MediaFormat::P1080,
        MachineFormat::P720 => MediaFormat::P720,
        MachineFormat::Original => MediaFormat::Original,
        MachineFormat::AudioMp3 => MediaFormat::AudioMp3,
        MachineFormat::AudioM4a => MediaFormat::AudioM4a,
    }
}

pub(super) fn format_view(format: MediaFormat) -> MachineFormat {
    match format {
        MediaFormat::Best => MachineFormat::Best,
        MediaFormat::BestMp4 => MachineFormat::BestMp4,
        MediaFormat::P1080 => MachineFormat::P1080,
        MediaFormat::P720 => MachineFormat::P720,
        MediaFormat::Original => MachineFormat::Original,
        MediaFormat::AudioMp3 => MachineFormat::AudioMp3,
        MediaFormat::AudioM4a => MachineFormat::AudioM4a,
    }
}

pub(super) fn engine_format(format: MediaFormat) -> EngineMediaFormat {
    match format {
        MediaFormat::Best => EngineMediaFormat::Best,
        MediaFormat::BestMp4 => EngineMediaFormat::BestMp4,
        MediaFormat::P1080 => EngineMediaFormat::P1080,
        MediaFormat::P720 => EngineMediaFormat::P720,
        MediaFormat::Original => EngineMediaFormat::Original,
        MediaFormat::AudioMp3 => EngineMediaFormat::AudioMp3,
        MediaFormat::AudioM4a => EngineMediaFormat::AudioM4a,
    }
}

pub(super) fn organization_from_view(value: MachineFileOrganization) -> FileOrganization {
    match value {
        MachineFileOrganization::Automatic => FileOrganization::Automatic,
        MachineFileOrganization::AlwaysCreateFolder => FileOrganization::AlwaysCreateFolder,
        MachineFileOrganization::KeepInSaveLocation => FileOrganization::KeepInSaveLocation,
    }
}

pub(super) fn organization_view(value: FileOrganization) -> MachineFileOrganization {
    match value {
        FileOrganization::Automatic => MachineFileOrganization::Automatic,
        FileOrganization::AlwaysCreateFolder => MachineFileOrganization::AlwaysCreateFolder,
        FileOrganization::KeepInSaveLocation => MachineFileOrganization::KeepInSaveLocation,
    }
}

pub(super) fn content_layout(value: FileOrganization) -> DownloadContentLayout {
    match value {
        FileOrganization::Automatic => DownloadContentLayout::Original,
        FileOrganization::AlwaysCreateFolder => DownloadContentLayout::Subfolder,
        FileOrganization::KeepInSaveLocation => DownloadContentLayout::NoSubfolder,
    }
}

pub(super) fn policy_from_view(value: &MachineMediaPolicy) -> MediaPolicy {
    MediaPolicy {
        download_subtitles: value.download_subtitles,
        subtitle_languages: value.subtitle_languages.clone(),
        embed_subtitles: value.embed_subtitles,
        write_thumbnail: value.write_thumbnail,
        write_metadata: value.write_metadata,
        filename_template: value.filename_template.clone(),
        user_agent: value.user_agent.clone(),
    }
}

pub(super) fn media_options(
    download: &MediaDownload,
    layout: DownloadContentLayout,
) -> MediaDownloadOptions {
    let referer = match &download.origin {
        MediaOrigin::PendingPage { url: page_url }
        | MediaOrigin::Page { page_url, .. }
        | MediaOrigin::Guided { page_url, .. } => Some(page_url.clone()),
        MediaOrigin::Direct { referer, .. } => referer.clone(),
    };
    MediaDownloadOptions {
        format: engine_format(download.format),
        content_layout: layout,
        download_subtitles: download.policy.download_subtitles,
        subtitle_languages: download.policy.subtitle_languages.clone(),
        embed_subtitles: download.policy.embed_subtitles,
        write_thumbnail: download.policy.write_thumbnail,
        write_metadata: download.policy.write_metadata,
        filename_template: download.policy.filename_template.clone(),
        referer,
        user_agent: download.policy.user_agent.clone(),
    }
}

pub(super) fn file_from_view(file: MachineTorrentFile) -> TorrentFileSelection {
    TorrentFileSelection {
        index: file.index,
        path: file.path,
        size: file.size,
        priority: match file.priority.as_str() {
            "skip" | "Skip" => TorrentFilePriority::Skip,
            "low" | "Low" => TorrentFilePriority::Low,
            "high" | "High" => TorrentFilePriority::High,
            _ => TorrentFilePriority::Normal,
        },
    }
}

pub(super) fn output_artifact(path: String, content_folder: Option<String>) -> OutputArtifact {
    let identity_path = content_folder.as_deref().unwrap_or(&path);
    let identity = fs::metadata(identity_path)
        .ok()
        .map(|metadata| OutputIdentity {
            byte_length: metadata.len(),
            modified_at_millis: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or(0),
            #[cfg(unix)]
            file_identifier: {
                use std::os::unix::fs::MetadataExt;
                Some(format!("{}:{}", metadata.dev(), metadata.ino()))
            },
            #[cfg(not(unix))]
            file_identifier: None,
        });
    OutputArtifact {
        path,
        content_folder,
        identity,
    }
}

pub(super) fn fraction_to_millionths(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    (value.clamp(0.0, 1.0) * 1_000_000.0).round() as u32
}

pub(super) fn fingerprint(command: &Command) -> Result<String, StateMachineError> {
    let bytes = serde_json::to_vec(command).map_err(|error| StateMachineError::Rejected {
        reason: format!("could not fingerprint command: {error}"),
    })?;
    Ok(hex::encode(Sha1::digest(bytes)))
}

pub(super) fn parse_id(value: &str, kind: &str) -> Result<Uuid, StateMachineError> {
    Uuid::parse_str(value).map_err(|error| StateMachineError::InvalidIdentifier {
        reason: format!("invalid {kind} ID: {error}"),
    })
}

pub(super) fn workspace_id(value: &str) -> Result<WorkspaceId, StateMachineError> {
    parse_id(value, "workspace").map(WorkspaceId)
}

pub(super) fn folder_id(value: &str) -> Result<FolderId, StateMachineError> {
    parse_id(value, "folder").map(FolderId)
}

pub(super) fn download_id(value: &str) -> Result<DownloadId, StateMachineError> {
    parse_id(value, "download").map(DownloadId)
}

pub(super) fn command_id(value: &str) -> Result<CommandId, StateMachineError> {
    parse_id(value, "command").map(CommandId)
}

pub(super) fn effect_id(value: &str) -> Result<EffectId, StateMachineError> {
    parse_id(value, "effect").map(EffectId)
}

pub(super) fn actor_error(error: ActorError) -> StateMachineError {
    match error {
        ActorError::Open(reason) => StateMachineError::Open { reason },
        ActorError::AlreadyOpen => StateMachineError::Open {
            reason: "the application state is already open".to_string(),
        },
        ActorError::Rejected(reason) | ActorError::ReadOnly(reason) => {
            StateMachineError::Rejected { reason }
        }
        ActorError::Durability(reason) => StateMachineError::Persistence { reason },
        ActorError::Stopped => StateMachineError::Open {
            reason: "the application state machine stopped".to_string(),
        },
    }
}

pub(super) fn open_error(error: ActorError) -> StateMachineError {
    actor_error(error)
}

pub(super) fn engine_error(error: crate::CoreError) -> StateMachineError {
    StateMachineError::Engine {
        reason: error.to_string(),
    }
}

pub(super) fn secure_write_new(path: &Path, bytes: &[u8]) -> Result<(), StateMachineError> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| StateMachineError::Persistence {
            reason: format!("could not create {}: {error}", path.display()),
        })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| StateMachineError::Persistence {
            reason: format!("could not write {}: {error}", path.display()),
        })?;
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| StateMachineError::Persistence {
                reason: format!("could not sync {}: {error}", parent.display()),
            })?;
    }
    Ok(())
}
