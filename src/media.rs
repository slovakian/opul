use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::collections::{HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use url::Url;
use uuid::Uuid;

use crate::CoreError;
use crate::destination::{DownloadContentLayout, resolve_download_destination};
use crate::persistence::{LoadedState, load_state, save_state};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, uniffi::Enum)]
pub enum MediaFormat {
    Best,
    BestMp4,
    P1080,
    P720,
    #[default]
    Original,
    AudioMp3,
    AudioM4a,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
pub struct MediaDownloadOptions {
    #[serde(default)]
    pub format: MediaFormat,
    #[serde(default)]
    pub content_layout: DownloadContentLayout,
    pub download_subtitles: bool,
    #[serde(default)]
    pub subtitle_languages: Vec<String>,
    #[serde(default)]
    pub embed_subtitles: bool,
    #[serde(default)]
    pub write_thumbnail: bool,
    #[serde(default)]
    pub write_metadata: bool,
    #[serde(default = "default_filename_template")]
    pub filename_template: String,
    #[serde(default)]
    pub referer: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
}

fn default_filename_template() -> String {
    "%(title)s.%(ext)s".to_string()
}

impl Default for MediaDownloadOptions {
    fn default() -> Self {
        Self {
            format: MediaFormat::Original,
            content_layout: DownloadContentLayout::Original,
            download_subtitles: false,
            subtitle_languages: Vec::new(),
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: default_filename_template(),
            referer: None,
            user_agent: None,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MediaEngineConfig {
    pub session_folder: String,
    pub resources_folder: Option<String>,
    pub yt_dlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
    pub max_concurrent_downloads: u32,
    pub max_download_bps: u64,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MediaToolStatus {
    pub yt_dlp_path: Option<String>,
    pub yt_dlp_version: Option<String>,
    pub ffmpeg_path: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub ready: bool,
    pub message: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MediaProbeItem {
    pub webpage_url: String,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub duration_seconds: Option<u64>,
    pub playlist_index: Option<u32>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MediaProbe {
    pub title: String,
    pub webpage_url: String,
    pub extractor: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_playlist: bool,
    pub items: Vec<MediaProbeItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, uniffi::Enum)]
pub enum MediaTransferState {
    #[default]
    Queued,
    Downloading,
    Processing,
    Paused,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct MediaTransferSnapshot {
    pub id: String,
    pub download_id: Option<String>,
    pub attempt: u32,
    pub source_url: String,
    pub page_url: String,
    pub title: String,
    pub state: MediaTransferState,
    pub progress: f64,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub download_rate: u64,
    pub eta_seconds: Option<u64>,
    pub output_path: Option<String>,
    pub content_folder: Option<String>,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub error_message: Option<String>,
    pub playlist_index: Option<u32>,
    pub format: MediaFormat,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, uniffi::Enum)]
pub enum GuidedMediaKind {
    Video,
    Audio,
    Stream,
    Subtitle,
    Image,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum GuidedObservationSource {
    Dom,
    Performance,
    Navigation,
    Popup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, uniffi::Record)]
pub struct GuidedHttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GuidedObservation {
    pub url: String,
    pub page_url: String,
    pub frame_url: Option<String>,
    pub title: String,
    pub initiator: String,
    pub mime_type: Option<String>,
    pub status_code: Option<u32>,
    pub content_length: Option<u64>,
    pub observed_at_ms: u64,
    pub source: GuidedObservationSource,
    pub is_main_frame: bool,
    pub element_key: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub headers: Vec<GuidedHttpHeader>,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GuidedCandidate {
    pub id: String,
    pub url: String,
    pub title: String,
    pub kind: GuidedMediaKind,
    pub referer: String,
    pub frame_url: Option<String>,
    pub initiator: String,
    pub mime_type: Option<String>,
    pub score: i32,
    pub occurrences: u32,
    pub first_seen_ms: u64,
    pub last_seen_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub is_expired: bool,
    pub reason: String,
    pub direct_download: bool,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct GuidedDownloadSelection {
    pub candidate_id: String,
    pub user_agent: Option<String>,
    pub cookies: String,
}

#[derive(Clone)]
struct GuidedCandidateEntry {
    candidate: GuidedCandidate,
    headers: Vec<GuidedHttpHeader>,
}

#[derive(Default)]
struct GuidedExtractionState {
    candidates: HashMap<String, GuidedCandidateEntry>,
    aliases: HashMap<String, String>,
    limit_reached: bool,
}

#[derive(uniffi::Object)]
pub struct GuidedExtractionSession {
    state: Mutex<GuidedExtractionState>,
}

#[uniffi::export]
impl GuidedExtractionSession {
    #[uniffi::constructor]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(GuidedExtractionState::default()),
        })
    }

    pub fn observe(&self, observation: GuidedObservation) -> Vec<GuidedCandidate> {
        self.observe_batch(vec![observation])
    }

    pub fn observe_batch(&self, observations: Vec<GuidedObservation>) -> Vec<GuidedCandidate> {
        let observations = observations.into_iter().take(256).collect::<Vec<_>>();
        let observed_at_ms = observations
            .iter()
            .map(|observation| observation.observed_at_ms)
            .max()
            .filter(|value| *value > 0)
            .unwrap_or_else(now_millis);
        let candidates = observations
            .iter()
            .filter_map(|observation| {
                candidate_from_observation(observation).map(|candidate| {
                    let identity = guided_candidate_identity(observation, candidate.candidate.kind);
                    (identity, candidate)
                })
            })
            .collect::<Vec<_>>();
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for (identity, candidate) in candidates {
            insert_guided_candidate(&mut state, identity, candidate);
        }
        guided_candidate_snapshot(&state, observed_at_ms)
    }

    pub fn candidates(&self) -> Vec<GuidedCandidate> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guided_candidate_snapshot(&state, now_millis())
    }

    pub fn limit_reached(&self) -> bool {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .limit_reached
    }

    pub fn clear(&self) {
        *self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = GuidedExtractionState::default();
    }
}

impl GuidedExtractionSession {
    fn resolve_candidate(&self, id: &str) -> Result<GuidedCandidateEntry, CoreError> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut entry = state
            .candidates
            .values()
            .find(|entry| entry.candidate.id == id)
            .cloned()
            .ok_or_else(|| CoreError::InvalidConfiguration {
                reason: "the guided media candidate is no longer available".to_string(),
            })?;
        entry.candidate.is_expired = entry
            .candidate
            .expires_at_ms
            .is_some_and(|expiry| expiry <= now_millis());
        if entry.candidate.is_expired {
            return Err(CoreError::InvalidConfiguration {
                reason: "that media URL has expired; play it again to capture a fresh URL"
                    .to_string(),
            });
        }
        Ok(entry)
    }
}

fn insert_guided_candidate(
    state: &mut GuidedExtractionState,
    identity: String,
    candidate: GuidedCandidateEntry,
) {
    let candidate_key = state.aliases.get(&identity).cloned().or_else(|| {
        state
            .candidates
            .iter()
            .find(|(_, existing)| {
                existing.candidate.kind == candidate.candidate.kind
                    && existing.candidate.url == candidate.candidate.url
                    && same_candidate_context(existing, &candidate)
            })
            .map(|(key, _)| key.clone())
    });
    if let Some(candidate_key) = candidate_key {
        if state.aliases.len() < 4_000 {
            state.aliases.insert(identity, candidate_key.clone());
        }
        if let Some(existing) = state.candidates.get_mut(&candidate_key) {
            merge_guided_candidate(existing, candidate);
        }
    } else if state.candidates.len() < 2_000 {
        if state.aliases.len() < 4_000 {
            state.aliases.insert(identity.clone(), identity.clone());
        }
        state.candidates.insert(identity, candidate);
    } else {
        state.limit_reached = true;
        let eviction = state
            .candidates
            .iter()
            .min_by_key(|(_, entry)| (entry.candidate.score, entry.candidate.last_seen_ms))
            .map(|(key, entry)| (key.clone(), entry.candidate.score));
        if let Some((evicted_key, evicted_score)) = eviction
            && candidate.candidate.score > evicted_score
        {
            state.candidates.remove(&evicted_key);
            state.aliases.retain(|_, key| key != &evicted_key);
            if state.aliases.len() < 4_000 {
                state.aliases.insert(identity.clone(), identity.clone());
            }
            state.candidates.insert(identity, candidate);
        }
    }
}

fn merge_guided_candidate(
    existing: &mut GuidedCandidateEntry,
    mut candidate: GuidedCandidateEntry,
) {
    let id = existing.candidate.id.clone();
    let first_seen_ms = existing
        .candidate
        .first_seen_ms
        .min(candidate.candidate.first_seen_ms);
    let last_seen_ms = existing
        .candidate
        .last_seen_ms
        .max(candidate.candidate.last_seen_ms);
    let occurrences = existing.candidate.occurrences.saturating_add(1);
    if existing.candidate.url != candidate.candidate.url
        && candidate.candidate.last_seen_ms < existing.candidate.last_seen_ms
    {
        existing.candidate.occurrences = occurrences;
        return;
    }
    if existing.candidate.url == candidate.candidate.url
        && candidate.candidate.score < existing.candidate.score
    {
        existing.candidate.last_seen_ms = last_seen_ms;
        existing.candidate.occurrences = occurrences;
        if existing.headers.is_empty() && !candidate.headers.is_empty() {
            existing.headers = candidate.headers;
        }
        if existing.candidate.mime_type.is_none() {
            existing.candidate.mime_type = candidate.candidate.mime_type;
        }
        if existing.candidate.frame_url.is_none() {
            existing.candidate.frame_url = candidate.candidate.frame_url;
            existing.candidate.referer = candidate.candidate.referer;
        }
        return;
    }
    let same_resource_origin = Url::parse(&existing.candidate.url)
        .ok()
        .zip(Url::parse(&candidate.candidate.url).ok())
        .is_some_and(|(existing_url, candidate_url)| same_origin(&existing_url, &candidate_url));
    if candidate.headers.is_empty() && same_resource_origin {
        candidate.headers = existing.headers.clone();
    }
    if candidate.candidate.mime_type.is_none() {
        candidate.candidate.mime_type = existing.candidate.mime_type.clone();
    }
    if candidate.candidate.frame_url.is_none() && existing.candidate.frame_url.is_some() {
        candidate.candidate.frame_url = existing.candidate.frame_url.clone();
        candidate.candidate.referer = existing.candidate.referer.clone();
    }
    candidate.candidate.id = id;
    candidate.candidate.first_seen_ms = first_seen_ms;
    candidate.candidate.last_seen_ms = last_seen_ms;
    candidate.candidate.occurrences = occurrences;
    *existing = candidate;
}

fn same_candidate_context(left: &GuidedCandidateEntry, right: &GuidedCandidateEntry) -> bool {
    if left.headers.iter().any(is_sensitive_guided_header)
        || right.headers.iter().any(is_sensitive_guided_header)
    {
        return left.candidate.referer == right.candidate.referer;
    }
    true
}

fn is_sensitive_guided_header(header: &GuidedHttpHeader) -> bool {
    matches!(header.name.as_str(), "authorization" | "origin")
}

fn candidate_from_observation(observation: &GuidedObservation) -> Option<GuidedCandidateEntry> {
    if observation.url.len() > 16_384
        || observation.page_url.len() > 16_384
        || observation.title.len() > 4_096
        || observation.initiator.len() > 128
        || observation.status_code.is_some_and(|status| status >= 400)
    {
        return None;
    }
    let mut url = Url::parse(&observation.url).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    if observation.source == GuidedObservationSource::Performance
        && (is_media_segment(&url)
            || observation.mime_type.as_deref().is_some_and(|mime| {
                let mime = mime.split(';').next().unwrap_or_default().trim();
                ["video/mp2t", "video/iso.segment", "audio/iso.segment"]
                    .iter()
                    .any(|segment_mime| mime.eq_ignore_ascii_case(segment_mime))
            }))
    {
        return None;
    }
    let mut kind = classify_guided_resource(
        url.as_str(),
        &observation.initiator,
        observation.mime_type.as_deref(),
    );
    if kind == GuidedMediaKind::Unknown
        && observation.source == GuidedObservationSource::Navigation
        && observation
            .status_code
            .is_some_and(|status| (200..300).contains(&status))
        && observation
            .content_length
            .is_some_and(|length| length >= 256 * 1024)
        && observation.mime_type.as_deref().is_some_and(|mime| {
            matches!(
                mime.split(';').next().unwrap_or_default().trim(),
                "application/octet-stream" | "binary/octet-stream"
            )
        })
    {
        kind = GuidedMediaKind::Video;
    }
    if kind == GuidedMediaKind::Unknown {
        return None;
    }
    let observed_at_ms = if observation.observed_at_ms == 0 {
        now_millis()
    } else {
        observation.observed_at_ms
    };
    let referer = valid_http_url(observation.frame_url.as_deref())
        .or_else(|| valid_http_url(Some(&observation.page_url)))
        .unwrap_or_else(|| url.as_str().to_string());
    let title = bounded_candidate_title(&observation.title, &url);
    let expires_at_ms = expiry_from_url(&url);
    let headers = sanitize_guided_headers(&observation.headers);
    let (score, reason) = score_guided_observation(observation, kind);
    let identity = guided_candidate_identity(observation, kind);
    let id = hex::encode(Sha1::digest(identity.as_bytes()));
    Some(GuidedCandidateEntry {
        candidate: GuidedCandidate {
            id,
            url: url.into(),
            title,
            kind,
            referer,
            frame_url: valid_http_url(observation.frame_url.as_deref()),
            initiator: observation.initiator.chars().take(128).collect(),
            mime_type: observation
                .mime_type
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.chars().take(256).collect()),
            score,
            occurrences: 1,
            first_seen_ms: observed_at_ms,
            last_seen_ms: observed_at_ms,
            expires_at_ms,
            is_expired: expires_at_ms.is_some_and(|expiry| expiry <= observed_at_ms),
            reason,
            direct_download: matches!(
                kind,
                GuidedMediaKind::Video
                    | GuidedMediaKind::Audio
                    | GuidedMediaKind::Subtitle
                    | GuidedMediaKind::Image
            ),
        },
        headers,
    })
}

fn guided_candidate_identity(observation: &GuidedObservation, kind: GuidedMediaKind) -> String {
    if let Some(element_key) = observation
        .element_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let frame = observation
            .frame_url
            .as_deref()
            .unwrap_or(&observation.page_url);
        return format!(
            "element:{kind:?}:{}:{}:{}",
            frame,
            observation.initiator.to_ascii_lowercase(),
            element_key.chars().take(512).collect::<String>()
        );
    }
    if kind == GuidedMediaKind::Stream
        && let Ok(url) = Url::parse(&observation.url)
    {
        let context = guided_sensitive_context(observation);
        return format!("stream:{}{context}", normalized_stream_url(url));
    }
    let mut url = observation.url.clone();
    if let Ok(mut parsed) = Url::parse(&url) {
        parsed.set_fragment(None);
        url = parsed.into();
    }
    let context = guided_sensitive_context(observation);
    format!("url:{url}{context}")
}

fn guided_sensitive_context(observation: &GuidedObservation) -> String {
    if observation.headers.iter().any(|header| {
        matches!(
            header.name.trim().to_ascii_lowercase().as_str(),
            "authorization" | "origin"
        )
    }) {
        format!(
            ":context:{}",
            observation
                .frame_url
                .as_deref()
                .unwrap_or(&observation.page_url)
        )
    } else {
        String::new()
    }
}

fn normalized_stream_url(mut url: Url) -> Url {
    let mut semantic_query = url
        .query_pairs()
        .filter(|(name, _)| !is_volatile_signature_parameter(name))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect::<Vec<_>>();
    semantic_query.sort();
    url.set_fragment(None);
    url.set_query(None);
    if !semantic_query.is_empty() {
        url.query_pairs_mut().extend_pairs(semantic_query);
    }
    url
}

fn is_volatile_signature_parameter(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    matches!(
        name.as_str(),
        "token"
            | "access_token"
            | "auth"
            | "authorization"
            | "jwt"
            | "sig"
            | "signature"
            | "policy"
            | "key-pair-id"
            | "expires"
            | "expire"
            | "expiry"
            | "exp"
            | "hdnts"
            | "st"
            | "se"
    ) || name.starts_with("x-amz-")
        || name.starts_with("x-goog-")
}

fn guided_candidate_snapshot(
    state: &GuidedExtractionState,
    observed_at_ms: u64,
) -> Vec<GuidedCandidate> {
    let mut candidates = state
        .candidates
        .values()
        .map(|entry| entry.candidate.clone())
        .collect::<Vec<_>>();
    for candidate in &mut candidates {
        candidate.is_expired = candidate
            .expires_at_ms
            .is_some_and(|expiry| expiry <= observed_at_ms);
    }
    candidates.sort_by(|left, right| {
        left.is_expired
            .cmp(&right.is_expired)
            .then_with(|| right.score.cmp(&left.score))
            .then_with(|| right.last_seen_ms.cmp(&left.last_seen_ms))
            .then_with(|| left.url.cmp(&right.url))
    });
    candidates
}

fn valid_http_url(value: Option<&str>) -> Option<String> {
    let value = value?.trim();
    if value.len() > 16_384 {
        return None;
    }
    let mut url = Url::parse(value).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url.set_fragment(None);
    Some(url.into())
}

fn bounded_candidate_title(title: &str, url: &Url) -> String {
    let trimmed = title.trim();
    if !trimmed.is_empty() {
        return trimmed.chars().take(512).collect();
    }
    url.path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .or_else(|| url.host_str())
        .unwrap_or("Detected media")
        .chars()
        .take(512)
        .collect()
}

fn score_guided_observation(
    observation: &GuidedObservation,
    kind: GuidedMediaKind,
) -> (i32, String) {
    let kind_score = match kind {
        GuidedMediaKind::Stream => 900,
        GuidedMediaKind::Video => 820,
        GuidedMediaKind::Audio => 780,
        GuidedMediaKind::Subtitle => 520,
        GuidedMediaKind::Image => 220,
        GuidedMediaKind::Unknown => 0,
    };
    let source_score = match observation.source {
        GuidedObservationSource::Dom => 90,
        GuidedObservationSource::Navigation => 70,
        GuidedObservationSource::Popup => 60,
        GuidedObservationSource::Performance => 30,
    };
    let mime_score = if observation.mime_type.as_deref().is_some_and(is_media_mime) {
        45
    } else {
        0
    };
    let response_score = if observation
        .status_code
        .is_some_and(|status| (200..300).contains(&status))
    {
        20
    } else {
        0
    };
    let size_score = observation
        .content_length
        .filter(|length| *length >= 1_000_000)
        .map(|_| 15)
        .unwrap_or(0);
    let tiny_image_penalty = if kind == GuidedMediaKind::Image
        && observation.width.is_some_and(|width| width < 160)
        && observation.height.is_some_and(|height| height < 90)
    {
        -180
    } else {
        0
    };
    let reason = format!(
        "{:?} observed via {:?} in {}{}",
        kind,
        observation.source,
        if observation.is_main_frame {
            "the main frame"
        } else {
            "a child frame"
        },
        observation
            .mime_type
            .as_deref()
            .map(|mime| format!(" ({mime})"))
            .unwrap_or_default()
    );
    (
        kind_score + source_score + mime_score + response_score + size_score + tiny_image_penalty,
        reason,
    )
}

fn expiry_from_url(url: &Url) -> Option<u64> {
    let query = url
        .query_pairs()
        .map(|(name, value)| (name.to_ascii_lowercase(), value.into_owned()))
        .collect::<HashMap<_, _>>();
    if let Some(expiry) = ["expires", "expire", "expiry", "exp"]
        .iter()
        .find_map(|name| query.get(*name).and_then(|value| value.parse::<u64>().ok()))
    {
        return Some(if expiry >= 10_000_000_000 {
            expiry
        } else {
            expiry.saturating_mul(1_000)
        });
    }
    if let Some(expiry) = query.get("hdnts").and_then(|value| {
        value.split('~').find_map(|part| {
            part.strip_prefix("exp=")
                .and_then(|value| value.parse::<u64>().ok())
        })
    }) {
        return Some(expiry.saturating_mul(1_000));
    }
    if let Some(expiry) = query.get("se").and_then(|value| parse_utc_seconds(value)) {
        return Some(expiry.saturating_mul(1_000));
    }
    for (date_key, duration_key) in [
        ("x-amz-date", "x-amz-expires"),
        ("x-goog-date", "x-goog-expires"),
    ] {
        if let Some(expiry) = query
            .get(date_key)
            .and_then(|value| parse_utc_seconds(value))
            .zip(
                query
                    .get(duration_key)
                    .and_then(|value| value.parse::<u64>().ok()),
            )
            .map(|(start, duration)| start.saturating_add(duration).saturating_mul(1_000))
        {
            return Some(expiry);
        }
    }
    None
}

fn parse_utc_seconds(value: &str) -> Option<u64> {
    if !value.is_ascii() {
        return None;
    }
    let digits =
        if value.len() == 16 && value.as_bytes().get(8) == Some(&b'T') && value.ends_with('Z') {
            format!(
                "{}{}{}{}{}{}",
                &value[0..4],
                &value[4..6],
                &value[6..8],
                &value[9..11],
                &value[11..13],
                &value[13..15]
            )
        } else if value.len() >= 20
            && value.as_bytes().get(4) == Some(&b'-')
            && value.as_bytes().get(7) == Some(&b'-')
            && value.as_bytes().get(10) == Some(&b'T')
            && value.as_bytes().get(13) == Some(&b':')
            && value.as_bytes().get(16) == Some(&b':')
            && value.ends_with('Z')
        {
            format!(
                "{}{}{}{}{}{}",
                &value[0..4],
                &value[5..7],
                &value[8..10],
                &value[11..13],
                &value[14..16],
                &value[17..19]
            )
        } else {
            return None;
        };
    let year = digits[0..4].parse::<i64>().ok()?;
    let month = digits[4..6].parse::<i64>().ok()?;
    let day = digits[6..8].parse::<i64>().ok()?;
    let hour = digits[8..10].parse::<i64>().ok()?;
    let minute = digits[10..12].parse::<i64>().ok()?;
    let second = digits[12..14].parse::<i64>().ok()?;
    let leap_year = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap_year => 29,
        2 => 28,
        _ => return None,
    };
    if !(1..=days_in_month).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days
        .checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?;
    u64::try_from(seconds).ok()
}

