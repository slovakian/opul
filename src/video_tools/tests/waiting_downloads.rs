use super::*;
use crate::application::{
    MachineDownloadCommand, MachineDownloadState, MachineFormat, MachineMediaItem,
    MachineMediaPolicy, MachineMediaRequest, OpulMachineConfig, OpulStateMachine,
};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;

#[test]
fn slow_page_probe_keeps_direct_downloads_pause_and_shutdown_responsive() {
    exercise_slow_probe(false, false);
}

#[test]
fn removal_stops_probe_preserves_partial_and_direct_output_and_checkpoints_waiting() {
    exercise_slow_probe(true, false);
}

#[test]
fn removal_waits_for_video_writer_exit_before_deleting_tools() {
    exercise_slow_probe(true, true);
}

fn exercise_slow_probe(remove: bool, writer: bool) {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    let started = fixture._temp.path().join("probe-started");
    let interpreter = tools.invocation().unwrap().interpreter;
    let stopped = fixture._temp.path().join("writer-stopped");
    let partial = fixture._temp.path().join("downloads/video.part");
    let program = if writer {
        format!(
            "#!/bin/sh\nprintf partial > '{}'\nprintf started > '{}'\ntrap 'test -e \"$0\" && printf stopped > {}; exit 0' TERM\nsleep 30 &\nwait\n",
            partial.display(),
            started.display(),
            stopped.display()
        )
    } else {
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexec /bin/sleep 30\n",
            started.display()
        )
    };
    std::fs::write(&interpreter, program).unwrap();
    std::fs::set_permissions(&interpreter, std::fs::Permissions::from_mode(0o755)).unwrap();
    let root = fixture._temp.path();
    std::fs::create_dir_all(root.join("downloads")).unwrap();
    let resources = root.join("resources");
    std::fs::create_dir_all(resources.join("media-tools")).unwrap();
    for name in ["ffmpeg", "ffprobe"] {
        let tool = resources.join("media-tools").join(name);
        std::fs::write(&tool, "#!/bin/sh\necho 'ffmpeg version fixture'\n").unwrap();
        std::fs::set_permissions(tool, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let config = OpulMachineConfig {
        application_support_folder: root.join("application").to_string_lossy().into_owned(),
        downloads_folder: root.join("downloads").to_string_lossy().into_owned(),
        torrent: crate::EngineConfig {
            session_folder: root.join("torrents").to_string_lossy().into_owned(),
            listen_port: 0,
            randomize_port: true,
            enable_dht: false,
            max_connections: 32,
            max_download_bps: 0,
            max_upload_bps: 0,
        },
        media: crate::MediaEngineConfig {
            session_folder: root.join("media").to_string_lossy().into_owned(),
            resources_folder: Some(resources.to_string_lossy().into_owned()),
            video_tools: Arc::clone(&tools),
            max_concurrent_downloads: 2,
            max_download_bps: 0,
        },
    };
    let machine = OpulStateMachine::open(config.clone()).unwrap();
    let snapshot = machine.snapshot();
    let workspace = &snapshot.workspaces[0];
    let request = |url: String| MachineMediaRequest {
        request_id: uuid::Uuid::new_v4().to_string(),
        expected_revision: None,
        workspace_id: workspace.id.clone(),
        folder_id: workspace.folders[0].id.clone(),
        format: MachineFormat::AudioM4a,
        policy: MachineMediaPolicy {
            download_subtitles: false,
            subtitle_languages: vec!["en".into()],
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: "%(title)s.%(ext)s".into(),
            user_agent: None,
        },
        items: vec![MachineMediaItem {
            title: url.clone(),
            page_url: url.clone(),
            url,
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
        }],
    };
    let submission = request("https://example.test/slow-page".into());
    let page = if writer {
        machine.add_media(submission)
    } else {
        machine.add_video_page(submission)
    }
    .unwrap()
    .created_download_ids[0]
        .clone();
    let wait = |condition: &dyn Fn() -> bool| {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !condition() {
            assert!(Instant::now() < deadline, "{:?}", machine.snapshot());
            thread::sleep(Duration::from_millis(20));
        }
    };
    wait(&|| started.exists());
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut bytes = [0; 4096];
                    assert!(stream.read(&mut bytes).unwrap() > 0);
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 4\r\nConnection: close\r\n\r\ndata").unwrap();
                    break;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("direct download did not reach server: {error}"),
            }
        }
    });
    let direct = machine
        .add_direct_media(request(format!("http://{address}/file.bin")))
        .unwrap()
        .created_download_ids[0]
        .clone();
    let state = |id: &str| {
        machine.snapshot().workspaces[0]
            .downloads
            .iter()
            .find(|item| item.id == id)
            .unwrap()
            .state
    };
    wait(&|| state(&direct) == MachineDownloadState::Finished);
    server.join().unwrap();
    if remove {
        let partial = root.join("downloads/video.part");
        if !writer {
            std::fs::write(&partial, b"partial").unwrap();
        }
        let interpreter = tools.invocation().unwrap().interpreter;
        machine.remove_video_tools(false).unwrap();
        assert!(!interpreter.exists());
        if writer {
            assert!(stopped.exists(), "writer exited while tools still existed");
        }
        assert_eq!(state(&page), MachineDownloadState::WaitingForVideoTools);
        assert_eq!(state(&direct), MachineDownloadState::Finished);
        let direct_output = machine.snapshot().workspaces[0]
            .downloads
            .iter()
            .find(|item| item.id == direct)
            .unwrap()
            .output_path
            .clone()
            .unwrap();
        assert_eq!(std::fs::read(direct_output).unwrap(), b"data");
        assert_eq!(std::fs::read(&partial).unwrap(), b"partial");
        drop(machine);
        let machine = OpulStateMachine::open(config.clone()).unwrap();
        assert_eq!(
            machine.snapshot().workspaces[0]
                .downloads
                .iter()
                .find(|item| item.id == page)
                .unwrap()
                .state,
            MachineDownloadState::WaitingForVideoTools
        );
        drop(machine);
        tools.execute(VideoToolsOperation::SetUp).unwrap();
        fixture.wait(&tools);
        let completed = root.join("downloads/video.m4a");
        // The resumed fixture consumes the bytes written by the stopped process.
        let resumed = format!(
            "#!/bin/sh\ncase \"$*\" in\n*--dump-single-json*) printf '%s\\n' '{{\"title\":\"Fixture video\",\"webpage_url\":\"https://example.test/video\"}}';;\n*) test \"$(cat '{}')\" = partial || exit 1; printf complete >> '{}'; mv '{}' '{}'; printf 'opul-file\\t%s\\n' '{}' ;;\nesac\n",
            partial.display(),
            partial.display(),
            partial.display(),
            completed.display(),
            completed.display()
        );
        std::fs::write(tools.invocation().unwrap().interpreter, resumed).unwrap();
        let machine = OpulStateMachine::open(config).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while machine.snapshot().workspaces[0]
            .downloads
            .iter()
            .find(|item| item.id == page)
            .unwrap()
            .state
            != MachineDownloadState::Finished
        {
            assert!(Instant::now() < deadline);
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!partial.exists());
        assert_eq!(std::fs::read(completed).unwrap(), b"partialcomplete");
        return;
    }
    machine
        .pause_download(MachineDownloadCommand {
            request_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: None,
            download_id: page.clone(),
        })
        .unwrap();
    assert_eq!(state(&page), MachineDownloadState::Paused);
    let shutdown = Instant::now();
    machine.begin_shutdown().unwrap();
    drop(machine);
    assert!(shutdown.elapsed() < Duration::from_secs(3));
}
