use super::*;
use crate::bencode::BValue;
use std::collections::BTreeMap;
use std::time::Instant;

#[derive(PartialEq, Eq)]
enum Layout {
    Single,
    NestedFile,
    FlatCollection,
}

#[test]
fn completed_single_file_torrent_is_finished_with_its_output() {
    check_output(Layout::Single);
}

#[test]
fn flattened_one_file_collection_uses_the_actual_nested_file() {
    check_output(Layout::NestedFile);
}

#[test]
fn flattened_collection_never_claims_a_nonexistent_collection_folder() {
    check_output(Layout::FlatCollection);
}

fn check_output(layout: Layout) {
    let root = tempfile::tempdir().unwrap();
    let downloads = root.path().join("downloads");
    let payload = b"Opul CC0 completed torrent fixture\n";
    let paths = match layout {
        Layout::Single => vec!["fixture.bin"],
        Layout::NestedFile => vec!["nested/fixture.bin"],
        Layout::FlatCollection => vec!["a.bin", "b.bin"],
    };
    for path in &paths {
        let file = downloads.join(path);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(file, payload).unwrap();
    }
    let machine = OpulStateMachine::open(OpulMachineConfig {
        application_support_folder: root.path().join("app").to_string_lossy().into_owned(),
        downloads_folder: downloads.to_string_lossy().into_owned(),
        torrent: EngineConfig {
            session_folder: root.path().join("torrent").to_string_lossy().into_owned(),
            listen_port: 0,
            randomize_port: true,
            enable_dht: false,
            max_connections: 8,
            max_download_bps: 0,
            max_upload_bps: 0,
        },
        media: MediaEngineConfig {
            session_folder: root.path().join("media").to_string_lossy().into_owned(),
            resources_folder: None,
            video_tools: crate::video_tools::not_set_up(),
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        },
    })
    .unwrap();
    let snapshot = machine.snapshot();
    let workspace = &snapshot.workspaces[0];
    machine
        .update_workspace(MachineWorkspaceRequest {
            request_id: Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id: workspace.id.clone(),
            name: workspace.name.clone(),
            icon: workspace.icon.clone(),
            appearance: workspace.appearance.clone(),
            folder_id: workspace.folders[0].id.clone(),
            folder_name: workspace.folders[0].name.clone(),
            folder_path: workspace.folders[0].path.clone(),
            default_format: workspace.default_format,
            file_organization: MachineFileOrganization::KeepInSaveLocation,
        })
        .unwrap();
    let all_bytes = payload.repeat(paths.len());
    let mut info = BTreeMap::from([
        (
            b"name".to_vec(),
            BValue::Bytes(if layout == Layout::Single {
                b"fixture.bin".to_vec()
            } else {
                b"Collection".to_vec()
            }),
        ),
        (b"piece length".to_vec(), BValue::Int(16384)),
        (
            b"pieces".to_vec(),
            BValue::Bytes(Sha1::digest(&all_bytes).to_vec()),
        ),
    ]);
    if layout == Layout::Single {
        info.insert(b"length".to_vec(), BValue::Int(payload.len() as i64));
    } else {
        info.insert(
            b"files".to_vec(),
            BValue::List(
                paths
                    .iter()
                    .map(|path| {
                        BValue::Dict(BTreeMap::from([
                            (b"length".to_vec(), BValue::Int(payload.len() as i64)),
                            (
                                b"path".to_vec(),
                                BValue::List(
                                    path.split('/')
                                        .map(|part| BValue::Bytes(part.as_bytes().to_vec()))
                                        .collect(),
                                ),
                            ),
                        ]))
                    })
                    .collect(),
            ),
        );
    }
    let id = machine
        .add_torrent(MachineTorrentRequest {
            request_id: Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id: workspace.id.clone(),
            folder_id: workspace.folders[0].id.clone(),
            source: "fixture.torrent".into(),
            metainfo: Some(crate::meta::encode_meta_for_test(info, None)),
            seed_ratio_limit: Some(0.0),
        })
        .unwrap()
        .created_download_ids[0]
        .clone();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let snapshot = machine.snapshot();
        let download = snapshot.workspaces[0]
            .downloads
            .iter()
            .find(|d| d.id == id)
            .unwrap();
        if layout == Layout::FlatCollection && download.progress_millionths == 1_000_000 {
            if let Some(path) = &download.output_path {
                assert!(
                    Path::new(path).exists(),
                    "claimed nonexistent output: {path}"
                );
                assert_ne!(
                    Path::new(path),
                    downloads,
                    "must not claim the entire save location"
                );
            }
            break;
        }
        // A zero ratio limit stops seeding at once: complete, but not seeding.
        if layout != Layout::FlatCollection && download.state == MachineDownloadState::NotSeeding {
            let output = downloads.join(paths[0]);
            assert_eq!(download.output_path.as_deref(), output.to_str());
            assert_eq!(fs::read(&output).unwrap(), payload);
            assert!(download.capabilities.can_start_seeding);
            assert!(!download.capabilities.can_cancel);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Completed torrent remained {:?}",
            download.state
        );
        thread::sleep(Duration::from_millis(20));
    }
    machine.begin_shutdown().unwrap();
}