fn sanitize_guided_headers(headers: &[GuidedHttpHeader]) -> Vec<GuidedHttpHeader> {
    headers
        .iter()
        .filter_map(|header| {
            let name = header.name.trim().to_ascii_lowercase();
            let value = header.value.trim();
            if !matches!(
                name.as_str(),
                "accept" | "accept-language" | "authorization" | "origin"
            ) || value.is_empty()
                || name.len() > 64
                || value.len() > 8_192
                || value.contains(['\r', '\n'])
            {
                return None;
            }
            Some(GuidedHttpHeader {
                name,
                value: value.to_string(),
            })
        })
        .take(16)
        .collect()
}

fn is_media_segment(url: &Url) -> bool {
    let path = url.path().to_ascii_lowercase();
    matches!(
        Path::new(&path)
            .extension()
            .and_then(|value| value.to_str()),
        Some("ts" | "m4s" | "cmfv" | "cmfa" | "m4f")
    ) || path.ends_with("/init.mp4")
        || path.ends_with("/init.webm")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredMediaDownload {
    id: String,
    #[serde(default)]
    download_id: Option<String>,
    #[serde(default)]
    attempt: u32,
    source_url: String,
    #[serde(default)]
    page_url: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    output_folder: String,
    #[serde(default)]
    content_folder: Option<String>,
    thumbnail_url: Option<String>,
    #[serde(default)]
    extractor: Option<String>,
    #[serde(default)]
    playlist_index: Option<u32>,
    #[serde(default)]
    options: MediaDownloadOptions,
    #[serde(default)]
    direct_download: bool,
    #[serde(default)]
    cookie_file: Option<String>,
    #[serde(default, skip_serializing)]
    request_headers: Vec<GuidedHttpHeader>,
    #[serde(default)]
    request_header_file: Option<String>,
    #[serde(default)]
    guided_kind: Option<GuidedMediaKind>,
    #[serde(default)]
    state: MediaTransferState,
    #[serde(default)]
    progress: f64,
    #[serde(default)]
    total_bytes: u64,
    #[serde(default)]
    downloaded_bytes: u64,
    #[serde(default)]
    download_rate: u64,
    #[serde(default)]
    eta_seconds: Option<u64>,
    #[serde(default)]
    output_path: Option<String>,
    #[serde(default)]
    error_message: Option<String>,
    #[serde(default)]
    created_at: u64,
}

#[derive(Default)]
struct GuidedTransferContext {
    direct_download: bool,
    cookie_contents: Option<String>,
    request_headers: Vec<GuidedHttpHeader>,
    guided_kind: Option<GuidedMediaKind>,
}

impl StoredMediaDownload {
    fn snapshot(&self) -> MediaTransferSnapshot {
        MediaTransferSnapshot {
            id: self.id.clone(),
            download_id: self.download_id.clone(),
            attempt: self.attempt,
            source_url: self.source_url.clone(),
            page_url: self.page_url.clone(),
            title: self.title.clone(),
            state: self.state,
            progress: self.progress,
            total_bytes: self.total_bytes,
            downloaded_bytes: self.downloaded_bytes,
            download_rate: self.download_rate,
            eta_seconds: self.eta_seconds,
            output_path: self.output_path.clone(),
            content_folder: self.content_folder.clone(),
            thumbnail_url: self.thumbnail_url.clone(),
            extractor: self.extractor.clone(),
            error_message: self.error_message.clone(),
            playlist_index: self.playlist_index,
            format: self.options.format,
        }
    }
}

#[derive(Debug)]
struct Gate {
    running: u32,
    maximum: u32,
    shutdown: bool,
}

#[derive(Debug, Default)]
struct WorkerRegistry {
    active: std::collections::HashSet<String>,
    retry_requested: std::collections::HashSet<String>,
}

#[derive(Clone)]
struct WorkerContext {
    yt_dlp: Option<PathBuf>,
    ffmpeg: Option<PathBuf>,
    cookie_directory: PathBuf,
    request_header_directory: PathBuf,
    store_path: PathBuf,
    items: Arc<Mutex<HashMap<String, StoredMediaDownload>>>,
    processes: Arc<Mutex<HashMap<String, u32>>>,
    workers: Arc<Mutex<WorkerRegistry>>,
    gate: Arc<(Mutex<Gate>, Condvar)>,
    max_download_bps: Arc<Mutex<u64>>,
}

#[derive(uniffi::Object)]
pub struct MediaEngine {
    yt_dlp: Option<PathBuf>,
    yt_dlp_version: Option<String>,
    ffmpeg: Option<PathBuf>,
    ffmpeg_version: Option<String>,
    cookie_path: PathBuf,
    store_path: PathBuf,
    items: Arc<Mutex<HashMap<String, StoredMediaDownload>>>,
    processes: Arc<Mutex<HashMap<String, u32>>>,
    workers: Arc<Mutex<WorkerRegistry>>,
    gate: Arc<(Mutex<Gate>, Condvar)>,
    max_download_bps: Arc<Mutex<u64>>,
    recovery_notice: Option<String>,
}

fn open_media_engine(
    config: MediaEngineConfig,
    restore_pending: bool,
) -> Result<Arc<MediaEngine>, CoreError> {
    let session_folder = PathBuf::from(&config.session_folder);
    fs::create_dir_all(&session_folder).map_err(|error| CoreError::Engine {
        reason: format!("could not create the media session folder: {error}"),
    })?;

    let resource_folder = config.resources_folder.as_deref().map(Path::new);
    let yt_dlp = locate_tool(
        config.yt_dlp_path.as_deref(),
        resource_folder,
        "yt-dlp",
        &["/opt/homebrew/bin/yt-dlp", "/usr/local/bin/yt-dlp"],
        &["--version"],
    );
    let ffmpeg = locate_tool(
        config.ffmpeg_path.as_deref(),
        resource_folder,
        "ffmpeg",
        &["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"],
        &["-version"],
    );
    let yt_dlp_version = yt_dlp
        .as_ref()
        .and_then(|path| tool_version(path, &["--version"]));
    let ffmpeg_version = ffmpeg
        .as_ref()
        .and_then(|path| tool_version(path, &["-version"]));
    let store_path = session_folder.join("media-downloads.json");
    let cookie_path = session_folder.join("browser-cookies.txt");
    let loaded = load_downloads(&store_path).map_err(|reason| CoreError::Engine { reason })?;
    let items = Arc::new(Mutex::new(loaded.items));
    let engine = Arc::new(MediaEngine {
        yt_dlp,
        yt_dlp_version,
        ffmpeg,
        ffmpeg_version,
        cookie_path,
        store_path,
        items,
        processes: Arc::new(Mutex::new(HashMap::new())),
        workers: Arc::new(Mutex::new(WorkerRegistry::default())),
        gate: Arc::new((
            Mutex::new(Gate {
                running: 0,
                maximum: config.max_concurrent_downloads.max(1),
                shutdown: false,
            }),
            Condvar::new(),
        )),
        max_download_bps: Arc::new(Mutex::new(config.max_download_bps)),
        recovery_notice: loaded.recovery_notice,
    });
    if restore_pending {
        engine.restore_pending();
    }
    Ok(engine)
}

#[uniffi::export]
impl MediaEngine {
    #[uniffi::constructor]
    pub fn new(config: MediaEngineConfig) -> Result<Arc<Self>, CoreError> {
        open_media_engine(config, true)
    }

    pub fn recovery_notice(&self) -> Option<String> {
        self.recovery_notice.clone()
    }

    pub fn tool_status(&self) -> MediaToolStatus {
        let ready = self.yt_dlp.is_some();
        MediaToolStatus {
            yt_dlp_path: self
                .yt_dlp
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            yt_dlp_version: self.yt_dlp_version.clone(),
            ffmpeg_path: self
                .ffmpeg
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            ffmpeg_version: self.ffmpeg_version.clone(),
            ready,
            message: if ready {
                if self.ffmpeg.is_some() {
                    "Video extraction and post-processing are ready.".to_string()
                } else {
                    "Video extraction is ready. Install FFmpeg to merge formats, convert audio, and embed subtitles.".to_string()
                }
            } else {
                "Install yt-dlp to download video pages. Torrents and direct guided files remain available."
                    .to_string()
            },
        }
    }

    pub fn probe_url(&self, url: String) -> Result<MediaProbe, CoreError> {
        validate_http_url(&url)?;
        let yt_dlp = self.require_yt_dlp()?;
        let cookie_path = self.cookie_path_if_present();
        probe_with_ytdlp(yt_dlp, cookie_path.as_deref(), &url)
    }

    pub fn add_download(
        &self,
        item: MediaProbeItem,
        output_folder: String,
        options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        self.enqueue_download(
            item,
            output_folder,
            options,
            GuidedTransferContext::default(),
            None,
        )
    }

    pub fn add_guided_download(
        &self,
        session: Arc<GuidedExtractionSession>,
        selection: GuidedDownloadSelection,
        output_folder: String,
        mut options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        let resolved = session.resolve_candidate(&selection.candidate_id)?;
        let candidate = resolved.candidate;
        let candidate_url = Url::parse(&candidate.url).map_err(|error| CoreError::InvalidUrl {
            reason: format!("invalid guided media URL: {error}"),
        })?;
        options.user_agent = selection.user_agent;
        let direct_download = direct_download_compatible(candidate.kind, &options);
        options.referer = if direct_download {
            direct_referer_for_target(&candidate.referer, &candidate_url)
        } else {
            origin_referer_for_target(&candidate.referer, &candidate_url)
        };
        if !direct_download && resolved.headers.iter().any(is_sensitive_guided_header) {
            return Err(CoreError::InvalidConfiguration {
                reason: "Authenticated request headers cannot be forwarded safely through yt-dlp across media hosts. Use an original direct download or cookie authentication."
                    .to_string(),
            });
        }
        self.enqueue_download(
            MediaProbeItem {
                webpage_url: candidate.url,
                title: candidate.title,
                thumbnail_url: None,
                extractor: Some("User-guided extraction".to_string()),
                duration_seconds: None,
                playlist_index: None,
            },
            output_folder,
            options,
            GuidedTransferContext {
                direct_download,
                cookie_contents: Some(selection.cookies),
                request_headers: resolved.headers,
                guided_kind: Some(candidate.kind),
            },
            None,
        )
    }
}

fn direct_download_compatible(kind: GuidedMediaKind, options: &MediaDownloadOptions) -> bool {
    let preserves_original = matches!(options.format, MediaFormat::Best | MediaFormat::Original)
        && !options.download_subtitles
        && !options.embed_subtitles
        && !options.write_thumbnail
        && !options.write_metadata;
    match kind {
        GuidedMediaKind::Image
        | GuidedMediaKind::Subtitle
        | GuidedMediaKind::Video
        | GuidedMediaKind::Audio => preserves_original,
        GuidedMediaKind::Stream | GuidedMediaKind::Unknown => false,
    }
}

impl MediaEngine {
    pub(crate) fn new_quiescent(config: MediaEngineConfig) -> Result<Arc<Self>, CoreError> {
        open_media_engine(config, false)
    }

    pub(crate) fn claim_legacy_transfer(
        &self,
        transfer_id: &str,
        download_id: &str,
        attempt: u32,
    ) -> Result<(), CoreError> {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(transfer_id) else {
            return Err(CoreError::Engine {
                reason: format!("legacy media transfer {transfer_id} is missing"),
            });
        };
        if let Some(existing) = item.download_id.as_deref()
            && (existing != download_id || item.attempt != attempt)
        {
            return Err(CoreError::Engine {
                reason: format!(
                    "media transfer {transfer_id} is already bound to download {existing}"
                ),
            });
        }
        item.download_id = Some(download_id.to_string());
        item.attempt = attempt;
        save_downloads(&self.store_path, &items);
        Ok(())
    }

    pub(crate) fn ensure_download(
        &self,
        download_id: &str,
        attempt: u32,
        item: MediaProbeItem,
        output_folder: String,
        options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        self.enqueue_download(
            item,
            output_folder,
            options,
            GuidedTransferContext::default(),
            Some((download_id, attempt)),
        )
    }

    pub(crate) fn ensure_guided_download(
        &self,
        download_id: &str,
        attempt: u32,
        session: Arc<GuidedExtractionSession>,
        selection: GuidedDownloadSelection,
        output_folder: String,
        mut options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        if let Some(existing) = self.transfer_id_for(download_id, attempt) {
            return Ok(existing);
        }
        let resolved = session.resolve_candidate(&selection.candidate_id)?;
        let candidate = resolved.candidate;
        let candidate_url = Url::parse(&candidate.url).map_err(|error| CoreError::InvalidUrl {
            reason: format!("invalid guided media URL: {error}"),
        })?;
        options.user_agent = selection.user_agent;
        let direct_download = direct_download_compatible(candidate.kind, &options);
        options.referer = if direct_download {
            direct_referer_for_target(&candidate.referer, &candidate_url)
        } else {
            origin_referer_for_target(&candidate.referer, &candidate_url)
        };
        if !direct_download && resolved.headers.iter().any(is_sensitive_guided_header) {
            return Err(CoreError::InvalidConfiguration {
                reason: "Authenticated request headers cannot be forwarded safely through yt-dlp across media hosts. Use an original direct download or cookie authentication."
                    .to_string(),
            });
        }
        self.enqueue_download(
            MediaProbeItem {
                webpage_url: candidate.url,
                title: candidate.title,
                thumbnail_url: None,
                extractor: Some("User-guided extraction".to_string()),
                duration_seconds: None,
                playlist_index: None,
            },
            output_folder,
            options,
            GuidedTransferContext {
                direct_download,
                cookie_contents: Some(selection.cookies),
                request_headers: resolved.headers,
                guided_kind: Some(candidate.kind),
            },
            Some((download_id, attempt)),
        )
    }

    pub(crate) fn transfer_id_for(&self, download_id: &str, attempt: u32) -> Option<String> {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .find(|item| {
                item.download_id.as_deref() == Some(download_id) && item.attempt == attempt
            })
            .map(|item| item.id.clone())
    }

    pub(crate) fn pause_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            self.pause(id);
        }
    }

    pub(crate) fn resume_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            let state = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&id)
                .map(|item| item.state);
            match state {
                Some(MediaTransferState::Queued) => spawn_worker(self.worker_context(), id),
                Some(MediaTransferState::Paused) => self.resume(id),
                _ => {}
            }
        }
    }

    pub(crate) fn cancel_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            self.cancel(id);
        }
    }

    pub(crate) fn remove_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            self.remove(id);
        }
    }

    pub(crate) fn stop_all(&self) {
        let ids = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            self.pause(id);
        }
    }

    fn enqueue_download(
        &self,
        item: MediaProbeItem,
        output_folder: String,
        options: MediaDownloadOptions,
        guided: GuidedTransferContext,
        identity: Option<(&str, u32)>,
    ) -> Result<String, CoreError> {
        let GuidedTransferContext {
            direct_download,
            cookie_contents,
            request_headers,
            guided_kind,
        } = guided;
        validate_http_url(&item.webpage_url)?;
        validate_filename_template(&options.filename_template)?;
        validate_output_folder(&output_folder)?;
        if !direct_download
            && self.ffmpeg.is_none()
            && matches!(
                options.format,
                MediaFormat::AudioMp3 | MediaFormat::AudioM4a
            )
        {
            return Err(CoreError::DependencyUnavailable {
                reason: "FFmpeg is required for audio conversion. Install it or select its path in Settings."
                    .to_string(),
            });
        }
        if !direct_download {
            self.require_yt_dlp()?;
        }
        let title = if item.title.trim().is_empty() {
            item.webpage_url.clone()
        } else {
            item.title.clone()
        };
        let naturally_grouped = options.download_subtitles
            || options.embed_subtitles
            || options.write_thumbnail
            || options.write_metadata;
        let destination = resolve_download_destination(
            Path::new(&output_folder),
            &title,
            options.content_layout,
            naturally_grouped,
        );
        fs::create_dir_all(&destination.output_folder).map_err(|error| CoreError::Engine {
            reason: format!("could not create download folder: {error}"),
        })?;
        if let Some((download_id, attempt)) = identity {
            let items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = items.values().find(|existing| {
                existing.download_id.as_deref() == Some(download_id) && existing.attempt == attempt
            }) {
                return Ok(existing.id.clone());
            }
        }
        let id = identity
            .map(|(download_id, attempt)| format!("opul-{download_id}-{attempt}"))
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let cookie_file = self.snapshot_cookie_jar(&id, cookie_contents.as_deref())?;
        let request_header_file = match self.snapshot_request_headers(&id, &request_headers) {
            Ok(path) => path,
            Err(error) => {
                if cookie_file.is_some()
                    && let Some(path) = owned_cookie_path(&self.cookie_directory(), &id)
                {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
        };
        let stored = StoredMediaDownload {
            id: id.clone(),
            download_id: identity.map(|(download_id, _)| download_id.to_string()),
            attempt: identity.map(|(_, attempt)| attempt).unwrap_or(0),
            source_url: item.webpage_url.clone(),
            page_url: options
                .referer
                .clone()
                .unwrap_or_else(|| item.webpage_url.clone()),
            title,
            output_folder: destination.output_folder.to_string_lossy().into_owned(),
            content_folder: destination
                .content_folder
                .map(|path| path.to_string_lossy().into_owned()),
            thumbnail_url: item.thumbnail_url,
            extractor: item.extractor,
            playlist_index: item.playlist_index,
            options,
            direct_download,
            cookie_file,
            request_headers,
            request_header_file,
            guided_kind,
            state: MediaTransferState::Queued,
            progress: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            download_rate: 0,
            eta_seconds: None,
            output_path: None,
            error_message: None,
            created_at: now_millis(),
        };
        {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            items.insert(id.clone(), stored);
            save_downloads(&self.store_path, &items);
        }
        spawn_worker(self.worker_context(), id.clone());
        Ok(id)
    }
}

#[uniffi::export]
impl MediaEngine {
    pub fn snapshots(&self) -> Vec<MediaTransferSnapshot> {
        let items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut values = items.values().collect::<Vec<_>>();
        values.sort_by_key(|item| item.created_at);
        values
            .into_iter()
            .map(StoredMediaDownload::snapshot)
            .collect()
    }

    pub fn pause(&self, id: String) {
        if !self.transition_state(
            &id,
            &[
                MediaTransferState::Queued,
                MediaTransferState::Downloading,
                MediaTransferState::Processing,
            ],
            MediaTransferState::Paused,
            None,
        ) {
            return;
        }
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "STOP");
        }
    }

    pub fn resume(&self, id: String) {
        if let Some(pid) = self.process_id(&id) {
            if self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Downloading,
                None,
            ) {
                signal_process(pid, "CONT");
            }
        } else if self.can_start(&id) {
            // Queued and restored transfers already own a worker waiting at the
            // concurrency gate. Waking that worker avoids starting a duplicate.
            self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Queued,
                None,
            );
        } else {
            self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Failed,
                Some("yt-dlp is required to resume this download".to_string()),
            );
        }
        self.gate.1.notify_all();
    }

    pub fn cancel(&self, id: String) {
        if !self.transition_state(
            &id,
            &[
                MediaTransferState::Queued,
                MediaTransferState::Downloading,
                MediaTransferState::Processing,
                MediaTransferState::Paused,
            ],
            MediaTransferState::Cancelled,
            None,
        ) {
            return;
        }
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "TERM");
        }
        self.gate.1.notify_all();
    }

    pub fn retry(&self, id: String) {
        {
            let items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get(&id) else {
                return;
            };
            if !matches!(
                item.state,
                MediaTransferState::Failed | MediaTransferState::Cancelled
            ) {
                return;
            }
        }
        {
            let mut workers = self
                .workers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if workers.active.contains(&id) {
                workers.retry_requested.insert(id);
                return;
            }
        }
        let should_retry = {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get_mut(&id) else {
                return;
            };
            if !matches!(
                item.state,
                MediaTransferState::Failed | MediaTransferState::Cancelled
            ) {
                return;
            }
            item.state = MediaTransferState::Queued;
            item.error_message = None;
            item.progress = 0.0;
            item.download_rate = 0;
            item.eta_seconds = None;
            save_downloads(&self.store_path, &items);
            true
        };
        if should_retry {
            if self.can_start(&id) {
                spawn_worker(self.worker_context(), id);
            } else {
                self.update_state(
                    &id,
                    MediaTransferState::Failed,
                    Some("yt-dlp is required to retry this download".to_string()),
                );
            }
        }
    }

    pub fn remove(&self, id: String) {
        self.workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retry_requested
            .remove(&id);
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "TERM");
        }
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let removed = items.remove(&id);
        save_downloads(&self.store_path, &items);
        if let Some(item) = removed {
            if item.cookie_file.is_some()
                && let Some(cookie_file) = owned_cookie_path(&self.cookie_directory(), &id)
            {
                let _ = fs::remove_file(cookie_file);
            }
            if item.request_header_file.is_some()
                && let Some(header_file) =
                    owned_request_header_path(&self.request_header_directory(), &id)
            {
                let _ = fs::remove_file(header_file);
            }
        }
        self.gate.1.notify_all();
    }

    pub fn set_max_concurrent_downloads(&self, maximum: u32) {
        let mut gate = self
            .gate
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        gate.maximum = maximum.max(1);
        drop(gate);
        self.gate.1.notify_all();
    }

    pub fn set_max_download_bps(&self, bytes_per_second: u64) {
        *self
            .max_download_bps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = bytes_per_second;
    }

    pub fn set_cookie_jar(&self, contents: String) -> Result<(), CoreError> {
        if contents.len() > 8 * 1024 * 1024 {
            return Err(CoreError::InvalidConfiguration {
                reason: "the browser cookie jar is unexpectedly large".to_string(),
            });
        }
        let trimmed = contents.trim();
        if trimmed.is_empty() {
            if self.cookie_path.exists() {
                fs::remove_file(&self.cookie_path).map_err(|error| CoreError::Engine {
                    reason: format!("could not clear browser cookies: {error}"),
                })?;
            }
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for item in items.values_mut() {
                if item.cookie_file.take().is_some()
                    && let Some(cookie_file) = owned_cookie_path(&self.cookie_directory(), &item.id)
                {
                    let _ = fs::remove_file(cookie_file);
                }
            }
            save_downloads(&self.store_path, &items);
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&self.cookie_path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not store browser cookies: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect browser cookies: {error}"),
            })?;
        file.write_all(trimmed.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not store browser cookies: {error}"),
            })
    }

    pub fn clear_guided_session_data(&self) -> Result<(), CoreError> {
        self.set_cookie_jar(String::new())?;
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for item in items.values_mut() {
            item.request_headers.clear();
            if item.request_header_file.take().is_some()
                && let Some(path) =
                    owned_request_header_path(&self.request_header_directory(), &item.id)
            {
                let _ = fs::remove_file(path);
            }
        }
        save_downloads(&self.store_path, &items);
        Ok(())
    }
}

