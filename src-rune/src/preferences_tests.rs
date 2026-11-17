use super::*;
use std::collections::{BTreeMap, BTreeSet};

fn saved(startup: Startup) -> Preferences {
    Preferences {
        workspace_expansion: BTreeMap::from([("one".into(), true), ("two".into(), true)]),
        workspace_expansion_startup: startup,
    }
}

#[test]
fn delayed_reads_merge_early_choices_in_both_startup_modes() {
    for startup in [Startup::Remember, Startup::Collapsed] {
        let mut state = Expansion::default();
        state.toggle("one".into());
        state.toggle("one".into());
        state.set("added".into(), true);
        assert!(state.write().is_none());
        state.loaded(Ok(saved(startup)));
        assert!(!state.expanded("one"));
        assert!(state.expanded("added"));
        assert_eq!(state.expanded("two"), startup == Startup::Remember);
        assert!(state.desired.workspace_expansion["two"]);
        let (_, written) = state.write().unwrap();
        assert!(!written.workspace_expansion["one"]);
        assert!(written.workspace_expansion["two"]);
    }
}

#[test]
fn failed_read_retries_without_discarding_session_intent() {
    let mut state = Expansion::default();
    state.set("one".into(), false);
    state.startup(Startup::Collapsed);
    state.loaded(Err("unreadable".into()));
    assert!(state.error.is_some());
    assert!(!state.loaded);
    assert!(state.write().is_none());
    state.loaded(Ok(saved(Startup::Remember)));
    assert!(!state.expanded("one"));
    assert!(state.expanded("two"));
    assert_eq!(
        state.desired.workspace_expansion_startup,
        Startup::Collapsed
    );
}

#[test]
fn stale_acknowledgements_and_failed_cleanup_keep_latest_choice() {
    let mut state = Expansion::default();
    state.loaded(Ok(saved(Startup::Remember)));
    state.toggle("one".into());
    let (old, _) = state.write().unwrap();
    state.toggle("one".into());
    state.retain(&BTreeSet::from(["one".into()]));
    state.saved(old, Ok(()));
    assert!(state.expanded("one"));
    assert!(!state.desired.workspace_expansion.contains_key("two"));
    let (latest, desired) = state.write().unwrap();
    state.saved(latest, Err("disk full".into()));
    assert!(state.error.is_some());
    assert_eq!(state.write().unwrap().1, desired);
    state.saved(latest, Ok(()));
    assert!(state.error.is_none());
    assert!(state.write().is_none());
    state.loaded(Ok(saved(Startup::Collapsed)));
    assert!(state.expanded("one"));
}

#[test]
fn real_atomic_failure_preserves_file_and_reopens_latest_retry() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("navigation-preferences.json");
    let original = saved(Startup::Remember);
    original.write(&path).unwrap();
    let temporary = path.with_extension("json.tmp");
    std::fs::create_dir(&temporary).unwrap();
    let mut latest = original.clone();
    latest.workspace_expansion.insert("one".into(), false);
    latest.workspace_expansion.remove("two");
    latest.workspace_expansion_startup = Startup::Collapsed;
    assert!(latest.write(&path).is_err());
    assert_eq!(Preferences::read(&path).unwrap(), original);
    std::fs::remove_dir(&temporary).unwrap();
    latest.write(&path).unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), latest);
    std::fs::write(&path, b"invalid").unwrap();
    assert!(Preferences::read(&path).is_err());
    std::fs::write(&path, b"{}").unwrap();
    assert_eq!(Preferences::read(&path).unwrap(), Preferences::default());
}
