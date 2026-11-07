use super::*;

pub(super) fn open_media_engine(
    config: MediaEngineConfig,
    restore_pending: bool,
) -> Result<Arc<MediaEngine>, CoreError> {
    let session_folder = PathBuf::from(&config.session_folder);
    fs::create_dir_all(&session_folder).map_err(|error| CoreError::Engine {
        reason: format!("could not create the media session folder: {error}"),
    })?;

    let resource_folder = config.resources_folder.as_deref().map(Path::new);
    let built_in_folder = resource_folder.map(|path| path.join("media-tools"));
    let ffmpeg = built_in_folder.as_ref().map(|path| path.join("ffmpeg"));
    let ffprobe_version = built_in_folder
        .as_ref()
        .and_then(|path| tool_version(&path.join("ffprobe"), &["-version"]));
    let ffmpeg_version = ffmpeg
        .as_ref()
        .and_then(|path| tool_version(path, &["-version"]));
    let ffmpeg = ffmpeg.filter(|_| ffmpeg_version.is_some() && ffprobe_version.is_some());
    let store_path = session_folder.join("media-downloads.json");
    let cookie_path = session_folder.join("browser-cookies.txt");
    let loaded = load_downloads(&store_path).map_err(|reason| CoreError::Engine { reason })?;
    let items = Arc::new(Mutex::new(loaded.items));
    let engine = Arc::new(MediaEngine {
        video_tools: config.video_tools,
        developer_override: Mutex::new(std::env::var_os("OPUL_YT_DLP").map(PathBuf::from)),
        ffmpeg,
        ffmpeg_version,
        ffprobe_version,
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

impl MediaEngine {
    pub fn new(config: MediaEngineConfig) -> Result<Arc<Self>, CoreError> {
        open_media_engine(config, true)
    }

    pub fn recovery_notice(&self) -> Option<String> {
        self.recovery_notice.clone()
    }

    pub fn built_in_tool_versions(&self) -> BuiltInToolVersions {
        BuiltInToolVersions {
            ffmpeg: self.ffmpeg_version.clone(),
            ffprobe: self.ffprobe_version.clone(),
        }
    }

    pub fn probe_url(&self, url: String) -> Result<MediaProbe, CoreError> {
        self.probe_url_while(url, || true)
    }

    pub(crate) fn probe_url_while(
        &self,
        url: String,
        keep_running: impl FnMut() -> bool,
    ) -> Result<MediaProbe, CoreError> {
        validate_http_url(&url)?;
        let yt_dlp = self.require_yt_dlp()?;
        let cookie_path = self.cookie_path_if_present();
        probe_with_ytdlp(&yt_dlp, cookie_path.as_deref(), &url, keep_running)
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

pub(super) fn direct_download_compatible(
    kind: GuidedMediaKind,
    options: &MediaDownloadOptions,
) -> bool {
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
