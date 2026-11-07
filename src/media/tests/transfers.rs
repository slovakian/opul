use super::*;

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
        resources_folder: Some(built_in_resources(&root).to_string_lossy().into_owned()),
        video_tools: None,
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    *engine.developer_override.lock().unwrap() = Some(tool_path);
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
    assert!(arguments.contains(&format!(
        "--ffmpeg-location\n{}",
        root.join("resources/media-tools").display()
    )));
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
        video_tools: None,
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
fn an_interrupted_direct_download_reports_its_partial_file() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (release, released) = std::sync::mpsc::channel::<()>();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0_u8; 4096];
        let _ = stream.read(&mut request).unwrap();
        stream
            .write_all(
                b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 1000\r\nConnection: close\r\n\r\npartial",
            )
            .unwrap();
        stream.flush().unwrap();
        let _ = released.recv_timeout(Duration::from_secs(5));
    });
    let root = std::env::temp_dir().join(format!("opul-partial-test-{}", Uuid::new_v4()));
    let downloads = root.join("downloads");
    fs::create_dir_all(&downloads).unwrap();
    let engine = MediaEngine::new(MediaEngineConfig {
        session_folder: root.join("session").to_string_lossy().into_owned(),
        resources_folder: None,
        video_tools: None,
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
    engine
        .ensure_guided_download(
            "download-1",
            0,
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
    let partial = loop {
        let partial = engine
            .owned_paths("download-1")
            .into_iter()
            .find(|path| path.exists() && path.to_string_lossy().ends_with(".opulpart"));
        if let Some(partial) = partial
            && fs::read(&partial).unwrap() == b"partial"
        {
            break partial;
        }
        assert!(Instant::now() < deadline, "partial file never appeared");
        thread::sleep(Duration::from_millis(25));
    };
    assert!(partial.starts_with(&downloads));
    assert!(engine.owned_paths("another-download").is_empty());
    engine.remove_download("download-1", 0);
    assert!(engine.owned_paths("download-1").is_empty());
    let _ = release.send(());
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
        video_tools: None,
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
                cookies:
                    "# Netscape HTTP Cookie File\n127.0.0.1\tFALSE\t/\tFALSE\t0\tsession\tallowed\n"
                        .to_string(),
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