impl MediaEngine {
    fn require_yt_dlp(&self) -> Result<&PathBuf, CoreError> {
        self.yt_dlp
            .as_ref()
            .ok_or_else(|| CoreError::DependencyUnavailable {
                reason:
                    "yt-dlp was not found. Install it with Homebrew or select its path in Settings."
                        .to_string(),
            })
    }

    fn cookie_path_if_present(&self) -> Option<PathBuf> {
        self.cookie_path.exists().then(|| self.cookie_path.clone())
    }

    fn worker_context(&self) -> WorkerContext {
        WorkerContext {
            yt_dlp: self.yt_dlp.clone(),
            ffmpeg: self.ffmpeg.clone(),
            cookie_directory: self.cookie_directory(),
            request_header_directory: self.request_header_directory(),
            store_path: self.store_path.clone(),
            items: Arc::clone(&self.items),
            processes: Arc::clone(&self.processes),
            workers: Arc::clone(&self.workers),
            gate: Arc::clone(&self.gate),
            max_download_bps: Arc::clone(&self.max_download_bps),
        }
    }

    fn snapshot_cookie_jar(
        &self,
        id: &str,
        supplied_contents: Option<&str>,
    ) -> Result<Option<String>, CoreError> {
        let contents = if let Some(contents) = supplied_contents {
            if contents.len() > 8 * 1024 * 1024 {
                return Err(CoreError::InvalidConfiguration {
                    reason: "the browser cookie jar is unexpectedly large".to_string(),
                });
            }
            if contents.trim().is_empty() {
                return Ok(None);
            }
            contents.as_bytes().to_vec()
        } else {
            let Ok(contents) = fs::read(&self.cookie_path) else {
                return Ok(None);
            };
            contents
        };
        let directory = self.cookie_directory();
        fs::create_dir_all(&directory).map_err(|error| CoreError::Engine {
            reason: format!("could not create the cookie folder: {error}"),
        })?;
        let path = directory.join(format!("{id}.txt"));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download cookies: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect download cookies: {error}"),
            })?;
        file.write_all(&contents)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download cookies: {error}"),
            })?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    fn cookie_directory(&self) -> PathBuf {
        self.cookie_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cookies")
    }

    fn snapshot_request_headers(
        &self,
        id: &str,
        headers: &[GuidedHttpHeader],
    ) -> Result<Option<String>, CoreError> {
        if headers.is_empty() {
            return Ok(None);
        }
        let directory = self.request_header_directory();
        fs::create_dir_all(&directory).map_err(|error| CoreError::Engine {
            reason: format!("could not create the request-header folder: {error}"),
        })?;
        let path = directory.join(format!("{id}.json"));
        let contents = serde_json::to_vec(headers).map_err(|error| CoreError::Engine {
            reason: format!("could not encode download request headers: {error}"),
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download request headers: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .and_then(|_| file.write_all(&contents))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect download request headers: {error}"),
            })?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    fn request_header_directory(&self) -> PathBuf {
        self.cookie_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("request-headers")
    }

    fn restore_pending(&self) {
        let ids = {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut ids = Vec::new();
            for item in items.values_mut() {
                if matches!(
                    item.state,
                    MediaTransferState::Downloading | MediaTransferState::Processing
                ) {
                    item.state = MediaTransferState::Queued;
                }
                if matches!(
                    item.state,
                    MediaTransferState::Queued | MediaTransferState::Paused
                ) && (item.direct_download || self.yt_dlp.is_some())
                {
                    ids.push(item.id.clone());
                }
            }
            save_downloads(&self.store_path, &items);
            ids
        };
        for id in ids {
            spawn_worker(self.worker_context(), id);
        }
    }

    fn can_start(&self, id: &str) -> bool {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .is_some_and(|item| item.direct_download || self.yt_dlp.is_some())
    }

    fn transition_state(
        &self,
        id: &str,
        allowed: &[MediaTransferState],
        state: MediaTransferState,
        error_message: Option<String>,
    ) -> bool {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(id) else {
            return false;
        };
        if !allowed.contains(&item.state) {
            return false;
        }
        item.state = state;
        item.error_message = error_message;
        if state != MediaTransferState::Downloading {
            item.download_rate = 0;
        }
        save_downloads(&self.store_path, &items);
        true
    }

    fn update_state(&self, id: &str, state: MediaTransferState, error_message: Option<String>) {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(id) else {
            return;
        };
        item.state = state;
        item.error_message = error_message;
        if state != MediaTransferState::Downloading {
            item.download_rate = 0;
        }
        save_downloads(&self.store_path, &items);
    }

    fn process_id(&self, id: &str) -> Option<u32> {
        self.processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .copied()
    }
}

