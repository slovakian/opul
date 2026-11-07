use super::*;
use crate::application::{
    MachineDownloadCommand, MachineDownloadState, MachineFormat, MachineMediaItem,
    MachineMediaPolicy, MachineMediaRequest, OpulMachineConfig, OpulStateMachine,
};
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;

#[test]
fn slow_page_probe_keeps_direct_downloads_pause_and_shutdown_responsive() {
    let fixture = Fixture::new(GOOD);
    let tools = fixture.open();
    tools.execute(VideoToolsOperation::SetUp).unwrap();
    assert!(matches!(
        fixture.wait(&tools).state,
        VideoToolsState::Ready { .. }
    ));
    let started = fixture._temp.path().join("probe-started");
    let interpreter = tools.invocation().unwrap().interpreter;
    std::fs::write(
        &interpreter,
        format!(
            "#!/bin/sh\nprintf started > '{}'\nexec /bin/sleep 30\n",
            started.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&interpreter, std::fs::Permissions::from_mode(0o755)).unwrap();
    let root = fixture._temp.path();
    std::fs::create_dir_all(root.join("downloads")).unwrap();
    let machine = OpulStateMachine::open(OpulMachineConfig {
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
            resources_folder: None,
            video_tools: Some(tools),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        },
    })
    .unwrap();
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
    let page = machine
        .add_video_page(request("https://example.test/slow-page".into()))
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
