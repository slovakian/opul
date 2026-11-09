use super::*;
use crate::commands::{DesktopCommand, ImportMode};
use opul_core::{EngineConfig, MediaEngineConfig, OpulMachineConfig};
use serde_json::{Value, json};
use std::sync::mpsc;
use std::time::Duration;

fn open_machine(root: &std::path::Path) -> Arc<OpulStateMachine> {
    std::fs::create_dir_all(root.join("downloads")).unwrap();
    let tools =
        opul_core::video_tools::VideoTools::open(opul_core::video_tools::VideoToolsConfig {
            nightly_available: false,
            root: root.join("Tools"),
            opul_version: "0.2.2".into(),
        })
        .unwrap();
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

/// Adds video pages, which wait for the video tools without touching the network.
fn add_pages(machine: &OpulStateMachine, count: usize) {
    let library = machine.snapshot();
    let workspace = &library.workspaces[0];
    for index in 0..count {
        DesktopCommand::AddUrl {
            workspace_id: workspace.id.clone(),
            folder_id: workspace.folders[0].id.clone(),
            // Distinct pages, because a workspace deduplicates a page it already holds.
            url: format!("https://example.com/video-{index}-{}", uuid::Uuid::new_v4()),
            mode: ImportMode::Page,
            format: MachineFormat::Best,
        }
        .execute(machine)
        .unwrap();
    }
}

fn library(count: usize) -> (tempfile::TempDir, Arc<OpulStateMachine>) {
    let root = tempfile::tempdir().unwrap();
    let machine = open_machine(root.path());
    add_pages(&machine, count);
    (root, machine)
}

fn json_of(message: &FeedMessage<'_>) -> Value {
    serde_json::to_value(message).unwrap()
}

fn feed() -> (Channel, mpsc::Receiver<Value>) {
    let (sender, receiver) = mpsc::channel();
    let sender = Mutex::new(sender);
    let channel = Channel::new(move |body| {
        let InvokeResponseBody::Json(json) = body else {
            panic!("the feed sends JSON");
        };
        let _ = sender
            .lock()
            .unwrap()
            .send(serde_json::from_str(&json).unwrap());
        Ok(())
    });
    (channel, receiver)
}

const WAIT: Duration = Duration::from_secs(10);

#[test]
fn an_unchanged_library_patches_only_its_position() {
    let (_root, machine) = library(2);
    let library = machine.snapshot();
    let mut next = library.clone();
    next.sequence += 1;
    assert_eq!(
        json_of(&changes(&library, &next)),
        json!({ "kind": "patch", "sequence": next.sequence, "revision": next.revision })
    );
}

#[test]
fn a_changed_download_travels_alone() {
    let (_root, machine) = library(3);
    let library = machine.snapshot();
    let mut next = library.clone();
    next.sequence += 1;
    let changed = &mut next.workspaces[0].downloads[1];
    changed.download_rate = 2_048;
    changed.progress_millionths = 500_000;
    let expected = serde_json::to_value(&*changed).unwrap();
    assert_eq!(
        json_of(&changes(&library, &next)),
        json!({
            "kind": "patch",
            "sequence": next.sequence,
            "revision": next.revision,
            "downloads": [expected],
        })
    );
}

#[test]
fn removing_a_download_sends_the_outlines_without_it() {
    let (_root, machine) = library(3);
    let library = machine.snapshot();
    let mut next = library.clone();
    next.sequence += 1;
    next.revision += 1;
    let removed = next.workspaces[0].downloads.remove(0);
    let message = json_of(&changes(&library, &next));
    let ids = &message["workspaces"][0]["download_ids"];
    assert_eq!(ids.as_array().unwrap().len(), 2);
    assert!(!ids.as_array().unwrap().contains(&json!(removed.id)));
    assert!(message.get("downloads").is_none());
}

#[test]
fn a_renamed_workspace_sends_the_outlines() {
    let (_root, machine) = library(1);
    let library = machine.snapshot();
    let mut next = library.clone();
    next.sequence += 1;
    next.workspaces[0].name = "Films".into();
    next.session = MachineSessionState::Quiescing;
    let message = json_of(&changes(&library, &next));
    assert_eq!(message["workspaces"][0]["name"], "Films");
    assert_eq!(message["session"], "Quiescing");
    assert!(message.get("downloads").is_none());
}

#[test]
fn a_new_boot_resends_the_whole_library() {
    let (_root, machine) = library(1);
    let library = machine.snapshot();
    let mut next = library.clone();
    next.boot_id = Some("another-boot".into());
    assert_eq!(changes(&library, &next), FeedMessage::Snapshot(&next));
    assert_eq!(json_of(&FeedMessage::Snapshot(&next))["kind"], "snapshot");
}

/// An outline must carry every workspace field except the downloads it replaces with ids.
#[test]
fn outlines_carry_every_workspace_field() {
    let (_root, machine) = library(2);
    let library = machine.snapshot();
    let workspace = &library.workspaces[0];
    let mut expected = serde_json::to_value(workspace).unwrap();
    expected.as_object_mut().unwrap().remove("downloads");
    let mut outline = serde_json::to_value(WorkspaceOutline::of(workspace)).unwrap();
    let ids = outline
        .as_object_mut()
        .unwrap()
        .remove("download_ids")
        .unwrap();
    assert_eq!(outline, expected);
    assert_eq!(ids.as_array().unwrap().len(), 2);
}

#[test]
fn the_feed_sends_the_library_then_each_change_and_nothing_while_idle() {
    let (_root, machine) = library(2);
    let feeds = LibraryFeeds::default();
    let (channel, messages) = feed();
    feeds
        .start("main".into(), Arc::clone(&machine), channel)
        .unwrap();

    let first = messages.recv_timeout(WAIT).unwrap();
    assert_eq!(first["kind"], "snapshot");
    assert_eq!(
        first["workspaces"][0]["downloads"]
            .as_array()
            .unwrap()
            .len(),
        2
    );

    add_pages(&machine, 1);
    // Unrelated publishes may land first; the patch that adds the page carries it whole.
    let patch = std::iter::from_fn(|| messages.recv_timeout(WAIT).ok())
        .find(|message| message.get("workspaces").is_some())
        .unwrap();
    assert_eq!(patch["kind"], "patch");
    let ids = patch["workspaces"][0]["download_ids"].as_array().unwrap();
    assert_eq!(ids.len(), 3);
    assert_eq!(patch["downloads"].as_array().unwrap().len(), 1, "{patch}");
    assert_eq!(patch["downloads"][0]["id"], ids[2]);

    // A burst of publishes may coalesce; drain whatever followed the command, then expect quiet.
    while messages.recv_timeout(Duration::from_millis(500)).is_ok() {}
    assert!(messages.recv_timeout(Duration::from_secs(2)).is_err());
    machine.begin_shutdown().unwrap();
}

#[test]
fn a_new_subscription_replaces_the_feed_of_the_same_webview() {
    let (_root, machine) = library(1);
    let feeds = LibraryFeeds::default();
    let (replaced, replaced_messages) = feed();
    feeds
        .start("main".into(), Arc::clone(&machine), replaced)
        .unwrap();
    assert_eq!(
        replaced_messages.recv_timeout(WAIT).unwrap()["kind"],
        "snapshot"
    );

    let (current, current_messages) = feed();
    feeds
        .start("main".into(), Arc::clone(&machine), current)
        .unwrap();
    assert_eq!(
        current_messages.recv_timeout(WAIT).unwrap()["kind"],
        "snapshot"
    );

    add_pages(&machine, 1);
    assert_eq!(
        current_messages.recv_timeout(WAIT).unwrap()["kind"],
        "patch"
    );
    assert!(
        replaced_messages
            .recv_timeout(Duration::from_secs(1))
            .is_err()
    );
    machine.begin_shutdown().unwrap();
}

/// Prints what one engine update costs to send: the whole library, as polling read it, against
/// the patch the feed sends. Run with
/// `cargo test --release -p Opul feed_cost -- --ignored --nocapture`.
#[test]
#[ignore = "benchmark"]
fn feed_cost() {
    let (_root, machine) = library(1);
    let template = machine.snapshot();
    let rounds = 200;
    for count in [500, 2000] {
        let mut previous = template.clone();
        let download = previous.workspaces[0].downloads[0].clone();
        previous.workspaces[0].downloads = (0..count)
            .map(|index| MachineDownload {
                id: format!("download-{index}"),
                ..download.clone()
            })
            .collect();
        let mut next = previous.clone();
        next.sequence += 1;
        next.workspaces[0].downloads[0].download_rate = 1_048_576;

        let started = std::time::Instant::now();
        let mut snapshot_bytes = 0;
        for _ in 0..rounds {
            snapshot_bytes = serde_json::to_string(std::hint::black_box(&next))
                .unwrap()
                .len();
        }
        let snapshot_time = started.elapsed() / rounds;

        let started = std::time::Instant::now();
        let mut patch_bytes = 0;
        for _ in 0..rounds {
            patch_bytes = serde_json::to_string(&changes(
                std::hint::black_box(&previous),
                std::hint::black_box(&next),
            ))
            .unwrap()
            .len();
        }
        let patch_time = started.elapsed() / rounds;
        println!(
            "{count} downloads: snapshot {snapshot_bytes} B in {snapshot_time:?}; \
             diff + patch {patch_bytes} B in {patch_time:?}"
        );
    }
    machine.begin_shutdown().unwrap();
}

/// Writes a library of idle downloads for measuring the app at rest. See
/// `tests/bench/README.md`.
#[test]
#[ignore = "benchmark setup"]
fn seed_benchmark_library() {
    let directory = std::path::PathBuf::from(std::env::var_os("OPUL_SEED_DIR").unwrap());
    let count = std::env::var("OPUL_SEED_COUNT").map_or(500, |count| count.parse().unwrap());
    std::fs::create_dir_all(directory.join("downloads")).unwrap();
    let tools =
        opul_core::video_tools::VideoTools::open(opul_core::video_tools::VideoToolsConfig {
            nightly_available: false,
            root: directory.join("Tools"),
            opul_version: "0.1.0".into(),
        })
        .unwrap();
    let machine = OpulStateMachine::open(OpulMachineConfig {
        application_support_folder: directory.to_string_lossy().into_owned(),
        downloads_folder: directory.join("downloads").to_string_lossy().into_owned(),
        torrent: EngineConfig {
            session_folder: directory
                .join("torrent-session")
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
            session_folder: directory
                .join("media-session")
                .to_string_lossy()
                .into_owned(),
            resources_folder: None,
            video_tools: tools,
            max_concurrent_downloads: 1,
            max_download_bps: 0,
        },
    })
    .unwrap();
    add_pages(&machine, count);
    machine.begin_shutdown().unwrap();
}