impl Drop for MediaEngine {
    fn drop(&mut self) {
        {
            let mut gate = self
                .gate
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            gate.shutdown = true;
        }
        self.gate.1.notify_all();
        let pids = self
            .processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .copied()
            .collect::<Vec<_>>();
        for pid in pids {
            signal_process(pid, "TERM");
        }
    }
}

#[uniffi::export]
pub fn classify_guided_url(url: String, initiator: String) -> GuidedMediaKind {
    classify_guided_resource(&url, &initiator, None)
}

fn classify_guided_resource(
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

fn is_media_mime(value: &str) -> bool {
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

fn probe_with_ytdlp(
    yt_dlp: &Path,
    cookie_path: Option<&Path>,
    url: &str,
) -> Result<MediaProbe, CoreError> {
    let mut command = Command::new(yt_dlp);
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

fn parse_probe(value: &Value, fallback_url: &str) -> Result<MediaProbe, CoreError> {
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

fn probe_item(
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

fn youtube_url(id: Option<&str>, extractor: Option<&str>) -> Option<String> {
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

fn spawn_worker(context: WorkerContext, id: String) {
    {
        let mut workers = context
            .workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !workers.active.insert(id.clone()) {
            return;
        }
    }
    let failure_workers = Arc::clone(&context.workers);
    let failure_items = Arc::clone(&context.items);
    let failure_store_path = context.store_path.clone();
    let failure_id = id.clone();
    let worker_label = id.chars().take(8).collect::<String>();
    let result = thread::Builder::new()
        .name(format!("opul-media-{worker_label}"))
        .spawn(move || {
            run_worker(context.clone(), id.clone());
            let should_retry = {
                let mut workers = context
                    .workers
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                workers.active.remove(&id);
                workers.retry_requested.remove(&id)
            };
            if should_retry {
                restart_download(context, id);
            }
        });
    if result.is_err() {
        failure_workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .active
            .remove(&failure_id);
        let mut items = failure_items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(item) = items.get_mut(&failure_id) {
            item.state = MediaTransferState::Failed;
            item.error_message = Some("could not start a download worker".to_string());
        }
        save_downloads(&failure_store_path, &items);
    }
}

fn restart_download(context: WorkerContext, id: String) {
    let shutdown = context
        .gate
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .shutdown;
    if shutdown {
        return;
    }
    let can_start = {
        let mut items = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(&id) else {
            return;
        };
        let can_start = item.direct_download || context.yt_dlp.is_some();
        item.state = if can_start {
            MediaTransferState::Queued
        } else {
            MediaTransferState::Failed
        };
        item.error_message =
            (!can_start).then(|| "yt-dlp is required to retry this download".to_string());
        item.progress = 0.0;
        item.download_rate = 0;
        item.eta_seconds = None;
        save_downloads(&context.store_path, &items);
        can_start
    };
    if can_start {
        spawn_worker(context, id);
    }
}

fn run_worker(context: WorkerContext, id: String) {
    if !wait_for_slot(&context, &id) {
        return;
    }
    let result = run_download(&context, &id);
    {
        let mut items = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(item) = items.get_mut(&id) {
            if item.state != MediaTransferState::Cancelled {
                match &result {
                    Ok(()) => {
                        item.state = MediaTransferState::Finished;
                        item.progress = 1.0;
                        item.downloaded_bytes = item.downloaded_bytes.max(item.total_bytes);
                        item.error_message = None;
                        remove_transfer_credentials(&context, item);
                    }
                    Err(reason) => {
                        item.state = MediaTransferState::Failed;
                        item.error_message = Some(reason.clone());
                    }
                }
            }
            item.download_rate = 0;
            item.eta_seconds = None;
        }
        save_downloads(&context.store_path, &items);
    }
    context
        .processes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&id);
    let mut gate = context
        .gate
        .0
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    gate.running = gate.running.saturating_sub(1);
    drop(gate);
    context.gate.1.notify_all();
}

fn wait_for_slot(context: &WorkerContext, id: &str) -> bool {
    loop {
        let state = context
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .map(|item| item.state);
        match state {
            None | Some(MediaTransferState::Cancelled | MediaTransferState::Finished) => {
                return false;
            }
            Some(MediaTransferState::Paused) => {
                let gate = context
                    .gate
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let _ = context
                    .gate
                    .1
                    .wait_timeout(gate, Duration::from_millis(250));
            }
            _ => {
                let mut gate = context
                    .gate
                    .0
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if gate.shutdown {
                    return false;
                }
                if gate.running < gate.maximum {
                    gate.running += 1;
                    drop(gate);
                    let mut items = context
                        .items
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    if let Some(item) = items.get_mut(id) {
                        item.state = MediaTransferState::Downloading;
                        item.error_message = None;
                    }
                    save_downloads(&context.store_path, &items);
                    return true;
                }
                let _ = context
                    .gate
                    .1
                    .wait_timeout(gate, Duration::from_millis(250));
            }
        }
    }
}

fn run_download(context: &WorkerContext, id: &str) -> Result<(), String> {
    let mut item = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(id)
        .cloned()
        .ok_or_else(|| "download disappeared from the queue".to_string())?;
    item.request_headers = item_request_headers(context, &item);
    if item.direct_download {
        return run_direct_download(context, id, &item);
    }
    let yt_dlp = context
        .yt_dlp
        .as_ref()
        .ok_or_else(|| "yt-dlp is required for this download".to_string())?;
    let mut command = Command::new(yt_dlp);
    command
        .arg("--newline")
        .arg("--continue")
        .arg("--no-playlist")
        .arg("--progress-template")
        .arg("download:opul-progress\t%(progress.status)s\t%(progress.downloaded_bytes)s\t%(progress.total_bytes_estimate)s\t%(progress.speed)s\t%(progress.eta)s")
        .arg("--progress-template")
        .arg("postprocess:opul-postprocess\t%(progress.status)s")
        .arg("--print")
        .arg("after_move:opul-file\t%(filepath)s")
        .arg("--output")
        .arg(output_template(&item))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_format_args(&mut command, item.options.format, context.ffmpeg.is_some());
    if item.options.download_subtitles {
        command.args(["--write-subs", "--write-auto-subs"]);
        let languages = if item.options.subtitle_languages.is_empty() {
            "all,-live_chat".to_string()
        } else {
            item.options.subtitle_languages.join(",")
        };
        command.arg("--sub-langs").arg(languages);
        if context.ffmpeg.is_some() {
            command.args(["--convert-subs", "srt"]);
            if item.options.embed_subtitles {
                command.arg("--embed-subs");
            }
        }
    }
    if item.options.write_thumbnail {
        command.arg("--write-thumbnail");
        if context.ffmpeg.is_some() {
            command.args(["--convert-thumbnails", "jpg"]);
        }
    }
    if item.options.write_metadata {
        command.arg("--write-info-json");
    }
    if let Some(referer) = item.options.referer.as_deref() {
        command.arg("--referer").arg(referer);
    }
    if let Some(user_agent) = item.options.user_agent.as_deref() {
        command.arg("--user-agent").arg(user_agent);
    }
    for header in &item.request_headers {
        command
            .arg("--add-header")
            .arg(format!("{}:{}", header.name, header.value));
    }
    if let Some(cookie_file) = item_cookie_path(context, &item) {
        command.arg("--cookies").arg(cookie_file);
    }
    if let Some(ffmpeg) = context.ffmpeg.as_ref() {
        command.arg("--ffmpeg-location").arg(ffmpeg);
    }
    let max_download_bps = *context
        .max_download_bps
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if max_download_bps > 0 {
        command
            .arg("--limit-rate")
            .arg(max_download_bps.to_string());
    }
    command.arg("--").arg(&item.source_url);

    command.process_group(0);
    let mut child = command
        .spawn()
        .map_err(|error| format!("could not run yt-dlp: {error}"))?;
    let pid = child.id();
    context
        .processes
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(id.to_string(), pid);
    let state = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(id)
        .map(|item| item.state);
    match state {
        Some(MediaTransferState::Paused) => signal_process(pid, "STOP"),
        None | Some(MediaTransferState::Cancelled) => signal_process(pid, "TERM"),
        _ => {}
    }

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "yt-dlp stdout was unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "yt-dlp stderr was unavailable".to_string())?;
    let stdout_items = Arc::clone(&context.items);
    let output_id = id.to_string();
    let stdout_thread = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            apply_progress_line(&stdout_items, &output_id, &line);
        }
    });
    let stderr_thread = thread::spawn(move || {
        let mut lines = VecDeque::with_capacity(12);
        for line in BufReader::new(stderr).lines().map_while(Result::ok) {
            if lines.len() == 12 {
                lines.pop_front();
            }
            lines.push_back(line);
        }
        lines
    });
    let status = child
        .wait()
        .map_err(|error| format!("could not wait for yt-dlp: {error}"))?;
    let _ = stdout_thread.join();
    let stderr_lines = stderr_thread.join().unwrap_or_default();
    if status.success() {
        Ok(())
    } else {
        let message = stderr_lines
            .into_iter()
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        Err(if message.is_empty() {
            format!("yt-dlp exited with {status}")
        } else {
            message
        })
    }
}

fn run_direct_download(
    context: &WorkerContext,
    id: &str,
    item: &StoredMediaDownload,
) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("could not start the direct downloader: {error}"))?;
    runtime.block_on(download_direct_resource(context, id, item))
}

