use super::*;

#[test]
fn setup_offer_is_claimed_once_and_survives_restart_and_stale_settings() {
    let directory = std::env::temp_dir().join(format!("opul-setup-{}", uuid::Uuid::new_v4()));
    fs::create_dir_all(&directory).unwrap();
    let store = PreferenceStore::open(&directory).unwrap();
    let stale = store.get();
    assert!(!stale.video_tools_setup_shown);
    assert!(store.mark_video_tools_setup_shown().unwrap());
    assert!(!store.mark_video_tools_setup_shown().unwrap());
    store.save(stale).unwrap();
    assert!(
        PreferenceStore::open(&directory)
            .unwrap()
            .get()
            .video_tools_setup_shown
    );
    fs::remove_dir_all(directory).unwrap();
}
