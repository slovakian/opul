use super::*;

pub(super) fn update_direct_progress(
    context: &WorkerContext,
    id: &str,
    downloaded: u64,
    total: u64,
    speed: u64,
) {
    let mut items = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(item) = items.get_mut(id) else {
        return;
    };
    if item.state == MediaTransferState::Cancelled {
        return;
    }
    if item.state != MediaTransferState::Paused {
        item.state = MediaTransferState::Downloading;
    }
    item.downloaded_bytes = downloaded;
    item.total_bytes = total;
    item.download_rate = speed;
    if total > 0 {
        item.progress = (downloaded as f64 / total as f64).clamp(0.0, 1.0);
        item.eta_seconds = (speed > 0).then(|| total.saturating_sub(downloaded) / speed);
    }
}

pub(super) fn apply_format_args(command: &mut Command, format: MediaFormat, has_ffmpeg: bool) {
    match format {
        MediaFormat::Best => {
            command.args([
                "--format",
                if has_ffmpeg {
                    "bestvideo*+bestaudio/best"
                } else {
                    "best"
                },
            ]);
        }
        MediaFormat::BestMp4 => {
            if has_ffmpeg {
                command.args([
                    "--format",
                    "bestvideo*[ext=mp4]+bestaudio[ext=m4a]/best[ext=mp4]/best",
                    "--merge-output-format",
                    "mp4",
                ]);
            } else {
                command.args(["--format", "best[ext=mp4]/best"]);
            }
        }
        MediaFormat::P1080 => {
            command.args([
                "--format",
                if has_ffmpeg {
                    "bestvideo*[height<=1080]+bestaudio/best[height<=1080]/best"
                } else {
                    "best[height<=1080]/best"
                },
            ]);
        }
        MediaFormat::P720 => {
            command.args([
                "--format",
                if has_ffmpeg {
                    "bestvideo*[height<=720]+bestaudio/best[height<=720]/best"
                } else {
                    "best[height<=720]/best"
                },
            ]);
        }
        MediaFormat::Original => {
            command.args(["--format", "best"]);
        }
        MediaFormat::AudioMp3 => {
            command.args([
                "--format",
                "bestaudio/best",
                "--extract-audio",
                "--audio-format",
                "mp3",
            ]);
        }
        MediaFormat::AudioM4a => {
            command.args([
                "--format",
                "bestaudio/best",
                "--extract-audio",
                "--audio-format",
                "m4a",
            ]);
        }
    }
}

pub(super) fn output_template(item: &StoredMediaDownload) -> PathBuf {
    let template = if item.options.filename_template.trim().is_empty() {
        "%(title).180B [%(id)s].%(ext)s"
    } else {
        item.options.filename_template.trim()
    };
    Path::new(&item.output_folder).join(template)
}

pub(super) fn apply_progress_line(
    items: &Arc<Mutex<HashMap<String, StoredMediaDownload>>>,
    id: &str,
    line: &str,
) {
    let mut parts = line.split('\t');
    match parts.next() {
        Some("opul-progress") => {
            let status = parts.next().unwrap_or_default();
            let downloaded = numeric_field(parts.next());
            let total = numeric_field(parts.next());
            let speed = numeric_field(parts.next());
            let eta = numeric_field(parts.next());
            // The path is last so a tab inside it cannot shift the numeric fields.
            let temporary = parts.collect::<Vec<_>>().join("\t");
            let mut items = items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get_mut(id) else {
                return;
            };
            remember_partial_file(item, temporary);
            if !matches!(
                item.state,
                MediaTransferState::Paused | MediaTransferState::Cancelled
            ) {
                item.state = if status == "finished" {
                    MediaTransferState::Processing
                } else {
                    MediaTransferState::Downloading
                };
            }
            item.downloaded_bytes = downloaded.unwrap_or(item.downloaded_bytes);
            item.total_bytes = total.unwrap_or(item.total_bytes);
            item.download_rate = speed.unwrap_or(0);
            item.eta_seconds = eta;
            if item.total_bytes > 0 {
                item.progress =
                    (item.downloaded_bytes as f64 / item.total_bytes as f64).clamp(0.0, 1.0);
            }
        }
        Some("opul-postprocess") => {
            let _status = parts.next();
            // The file the postprocessor rewrites, which it writes through a temporary
            // sibling first; a merge names a file no download progress line mentioned.
            let target = parts.collect::<Vec<_>>().join("\t");
            let mut items = items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get_mut(id) else {
                return;
            };
            remember_partial_file(item, target);
            if !matches!(
                item.state,
                MediaTransferState::Paused | MediaTransferState::Cancelled
            ) {
                item.state = MediaTransferState::Processing;
                item.download_rate = 0;
                item.eta_seconds = None;
            }
        }
        Some("opul-file") => {
            let path = parts.collect::<Vec<_>>().join("\t");
            let mut items = items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(item) = items.get_mut(id) {
                item.output_path = (!path.trim().is_empty()).then_some(path);
            }
        }
        _ => {}
    }
}

