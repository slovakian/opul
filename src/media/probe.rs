use super::*;

pub fn classify_guided_url(url: String, initiator: String) -> GuidedMediaKind {
    classify_guided_resource(&url, &initiator, None)
}

pub(super) fn classify_guided_resource(
    url: &str,
    initiator: &str,
    mime_type: Option<&str>,
) -> GuidedMediaKind {
    let lower_initiator = initiator.to_ascii_lowercase();
    let parsed = Url::parse(url).ok();
    let extension = parsed
        .as_ref()
        .and_then(|parsed| {
            Path::new(parsed.path())
                .extension()
                .map(|value| value.to_string_lossy().to_ascii_lowercase())
        })
        .unwrap_or_default();
    let mime = mime_type
        .unwrap_or_default()
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if matches!(
        mime.as_str(),
        "application/vnd.apple.mpegurl"
            | "application/x-mpegurl"
            | "audio/mpegurl"
            | "audio/x-mpegurl"
            | "application/dash+xml"
    ) {
        return GuidedMediaKind::Stream;
    }
    if mime.starts_with("video/") {
        return GuidedMediaKind::Video;
    }
    if mime.starts_with("audio/") {
        return GuidedMediaKind::Audio;
    }
    if mime.starts_with("image/") {
        return GuidedMediaKind::Image;
    }
    if matches!(
        mime.as_str(),
        "text/vtt" | "application/ttml+xml" | "application/x-subrip"
    ) {
        return GuidedMediaKind::Subtitle;
    }
    match extension.as_str() {
        "m3u8" | "mpd" => GuidedMediaKind::Stream,
        "mp4" | "m4v" | "mov" | "mkv" | "webm" | "flv" => GuidedMediaKind::Video,
        "mp3" | "m4a" | "aac" | "flac" | "wav" | "ogg" | "opus" => GuidedMediaKind::Audio,
        "vtt" | "srt" | "ttml" | "dfxp" | "ass" => GuidedMediaKind::Subtitle,
        "jpg" | "jpeg" | "png" | "webp" | "gif" | "avif" | "heic" => GuidedMediaKind::Image,
        _ if lower_initiator == "audio" => GuidedMediaKind::Audio,
        _ if lower_initiator == "track" => GuidedMediaKind::Subtitle,
        _ if matches!(lower_initiator.as_str(), "video" | "source") => GuidedMediaKind::Video,
        _ if lower_initiator == "img" || lower_initiator == "image" => GuidedMediaKind::Image,
        _ if parsed.as_ref().is_some_and(|url| {
            let value = url.path().to_ascii_lowercase();
            value.contains("videoplayback")
        }) =>
        {
            GuidedMediaKind::Video
        }
        _ if parsed.as_ref().is_some_and(|url| {
            let value = url.path().to_ascii_lowercase();
            value.contains("manifest") || value.contains("playlist")
        }) =>
        {
            GuidedMediaKind::Stream
        }
        _ => GuidedMediaKind::Unknown,
    }
}

pub(super) fn is_media_mime(value: &str) -> bool {
    let mime = value
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    mime.starts_with("video/")
        || mime.starts_with("audio/")
        || mime.starts_with("image/")
        || matches!(
            mime.as_str(),
            "application/vnd.apple.mpegurl"
                | "application/x-mpegurl"
                | "application/dash+xml"
                | "text/vtt"
                | "application/ttml+xml"
                | "application/x-subrip"
        )
}

pub(super) fn probe_with_ytdlp(
    yt_dlp: &YtDlpCommand,
    cookie_path: Option<&Path>,
    url: &str,
) -> Result<MediaProbe, CoreError> {
    let mut command = yt_dlp.command();
    command.args(["--dump-single-json", "--flat-playlist", "--no-warnings"]);
    if let Some(cookie_path) = cookie_path {
        command.arg("--cookies").arg(cookie_path);
    }
    command.arg("--").arg(url);
    let output = command.output().map_err(|error| CoreError::Extraction {
        reason: format!("could not run yt-dlp: {error}"),
    })?;
    if !output.status.success() {
        return Err(CoreError::Extraction {
            reason: concise_process_error(&output.stderr, "yt-dlp could not inspect that address"),
        });
    }
    let value: Value =
        serde_json::from_slice(&output.stdout).map_err(|error| CoreError::Extraction {
            reason: format!("yt-dlp returned invalid metadata: {error}"),
        })?;
    parse_probe(&value, url)
}

pub(super) fn parse_probe(value: &Value, fallback_url: &str) -> Result<MediaProbe, CoreError> {
    let title = json_string(value, "title").unwrap_or_else(|| fallback_url.to_string());
    let webpage_url = json_string(value, "webpage_url")
        .or_else(|| json_string(value, "original_url"))
        .unwrap_or_else(|| fallback_url.to_string());
    let extractor = json_string(value, "extractor_key").or_else(|| json_string(value, "extractor"));
    let thumbnail_url = json_string(value, "thumbnail");
    let entries = value
        .get("entries")
        .and_then(Value::as_array)
        .filter(|entries| !entries.is_empty());
    let items = if let Some(entries) = entries {
        entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| probe_item(entry, index as u32 + 1, extractor.as_deref()))
            .collect::<Vec<_>>()
    } else {
        vec![MediaProbeItem {
            webpage_url: webpage_url.clone(),
            title: title.clone(),
            thumbnail_url: thumbnail_url.clone(),
            extractor: extractor.clone(),
            duration_seconds: json_u64(value, "duration"),
            playlist_index: None,
        }]
    };
    if items.is_empty() {
        return Err(CoreError::Extraction {
            reason: "yt-dlp found a collection but no downloadable entries.".to_string(),
        });
    }
    Ok(MediaProbe {
        title,
        webpage_url,
        extractor,
        thumbnail_url,
        is_playlist: entries.is_some(),
        items,
    })
}

pub(super) fn probe_item(
    value: &Value,
    fallback_index: u32,
    parent_extractor: Option<&str>,
) -> Option<MediaProbeItem> {
    let id = json_string(value, "id");
    let extractor = json_string(value, "extractor_key")
        .or_else(|| json_string(value, "extractor"))
        .or_else(|| parent_extractor.map(str::to_string));
    let webpage_url = json_string(value, "webpage_url")
        .or_else(|| json_string(value, "original_url"))
        .or_else(|| json_string(value, "url"))
        .or_else(|| youtube_url(id.as_deref(), extractor.as_deref()))?;
    let webpage_url = if webpage_url.starts_with("http://") || webpage_url.starts_with("https://") {
        webpage_url
    } else {
        youtube_url(Some(&webpage_url), extractor.as_deref())?
    };
    Some(MediaProbeItem {
        webpage_url,
        title: json_string(value, "title")
            .or(id)
            .unwrap_or_else(|| "Untitled download".to_string()),
        thumbnail_url: json_string(value, "thumbnail"),
        extractor,
        duration_seconds: json_u64(value, "duration"),
        playlist_index: json_u64(value, "playlist_index")
            .and_then(|value| u32::try_from(value).ok())
            .or(Some(fallback_index)),
    })
}

pub(super) fn youtube_url(id: Option<&str>, extractor: Option<&str>) -> Option<String> {
    let id = id?;
    if extractor
        .unwrap_or_default()
        .to_ascii_lowercase()
        .contains("youtube")
    {
        Some(format!("https://www.youtube.com/watch?v={id}"))
    } else {
        None
    }
}