async fn download_direct_resource(
    context: &WorkerContext,
    id: &str,
    item: &StoredMediaDownload,
) -> Result<(), String> {
    let source = Url::parse(&item.source_url)
        .map_err(|error| format!("invalid direct download URL: {error}"))?;
    let provisional_filename = direct_filename(&source, &item.title);
    let folder = Path::new(&item.output_folder);
    let temporary = folder.join(format!(".{provisional_filename}.{id}.opulpart"));
    let existing_bytes = fs::metadata(&temporary)
        .map(|metadata| metadata.len())
        .unwrap_or(0);

    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .read_timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|error| format!("could not prepare the direct downloader: {error}"))?;
    let cookie_file = item_cookie_path(context, item);
    let (mut response, final_url) = send_direct_request(
        &client,
        &source,
        existing_bytes,
        item,
        cookie_file.as_deref(),
    )
    .await?;
    response
        .error_for_status_ref()
        .map_err(|error| format!("direct download request failed: {error}"))?;
    validate_direct_content_type(&response, &final_url, item.guided_kind)?;
    let filename = direct_response_filename(&response, &final_url, &item.title);
    let resumed = existing_bytes > 0
        && response.status() == reqwest::StatusCode::PARTIAL_CONTENT
        && response
            .headers()
            .get(reqwest::header::CONTENT_RANGE)
            .and_then(|value| value.to_str().ok())
            .is_some_and(|value| value.starts_with(&format!("bytes {existing_bytes}-")));
    let starting_bytes = if resumed { existing_bytes } else { 0 };
    let total_bytes = response
        .content_length()
        .map(|length| length.saturating_add(starting_bytes))
        .unwrap_or(0);
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .append(resumed)
        .truncate(!resumed)
        .open(&temporary)
        .map_err(|error| format!("could not prepare the direct download file: {error}"))?;
    let started = Instant::now();
    let mut downloaded = starting_bytes;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| format!("direct download interrupted: {error}"))?
    {
        loop {
            let state = context
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(id)
                .map(|item| item.state);
            match state {
                Some(MediaTransferState::Paused) => {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                }
                Some(MediaTransferState::Cancelled) | None => {
                    drop(file);
                    let _ = fs::remove_file(&temporary);
                    return Ok(());
                }
                _ => break,
            }
        }
        file.write_all(&chunk)
            .map_err(|error| format!("could not write the direct download: {error}"))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let transferred = downloaded.saturating_sub(starting_bytes);
        let speed = (transferred as f64 / elapsed) as u64;
        update_direct_progress(context, id, downloaded, total_bytes, speed);

        let limit = *context
            .max_download_bps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if limit > 0 {
            let expected = Duration::from_secs_f64(transferred as f64 / limit as f64);
            if expected > started.elapsed() {
                tokio::time::sleep(expected - started.elapsed()).await;
            }
        }
    }
    file.flush()
        .map_err(|error| format!("could not finish the direct download: {error}"))?;
    drop(file);
    let destination = reserve_direct_destination(folder, &filename, &temporary)?;
    let mut items = context
        .items
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(stored) = items.get_mut(id) {
        stored.output_path = Some(destination.to_string_lossy().into_owned());
        stored.total_bytes = downloaded;
        stored.downloaded_bytes = downloaded;
        stored.progress = 1.0;
    }
    Ok(())
}

async fn send_direct_request(
    client: &reqwest::Client,
    source: &Url,
    existing_bytes: u64,
    item: &StoredMediaDownload,
    cookie_file: Option<&Path>,
) -> Result<(reqwest::Response, Url), String> {
    let mut current = source.clone();
    let mut redirect_cookies = Vec::<RedirectCookie>::new();
    for redirects in 0..=10 {
        let mut request = client.get(current.as_str());
        if existing_bytes > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={existing_bytes}-"));
        }
        if let Some(referer) = item
            .options
            .referer
            .as_deref()
            .and_then(|referer| direct_referer_for_target(referer, &current))
        {
            request = request.header(reqwest::header::REFERER, referer);
        }
        if let Some(user_agent) = item.options.user_agent.as_deref() {
            request = request.header(reqwest::header::USER_AGENT, user_agent);
        }
        let same_origin = same_origin(source, &current);
        let authorization_scope = default_cookie_path(source.path());
        for header in &item.request_headers {
            let allowed = match header.name.as_str() {
                "authorization" => {
                    same_origin && cookie_path_matches(&authorization_scope, current.path())
                }
                "origin" => same_origin,
                "accept" | "accept-language" => true,
                _ => false,
            };
            if allowed {
                request = request.header(header.name.as_str(), header.value.as_str());
            }
        }
        let mut cookies = redirect_cookie_header(&redirect_cookies, &current);
        if let Some(cookie_header) = direct_cookie_header(cookie_file, &current) {
            cookies.push(cookie_header);
        }
        if !cookies.is_empty() {
            request = request.header(reqwest::header::COOKIE, cookies.join("; "));
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("direct download request failed: {error}"))?;
        update_redirect_cookies(&mut redirect_cookies, &current, response.headers());
        if !response.status().is_redirection() {
            return Ok((response, current));
        }
        if redirects == 10 {
            return Err("the direct media URL redirected more than 10 times".to_string());
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                "the direct media URL redirected without a valid Location".to_string()
            })?;
        let next = current
            .join(location)
            .map_err(|error| format!("the direct media redirect was invalid: {error}"))?;
        if !matches!(next.scheme(), "http" | "https") {
            return Err("the direct media URL redirected to an unsupported scheme".to_string());
        }
        current = next;
    }
    unreachable!("bounded redirect loop always returns")
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

#[derive(Clone)]
struct RedirectCookie {
    name: String,
    value: String,
    host: String,
    path: String,
    secure: bool,
}

fn update_redirect_cookies(
    cookies: &mut Vec<RedirectCookie>,
    source: &Url,
    headers: &reqwest::header::HeaderMap,
) {
    for value in headers.get_all(reqwest::header::SET_COOKIE) {
        let Some(value) = value.to_str().ok().filter(|value| value.len() <= 8_192) else {
            continue;
        };
        let mut parts = value.split(';');
        let Some((name, cookie_value)) = parts.next().and_then(|pair| pair.split_once('=')) else {
            continue;
        };
        let name = name.trim();
        let cookie_value = cookie_value.trim();
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={} \t".contains(&byte))
            || cookie_value
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte == b';')
        {
            continue;
        }
        let Some(host) = source.host_str().map(str::to_ascii_lowercase) else {
            continue;
        };
        let mut path = default_cookie_path(source.path());
        let mut secure = false;
        let mut remove = false;
        let mut rejected = false;
        for attribute in parts {
            let (attribute_name, attribute_value) = attribute
                .trim()
                .split_once('=')
                .map_or((attribute.trim(), ""), |(name, value)| {
                    (name.trim(), value.trim())
                });
            match attribute_name.to_ascii_lowercase().as_str() {
                // Redirect cookies are intentionally host-only. Correct public-suffix validation
                // for Domain cookies would require a browser cookie implementation.
                "domain" => rejected = true,
                "path" if attribute_value.starts_with('/') => path = attribute_value.to_string(),
                "secure" => secure = true,
                "max-age" => {
                    remove = attribute_value
                        .parse::<i64>()
                        .ok()
                        .is_some_and(|seconds| seconds <= 0)
                }
                _ => {}
            }
        }
        if rejected {
            continue;
        }
        cookies
            .retain(|cookie| !(cookie.name == name && cookie.host == host && cookie.path == path));
        if !remove && cookies.len() < 64 {
            cookies.push(RedirectCookie {
                name: name.to_string(),
                value: cookie_value.to_string(),
                host,
                path,
                secure,
            });
        }
    }
}

fn default_cookie_path(request_path: &str) -> String {
    if !request_path.starts_with('/') || request_path == "/" {
        return "/".to_string();
    }
    request_path
        .rsplit_once('/')
        .map(|(directory, _)| if directory.is_empty() { "/" } else { directory })
        .unwrap_or("/")
        .to_string()
}

fn redirect_cookie_header(cookies: &[RedirectCookie], target: &Url) -> Vec<String> {
    let Some(host) = target.host_str() else {
        return Vec::new();
    };
    let mut length = 0;
    cookies
        .iter()
        .filter(|cookie| {
            cookie.host.eq_ignore_ascii_case(host)
                && cookie_path_matches(&cookie.path, target.path())
                && (!cookie.secure || target.scheme() == "https")
        })
        .map_while(|cookie| {
            let value = format!("{}={}", cookie.name, cookie.value);
            length += value.len() + 2;
            (length <= 32 * 1024).then_some(value)
        })
        .collect()
}

fn cookie_path_matches(cookie_path: &str, request_path: &str) -> bool {
    request_path == cookie_path
        || (request_path.starts_with(cookie_path)
            && (cookie_path.ends_with('/')
                || request_path.as_bytes().get(cookie_path.len()) == Some(&b'/')))
}

fn direct_referer_for_target(referer: &str, target: &Url) -> Option<String> {
    let mut referer = Url::parse(referer).ok()?;
    if !matches!(referer.scheme(), "http" | "https") {
        return None;
    }
    if referer.scheme() == "https" && target.scheme() == "http" {
        return None;
    }
    referer.set_fragment(None);
    let _ = referer.set_username("");
    let _ = referer.set_password(None);
    if same_origin(&referer, target) {
        Some(referer.into())
    } else {
        Some(format!("{}/", referer.origin().ascii_serialization()))
    }
}

fn origin_referer_for_target(referer: &str, target: &Url) -> Option<String> {
    let referer = Url::parse(referer).ok()?;
    if !matches!(referer.scheme(), "http" | "https")
        || (referer.scheme() == "https" && target.scheme() == "http")
    {
        return None;
    }
    Some(format!("{}/", referer.origin().ascii_serialization()))
}

