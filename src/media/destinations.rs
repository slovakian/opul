use super::*;

pub(super) fn validate_direct_content_type(
    response: &reqwest::Response,
    final_url: &Url,
    expected: Option<GuidedMediaKind>,
) -> Result<(), String> {
    let Some(expected) = expected.filter(|kind| *kind != GuidedMediaKind::Unknown) else {
        return Ok(());
    };
    let Some(content_type) = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
    else {
        return Ok(());
    };
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    let compatible = match expected {
        GuidedMediaKind::Video => {
            mime.starts_with("video/")
                || matches!(mime.as_str(), "application/mp4" | "application/ogg")
        }
        GuidedMediaKind::Audio => {
            mime.starts_with("audio/")
                || matches!(mime.as_str(), "application/mp4" | "application/ogg")
        }
        GuidedMediaKind::Image => mime.starts_with("image/"),
        GuidedMediaKind::Subtitle => matches!(
            mime.as_str(),
            "text/vtt"
                | "text/plain"
                | "application/x-subrip"
                | "application/ttml+xml"
                | "application/xml"
                | "text/xml"
        ),
        GuidedMediaKind::Stream | GuidedMediaKind::Unknown => true,
    };
    let generic = matches!(
        mime.as_str(),
        "application/octet-stream" | "binary/octet-stream"
    );
    if !compatible && !generic {
        return Err(format!(
            "the media request for {final_url} returned {mime} instead of the expected {expected:?} content"
        ));
    }
    Ok(())
}

pub(super) fn direct_response_filename(
    response: &reqwest::Response,
    final_url: &Url,
    fallback: &str,
) -> String {
    let disposition_name = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .and_then(content_disposition_filename);
    let mut filename = disposition_name
        .as_deref()
        .map(sanitize_direct_filename)
        .filter(|value| value != "download")
        .unwrap_or_else(|| direct_filename(final_url, fallback));
    if Path::new(&filename).extension().is_none()
        && let Some(extension) = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(direct_mime_extension)
    {
        filename.push('.');
        filename.push_str(extension);
    }
    filename
}

pub(super) fn content_disposition_filename(value: &str) -> Option<String> {
    let parameters = value.split(';').skip(1).filter_map(|parameter| {
        let (name, value) = parameter.trim().split_once('=')?;
        Some((name.trim().to_ascii_lowercase(), value.trim()))
    });
    let mut plain = None;
    for (name, value) in parameters {
        if name == "filename*" {
            let encoded = value.trim_matches('"');
            let encoded = encoded
                .split_once("''")
                .map(|(_, value)| value)
                .unwrap_or(encoded);
            if let Some(decoded) = percent_decode_utf8(encoded) {
                return Some(decoded);
            }
        } else if name == "filename" {
            plain = Some(value.trim_matches('"').replace("\\\"", "\""));
        }
    }
    plain
}

pub(super) fn percent_decode_utf8(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = hex_digit(*bytes.get(index + 1)?)?;
            let low = hex_digit(*bytes.get(index + 2)?)?;
            decoded.push(high * 16 + low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

pub(super) fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

pub(super) fn direct_mime_extension(value: &str) -> Option<&'static str> {
    match value
        .split(';')
        .next()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "video/mp4" | "application/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "audio/mpeg" => Some("mp3"),
        "audio/mp4" | "audio/x-m4a" => Some("m4a"),
        "audio/ogg" | "application/ogg" => Some("ogg"),
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/webp" => Some("webp"),
        "image/gif" => Some("gif"),
        "image/avif" => Some("avif"),
        "text/vtt" => Some("vtt"),
        "application/x-subrip" => Some("srt"),
        "application/ttml+xml" => Some("ttml"),
        _ => None,
    }
}

pub(super) fn direct_temporary_path(folder: &str, source: &Url, title: &str, id: &str) -> PathBuf {
    let provisional_filename = direct_filename(source, title);
    Path::new(folder).join(format!(".{provisional_filename}.{id}.opulpart"))
}

/// The files yt-dlp derives from one file it reported: the file itself, the file it
/// becomes, the resume record next to that, the copy a postprocessor writes before
/// replacing it, and the fragments of a segmented stream.
pub(super) fn yt_dlp_partial_paths(temporary: &Path) -> Vec<PathBuf> {
    let mut paths = vec![temporary.to_path_buf()];
    let name = temporary.to_string_lossy();
    let final_path = match name.strip_suffix(".part") {
        Some(final_name) => {
            paths.push(PathBuf::from(final_name));
            paths.push(PathBuf::from(format!("{final_name}.ytdl")));
            PathBuf::from(final_name)
        }
        None => temporary.to_path_buf(),
    };
    paths.extend(yt_dlp_postprocessor_path(&final_path));
    if let (Some(folder), Some(file_name)) = (temporary.parent(), temporary.file_name()) {
        let fragment_prefix = format!("{}-Frag", file_name.to_string_lossy());
        if let Ok(entries) = fs::read_dir(folder) {
            paths.extend(
                entries
                    .flatten()
                    .filter(|entry| {
                        entry
                            .file_name()
                            .to_string_lossy()
                            .starts_with(&fragment_prefix)
                    })
                    .map(|entry| entry.path()),
            );
        }
    }
    paths
}