pub(super) fn remember_partial_file(item: &mut StoredMediaDownload, path: String) {
    if !path.trim().is_empty() && path != "NA" && !item.partial_files.contains(&path) {
        item.partial_files.push(path);
    }
}

pub(super) fn numeric_field(value: Option<&str>) -> Option<u64> {
    let value = value?.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("NA") || value.eq_ignore_ascii_case("none") {
        return None;
    }
    value.parse::<u64>().ok().or_else(|| {
        value
            .parse::<f64>()
            .ok()
            .map(|number| number.max(0.0) as u64)
    })
}

pub(super) fn validate_http_url(value: &str) -> Result<(), CoreError> {
    if value.len() > 16_384 {
        return Err(CoreError::InvalidUrl {
            reason: "media URLs cannot exceed 16 KB".to_string(),
        });
    }
    let parsed = Url::parse(value).map_err(|error| CoreError::InvalidUrl {
        reason: format!("invalid URL: {error}"),
    })?;
    if !matches!(parsed.scheme(), "http" | "https") || parsed.host_str().is_none() {
        return Err(CoreError::InvalidUrl {
            reason: "media downloads require an http or https URL".to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_filename_template(value: &str) -> Result<(), CoreError> {
    if value.len() > 1_024 {
        return Err(CoreError::InvalidConfiguration {
            reason: "the filename template cannot exceed 1 KB".to_string(),
        });
    }
    let path = Path::new(value.trim());
    if path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir))
    {
        return Err(CoreError::InvalidConfiguration {
            reason: "the filename template must stay inside the selected download folder"
                .to_string(),
        });
    }
    Ok(())
}

pub(super) fn validate_output_folder(value: &str) -> Result<(), CoreError> {
    if value.is_empty() || !Path::new(value).is_absolute() {
        return Err(CoreError::InvalidConfiguration {
            reason: "the download folder must be an absolute path".to_string(),
        });
    }
    Ok(())
}

pub(super) fn tool_version(path: &Path, arguments: &[&str]) -> Option<String> {
    let output = Command::new(path).args(arguments).output().ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout)
        .ok()?
        .lines()
        .next()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            line.strip_prefix("ffmpeg version ")
                .or_else(|| line.strip_prefix("ffprobe version "))
                .and_then(|version| version.split_whitespace().next())
                .unwrap_or(line)
                .to_string()
        })
}

pub(super) fn signal_process(pid: u32, signal: &str) {
    let _ = Command::new("/bin/kill")
        .arg(format!("-{signal}"))
        .arg("--")
        .arg(format!("-{pid}"))
        .status();
}

pub(super) fn load_downloads(
    path: &Path,
) -> Result<LoadedState<HashMap<String, StoredMediaDownload>>, String> {
    load_state(path, "media session", validate_downloads)
}

pub(super) fn save_downloads(path: &Path, items: &HashMap<String, StoredMediaDownload>) {
    if let Err(error) = save_state(path, items, validate_downloads) {
        eprintln!("Warning: could not save media session: {error}");
    }
}

pub(super) fn validate_downloads(
    items: &HashMap<String, StoredMediaDownload>,
) -> Result<(), String> {
    for (key, item) in items {
        if key != &item.id || Uuid::parse_str(key).is_err() {
            return Err(format!("media download {key:?} has an invalid identifier"));
        }
        validate_http_url(&item.source_url)
            .map_err(|error| format!("media download {key} has an invalid source URL: {error}"))?;
        validate_filename_template(&item.options.filename_template).map_err(|error| {
            format!("media download {key} has an invalid filename template: {error}")
        })?;
        validate_output_folder(&item.output_folder).map_err(|error| {
            format!("media download {key} has an invalid output folder: {error}")
        })?;
    }
    Ok(())
}

pub(super) fn concise_process_error(stderr: &[u8], fallback: &str) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

pub(super) fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

pub(super) fn json_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_f64().map(|number| number.max(0.0) as u64))
    })
}

pub(super) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