fn validate_direct_content_type(
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

fn direct_response_filename(
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

fn content_disposition_filename(value: &str) -> Option<String> {
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

fn percent_decode_utf8(value: &str) -> Option<String> {
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

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn direct_mime_extension(value: &str) -> Option<&'static str> {
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

fn direct_filename(source: &Url, fallback: &str) -> String {
    let candidate = source
        .path_segments()
        .and_then(|mut segments| segments.next_back())
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback);
    sanitize_direct_filename(candidate)
}

fn sanitize_direct_filename(candidate: &str) -> String {
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

fn reserve_direct_destination(
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

fn direct_cookie_header(cookie_file: Option<&Path>, source: &Url) -> Option<String> {
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

fn item_cookie_path(context: &WorkerContext, item: &StoredMediaDownload) -> Option<PathBuf> {
    item.cookie_file.as_ref()?;
    owned_cookie_path(&context.cookie_directory, &item.id).filter(|path| path.is_file())
}

fn item_request_headers(
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

fn remove_transfer_credentials(context: &WorkerContext, item: &mut StoredMediaDownload) {
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

fn owned_cookie_path(directory: &Path, id: &str) -> Option<PathBuf> {
    let id = Uuid::parse_str(id).ok()?;
    Some(directory.join(format!("{id}.txt")))
}

fn owned_request_header_path(directory: &Path, id: &str) -> Option<PathBuf> {
    let id = Uuid::parse_str(id).ok()?;
    Some(directory.join(format!("{id}.json")))
}

fn update_direct_progress(
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

fn apply_format_args(command: &mut Command, format: MediaFormat, has_ffmpeg: bool) {
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

fn output_template(item: &StoredMediaDownload) -> PathBuf {
    let template = if item.options.filename_template.trim().is_empty() {
        "%(title).180B [%(id)s].%(ext)s"
    } else {
        item.options.filename_template.trim()
    };
    Path::new(&item.output_folder).join(template)
}

fn apply_progress_line(
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
            let mut items = items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get_mut(id) else {
                return;
            };
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
            let mut items = items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(item) = items.get_mut(id)
                && !matches!(
                    item.state,
                    MediaTransferState::Paused | MediaTransferState::Cancelled
                )
            {
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

fn numeric_field(value: Option<&str>) -> Option<u64> {
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

fn validate_http_url(value: &str) -> Result<(), CoreError> {
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

fn validate_filename_template(value: &str) -> Result<(), CoreError> {
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

fn validate_output_folder(value: &str) -> Result<(), CoreError> {
    if value.is_empty() || !Path::new(value).is_absolute() {
        return Err(CoreError::InvalidConfiguration {
            reason: "the download folder must be an absolute path".to_string(),
        });
    }
    Ok(())
}

fn locate_tool(
    configured: Option<&str>,
    resources: Option<&Path>,
    name: &str,
    standard_paths: &[&str],
    version_arguments: &[&str],
) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(configured) = configured.filter(|value| !value.trim().is_empty()) {
        candidates.push(PathBuf::from(configured));
    }
    if let Some(resources) = resources {
        candidates.push(resources.join(name));
    }
    candidates.extend(standard_paths.iter().map(PathBuf::from));
    candidates.push(PathBuf::from(name));
    candidates.into_iter().find(|candidate| {
        Command::new(candidate)
            .args(version_arguments)
            .output()
            .is_ok_and(|output| output.status.success())
    })
}

fn tool_version(path: &Path, arguments: &[&str]) -> Option<String> {
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
        .map(str::to_string)
}

fn signal_process(pid: u32, signal: &str) {
    let _ = Command::new("/bin/kill")
        .arg(format!("-{signal}"))
        .arg("--")
        .arg(format!("-{pid}"))
        .status();
}

fn load_downloads(
    path: &Path,
) -> Result<LoadedState<HashMap<String, StoredMediaDownload>>, String> {
    load_state(path, "media session", validate_downloads)
}

fn save_downloads(path: &Path, items: &HashMap<String, StoredMediaDownload>) {
    if let Err(error) = save_state(path, items, validate_downloads) {
        eprintln!("Warning: could not save media session: {error}");
    }
}

fn validate_downloads(items: &HashMap<String, StoredMediaDownload>) -> Result<(), String> {
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

fn concise_process_error(stderr: &[u8], fallback: &str) -> String {
    let text = String::from_utf8_lossy(stderr);
    text.lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| fallback.to_string())
}

fn json_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)?
        .as_str()
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn json_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(|value| {
        value
            .as_u64()
            .or_else(|| value.as_f64().map(|number| number.max(0.0) as u64))
    })
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        GuidedDownloadSelection, GuidedExtractionSession, GuidedHttpHeader, GuidedMediaKind,
        GuidedObservation, GuidedObservationSource, MediaDownloadOptions, MediaEngine,
        MediaEngineConfig, MediaFormat, MediaTransferState, StoredMediaDownload, apply_format_args,
        apply_progress_line, classify_guided_url, direct_cookie_header, direct_download_compatible,
        direct_filename, direct_referer_for_target, expiry_from_url, load_downloads, numeric_field,
        owned_cookie_path, parse_probe, parse_utc_seconds, save_downloads,
        validate_filename_template, validate_output_folder,
    };
    use crate::{CoreError, DownloadContentLayout};
    use serde_json::json;
    use std::collections::HashMap;
    use std::fs;
    use std::io::{Read, Write as _};
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt as _;
    use std::process::Command;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};
    use url::Url;
    use uuid::Uuid;

    #[test]
    fn classifies_guided_media_by_path_and_initiator() {
        assert_eq!(
            classify_guided_url(
                "https://cdn.example/video/master.m3u8?token=x".into(),
                "fetch".into()
            ),
            GuidedMediaKind::Stream
        );
        assert_eq!(
            classify_guided_url("https://cdn.example/blob".into(), "audio".into()),
            GuidedMediaKind::Audio
        );
        assert_eq!(
            classify_guided_url("https://cdn.example/poster.webp".into(), "img".into()),
            GuidedMediaKind::Image
        );
        assert_eq!(
            classify_guided_url("https://cdn.example/captions".into(), "track".into()),
            GuidedMediaKind::Subtitle
        );
    }

    fn guided_observation(
        url: &str,
        initiator: &str,
        source: GuidedObservationSource,
    ) -> GuidedObservation {
        GuidedObservation {
            url: url.to_string(),
            page_url: "https://app.example/watch".to_string(),
            frame_url: None,
            title: "A film".to_string(),
            initiator: initiator.to_string(),
            mime_type: None,
            status_code: None,
            content_length: None,
            observed_at_ms: 1_700_000_000_000,
            source,
            is_main_frame: true,
            element_key: None,
            width: None,
            height: None,
            headers: Vec::new(),
        }
    }

    #[test]
    fn guided_session_ranks_direct_media_and_stream_manifests() {
        let session = GuidedExtractionSession::new();
        let candidates = session.observe(guided_observation(
            "https://cdn.example/movie.mp4",
            "video",
            GuidedObservationSource::Dom,
        ));
        assert_eq!(candidates[0].kind, GuidedMediaKind::Video);
        assert!(candidates[0].direct_download);

        let candidates = session.observe(guided_observation(
            "https://cdn.example/hls/master.m3u8",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        assert_eq!(candidates[0].kind, GuidedMediaKind::Stream);
        assert!(!candidates[0].direct_download);

        let candidates = session.observe(guided_observation(
            "https://cdn.example/dash/manifest.mpd",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| candidate.kind == GuidedMediaKind::Stream)
                .count(),
            2
        );

        let before_segments = candidates.len();
        let mut segment = guided_observation(
            "https://cdn.example/hls/chunk-12.ts",
            "fetch",
            GuidedObservationSource::Performance,
        );
        segment.mime_type = Some("video/mp2t".to_string());
        let candidates = session.observe(segment);
        assert_eq!(candidates.len(), before_segments);

        let mut dash_segment = guided_observation(
            "https://cdn.example/dash/video/segment-20.m4s",
            "fetch",
            GuidedObservationSource::Performance,
        );
        dash_segment.mime_type = Some("video/iso.segment".to_string());
        let candidates = session.observe(dash_segment);
        assert_eq!(candidates.len(), before_segments);

        let init_segment = guided_observation(
            "https://cdn.example/dash/video/init.mp4",
            "fetch",
            GuidedObservationSource::Performance,
        );
        let candidates = session.observe(init_segment);
        assert_eq!(candidates.len(), before_segments);

        let mut extensionless_segment = guided_observation(
            "https://cdn.example/hls/segment?id=12",
            "fetch",
            GuidedObservationSource::Performance,
        );
        extensionless_segment.mime_type = Some("video/mp2t".to_string());
        let candidates = session.observe(extensionless_segment);
        assert_eq!(candidates.len(), before_segments);
    }

    #[test]
    fn guided_session_preserves_semantic_stream_queries_and_audio_source_types() {
        let session = GuidedExtractionSession::new();
        session.observe(guided_observation(
            "https://cdn.example/live/master.m3u8?channel=one&token=old",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        let candidates = session.observe(guided_observation(
            "https://cdn.example/live/master.m3u8?channel=two&token=new",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| candidate.kind == GuidedMediaKind::Stream)
                .count(),
            2
        );

        let mut audio = guided_observation(
            "https://cdn.example/media/extensionless",
            "audio",
            GuidedObservationSource::Dom,
        );
        audio.mime_type = Some("audio/mp4".to_string());
        let candidates = session.observe(audio);
        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.kind == GuidedMediaKind::Audio)
        );
    }

    #[test]
    fn guided_session_admits_navigation_octet_streams_and_evicts_noise() {
        let session = GuidedExtractionSession::new();
        let images = (0..2_000)
            .map(|index| {
                guided_observation(
                    &format!("https://img.example/{index}.png"),
                    "img",
                    GuidedObservationSource::Performance,
                )
            })
            .collect::<Vec<_>>();
        for batch in images.chunks(256) {
            session.observe_batch(batch.to_vec());
        }
        let candidates = session.observe(guided_observation(
            "https://cdn.example/live/master.m3u8",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        assert_eq!(candidates.len(), 2_000);
        assert_eq!(candidates[0].kind, GuidedMediaKind::Stream);
        assert!(session.limit_reached());

        let octet_session = GuidedExtractionSession::new();
        let mut octet = guided_observation(
            "https://files.example/download/42",
            "navigation-response",
            GuidedObservationSource::Navigation,
        );
        octet.mime_type = Some("application/octet-stream".to_string());
        octet.status_code = Some(200);
        octet.content_length = Some(2_000_000);
        let candidates = octet_session.observe(octet);
        assert_eq!(candidates[0].kind, GuidedMediaKind::Video);
        assert!(candidates[0].direct_download);
    }

    #[test]
    fn guided_batch_processing_is_bounded() {
        let session = GuidedExtractionSession::new();
        let observations = (0..300)
            .map(|index| {
                guided_observation(
                    &format!("https://img.example/{index}.jpg"),
                    "img",
                    GuidedObservationSource::Dom,
                )
            })
            .collect();
        assert_eq!(session.observe_batch(observations).len(), 256);
    }

    #[test]
    fn guided_session_preserves_iframe_popup_and_auth_context() {
        let session = GuidedExtractionSession::new();
        let mut iframe = guided_observation(
            "https://cdn.example/private/video.mp4",
            "navigation",
            GuidedObservationSource::Navigation,
        );
        iframe.frame_url = Some("https://embed.example/player/42".to_string());
        iframe.is_main_frame = false;
        iframe.headers = vec![
            GuidedHttpHeader {
                name: "Authorization".to_string(),
                value: "Bearer token".to_string(),
            },
            GuidedHttpHeader {
                name: "Cookie".to_string(),
                value: "must-not-be-copied".to_string(),
            },
            GuidedHttpHeader {
                name: "Host".to_string(),
                value: "must-not-be-copied".to_string(),
            },
        ];
        session.observe(iframe);
        let mut response = guided_observation(
            "https://cdn.example/private/video.mp4",
            "navigation-response",
            GuidedObservationSource::Navigation,
        );
        response.mime_type = Some("video/mp4".to_string());
        response.status_code = Some(200);
        response.observed_at_ms += 1;
        response.frame_url = Some("https://embed.example/player/42".to_string());
        response.is_main_frame = false;
        response.headers = vec![GuidedHttpHeader {
            name: "Authorization".to_string(),
            value: "Bearer token".to_string(),
        }];
        let candidates = session.observe(response);
        assert_eq!(candidates[0].referer, "https://embed.example/player/42");
        assert_eq!(
            candidates[0].frame_url.as_deref(),
            Some("https://embed.example/player/42")
        );
        let resolved = session.resolve_candidate(&candidates[0].id).unwrap();
        assert_eq!(resolved.headers.len(), 1);
        assert_eq!(resolved.headers[0].name, "authorization");

        let mut other_frame = guided_observation(
            "https://cdn.example/private/video.mp4",
            "navigation",
            GuidedObservationSource::Navigation,
        );
        other_frame.frame_url = Some("https://other.example/player/7".to_string());
        other_frame.is_main_frame = false;
        other_frame.headers = vec![GuidedHttpHeader {
            name: "Authorization".to_string(),
            value: "Bearer other".to_string(),
        }];
        let candidates = session.observe(other_frame);
        assert_eq!(
            candidates
                .iter()
                .filter(|candidate| candidate.url.contains("/private/video.mp4"))
                .count(),
            2
        );

        let mut popup = guided_observation(
            "https://cdn.example/popup-resource",
            "navigation",
            GuidedObservationSource::Popup,
        );
        popup.mime_type = Some("audio/mpeg".to_string());
        let candidates = session.observe(popup);
        assert!(candidates.iter().any(|candidate| {
            candidate.kind == GuidedMediaKind::Audio && candidate.reason.contains("Popup")
        }));
    }

    #[test]
    fn guided_session_refreshes_expiring_manifest_urls_without_duplicates() {
        let session = GuidedExtractionSession::new();
        let first = session.observe(guided_observation(
            "https://cdn.example/live/master.m3u8?token=old&expires=4102444800",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        let first_id = first[0].id.clone();
        let second = session.observe(guided_observation(
            "https://cdn.example/live/master.m3u8?token=new&expires=4102444900",
            "fetch",
            GuidedObservationSource::Performance,
        ));
        assert_eq!(second.len(), 1);
        assert_eq!(second[0].id, first_id);
        assert!(second[0].url.contains("token=new"));
        assert_eq!(second[0].occurrences, 2);
        assert_eq!(second[0].expires_at_ms, Some(4_102_444_900_000));
    }

    #[test]
    fn guided_expiry_supports_common_provider_formats_and_rejects_expired_urls() {
        assert_eq!(
            expiry_from_url(
                &Url::parse(
                    "https://cdn.example/file?X-Amz-Date=20300102T030405Z&X-Amz-Expires=60"
                )
                .unwrap()
            ),
            parse_utc_seconds("20300102T030405Z").map(|value| (value + 60) * 1_000)
        );
        assert_eq!(
            expiry_from_url(
                &Url::parse("https://cdn.example/file?se=2030-01-02T03:04:05Z").unwrap()
            ),
            parse_utc_seconds("2030-01-02T03:04:05Z").map(|value| value * 1_000)
        );
        assert_eq!(
            expiry_from_url(
                &Url::parse("https://cdn.example/file?hdnts=st=1~exp=4102444800~acl=/*").unwrap()
            ),
            Some(4_102_444_800_000)
        );
        assert_eq!(
            expiry_from_url(&Url::parse("https://cdn.example/file?expire=4102444800").unwrap()),
            Some(4_102_444_800_000)
        );
        assert_eq!(parse_utc_seconds("20250230T010203Z"), None);

        let session = GuidedExtractionSession::new();
        let candidate = session
            .observe(guided_observation(
                "https://cdn.example/live/master.m3u8?expires=1",
                "fetch",
                GuidedObservationSource::Performance,
            ))
            .remove(0);
        assert!(candidate.is_expired);
        assert!(matches!(
            session.resolve_candidate(&candidate.id),
            Err(CoreError::InvalidConfiguration { .. })
        ));
    }

    #[test]
    fn authenticated_adaptive_headers_are_not_forwarded_to_yt_dlp() {
        let root = std::env::temp_dir().join(format!("opul-auth-stream-test-{}", Uuid::new_v4()));
        let engine = MediaEngine::new(MediaEngineConfig {
            session_folder: root.join("session").to_string_lossy().into_owned(),
            resources_folder: None,
            yt_dlp_path: Some("/does/not/exist".to_string()),
            ffmpeg_path: Some("/does/not/exist".to_string()),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        })
        .unwrap();
        let session = GuidedExtractionSession::new();
        let mut observation = guided_observation(
            "https://cdn.example/private/master.m3u8",
            "fetch",
            GuidedObservationSource::Performance,
        );
        observation.headers = vec![GuidedHttpHeader {
            name: "Authorization".to_string(),
            value: "Bearer secret".to_string(),
        }];
        let candidate_id = session.observe(observation)[0].id.clone();
        let error = engine
            .add_guided_download(
                session,
                GuidedDownloadSelection {
                    candidate_id,
                    user_agent: None,
                    cookies: String::new(),
                },
                root.to_string_lossy().into_owned(),
                MediaDownloadOptions {
                    format: MediaFormat::Best,
                    content_layout: DownloadContentLayout::Original,
                    download_subtitles: false,
                    subtitle_languages: Vec::new(),
                    embed_subtitles: false,
                    write_thumbnail: false,
                    write_metadata: false,
                    filename_template: "%(title)s.%(ext)s".to_string(),
                    referer: None,
                    user_agent: None,
                },
            )
            .unwrap_err();
        assert!(matches!(error, CoreError::InvalidConfiguration { .. }));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guided_session_merges_duplicate_sources_and_ignores_stale_refreshes() {
        let session = GuidedExtractionSession::new();
        let url = "https://cdn.example/movie.mp4?token=current";
        let first = session.observe(guided_observation(
            url,
            "fetch",
            GuidedObservationSource::Performance,
        ));
        let mut dom = guided_observation(url, "video", GuidedObservationSource::Dom);
        dom.element_key = Some("video-1".to_string());
        dom.observed_at_ms += 10;
        let merged = session.observe(dom);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].id, first[0].id);
        assert_eq!(merged[0].occurrences, 2);
        assert!(merged[0].reason.contains("Dom"));

        let mut refreshed = guided_observation(
            "https://cdn.example/movie.mp4?token=fresh",
            "video",
            GuidedObservationSource::Dom,
        );
        refreshed.element_key = Some("video-1".to_string());
        refreshed.observed_at_ms += 20;
        let refreshed = session.observe(refreshed);
        assert_eq!(refreshed.len(), 1);
        assert!(refreshed[0].url.contains("token=fresh"));

        let mut stale = guided_observation(
            "https://cdn.example/movie.mp4?token=stale",
            "video",
            GuidedObservationSource::Dom,
        );
        stale.element_key = Some("video-1".to_string());
        let stale = session.observe(stale);
        assert_eq!(stale.len(), 1);
        assert!(stale[0].url.contains("token=fresh"));
        assert_eq!(stale[0].occurrences, 4);
    }

    #[test]
    fn parses_single_video_probe() {
        let probe = parse_probe(
            &json!({
                "title": "A film",
                "webpage_url": "https://media.example/watch/1",
                "extractor_key": "Generic",
                "duration": 42
            }),
            "https://media.example/watch/1",
        )
        .unwrap();
        assert!(!probe.is_playlist);
        assert_eq!(probe.items.len(), 1);
        assert_eq!(probe.items[0].title, "A film");
        assert_eq!(probe.items[0].duration_seconds, Some(42));
    }

    #[test]
    fn expands_playlist_probe_into_entries() {
        let probe = parse_probe(
            &json!({
                "title": "Course",
                "webpage_url": "https://media.example/list/1",
                "entries": [
                    {"title": "Part 1", "url": "https://media.example/watch/1"},
                    {"title": "Part 2", "url": "https://media.example/watch/2"}
                ]
            }),
            "https://media.example/list/1",
        )
        .unwrap();
        assert!(probe.is_playlist);
        assert_eq!(probe.items.len(), 2);
        assert_eq!(probe.items[1].playlist_index, Some(2));
    }

    #[test]
    fn parses_progress_numbers_without_panicking_on_na() {
        assert_eq!(numeric_field(Some("1048576")), Some(1_048_576));
        assert_eq!(numeric_field(Some("1250.5")), Some(1_250));
        assert_eq!(numeric_field(Some("NA")), None);
    }

    #[test]
    fn late_progress_cannot_revive_a_cancelled_transfer() {
        let id = Uuid::new_v4().to_string();
        let item = StoredMediaDownload {
            id: id.clone(),
            download_id: None,
            attempt: 0,
            source_url: "https://media.example/file.mp4".to_string(),
            page_url: "https://media.example/watch".to_string(),
            title: "Cancelled".to_string(),
            output_folder: "/tmp".to_string(),
            content_folder: None,
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
            options: MediaDownloadOptions {
                format: MediaFormat::Best,
                content_layout: DownloadContentLayout::Original,
                download_subtitles: false,
                subtitle_languages: Vec::new(),
                embed_subtitles: false,
                write_thumbnail: false,
                write_metadata: false,
                filename_template: "%(title)s.%(ext)s".to_string(),
                referer: None,
                user_agent: None,
            },
            direct_download: false,
            cookie_file: None,
            request_headers: Vec::new(),
            request_header_file: None,
            guided_kind: None,
            state: MediaTransferState::Cancelled,
            progress: 0.25,
            total_bytes: 100,
            downloaded_bytes: 25,
            download_rate: 0,
            eta_seconds: None,
            output_path: None,
            error_message: None,
            created_at: 0,
        };
        let items = Arc::new(Mutex::new(HashMap::from([(id.clone(), item)])));
        apply_progress_line(&items, &id, "opul-progress\tdownloading\t50\t100\t25\t2");
        assert_eq!(
            items.lock().unwrap().get(&id).unwrap().state,
            MediaTransferState::Cancelled
        );
    }

    #[test]
    fn filename_templates_cannot_escape_the_download_folder() {
        assert!(validate_filename_template("%(title)s.%(ext)s").is_ok());
        assert!(validate_filename_template("series/%(title)s.%(ext)s").is_ok());
        assert!(validate_filename_template("../outside/%(title)s").is_err());
        assert!(validate_filename_template("/tmp/%(title)s").is_err());
        assert!(validate_output_folder("/Users/example/Downloads").is_ok());
        assert!(validate_output_folder("relative/downloads").is_err());
    }

    #[test]
    fn formats_fall_back_to_single_file_without_ffmpeg() {
        let mut without_ffmpeg = Command::new("yt-dlp");
        apply_format_args(&mut without_ffmpeg, MediaFormat::Best, false);
        let arguments = without_ffmpeg
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(arguments, ["--format", "best"]);

        let mut with_ffmpeg = Command::new("yt-dlp");
        apply_format_args(&mut with_ffmpeg, MediaFormat::Best, true);
        let arguments = with_ffmpeg
            .get_args()
            .map(|argument| argument.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(arguments, ["--format", "bestvideo*+bestaudio/best"]);
    }

    #[test]
    fn direct_media_only_bypasses_yt_dlp_for_compatible_options() {
        let mut options = MediaDownloadOptions {
            format: MediaFormat::Original,
            content_layout: DownloadContentLayout::Original,
            download_subtitles: false,
            subtitle_languages: Vec::new(),
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: "%(title)s.%(ext)s".to_string(),
            referer: None,
            user_agent: None,
        };
        assert!(direct_download_compatible(GuidedMediaKind::Video, &options));
        options.format = MediaFormat::P1080;
        assert!(!direct_download_compatible(
            GuidedMediaKind::Video,
            &options
        ));
        options.format = MediaFormat::Original;
        options.write_metadata = true;
        assert!(!direct_download_compatible(
            GuidedMediaKind::Audio,
            &options
        ));
        assert!(!direct_download_compatible(
            GuidedMediaKind::Image,
            &options
        ));
        options.write_metadata = false;
        assert!(direct_download_compatible(GuidedMediaKind::Image, &options));
        assert!(!direct_download_compatible(
            GuidedMediaKind::Stream,
            &options
        ));
    }

    #[test]
    fn referrers_follow_strict_origin_when_crossing_origins() {
        let target = Url::parse("https://cdn.example/media/video.mp4").unwrap();
        assert_eq!(
            direct_referer_for_target(
                "https://cdn.example/watch/42?session=page#fragment",
                &target
            )
            .as_deref(),
            Some("https://cdn.example/watch/42?session=page")
        );
        assert_eq!(
            direct_referer_for_target("https://app.example/watch/42?secret=yes", &target)
                .as_deref(),
            Some("https://app.example/")
        );
        let downgrade = Url::parse("http://cdn.example/media/video.mp4").unwrap();
        assert_eq!(
            direct_referer_for_target("https://app.example/watch/42", &downgrade),
            None
        );
    }

    #[test]
    fn direct_download_names_stay_flat() {
        let source = Url::parse("https://cdn.example/media/poster.jpg?signature=abc").unwrap();
        assert_eq!(direct_filename(&source, "ignored"), "poster.jpg");
        let unnamed = Url::parse("https://cdn.example/").unwrap();
        assert_eq!(
            direct_filename(&unnamed, "Course: Lesson 1"),
            "Course_ Lesson 1"
        );
    }

    #[test]
    fn direct_download_only_sends_matching_cookies() {
        let path = std::env::temp_dir().join(format!("opul-cookie-test-{}.txt", Uuid::new_v4()));
        fs::write(
            &path,
            ".example.test\tTRUE\t/media\tTRUE\t0\tsession\tallowed\nother.test\tFALSE\t/\tFALSE\t0\tsecret\tblocked\n",
        )
        .unwrap();
        let source = Url::parse("https://cdn.example.test/media/poster.jpg").unwrap();
        assert_eq!(
            direct_cookie_header(Some(path.as_path()), &source).as_deref(),
            Some("session=allowed")
        );
        let sibling = Url::parse("https://cdn.example.test/mediator/poster.jpg").unwrap();
        assert_eq!(direct_cookie_header(Some(path.as_path()), &sibling), None);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn cookie_paths_are_derived_from_engine_owned_ids() {
        let directory = std::env::temp_dir().join("opul-cookie-path-test");
        assert!(owned_cookie_path(&directory, "../../outside").is_none());
        let id = Uuid::new_v4();
        assert_eq!(
            owned_cookie_path(&directory, &id.to_string()),
            Some(directory.join(format!("{id}.txt")))
        );
    }

    #[test]
    fn guided_hls_and_dash_reach_the_scoped_yt_dlp_handoff() {
        let root = std::env::temp_dir().join(format!("opul-stream-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let arguments_path = root.join("arguments.txt");
        let tool_path = root.join("yt-dlp-fixture");
        fs::write(
            &tool_path,
            format!(
                "#!/bin/sh\nprintf '%s\\n' '---' \"$@\" >> '{}'\nexit 0\n",
                arguments_path.display()
            ),
        )
        .unwrap();
        fs::set_permissions(&tool_path, fs::Permissions::from_mode(0o755)).unwrap();
        let engine = MediaEngine::new(MediaEngineConfig {
            session_folder: root.join("session").to_string_lossy().into_owned(),
            resources_folder: None,
            yt_dlp_path: Some(tool_path.to_string_lossy().into_owned()),
            ffmpeg_path: None,
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        })
        .unwrap();
        fs::write(&arguments_path, "").unwrap();

        let options = || MediaDownloadOptions {
            format: MediaFormat::Best,
            content_layout: DownloadContentLayout::Original,
            download_subtitles: false,
            subtitle_languages: Vec::new(),
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: "%(title)s.%(ext)s".to_string(),
            referer: None,
            user_agent: None,
        };
        let mut ids = Vec::new();
        for url in [
            "https://cdn.example/live/master.m3u8?token=fresh",
            "https://cdn.example/dash/manifest.mpd?token=fresh",
        ] {
            let session = GuidedExtractionSession::new();
            let mut observation =
                guided_observation(url, "fetch", GuidedObservationSource::Performance);
            observation.page_url = "https://cdn.example/watch/42?secret=page".to_string();
            let candidate_id = session.observe(observation)[0].id.clone();
            ids.push(
                engine
                    .add_guided_download(
                        session,
                        GuidedDownloadSelection {
                            candidate_id,
                            user_agent: Some("Opul stream test".to_string()),
                            cookies: "# Netscape HTTP Cookie File\n.example\tTRUE\t/\tTRUE\t0\tsession\tallowed\n"
                                .to_string(),
                        },
                        root.join("downloads").to_string_lossy().into_owned(),
                        options(),
                    )
                    .unwrap(),
            );
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let finished = engine
                .snapshots()
                .iter()
                .filter(|snapshot| ids.contains(&snapshot.id))
                .all(|snapshot| snapshot.state == MediaTransferState::Finished);
            if finished
                && engine
                    .snapshots()
                    .iter()
                    .filter(|item| ids.contains(&item.id))
                    .count()
                    == 2
            {
                break;
            }
            assert!(Instant::now() < deadline, "adaptive handoff timed out");
            thread::sleep(Duration::from_millis(25));
        }
        let arguments = fs::read_to_string(&arguments_path).unwrap();
        assert!(arguments.contains("https://cdn.example/live/master.m3u8?token=fresh"));
        assert!(arguments.contains("https://cdn.example/dash/manifest.mpd?token=fresh"));
        assert!(arguments.contains("--cookies"));
        assert!(arguments.contains("--user-agent\nOpul stream test"));
        assert!(arguments.contains("--referer\nhttps://cdn.example/"));
        assert!(!arguments.contains("https://cdn.example/watch"));
        drop(engine);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn downloads_direct_guided_video_without_yt_dlp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request);
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 11\r\nConnection: close\r\n\r\nvideo-bytes",
                )
                .unwrap();
        });
        let root = std::env::temp_dir().join(format!("opul-direct-test-{}", Uuid::new_v4()));
        let session = root.join("session");
        let downloads = root.join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let engine = MediaEngine::new(MediaEngineConfig {
            session_folder: session.to_string_lossy().into_owned(),
            resources_folder: None,
            yt_dlp_path: Some("/does/not/exist".to_string()),
            ffmpeg_path: Some("/does/not/exist".to_string()),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        })
        .unwrap();
        let extraction = GuidedExtractionSession::new();
        let candidate_id = extraction.observe(guided_observation(
            &format!("http://{address}/movie.mp4"),
            "video",
            GuidedObservationSource::Dom,
        ))[0]
            .id
            .clone();
        let id = engine
            .add_guided_download(
                extraction,
                GuidedDownloadSelection {
                    candidate_id,
                    user_agent: None,
                    cookies: String::new(),
                },
                downloads.to_string_lossy().into_owned(),
                MediaDownloadOptions {
                    format: MediaFormat::Best,
                    content_layout: DownloadContentLayout::Original,
                    download_subtitles: false,
                    subtitle_languages: Vec::new(),
                    embed_subtitles: false,
                    write_thumbnail: false,
                    write_metadata: false,
                    filename_template: "%(title)s.%(ext)s".to_string(),
                    referer: None,
                    user_agent: None,
                },
            )
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let output_path = loop {
            let snapshot = engine
                .snapshots()
                .into_iter()
                .find(|snapshot| snapshot.id == id)
                .unwrap();
            match snapshot.state {
                MediaTransferState::Finished => break snapshot.output_path.unwrap(),
                MediaTransferState::Failed => panic!("{:?}", snapshot.error_message),
                _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                _ => panic!("direct download timed out"),
            }
        };
        assert_eq!(fs::read(output_path).unwrap(), b"video-bytes");
        server.join().unwrap();
        drop(engine);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn guided_direct_download_scopes_authentication_across_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let cross_origin_listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let cross_origin_address = cross_origin_listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut first, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let count = first.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]).to_ascii_lowercase();
            assert!(request.contains("authorization: bearer secret"));
            assert!(request.contains("cookie: session=allowed"));
            assert!(request.contains(&format!("referer: http://{address}/gallery")));
            first
                .write_all(
                    b"HTTP/1.1 302 Found\r\nLocation: /final.jpg\r\nSet-Cookie: redirect=accepted; Path=/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                )
                .unwrap();

            let (mut second, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let count = second.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]).to_ascii_lowercase();
            assert!(request.starts_with("get /final.jpg "));
            assert!(request.contains("authorization: bearer secret"));
            assert!(request.contains("session=allowed"));
            assert!(request.contains("redirect=accepted"));
            assert!(request.contains(&format!("referer: http://{address}/gallery")));
            let response = format!(
                "HTTP/1.1 302 Found\r\nLocation: http://{cross_origin_address}/cross-origin.jpg\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            );
            second.write_all(response.as_bytes()).unwrap();
        });
        let cross_origin_server = thread::spawn(move || {
            let (mut stream, _) = cross_origin_listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let count = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..count]).to_ascii_lowercase();
            assert!(request.starts_with("get /cross-origin.jpg "));
            assert!(!request.contains("authorization:"));
            assert!(!request.contains("/gallery"));
            assert!(request.contains(&format!("referer: http://{address}/")));
            stream
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: image/jpeg\r\nContent-Length: 5\r\nConnection: close\r\n\r\nimage",
                )
                .unwrap();
        });
        let root = std::env::temp_dir().join(format!("opul-redirect-test-{}", Uuid::new_v4()));
        let downloads = root.join("downloads");
        fs::create_dir_all(&downloads).unwrap();
        let engine = MediaEngine::new(MediaEngineConfig {
            session_folder: root.join("session").to_string_lossy().into_owned(),
            resources_folder: None,
            yt_dlp_path: Some("/does/not/exist".to_string()),
            ffmpeg_path: Some("/does/not/exist".to_string()),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        })
        .unwrap();
        let extraction = GuidedExtractionSession::new();
        let mut observation = guided_observation(
            &format!("http://{address}/start"),
            "img",
            GuidedObservationSource::Navigation,
        );
        observation.page_url = format!("http://{address}/gallery");
        observation.mime_type = Some("image/jpeg".to_string());
        observation.headers = vec![GuidedHttpHeader {
            name: "Authorization".to_string(),
            value: "Bearer secret".to_string(),
        }];
        let candidate_id = extraction.observe(observation)[0].id.clone();
        let id = engine
            .add_guided_download(
                extraction,
                GuidedDownloadSelection {
                    candidate_id,
                    user_agent: Some("Opul test".to_string()),
                    cookies: "# Netscape HTTP Cookie File\n127.0.0.1\tFALSE\t/\tFALSE\t0\tsession\tallowed\n".to_string(),
                },
                downloads.to_string_lossy().into_owned(),
                MediaDownloadOptions {
                    format: MediaFormat::Best,
                    content_layout: DownloadContentLayout::Original,
                    download_subtitles: false,
                    subtitle_languages: Vec::new(),
                    embed_subtitles: false,
                    write_thumbnail: false,
                    write_metadata: false,
                    filename_template: "%(title)s.%(ext)s".to_string(),
                    referer: None,
                    user_agent: None,
                },
            )
            .unwrap();
        let header_file = root
            .join("session/request-headers")
            .join(format!("{id}.json"));
        assert_eq!(
            fs::metadata(&header_file).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            fs::metadata(root.join("session/media-downloads.json"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        let deadline = Instant::now() + Duration::from_secs(5);
        let output_path = loop {
            let snapshot = engine
                .snapshots()
                .into_iter()
                .find(|snapshot| snapshot.id == id)
                .unwrap();
            match snapshot.state {
                MediaTransferState::Finished => break snapshot.output_path.unwrap(),
                MediaTransferState::Failed => panic!("{:?}", snapshot.error_message),
                _ if Instant::now() < deadline => thread::sleep(Duration::from_millis(25)),
                _ => panic!("redirected direct download timed out"),
            }
        };
        assert_eq!(fs::read(&output_path).unwrap(), b"image");
        assert!(output_path.ends_with("cross-origin.jpg"));
        server.join().unwrap();
        cross_origin_server.join().unwrap();
        assert!(!header_file.exists());
        assert!(
            !root
                .join("session/cookies")
                .join(format!("{id}.txt"))
                .exists()
        );
        engine.clear_guided_session_data().unwrap();
        assert!(!header_file.exists());
        assert!(
            !root
                .join("session/cookies")
                .join(format!("{id}.txt"))
                .exists()
        );
        drop(engine);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn load_downloads_corrupt_file_preserves_bad_file_and_recovers_from_backup() {
        let dir =
            std::env::temp_dir().join(format!("opul-media-test-corrupt-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("media-downloads.json");
        let backup_path = dir.join("media-downloads.json.bak");

        let id = Uuid::new_v4().to_string();
        let mut good_downloads = HashMap::new();
        good_downloads.insert(
            id.clone(),
            StoredMediaDownload {
                id: id.clone(),
                download_id: None,
                attempt: 0,
                source_url: "https://example.com/video.mp4".to_string(),
                page_url: "https://example.com/watch".to_string(),
                title: "Test Video".to_string(),
                output_folder: "/tmp".to_string(),
                content_folder: None,
                thumbnail_url: None,
                extractor: None,
                playlist_index: None,
                options: MediaDownloadOptions::default(),
                direct_download: true,
                cookie_file: None,
                request_headers: Vec::new(),
                request_header_file: None,
                guided_kind: None,
                state: MediaTransferState::Finished,
                progress: 1.0,
                total_bytes: 1000,
                downloaded_bytes: 1000,
                download_rate: 0,
                eta_seconds: None,
                output_path: Some("/tmp/video.mp4".to_string()),
                error_message: None,
                created_at: 100000,
            },
        );
        fs::write(
            &backup_path,
            serde_json::to_vec_pretty(&good_downloads).unwrap(),
        )
        .unwrap();
        // Write corrupt truncated JSON
        fs::write(&path, b"{\"some-id\": {\"source_url\": ").unwrap();

        let loaded = load_downloads(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items.get(&id).unwrap().title, "Test Video");
        assert!(loaded.recovery_notice.is_some());
        assert_eq!(fs::read(&path).unwrap(), fs::read(&backup_path).unwrap());

        let corrupt_files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .collect();
        assert_eq!(corrupt_files.len(), 1);
        let corrupt_content = fs::read(corrupt_files[0].path()).unwrap();
        assert_eq!(corrupt_content, b"{\"some-id\": {\"source_url\": ");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_downloads_incompatible_schema_preserves_bad_file() {
        let dir = std::env::temp_dir().join(format!(
            "opul-media-test-incompatible-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("media-downloads.json");

        fs::write(&path, br#"{"schemaVersion":2,"items":{}}"#).unwrap();

        let loaded = load_downloads(&path).unwrap();
        assert!(loaded.items.is_empty());
        assert!(loaded.recovery_notice.is_some());

        let corrupt_files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .collect();
        assert_eq!(corrupt_files.len(), 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_downloads_preserves_whole_store_when_one_record_is_invalid() {
        let dir = std::env::temp_dir().join(format!(
            "opul-media-test-invalid-record-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("media-downloads.json");
        let id = Uuid::new_v4();
        let invalid = format!(
            r#"{{"{id}":{{"id":"{id}","source_url":"file:///private/video.mp4","output_folder":"/tmp"}}}}"#
        );
        fs::write(&path, invalid.as_bytes()).unwrap();

        let loaded = load_downloads(&path).unwrap();

        assert!(loaded.items.is_empty());
        assert!(loaded.recovery_notice.is_some());
        assert!(path.exists());
        let corrupt_files = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("corrupt"))
            .count();
        assert_eq!(corrupt_files, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_downloads_atomic_and_keeps_backup() {
        let dir =
            std::env::temp_dir().join(format!("opul-media-test-atomic-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("media-downloads.json");
        let backup_path = dir.join("media-downloads.json.bak");

        let id1 = Uuid::new_v4().to_string();
        let mut map1 = HashMap::new();
        map1.insert(
            id1.clone(),
            StoredMediaDownload {
                id: id1.clone(),
                download_id: None,
                attempt: 0,
                source_url: "https://example.com/1.mp4".to_string(),
                page_url: "https://example.com/1".to_string(),
                title: "Video 1".to_string(),
                output_folder: "/tmp".to_string(),
                content_folder: None,
                thumbnail_url: None,
                extractor: None,
                playlist_index: None,
                options: MediaDownloadOptions::default(),
                direct_download: false,
                cookie_file: None,
                request_headers: Vec::new(),
                request_header_file: None,
                guided_kind: None,
                state: MediaTransferState::Queued,
                progress: 0.0,
                total_bytes: 0,
                downloaded_bytes: 0,
                download_rate: 0,
                eta_seconds: None,
                output_path: None,
                error_message: None,
                created_at: 1,
            },
        );
        save_downloads(&path, &map1);
        assert!(path.exists());
        assert!(!backup_path.exists());

        let id2 = Uuid::new_v4().to_string();
        let mut map2 = HashMap::new();
        map2.insert(
            id2.clone(),
            StoredMediaDownload {
                id: id2.clone(),
                download_id: None,
                attempt: 0,
                source_url: "https://example.com/2.mp4".to_string(),
                page_url: "https://example.com/2".to_string(),
                title: "Video 2".to_string(),
                output_folder: "/tmp".to_string(),
                content_folder: None,
                thumbnail_url: None,
                extractor: None,
                playlist_index: None,
                options: MediaDownloadOptions::default(),
                direct_download: false,
                cookie_file: None,
                request_headers: Vec::new(),
                request_header_file: None,
                guided_kind: None,
                state: MediaTransferState::Queued,
                progress: 0.0,
                total_bytes: 0,
                downloaded_bytes: 0,
                download_rate: 0,
                eta_seconds: None,
                output_path: None,
                error_message: None,
                created_at: 2,
            },
        );
        save_downloads(&path, &map2);
        assert!(path.exists());
        assert!(backup_path.exists());

        let loaded = load_downloads(&path).unwrap();
        assert_eq!(loaded.items.get(&id2).unwrap().title, "Video 2");

        let loaded_bak = load_downloads(&backup_path).unwrap();
        assert_eq!(loaded_bak.items.get(&id1).unwrap().title, "Video 1");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_downloads_tolerates_010_and_unknown_fields() {
        let dir = std::env::temp_dir().join(format!("opul-media-test-010-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("media-downloads.json");

        let id = Uuid::new_v4().to_string();
        // Early 0.1.0 format with missing optional fields and extra unknown fields.
        let json_010 = format!(
            r#"{{
                "{id}": {{
                    "id": "{id}",
                    "source_url": "https://example.com/media.mp4",
                    "output_folder": "/tmp",
                    "title": "Legacy 0.1.0 Media",
                    "options": {{
                        "format": "Original",
                        "download_subtitles": false,
                        "subtitle_languages": [],
                        "embed_subtitles": false,
                        "write_thumbnail": false,
                        "write_metadata": false,
                        "filename_template": "%(title)s.%(ext)s"
                    }},
                    "state": "Finished",
                    "progress": 1.0,
                    "unknown_field_123": "ignore_me"
                }}
            }}"#
        );
        fs::write(&path, json_010.as_bytes()).unwrap();

        let loaded = load_downloads(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        let item = loaded.items.get(&id).unwrap();
        assert_eq!(item.title, "Legacy 0.1.0 Media");
        assert_eq!(item.state, MediaTransferState::Finished);

        let _ = fs::remove_dir_all(&dir);
    }
}
