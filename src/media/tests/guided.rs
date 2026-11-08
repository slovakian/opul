use super::*;

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
            &Url::parse("https://cdn.example/file?X-Amz-Date=20300102T030405Z&X-Amz-Expires=60")
                .unwrap()
        ),
        parse_utc_seconds("20300102T030405Z").map(|value| (value + 60) * 1_000)
    );
    assert_eq!(
        expiry_from_url(&Url::parse("https://cdn.example/file?se=2030-01-02T03:04:05Z").unwrap()),
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
        video_tools: crate::video_tools::not_set_up(),
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
