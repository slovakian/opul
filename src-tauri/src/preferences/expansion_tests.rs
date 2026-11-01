use super::*;

#[test]
fn expansion_cleanup_is_atomic_preserves_dormant_choices_and_retries_latest_map() {
    let directory = tempfile::tempdir().unwrap();
    let store = PreferenceStore::open(directory.path()).unwrap();
    let mut settings = store.get();
    settings.theme = Theme::Dark;
    settings.max_download_kbps = 2048;
    settings.workspace_expansion_startup = WorkspaceExpansionStartup::Collapsed;
    store.save(settings).unwrap();
    store
        .save_workspace_expansion(
            [
                ("deleted".into(), Some(true)),
                ("dormant".into(), Some(false)),
                ("other".into(), Some(true)),
            ]
            .into(),
        )
        .unwrap();
    let before = PreferenceStore::open(directory.path()).unwrap().get();
    let stale_settings = before.clone();
    let temporary = store.path.with_extension("json.tmp");
    fs::create_dir(&temporary).unwrap();
    assert!(
        store
            .save_workspace_expansion(
                [("deleted".into(), None), ("other".into(), Some(false)),].into()
            )
            .is_err()
    );
    assert_eq!(
        serde_json::to_value(store.get()).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    assert_eq!(
        serde_json::to_value(PreferenceStore::open(directory.path()).unwrap().get()).unwrap(),
        serde_json::to_value(&before).unwrap()
    );
    // Removing an already absent identity does not attempt a write.
    store
        .save_workspace_expansion([("absent".into(), None)].into())
        .unwrap();
    fs::remove_dir(&temporary).unwrap();
    store
        .save_workspace_expansion(
            [
                ("deleted".into(), None),
                ("other".into(), Some(false)),
                ("latest".into(), Some(true)),
            ]
            .into(),
        )
        .unwrap();
    store.save(stale_settings).unwrap();
    store
        .save_aggregate_view(AggregateView::All, DisplayPreferences::default())
        .unwrap();
    let restored = PreferenceStore::open(directory.path()).unwrap().get();
    assert_eq!(
        restored.workspace_expansion,
        [
            ("dormant".into(), false),
            ("other".into(), false),
            ("latest".into(), true),
        ]
        .into()
    );
    assert!(matches!(restored.theme, Theme::Dark));
    assert_eq!(restored.max_download_kbps, before.max_download_kbps);
    assert_eq!(
        restored.workspace_expansion_startup,
        before.workspace_expansion_startup
    );
}