/// Where a yt-dlp postprocessor writes before replacing `path`: `Film.mp4` becomes
/// `Film.temp.mp4`, as yt-dlp's `prepend_extension(path, "temp")` does.
pub(super) fn yt_dlp_postprocessor_path(path: &Path) -> Option<PathBuf> {
    let stem = path.file_stem()?.to_string_lossy();
    let name = match path.extension() {
        Some(extension) => format!("{stem}.temp.{}", extension.to_string_lossy()),
        None => format!("{stem}.temp"),
    };
    Some(path.with_file_name(name))
}

pub(super) fn direct_filename(source: &Url, fallback: &str) -> String {
    let candidate = source
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    sanitize_direct_filename(candidate)
}

pub(super) fn sanitize_direct_filename(candidate: &str) -> String {
    let sanitized = candidate
        .chars()
        .map(|character| {
            if character.is_control() || matches!(character, '/' | '\\' | ':') {
                '_'
            } else {
                character
            }
        })
        .take(180)
        .collect::<String>();
    match sanitized.trim().trim_start_matches('.') {
        "" | "." | ".." => "download".to_string(),
        value => value.to_string(),
    }
}

pub(super) fn reserve_direct_destination(
    folder: &Path,
    filename: &str,
    temporary: &Path,
) -> Result<PathBuf, String> {
    let original = Path::new(filename);
    let stem = original
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("download");
    let extension = original.extension().and_then(|value| value.to_str());
    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            filename.to_string()
        } else if let Some(extension) = extension {
            format!("{stem} ({suffix}).{extension}")
        } else {
            format!("{stem} ({suffix})")
        };
        let destination = folder.join(name);
        match fs::hard_link(temporary, &destination) {
            Ok(()) => {
                let _ = fs::remove_file(temporary);
                return Ok(destination);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(format!("could not finalize the direct download: {error}")),
        }
    }
    Err("could not choose a unique filename for the direct download".to_string())
}

pub(super) fn direct_cookie_header(cookie_file: Option<&Path>, source: &Url) -> Option<String> {
    let contents = fs::read_to_string(cookie_file?).ok()?;
    let host = source.host_str()?.to_ascii_lowercase();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut values = Vec::new();
    let mut header_length = 0;
    for original in contents.lines() {
        let line = original.strip_prefix("#HttpOnly_").unwrap_or(original);
        if line.starts_with('#') || line.trim().is_empty() {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 7 {
            continue;
        }
        let domain = fields[0].trim_start_matches('.').to_ascii_lowercase();
        let include_subdomains = fields[1].eq_ignore_ascii_case("TRUE");
        let domain_matches =
            host == domain || (include_subdomains && host.ends_with(&format!(".{domain}")));
        let cookie_path = fields[2];
        let request_path = source.path();
        let path_matches = cookie_path_matches(cookie_path, request_path);
        let secure_matches = !fields[3].eq_ignore_ascii_case("TRUE") || source.scheme() == "https";
        let expiry_matches = fields[4]
            .parse::<u64>()
            .map(|expiry| expiry == 0 || expiry > now)
            .unwrap_or(false);
        if !(domain_matches && path_matches && secure_matches && expiry_matches) {
            continue;
        }
        let value = format!("{}={}", fields[5], fields[6]);
        if header_length + value.len() + 2 > 32 * 1024 {
            break;
        }
        header_length += value.len() + 2;
        values.push(value);
    }
    (!values.is_empty()).then(|| values.join("; "))
}

pub(super) fn item_cookie_path(
    context: &WorkerContext,
    item: &StoredMediaDownload,
) -> Option<PathBuf> {
    item.cookie_file.as_ref()?;
    owned_cookie_path(&context.cookie_directory, &item.id).filter(|path| path.is_file())
}

pub(super) fn item_request_headers(
    context: &WorkerContext,
    item: &StoredMediaDownload,
) -> Vec<GuidedHttpHeader> {
    if !item.request_headers.is_empty() {
        return item.request_headers.clone();
    }
    if item.request_header_file.is_none() {
        return Vec::new();
    }
    let Some(path) = owned_request_header_path(&context.request_header_directory, &item.id) else {
        return Vec::new();
    };
    fs::read(path)
        .ok()
        .and_then(|contents| serde_json::from_slice(&contents).ok())
        .map(|headers: Vec<GuidedHttpHeader>| sanitize_guided_headers(&headers))
        .unwrap_or_default()
}

pub(super) fn remove_transfer_credentials(context: &WorkerContext, item: &mut StoredMediaDownload) {
    item.request_headers.clear();
    if item.cookie_file.take().is_some()
        && let Some(path) = owned_cookie_path(&context.cookie_directory, &item.id)
    {
        let _ = fs::remove_file(path);
    }
    if item.request_header_file.take().is_some()
        && let Some(path) = owned_request_header_path(&context.request_header_directory, &item.id)
    {
        let _ = fs::remove_file(path);
    }
}

pub(super) fn owned_cookie_path(directory: &Path, id: &str) -> Option<PathBuf> {
    let id = Uuid::parse_str(id).ok()?;
    Some(directory.join(format!("{id}.txt")))
}

pub(super) fn owned_request_header_path(directory: &Path, id: &str) -> Option<PathBuf> {
    let id = Uuid::parse_str(id).ok()?;
    Some(directory.join(format!("{id}.json")))
}
