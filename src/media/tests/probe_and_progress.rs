use super::*;

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
        partial_files: Vec::new(),
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
fn progress_lines_remember_the_temporary_files_yt_dlp_writes() {
    let id = "opul-test".to_string();
    let item: StoredMediaDownload = serde_json::from_value(json!({
        "id": id,
        "source_url": "https://example.com/watch?v=1",
    }))
    .unwrap();
    let items = Arc::new(Mutex::new(HashMap::from([(id.clone(), item)])));
    let line = "opul-progress\tdownloading\t50\t100\t25\t2\t/tmp/Film.f137.mp4.part";
    apply_progress_line(&items, &id, line);
    apply_progress_line(&items, &id, line);
    apply_progress_line(
        &items,
        &id,
        "opul-progress\tdownloading\t60\t100\t25\t2\tNA",
    );
    apply_progress_line(&items, &id, "opul-postprocess\tstarted\t/tmp/Film.mp4");
    apply_progress_line(&items, &id, "opul-postprocess\tstarted\tNA");
    let items = items.lock().unwrap();
    let item = items.get(&id).unwrap();
    assert_eq!(
        item.partial_files,
        ["/tmp/Film.f137.mp4.part", "/tmp/Film.mp4"]
    );
    assert_eq!(item.downloaded_bytes, 60);
    assert_eq!(item.state, MediaTransferState::Processing);
}

#[test]
fn yt_dlp_partials_include_resume_records_and_fragments() {
    let dir = std::env::temp_dir().join(format!("opul-partials-{}", Uuid::new_v4()));
    fs::create_dir_all(&dir).unwrap();
    let temporary = dir.join("Film.mp4.part");
    for name in [
        "Film.mp4.part-Frag1",
        "Film.mp4.part-Frag2",
        "Other.mp4.part-Frag1",
    ] {
        fs::write(dir.join(name), b"").unwrap();
    }
    let mut paths = yt_dlp_partial_paths(&temporary);
    paths.sort();
    let mut expected = vec![
        dir.join("Film.mp4"),
        dir.join("Film.mp4.part"),
        dir.join("Film.mp4.part-Frag1"),
        dir.join("Film.mp4.part-Frag2"),
        dir.join("Film.mp4.ytdl"),
        dir.join("Film.temp.mp4"),
    ];
    expected.sort();
    assert_eq!(paths, expected);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_postprocessed_file_includes_the_copy_ffmpeg_was_writing() {
    // A merge reports the merged file, which ffmpeg writes as a temporary sibling first.
    assert_eq!(
        yt_dlp_partial_paths(Path::new("/tmp/Downloads/Film [x].mkv")),
        [
            PathBuf::from("/tmp/Downloads/Film [x].mkv"),
            PathBuf::from("/tmp/Downloads/Film [x].temp.mkv"),
        ]
    );
    assert_eq!(
        yt_dlp_partial_paths(Path::new("/tmp/Downloads/Film")),
        [
            PathBuf::from("/tmp/Downloads/Film"),
            PathBuf::from("/tmp/Downloads/Film.temp"),
        ]
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
        direct_referer_for_target("https://app.example/watch/42?secret=yes", &target).as_deref(),
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
fn metadata_probe_cancellation_kills_the_process_and_preserves_error_text() {
    let root = tempfile::tempdir().unwrap();
    let tool = root.path().join("probe");
    fs::write(&tool, "#!/bin/sh\nexec /bin/sleep 30\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let command = crate::media::types::YtDlpCommand::DeveloperOverride(tool.clone());
    let started = Instant::now();
    let result =
        crate::media::probe::probe_with_ytdlp(&command, None, "https://example.com/page", || {
            started.elapsed() < Duration::from_millis(200)
        });
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("inspection stopped")
    );
    assert!(started.elapsed() < Duration::from_secs(3));
    fs::write(
        &tool,
        "#!/bin/sh\nprintf '%s' 'The site says: access refused' >&2\nexit 1\n",
    )
    .unwrap();
    let error =
        crate::media::probe::probe_with_ytdlp(&command, None, "https://example.com/page", || true)
            .unwrap_err();
    assert!(error.to_string().contains("The site says: access refused"));
}

#[test]
fn metadata_probe_does_not_wait_on_pipes_inherited_by_descendants() {
    let root = tempfile::tempdir().unwrap();
    let tool = root.path().join("probe");
    fs::write(&tool, "#!/bin/sh\n/bin/sleep 30 &\nexit 0\n").unwrap();
    fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
    let command = crate::media::types::YtDlpCommand::DeveloperOverride(tool);
    let started = Instant::now();
    let result =
        crate::media::probe::probe_with_ytdlp(&command, None, "https://example.com/page", || true);
    assert!(result.unwrap_err().to_string().contains("invalid metadata"));
    assert!(started.elapsed() < Duration::from_secs(3));
}
