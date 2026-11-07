use super::*;
use crate::preferences::WorkspaceExpansionStartup;
use opul_core::{EngineConfig, MachineSessionState, MediaEngineConfig, OpulMachineConfig};
use std::path::Path;

fn open_machine(root: &Path, preferences: &Preferences) -> Result<Arc<OpulStateMachine>, String> {
    OpulStateMachine::open(OpulMachineConfig {
        application_support_folder: root.join("state").to_string_lossy().into_owned(),
        downloads_folder: root.join("downloads").to_string_lossy().into_owned(),
        torrent: EngineConfig {
            session_folder: root.join("torrent").to_string_lossy().into_owned(),
            listen_port: preferences.listen_port,
            randomize_port: preferences.randomize_port,
            enable_dht: preferences.enable_dht,
            max_connections: preferences.max_connections,
            max_download_bps: u64::from(preferences.max_download_kbps) * 1024,
            max_upload_bps: u64::from(preferences.max_upload_kbps) * 1024,
        },
        media: MediaEngineConfig {
            session_folder: root.join("media").to_string_lossy().into_owned(),
            resources_folder: None,
            video_tools: None,
            max_concurrent_downloads: preferences.concurrent_downloads,
            max_download_bps: u64::from(preferences.max_download_kbps) * 1024,
        },
    })
    .map_err(|error| error.to_string())
}

fn recover_preferences(unreadable: bool) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("state");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::create_dir_all(root.path().join("downloads")).unwrap();
    let path = directory.join("desktop-preferences.json");
    if unreadable {
        // Reading a directory as a file fails even when tests run as root.
        std::fs::create_dir(&path).unwrap();
    } else {
        std::fs::write(&path, b"{broken").unwrap();
    }
    let initialize = |preferences: &Preferences| open_machine(root.path(), preferences);
    let desktop = Desktop::open(Ok(directory.clone()), initialize);
    assert!(desktop.preferences().is_err());
    assert!(desktop.machine().is_err());
    assert!(
        desktop
            .load_preferences(Ok(directory.clone()), initialize)
            .is_err()
    );
    assert!(!directory.join("application-state.json").exists());
    if unreadable {
        assert!(path.is_dir());
        std::fs::remove_dir(&path).unwrap();
        std::fs::write(&path, b"{still broken").unwrap();
        let error = desktop
            .load_preferences(Ok(directory.clone()), initialize)
            .unwrap_err();
        assert!(error.starts_with("Could not read settings:"));
    } else {
        assert_eq!(std::fs::read(&path).unwrap(), b"{broken");
    }

    let repaired = br#"{
        "theme":"dark", "workspace_expansion_startup":"collapsed",
        "workspace_expansion":{"untouched":true,"changed":false},
        "aggregate_views":{"all":{"layout":"Table"}},
        "max_download_kbps":2048, "concurrent_downloads":2,
        "listen_port":0, "enable_dht":false
    }"#;
    std::fs::write(&path, repaired).unwrap();
    let loaded = std::thread::scope(|scope| {
        let read = || {
            desktop
                .load_preferences(Ok(directory.clone()), initialize)
                .unwrap()
        };
        let first = scope.spawn(read);
        let second = scope.spawn(read);
        assert_eq!(second.join().unwrap().max_download_kbps, 2048);
        first.join().unwrap()
    });
    assert_eq!(loaded.max_download_kbps, 2048);
    assert_eq!(
        loaded.workspace_expansion_startup,
        WorkspaceExpansionStartup::Collapsed
    );
    assert_eq!(std::fs::read(&path).unwrap(), repaired);
    let machine = desktop
        .machine()
        .expect("Retry must also unblock the library");
    assert_eq!(machine.snapshot().session, MachineSessionState::Running);
    assert!(!machine.snapshot().workspaces.is_empty());

    // Pending session intent changes only its own entry after a successful read.
    let store = desktop.preferences().unwrap();
    store
        .save_workspace_expansion([("changed".into(), Some(true))].into())
        .unwrap();
    let saved = serde_json::to_value(PreferenceStore::open(&directory).unwrap().get()).unwrap();
    assert_eq!(
        saved["workspace_expansion"],
        serde_json::json!({"untouched":true,"changed":true})
    );
    assert_eq!(saved["theme"], "dark");
    assert_eq!(saved["aggregate_views"]["all"]["layout"], "Table");
    assert_eq!(saved["max_download_kbps"], 2048);
    assert_eq!(saved["concurrent_downloads"], 2);

    // Repeated reads reuse the live store and process-wide machine.
    let read_again = desktop
        .load_preferences(Ok(directory), |_| panic!("machine reopened"))
        .unwrap();
    assert!(read_again.workspace_expansion["changed"]);
    assert!(Arc::ptr_eq(&store, &desktop.preferences().unwrap()));
    assert!(Arc::ptr_eq(&machine, &desktop.machine().unwrap()));
    machine.begin_shutdown().unwrap();
}

#[test]
fn preferences_retry_recovers_malformed_file_and_deferred_machine() {
    recover_preferences(false);
}

#[test]
fn preferences_retry_recovers_unreadable_file_and_deferred_machine() {
    recover_preferences(true);
}

#[test]
fn preferences_reads_preserve_live_store_and_unrelated_machine_failure() {
    let root = tempfile::tempdir().unwrap();
    let desktop = Desktop::open(Ok(root.path().into()), |_| Err("engine failed".into()));
    let store = desktop.preferences().unwrap();
    store
        .save_workspace_expansion([("changed".into(), Some(true))].into())
        .unwrap();
    // A successful store remains authoritative even if the disk later changes.
    std::fs::write(root.path().join("desktop-preferences.json"), b"{broken").unwrap();
    let loaded = desktop
        .load_preferences(Ok(root.path().into()), |_| panic!("machine reopened"))
        .unwrap();
    assert!(loaded.workspace_expansion["changed"]);
    assert!(Arc::ptr_eq(&store, &desktop.preferences().unwrap()));
    assert_eq!(desktop.machine().err().unwrap(), "engine failed");
}
