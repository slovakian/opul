use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

pub(super) fn default_filename_template() -> String {
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

#[derive(Debug, Clone)]
pub struct MediaEngineConfig {
    pub session_folder: String,
    pub resources_folder: Option<String>,
    pub video_tools: Option<Arc<VideoTools>>,
    pub max_concurrent_downloads: u32,
    pub max_download_bps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInToolVersions {
    pub ffmpeg: Option<String>,
    pub ffprobe: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) enum YtDlpCommand {
    Managed(VideoToolsInvocation),
    DeveloperOverride(PathBuf),
}

impl YtDlpCommand {
    pub(super) fn command(&self) -> Command {
        match self {
            Self::Managed(invocation) => invocation.command(),
            Self::DeveloperOverride(path) => Command::new(path),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaProbeItem {
    pub webpage_url: String,
    pub title: String,
    pub thumbnail_url: Option<String>,
    pub extractor: Option<String>,
    pub duration_seconds: Option<u64>,
    pub playlist_index: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct MediaProbe {
    pub title: String,
    pub webpage_url: String,
    pub extractor: Option<String>,
    pub thumbnail_url: Option<String>,
    pub is_playlist: bool,
    pub items: Vec<MediaProbeItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GuidedMediaKind {
    Video,
    Audio,
    Stream,
    Subtitle,
    Image,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuidedObservationSource {
    Dom,
    Performance,
    Navigation,
    Popup,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuidedHttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct GuidedDownloadSelection {
    pub candidate_id: String,
    pub user_agent: Option<String>,
    pub cookies: String,
}

#[derive(Clone)]
pub(super) struct GuidedCandidateEntry {
    pub(super) candidate: GuidedCandidate,
    pub(super) headers: Vec<GuidedHttpHeader>,
}

#[derive(Default)]
pub(super) struct GuidedExtractionState {
    pub(super) candidates: HashMap<String, GuidedCandidateEntry>,
    pub(super) aliases: HashMap<String, String>,
    pub(super) limit_reached: bool,
}

pub struct GuidedExtractionSession {
    pub(super) state: Mutex<GuidedExtractionState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct StoredMediaDownload {
    pub(super) id: String,
    #[serde(default)]
    pub(super) download_id: Option<String>,
    #[serde(default)]
    pub(super) attempt: u32,
    pub(super) source_url: String,
    #[serde(default)]
    pub(super) page_url: String,
    #[serde(default)]
    pub(super) title: String,
    #[serde(default)]
    pub(super) output_folder: String,
    #[serde(default)]
    pub(super) content_folder: Option<String>,
    pub(super) thumbnail_url: Option<String>,
    #[serde(default)]
    pub(super) extractor: Option<String>,
    #[serde(default)]
    pub(super) playlist_index: Option<u32>,
    #[serde(default)]
    pub(super) options: MediaDownloadOptions,
    #[serde(default)]
    pub(super) direct_download: bool,
    #[serde(default)]
    pub(super) cookie_file: Option<String>,
    #[serde(default, skip_serializing)]
    pub(super) request_headers: Vec<GuidedHttpHeader>,
    #[serde(default)]
    pub(super) request_header_file: Option<String>,
    #[serde(default)]
    pub(super) guided_kind: Option<GuidedMediaKind>,
    #[serde(default)]
    pub(super) state: MediaTransferState,
    #[serde(default)]
    pub(super) progress: f64,
    #[serde(default)]
    pub(super) total_bytes: u64,
    #[serde(default)]
    pub(super) downloaded_bytes: u64,
    #[serde(default)]
    pub(super) download_rate: u64,
    #[serde(default)]
    pub(super) eta_seconds: Option<u64>,
    #[serde(default)]
    pub(super) output_path: Option<String>,
    /// Files yt-dlp reported writing, temporary downloads and the files its postprocessors
    /// rewrite, so removal can find what it left behind.
    #[serde(default)]
    pub(super) partial_files: Vec<String>,
    #[serde(default)]
    pub(super) error_message: Option<String>,
    #[serde(default)]
    pub(super) created_at: u64,
}

#[derive(Default)]
pub(super) struct GuidedTransferContext {
    pub(super) direct_download: bool,
    pub(super) cookie_contents: Option<String>,
    pub(super) request_headers: Vec<GuidedHttpHeader>,
    pub(super) guided_kind: Option<GuidedMediaKind>,
}

impl StoredMediaDownload {
    pub(super) fn snapshot(&self) -> MediaTransferSnapshot {
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
pub(super) struct Gate {
    pub(super) running: u32,
    pub(super) maximum: u32,
    pub(super) shutdown: bool,
}

#[derive(Debug, Default)]
pub(super) struct WorkerRegistry {
    pub(super) active: std::collections::HashSet<String>,
    pub(super) retry_requested: std::collections::HashSet<String>,
}

#[derive(Clone)]
pub(super) struct WorkerContext {
    pub(super) video_tools: Option<Arc<VideoTools>>,
    pub(super) yt_dlp: Option<YtDlpCommand>,
    pub(super) ffmpeg: Option<PathBuf>,
    pub(super) cookie_directory: PathBuf,
    pub(super) request_header_directory: PathBuf,
    pub(super) store_path: PathBuf,
    pub(super) items: Arc<Mutex<HashMap<String, StoredMediaDownload>>>,
    pub(super) processes: Arc<Mutex<HashMap<String, u32>>>,
    pub(super) workers: Arc<Mutex<WorkerRegistry>>,
    pub(super) gate: Arc<(Mutex<Gate>, Condvar)>,
    pub(super) max_download_bps: Arc<Mutex<u64>>,
}

pub struct MediaEngine {
    pub(super) video_tools: Option<Arc<VideoTools>>,
    pub(super) developer_override: Mutex<Option<PathBuf>>,
    pub(super) ffmpeg: Option<PathBuf>,
    pub(super) ffmpeg_version: Option<String>,
    pub(super) ffprobe_version: Option<String>,
    pub(super) cookie_path: PathBuf,
    pub(super) store_path: PathBuf,
    pub(super) items: Arc<Mutex<HashMap<String, StoredMediaDownload>>>,
    pub(super) processes: Arc<Mutex<HashMap<String, u32>>>,
    pub(super) workers: Arc<Mutex<WorkerRegistry>>,
    pub(super) gate: Arc<(Mutex<Gate>, Condvar)>,
    pub(super) max_download_bps: Arc<Mutex<u64>>,
    pub(super) recovery_notice: Option<String>,
}
