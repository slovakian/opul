use super::*;
use opul_core::{
    EngineConfig, MachineDownloadState, MachineSource, MediaEngineConfig, OpulMachineConfig,
};
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn test_machine(root: &std::path::Path) -> Arc<OpulStateMachine> {
    test_machine_with_tools(root, None)
}

fn test_machine_with_tools(
    root: &std::path::Path,
    tools: Option<Arc<opul_core::video_tools::VideoTools>>,
) -> Arc<OpulStateMachine> {
    OpulStateMachine::open(OpulMachineConfig {
        application_support_folder: root.join("state").to_string_lossy().into_owned(),
        downloads_folder: root.join("downloads").to_string_lossy().into_owned(),
        torrent: EngineConfig {
            session_folder: root
                .join("state/torrent-session")
                .to_string_lossy()
                .into_owned(),
            listen_port: 0,
            randomize_port: true,
            enable_dht: false,
            max_connections: 32,
            max_download_bps: 0,
            max_upload_bps: 0,
        },
        media: MediaEngineConfig {
            session_folder: root
                .join("state/media-session")
                .to_string_lossy()
                .into_owned(),
            resources_folder: None,
            video_tools: tools,
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        },
    })
    .unwrap()
}

#[test]
fn desktop_video_submission_waits_without_probing_and_survives_restart() {
    use opul_core::video_tools::{VideoTools, VideoToolsConfig};
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("downloads")).unwrap();
    let tools = VideoTools::open(VideoToolsConfig {
        root: root.path().join("Tools"),
        opul_version: "0.2.2".into(),
        built_in_tools: root.path().join("resources"),
    })
    .unwrap();
    let machine = test_machine_with_tools(root.path(), Some(tools.clone()));
    let snapshot = machine.snapshot();
    let workspace = &snapshot.workspaces[0];
    let receipt = DesktopCommand::AddUrl {
        workspace_id: workspace.id.clone(),
        folder_id: workspace.folders[0].id.clone(),
        url: "https://example.com/playlist".into(),
        mode: ImportMode::Page,
        format: MachineFormat::AudioM4a,
    }
    .execute(&machine)
    .unwrap();
    assert_eq!(receipt.created_download_ids.len(), 1);
    let id = &receipt.created_download_ids[0];
    let waiting = machine.snapshot();
    let download = &waiting.workspaces[0].downloads[0];
    assert_eq!(&download.id, id);
    assert_eq!(download.state, MachineDownloadState::WaitingForVideoTools);
    assert_eq!(download.format, Some(MachineFormat::AudioM4a));
    assert_eq!(download.folder_id, workspace.folders[0].id);
    assert_eq!(download.source_url, "https://example.com/playlist");
    machine.begin_shutdown().unwrap();
    drop(machine);
    let reopened = test_machine_with_tools(root.path(), Some(tools));
    let saved = reopened.snapshot();
    assert_eq!(saved.workspaces[0].downloads.len(), 1);
    assert_eq!(saved.workspaces[0].downloads[0], *download);
    reopened.begin_shutdown().unwrap();
}

#[test]
fn desktop_command_downloads_a_real_file_without_tools_and_reopens_its_library() {
    let root = std::env::temp_dir().join(format!("opul-bridge-{}", Uuid::new_v4()));
    std::fs::create_dir_all(root.join("downloads")).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(3)))
                        .unwrap();
                    let mut request = [0_u8; 4096];
                    assert!(stream.read(&mut request).unwrap() > 0);
                    stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: 15\r\nConnection: close\r\n\r\nopul-test-bytes").unwrap();
                    break;
                }
                Err(error)
                    if error.kind() == std::io::ErrorKind::WouldBlock
                        && Instant::now() < deadline =>
                {
                    std::thread::sleep(Duration::from_millis(10))
                }
                Err(error) => panic!("HTTP fixture failed: {error}"),
            }
        }
    });
    let machine = test_machine(&root);
    let initial = machine.snapshot();
    assert_eq!(initial.workspaces[0].icon.value, "📁");
    let workspace_id = initial.workspaces[0].id.clone();
    let folder_id = initial.workspaces[0].folders[0].id.clone();
    let receipt = DesktopCommand::AddUrl {
        workspace_id: workspace_id.clone(),
        folder_id,
        url: format!("http://{address}/fixture.bin"),
        mode: ImportMode::Direct,
        format: MachineFormat::Original,
    }
    .execute(&machine)
    .unwrap();
    let id = receipt.created_download_ids[0].clone();
    let deadline = Instant::now() + Duration::from_secs(10);
    let finished = loop {
        let snapshot = machine.snapshot();
        let download = snapshot.workspaces[0]
            .downloads
            .iter()
            .find(|download| download.id == id)
            .unwrap();
        if download.state == MachineDownloadState::Finished {
            break download.clone();
        }
        assert_ne!(
            download.state,
            MachineDownloadState::Failed,
            "{:?}",
            download.error_message
        );
        assert!(
            Instant::now() < deadline,
            "Download timed out in {:?}",
            download.state
        );
        std::thread::sleep(Duration::from_millis(25));
    };
    let output = finished.output_path.unwrap();
    assert_eq!(std::fs::read(&output).unwrap(), b"opul-test-bytes");
    assert_eq!(finished.source, MachineSource::Direct);
    let transfer_checkpoint = std::fs::read(root.join("state/media-session/media-downloads.json"))
        .expect("The transfer must have a durable adapter checkpoint");
    let checkpoint: serde_json::Value = serde_json::from_slice(&transfer_checkpoint).unwrap();
    assert!(checkpoint.to_string().contains(&id));
    let wire = serde_json::to_value(machine.snapshot()).unwrap();
    assert_eq!(wire["workspaces"][0]["downloads"][0]["state"], "Finished");
    machine.begin_shutdown().unwrap();
    drop(machine);
    let machine = test_machine(&root);
    assert_eq!(machine.snapshot().workspaces[0].downloads[0].id, id);
    DesktopCommand::RemoveDownloads {
        download_ids: vec![id],
    }
    .execute(&machine)
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !machine.snapshot().workspaces[0].downloads.is_empty() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(
        std::path::Path::new(&output).is_file(),
        "Removing history must preserve downloaded files"
    );
    machine.begin_shutdown().unwrap();
    drop(machine);
    server.join().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn import_urls_reject_non_download_schemes() {
    assert!(validate_url("https://example.com/file.zip", false).is_ok());
    assert!(validate_url("magnet:?xt=urn:btih:abcd", true).is_ok());
    for url in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "example.com",
        "magnet:?xt=a",
    ] {
        assert!(validate_url(url, false).is_err());
    }
}
