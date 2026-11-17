use super::*;

#[test]
fn resume_during_worker_exit_hands_queued_intent_to_a_new_writer() {
    let root = tempfile::tempdir().unwrap();
    let downloads = root.path().join("downloads");
    fs::create_dir(&downloads).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(4);
        let mut requests = 0;
        while requests < 2 && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                thread::sleep(Duration::from_millis(5));
                continue;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 2048];
            stream.read(&mut request).unwrap();
            let response: &[u8] = if requests == 0 {
                b"HTTP/1.1 503 Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            } else {
                b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 7\r\nConnection: close\r\n\r\nresumed"
            };
            stream.write_all(response).unwrap();
            requests += 1;
        }
        requests
    });
    let engine = MediaEngine::new(MediaEngineConfig {
        session_folder: root.path().join("session").to_string_lossy().into_owned(),
        resources_folder: None,
        video_tools: crate::video_tools::not_set_up(),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    let id = engine
        .ensure_download(
            &Uuid::new_v4().to_string(),
            0,
            true,
            MediaProbeItem {
                webpage_url: format!("http://{address}/resume.mp4"),
                title: "resume".into(),
                thumbnail_url: None,
                extractor: None,
                duration_seconds: None,
                playlist_index: None,
            },
            downloads.to_string_lossy().into_owned(),
            MediaDownloadOptions::default(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while !engine.workers.lock().unwrap().active.is_empty() {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    // Freeze the exact boundary after a paused worker's outcome was saved but before
    // its registration is released. Resume must not lose intent across that boundary.
    engine.items.lock().unwrap().get_mut(&id).unwrap().state = MediaTransferState::Paused;
    engine.workers.lock().unwrap().active.insert(id.clone());
    engine.resume(id.clone());
    super::super::finish_worker(engine.worker_context(), id.clone());
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let snapshot = engine.snapshots().pop().unwrap();
        if snapshot.state == MediaTransferState::Finished {
            assert_eq!(fs::read(snapshot.output_path.unwrap()).unwrap(), b"resumed");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Resume lost its writer: {:?}",
            snapshot.state
        );
        thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(server.join().unwrap(), 2);
}

#[test]
fn shutdown_interrupts_a_stalled_http_body_and_joins_its_writer() {
    let root = tempfile::tempdir().unwrap();
    let downloads = root.path().join("downloads");
    fs::create_dir(&downloads).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let (sent, received) = std::sync::mpsc::channel();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut request = [0_u8; 2048];
        stream.read(&mut request).unwrap();
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: video/mp4\r\nContent-Length: 1000000\r\n\r\npartial").unwrap();
        sent.send(()).unwrap();
        // No more body arrives. A normal shutdown must close the socket rather than
        // waiting for the downloader's sixty-second read timeout.
        let mut byte = [0];
        assert_eq!(stream.read(&mut byte).unwrap(), 0);
    });
    let engine = MediaEngine::new(MediaEngineConfig {
        session_folder: root.path().join("session").to_string_lossy().into_owned(),
        resources_folder: None,
        video_tools: crate::video_tools::not_set_up(),
        max_concurrent_downloads: 1,
        max_download_bps: 0,
    })
    .unwrap();
    engine
        .ensure_download(
            &Uuid::new_v4().to_string(),
            0,
            true,
            MediaProbeItem {
                webpage_url: format!("http://{address}/stalled.mp4"),
                title: "stalled".into(),
                thumbnail_url: None,
                extractor: None,
                duration_seconds: None,
                playlist_index: None,
            },
            downloads.to_string_lossy().into_owned(),
            MediaDownloadOptions::default(),
        )
        .unwrap();
    received.recv_timeout(Duration::from_secs(5)).unwrap();
    let started = Instant::now();
    engine.shut_down().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    let workers = engine.workers.lock().unwrap();
    assert!(workers.active.is_empty());
    assert!(workers.handles.is_empty());
    drop(workers);
    assert_eq!(engine.snapshots()[0].state, MediaTransferState::Paused);
    server.join().unwrap();
}
