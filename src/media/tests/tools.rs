use super::*;

#[test]
fn built_in_tools_enable_audio_formats_and_require_ffprobe() {
    let root = std::env::temp_dir().join(format!("opul-built-in-audio-test-{}", Uuid::new_v4()));
    let resources = built_in_resources(&root);
    let yt_dlp = root.join("yt-dlp");
    fs::write(&yt_dlp, "#!/bin/sh\necho test-version\n").unwrap();
    fs::set_permissions(&yt_dlp, fs::Permissions::from_mode(0o755)).unwrap();
    let config = MediaEngineConfig {
        session_folder: root.join("session").to_string_lossy().into_owned(),
        resources_folder: Some(resources.to_string_lossy().into_owned()),
        video_tools: crate::video_tools::not_set_up(),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    };
    let engine = MediaEngine::new(config.clone()).unwrap();
    *engine.yt_dlp.developer_override.lock().unwrap() = Some(yt_dlp);
    let status = engine.built_in_tool_versions();
    assert_eq!(status.ffmpeg.as_deref(), Some("test"));
    assert_eq!(status.ffprobe.as_deref(), Some("test"));
    for format in [MediaFormat::AudioMp3, MediaFormat::AudioM4a] {
        let item = super::MediaProbeItem {
            webpage_url: "https://example.com/cc0-tone".into(),
            title: "CC0 tone".into(),
            thumbnail_url: None,
            extractor: None,
            duration_seconds: None,
            playlist_index: None,
        };
        assert!(
            engine
                .add_download(
                    item,
                    root.join("downloads").to_string_lossy().into_owned(),
                    MediaDownloadOptions {
                        format,
                        ..MediaDownloadOptions::default()
                    }
                )
                .is_ok()
        );
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while engine.snapshots().iter().any(|item| {
        !matches!(
            item.state,
            MediaTransferState::Finished | MediaTransferState::Failed
        )
    }) {
        assert!(Instant::now() < deadline, "audio fixture handoff timed out");
        thread::sleep(Duration::from_millis(25));
    }
    drop(engine);
    fs::remove_file(resources.join("media-tools/ffprobe")).unwrap();
    let incomplete = MediaEngine::new(config).unwrap();
    *incomplete.yt_dlp.developer_override.lock().unwrap() = Some(root.join("yt-dlp"));
    assert!(incomplete.built_in_tool_versions().ffprobe.is_none());
    let refused = incomplete.add_download(
        super::MediaProbeItem {
            webpage_url: "https://example.com/cc0-tone".into(),
            title: "CC0 tone".into(),
            thumbnail_url: None,
            extractor: None,
            duration_seconds: None,
            playlist_index: None,
        },
        root.join("downloads").to_string_lossy().into_owned(),
        MediaDownloadOptions {
            format: MediaFormat::AudioMp3,
            ..MediaDownloadOptions::default()
        },
    );
    assert!(matches!(
        refused,
        Err(CoreError::DependencyUnavailable { .. })
    ));
    drop(incomplete);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn built_in_tools_never_fall_back_to_homebrew_or_path() {
    let root = std::env::temp_dir().join(format!("opul-built-in-test-{}", Uuid::new_v4()));
    let engine = MediaEngine::new(MediaEngineConfig {
        session_folder: root.join("session").to_string_lossy().into_owned(),
        resources_folder: Some(
            root.join("missing-resources")
                .to_string_lossy()
                .into_owned(),
        ),
        video_tools: crate::video_tools::not_set_up(),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    assert!(engine.built_in_tool_versions().ffmpeg.is_none());
    assert!(engine.yt_dlp.acquire().is_err());
    fs::remove_dir_all(root).unwrap();
}
