use crate::CoreError;
use crate::bencode::{self, BValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentFileInfo {
    pub file_index: u32,
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TorrentMeta {
    pub info_hash: String,
    pub name: String,
    pub piece_length: u64,
    pub piece_count: u32,
    pub total_size: u64,
    pub is_multi_file: bool,
    pub files: Vec<TorrentFileInfo>,
    pub trackers: Vec<String>,
    pub comment: Option<String>,
    pub created_by: Option<String>,
    pub creation_date: Option<i64>,
    pub is_private: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagnetMeta {
    pub info_hash: String,
    pub name: Option<String>,
    pub trackers: Vec<String>,
    pub exact_length: Option<u64>,
}

/// Where a torrent Download is in its transfer.
///
/// Seeding means the selected files are on disk and Opul is still uploading.
/// Finished means seeding stopped because the ratio was met or the person stopped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorrentState {
    Queued,
    Fetching,
    Paused,
    Seeding,
    Finished,
    Failed,
}

/// Folx stores one signed byte per file. Zero skips the file.
/// librqbit only has an include mask, so high and low stay on the Download
/// and skip is the bit the engine honors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilePriority {
    Normal,
    High,
    Low,
    Skip,
}

impl FilePriority {
    pub fn as_label(self) -> &'static str {
        match self {
            FilePriority::Normal => "Normal",
            FilePriority::High => "High",
            FilePriority::Low => "Low",
            FilePriority::Skip => "Skip",
        }
    }

    pub fn included(self) -> bool {
        !matches!(self, FilePriority::Skip)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTransferStatus {
    pub file_index: u32,
    pub path: String,
    pub size: u64,
    pub bytes_completed: u64,
    pub is_included: bool,
    pub priority: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TorrentTransferSnapshot {
    pub download_id: Option<String>,
    pub attempt: u32,
    pub info_hash: String,
    pub source: String,
    pub name: String,
    pub save_folder: String,
    pub content_folder: Option<String>,
    pub state: TorrentState,
    pub total_size: u64,
    pub selected_size: u64,
    pub bytes_downloaded: u64,
    pub bytes_uploaded: u64,
    pub download_rate: u64,
    pub upload_rate: u64,
    pub peers: u32,
    pub seeds: u32,
    pub seed_ratio: f64,
    pub seed_ratio_limit: Option<f64>,
    pub progress: f64,
    pub eta_seconds: Option<u64>,
    pub piece_length: u64,
    pub piece_count: u32,
    pub comment: Option<String>,
    pub trackers: Vec<String>,
    pub files: Vec<FileTransferStatus>,
    pub verifying: bool,
    pub error_message: Option<String>,
}

pub fn parse_torrent_bytes(bytes: &[u8]) -> Result<TorrentMeta, CoreError> {
    let root = bencode::decode(bytes).map_err(|reason| CoreError::InvalidTorrent { reason })?;
    let info = root
        .get_dict_entry("info")
        .ok_or_else(|| CoreError::InvalidTorrent {
            reason: "Missing 'info' dictionary in torrent".to_string(),
        })?;

    let info_hash = hex::encode(info.sha1_digest());
    let name = info
        .get_dict_entry("name")
        .and_then(|v| v.as_string())
        .unwrap_or_else(|| "Unknown torrent".to_string());

    let piece_length = info
        .get_dict_entry("piece length")
        .and_then(|v| v.as_int())
        .map(|v| v as u64)
        .unwrap_or(0);

    let pieces_bytes = info
        .get_dict_entry("pieces")
        .and_then(|v| match v {
            BValue::Bytes(b) => Some(b.as_slice()),
            _ => None,
        })
        .unwrap_or(&[]);
    let piece_count = (pieces_bytes.len() / 20) as u32;

    let is_private = info
        .get_dict_entry("private")
        .and_then(|v| v.as_int())
        .map(|v| v == 1)
        .unwrap_or(false);

    let mut trackers = Vec::new();
    if let Some(ann) = root.get_dict_entry("announce").and_then(|v| v.as_string())
        && !ann.is_empty()
    {
        trackers.push(ann);
    }
    if let Some(ann_list) = root
        .get_dict_entry("announce-list")
        .and_then(|v| v.as_list())
    {
        for tier in ann_list {
            if let Some(tier_list) = tier.as_list() {
                for tracker in tier_list {
                    if let Some(url) = tracker.as_string()
                        && !trackers.contains(&url)
                    {
                        trackers.push(url);
                    }
                }
            }
        }
    }

    let comment = root.get_dict_entry("comment").and_then(|v| v.as_string());
    let created_by = root
        .get_dict_entry("created by")
        .and_then(|v| v.as_string());
    let creation_date = root
        .get_dict_entry("creation date")
        .and_then(|v| v.as_int());

    let mut files = Vec::new();
    let mut total_size: u64 = 0;
    let is_multi_file = info
        .get_dict_entry("files")
        .and_then(|v| v.as_list())
        .is_some();
    if let Some(files_list) = info.get_dict_entry("files").and_then(|v| v.as_list()) {
        for (index, file_val) in files_list.iter().enumerate() {
            let length = file_val
                .get_dict_entry("length")
                .and_then(|v| v.as_int())
                .map(|v| v as u64)
                .unwrap_or(0);
            total_size += length;
            let path_parts: Vec<String> = file_val
                .get_dict_entry("path")
                .and_then(|v| v.as_list())
                .map(|parts| parts.iter().filter_map(|p| p.as_string()).collect())
                .unwrap_or_default();
            let full_path = if path_parts.is_empty() {
                format!("file_{index}")
            } else {
                path_parts.join("/")
            };
            files.push(TorrentFileInfo {
                file_index: index as u32,
                path: full_path,
                size: length,
            });
        }
    } else {
        let length = info
            .get_dict_entry("length")
            .and_then(|v| v.as_int())
            .map(|v| v as u64)
            .unwrap_or(0);
        total_size = length;
        files.push(TorrentFileInfo {
            file_index: 0,
            path: name.clone(),
            size: length,
        });
    }

    Ok(TorrentMeta {
        info_hash,
        name,
        piece_length,
        piece_count,
        total_size,
        is_multi_file,
        files,
        trackers,
        comment,
        created_by,
        creation_date,
        is_private,
    })
}

pub fn parse_magnet_uri(uri: &str) -> Result<MagnetMeta, CoreError> {
    let trimmed = uri.trim();
    if !trimmed.to_ascii_lowercase().starts_with("magnet:?") {
        return Err(CoreError::InvalidMagnet {
            reason: "URI does not start with magnet:?".to_string(),
        });
    }
    let query = &trimmed["magnet:?".len()..];
    let mut info_hash = None;
    let mut name = None;
    let mut exact_length = None;
    let mut trackers = Vec::new();

    for param in query.split('&') {
        if param.is_empty() {
            continue;
        }
        let mut parts = param.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let decoded = url_decode(value);
        match key {
            "xt" => {
                if let Some(hash) = extract_btih(&decoded) {
                    info_hash = Some(hash);
                }
            }
            "dn" => name = Some(decoded),
            "tr" => {
                if !decoded.is_empty() && !trackers.contains(&decoded) {
                    trackers.push(decoded);
                }
            }
            "xl" => {
                if let Ok(length) = decoded.parse::<u64>() {
                    exact_length = Some(length);
                }
            }
            _ => {}
        }
    }

    let info_hash = info_hash.ok_or_else(|| CoreError::InvalidMagnet {
        reason: "No valid BTIH xt parameter found in magnet URI".to_string(),
    })?;
    Ok(MagnetMeta {
        info_hash,
        name,
        trackers,
        exact_length,
    })
}

pub fn create_magnet_uri(info_hash: &str, name: Option<String>, trackers: Vec<String>) -> String {
    let mut uri = format!("magnet:?xt=urn:btih:{}", info_hash.to_ascii_lowercase());
    if let Some(name) = name {
        uri.push_str("&dn=");
        uri.push_str(&url_encode(&name));
    }
    for tracker in trackers {
        uri.push_str("&tr=");
        uri.push_str(&url_encode(&tracker));
    }
    uri
}

fn extract_btih(value: &str) -> Option<String> {
    let lower = value.to_ascii_lowercase();
    let prefix = "urn:btih:";
    let hash_part = lower.split(prefix).nth(1)?;
    let hash_part = hash_part.split('&').next().unwrap_or(hash_part);
    if hash_part.len() == 40 && hash_part.chars().all(|c| c.is_ascii_hexdigit()) {
        return Some(hash_part.to_string());
    }
    if hash_part.len() == 32 {
        return base32_to_hex(hash_part);
    }
    None
}

fn base32_to_hex(input: &str) -> Option<String> {
    const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";
    let mut buffer: u64 = 0;
    let mut bits = 0;
    let mut bytes = Vec::new();
    for c in input.chars() {
        let byte = c.to_ascii_lowercase() as u8;
        let val = ALPHABET.iter().position(|&x| x == byte)? as u64;
        buffer = (buffer << 5) | val;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if bytes.len() == 20 {
        Some(hex::encode(bytes))
    } else {
        None
    }
}

fn url_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut decoded = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                std::str::from_utf8(&bytes[index + 1..index + 3]).unwrap_or(""),
                16,
            ) {
                decoded.push(byte);
                index += 3;
                continue;
            }
        } else if bytes[index] == b'+' {
            decoded.push(b' ');
            index += 1;
            continue;
        }
        decoded.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn url_encode(input: &str) -> String {
    let mut encoded = String::new();
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[cfg(test)]
use std::collections::BTreeMap;

#[cfg(test)]
pub fn encode_meta_for_test(info: BTreeMap<Vec<u8>, BValue>, announce: Option<&str>) -> Vec<u8> {
    let mut root = BTreeMap::new();
    if let Some(announce) = announce {
        root.insert(
            b"announce".to_vec(),
            BValue::Bytes(announce.as_bytes().to_vec()),
        );
    }
    root.insert(b"info".to_vec(), BValue::Dict(info));
    BValue::Dict(root).encode()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn magnet_uri_parses_hex_name_and_tracker() {
        let hash = "4a5c6d7e8f901234567890abcdef1234567890ab";
        let uri = format!(
            "magnet:?xt=urn:btih:{hash}&dn=Linux+ISO&tr=udp%3A%2F%2Ftracker.example.com%3A80"
        );
        let meta = parse_magnet_uri(&uri).unwrap();
        assert_eq!(meta.info_hash, hash);
        assert_eq!(meta.name.as_deref(), Some("Linux ISO"));
        assert_eq!(
            meta.trackers,
            vec!["udp://tracker.example.com:80".to_string()]
        );
        let generated = create_magnet_uri(&meta.info_hash, meta.name, meta.trackers);
        assert!(generated.contains("xt=urn:btih:4a5c6d7e8f901234567890abcdef1234567890ab"));
        assert!(generated.contains("dn=Linux%20ISO"));
    }

    #[test]
    fn magnet_uri_accepts_base32_info_hash() {
        let uri = "magnet:?xt=urn:btih:MFRGGZDFMZTWQ2LKNNWG23TPOBYXE43U";
        let meta = parse_magnet_uri(uri).unwrap();
        assert_eq!(meta.info_hash.len(), 40);
        assert!(meta.info_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn torrent_single_file_parsing() {
        let mut info = BTreeMap::new();
        info.insert(b"name".to_vec(), BValue::Bytes(b"test_movie.mp4".to_vec()));
        info.insert(b"length".to_vec(), BValue::Int(1_048_576));
        info.insert(b"piece length".to_vec(), BValue::Int(262_144));
        info.insert(b"pieces".to_vec(), BValue::Bytes(vec![0u8; 80]));
        let encoded = encode_meta_for_test(info, Some("http://tracker.test/announce"));
        let meta = parse_torrent_bytes(&encoded).unwrap();
        assert_eq!(meta.name, "test_movie.mp4");
        assert_eq!(meta.total_size, 1_048_576);
        assert_eq!(meta.piece_count, 4);
        assert_eq!(meta.files.len(), 1);
        assert!(!meta.is_multi_file);
        assert_eq!(
            meta.trackers,
            vec!["http://tracker.test/announce".to_string()]
        );
        assert_eq!(meta.info_hash.len(), 40);
    }

    #[test]
    fn torrent_multi_file_parsing() {
        let mut file1 = BTreeMap::new();
        file1.insert(b"length".to_vec(), BValue::Int(500));
        file1.insert(
            b"path".to_vec(),
            BValue::List(vec![
                BValue::Bytes(b"subfolder".to_vec()),
                BValue::Bytes(b"file1.txt".to_vec()),
            ]),
        );
        let mut file2 = BTreeMap::new();
        file2.insert(b"length".to_vec(), BValue::Int(1500));
        file2.insert(
            b"path".to_vec(),
            BValue::List(vec![BValue::Bytes(b"file2.mp4".to_vec())]),
        );
        let mut info = BTreeMap::new();
        info.insert(b"name".to_vec(), BValue::Bytes(b"MyMultiTorrent".to_vec()));
        info.insert(
            b"files".to_vec(),
            BValue::List(vec![BValue::Dict(file1), BValue::Dict(file2)]),
        );
        info.insert(b"piece length".to_vec(), BValue::Int(1024));
        info.insert(b"pieces".to_vec(), BValue::Bytes(vec![0u8; 40]));
        let encoded = encode_meta_for_test(info, None);
        let meta = parse_torrent_bytes(&encoded).unwrap();
        assert_eq!(meta.name, "MyMultiTorrent");
        assert_eq!(meta.total_size, 2000);
        assert_eq!(meta.files[0].path, "subfolder/file1.txt");
        assert_eq!(meta.files[1].path, "file2.mp4");
        assert!(meta.is_multi_file);
    }
}
