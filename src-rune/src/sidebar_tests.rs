use super::*;
use crate::navigation::Location;
use crate::preferences::{Preferences, Startup};
use opul_core::MachineWorkspaceRequest;

fn two_workspaces(root: &Path) -> (String, String) {
    let paths = paths::Paths {
        data: root.join("data"),
        downloads: root.join("Downloads"),
        resources: None,
    };
    let machine = paths.open().unwrap();
    let first = machine.snapshot().workspaces[0].clone();
    let draft = machine.workspace_draft().unwrap();
    let second = draft.workspace_id.clone();
    let folder = root.join("Research");
    std::fs::create_dir(&folder).unwrap();
    machine
        .create_workspace(MachineWorkspaceRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id: second.clone(),
            name: "Research".into(),
            icon: first.icon.clone(),
            appearance: draft.appearance,
            folder_id: draft.folder_id,
            folder_name: "Research files".into(),
            folder_path: folder.to_string_lossy().into(),
            default_format: first.default_format,
            file_organization: first.file_organization,
        })
        .unwrap();
    machine.begin_shutdown().unwrap();
    (first.id, second)
}

fn visible(app: &Harness, id: &str) -> bool {
    app.rt.find(&format!("download-{id}")).is_some()
}

#[test]
fn choosing_the_only_saved_folder_keeps_the_simple_workspace_selected() {
    let root = tempfile::tempdir().unwrap();
    let mut app = Harness::open(root.path());
    let workspace = app.rt.state.workspace().unwrap().clone();
    app.click("choose-folder");
    app.rt
        .resolve_file_dialog(Some(workspace.folders[0].path.clone().into()))
        .unwrap();
    app.rt.frame(0.016);
    assert_eq!(
        app.rt.state.location,
        Location::Workspace(workspace.id.clone())
    );
    assert_eq!(app.rt.state.breadcrumb(), workspace.name);
    assert!(app.rt.elements().iter().any(|e| e.key.as_deref()
        == Some(&format!("workspace-{}", workspace.id))
        && e.item_selected));
    assert!(
        app.rt
            .find(&format!("disclosure-{}", workspace.id))
            .is_none()
    );
    app.close();
}

fn focused(app: &Harness, key: &str) -> bool {
    app.rt
        .elements()
        .iter()
        .any(|e| e.key.as_deref() == Some(key) && e.focused)
}

#[test]
fn real_download_filters_counts_breadcrumbs_and_destinations_agree() {
    let root = tempfile::tempdir().unwrap();
    let (first, second) = two_workspaces(root.path());
    let server = Server::start();
    let mut app = Harness::open(root.path());
    app.click(&format!("workspace-{first}"));
    let completed = app.submit(&server.url("completed"));
    app.wait_state(&completed, MachineDownloadState::Finished);
    app.click(&format!("workspace-{second}"));
    let second_folder = app.rt.state.selected_folder.clone().unwrap();
    let saved = root.path().join("Saved");
    std::fs::create_dir(&saved).unwrap();
    app.click("choose-folder");
    app.rt.resolve_file_dialog(Some(saved.clone())).unwrap();
    app.wait(|s| !s.busy() && s.workspace().unwrap().folders.len() == 2);
    let selected = app.rt.state.selected_folder.clone().unwrap();
    assert_eq!(app.rt.state.breadcrumb(), "Research › Saved");
    assert!(
        app.rt
            .state
            .workspace()
            .unwrap()
            .folders
            .iter()
            .any(|f| f.id == second_folder && f.is_default)
    );
    let active = app.submit(&server.url("active"));
    app.wait_state(&active, MachineDownloadState::Fetching);
    let download = &app.rt.state.workspace().unwrap().downloads[0];
    assert_eq!(download.folder_id, selected);
    assert!(visible(&app, &active));
    assert!(!visible(&app, &completed));
    app.click("nav-all");
    assert_eq!(app.rt.state.count(&Location::All), 2);
    assert!(visible(&app, &active) && visible(&app, &completed));
    assert!(
        app.rt.state.selected_folder.is_none(),
        "aggregate must not silently borrow a Workspace destination"
    );
    app.click("nav-progress");
    assert!(visible(&app, &active) && !visible(&app, &completed));
    assert_eq!(app.rt.state.count(&Location::InProgress), 1);
    app.click("nav-completed");
    assert!(!visible(&app, &active) && visible(&app, &completed));
    app.click(&format!("folder-{second_folder}"));
    assert!(!visible(&app, &active));
    app.click(&format!("folder-{selected}"));
    app.wait_state(&active, MachineDownloadState::Finished);
    let download = &app.rt.state.workspace().unwrap().downloads[0];
    let output = download.output_path.as_ref().unwrap();
    assert!(Path::new(output).starts_with(&saved));
    assert_eq!(std::fs::read(output).unwrap(), *server.payload);
    app.click("breadcrumb-workspace");
    assert_eq!(
        app.rt.state.selected_folder.as_deref(),
        Some(second_folder.as_str())
    );
    assert_eq!(app.rt.state.breadcrumb(), "Research");
    app.close();
}

#[test]
fn disclosure_menu_keyboard_resize_and_draft_preservation() {
    let root = tempfile::tempdir().unwrap();
    let mut app = Harness::open(root.path());
    let id = app.rt.state.workspace().unwrap().id.clone();
    let folder = root.path().join("Saved");
    std::fs::create_dir(&folder).unwrap();
    app.click("choose-folder");
    app.rt.resolve_file_dialog(Some(folder)).unwrap();
    app.wait(|s| !s.busy() && s.workspace().unwrap().folders.len() == 2);
    let selected = app.rt.state.selected_folder.clone().unwrap();
    app.click("direct-url");
    app.rt.paste("https://example.com/draft.mp4");
    app.click(&format!("disclosure-{id}"));
    assert!(!app.rt.state.expansion.expanded(&id));
    app.rt.key(Key::Right.into());
    app.rt.frame(0.016);
    assert!(app.rt.state.expansion.expanded(&id));
    app.rt.key(Key::Left.into());
    app.rt.frame(0.016);
    assert!(!app.rt.state.expansion.expanded(&id));
    app.rt.key(Key::Enter.into());
    app.rt.frame(0.016);
    assert!(app.rt.state.expansion.expanded(&id));
    let trigger = format!("workspace-actions-{id}");
    app.click(&trigger);
    assert!(app.rt.find("workspace-menu").is_some());
    app.rt.key(Key::Escape.into());
    app.rt.frame(0.016);
    assert!(app.rt.find("workspace-menu").is_none());
    assert!(focused(&app, &trigger));
    app.rt.key(Key::Enter.into());
    app.rt.frame(0.016);
    assert!(app.rt.find("workspace-menu").is_some());
    app.rt.key(Key::Escape.into());
    app.rt.frame(0.016);
    app.click("look");
    app.rt.set_viewport((680.0, 650.0), 1.0);
    app.rt.frame(0.016);
    app.click("open-navigation");
    assert!(app.rt.find("navigation-sheet").is_some());
    app.click(&format!("disclosure-{id}"));
    assert!(app.rt.find("navigation-sheet").is_some());
    assert_eq!(app.rt.state.selected_folder.as_ref(), Some(&selected));
    app.click(&format!("disclosure-{id}"));
    app.click(&trigger);
    app.rt.key(Key::Escape.into());
    app.rt.frame(0.016);
    assert!(
        app.rt.find("navigation-sheet").is_some(),
        "nested Escape closes only the menu"
    );
    app.rt.key(Key::Escape.into());
    app.rt.frame(0.016);
    assert!(app.rt.find("navigation-sheet").is_none());
    assert!(focused(&app, "open-navigation"));
    app.click("open-navigation");
    app.click(&format!("folder-{selected}"));
    assert!(app.rt.find("navigation-sheet").is_none());
    app.rt.set_viewport((1180.0, 760.0), 1.0);
    app.rt.frame(0.016);
    assert!(app.rt.state.expansion.expanded(&id));
    assert!(app.rt.find(&format!("folder-{selected}")).is_some());
    assert_eq!(app.rt.state.url.text, "https://example.com/draft.mp4");
    app.close();
}

#[test]
fn preferences_failure_retry_and_relaunch_through_controls() {
    let root = tempfile::tempdir().unwrap();
    let mut app = Harness::open(root.path());
    let id = app.rt.state.workspace().unwrap().id.clone();
    let folder = root.path().join("Saved");
    std::fs::create_dir(&folder).unwrap();
    app.click("choose-folder");
    app.rt.resolve_file_dialog(Some(folder)).unwrap();
    app.wait(|s| {
        !s.busy()
            && s.expansion.loaded
            && !s.expansion.saving
            && s.workspace().unwrap().folders.len() == 2
    });
    let path = root.path().join("data/navigation-preferences.json");
    let temporary = path.with_extension("json.tmp");
    std::fs::create_dir(&temporary).unwrap();
    for _ in 0..3 {
        app.click(&format!("disclosure-{id}"));
    }
    app.wait(|s| s.expansion.error.is_some());
    assert!(!app.rt.state.expansion.expanded(&id));
    assert!(Preferences::read(&path).unwrap().workspace_expansion[&id]);
    std::fs::remove_dir(&temporary).unwrap();
    app.click("retry-expansion");
    app.wait(|s| s.expansion.error.is_none() && !s.expansion.saving);
    assert!(!Preferences::read(&path).unwrap().workspace_expansion[&id]);
    app.click(&format!("disclosure-{id}"));
    app.click("expansion-startup");
    app.wait(|s| !s.expansion.saving);
    assert!(
        app.rt.state.expansion.expanded(&id),
        "startup policy only applies next launch"
    );
    app.close();
    let mut app = Harness::open(root.path());
    app.wait(|s| s.expansion.loaded);
    assert_eq!(
        app.rt.state.expansion.desired.workspace_expansion_startup,
        Startup::Collapsed
    );
    assert!(!app.rt.state.expansion.expanded(&id));
    assert!(Preferences::read(&path).unwrap().workspace_expansion[&id]);
    app.click("expansion-startup");
    app.wait(|s| !s.expansion.saving);
    app.close();
    let mut app = Harness::open(root.path());
    app.wait(|s| s.expansion.loaded);
    assert!(app.rt.state.expansion.expanded(&id));
    app.close();
}

#[test]
fn malformed_preferences_can_be_repaired_without_losing_early_choices() {
    let root = tempfile::tempdir().unwrap();
    let (id, other) = two_workspaces(root.path());
    let path = root.path().join("data/navigation-preferences.json");
    std::fs::write(&path, b"malformed").unwrap();
    let mut app = Harness::open(root.path());
    app.wait(|s| s.expansion.error.is_some());
    app.click("expansion-startup");
    assert!(!app.rt.state.expansion.loaded);
    assert_eq!(std::fs::read(&path).unwrap(), b"malformed");
    Preferences {
        workspace_expansion: [(id.clone(), true), (other.clone(), true)].into(),
        workspace_expansion_startup: Startup::Remember,
    }
    .write(&path)
    .unwrap();
    app.click("retry-expansion");
    app.wait(|s| s.expansion.loaded && !s.expansion.saving);
    assert!(app.rt.state.expansion.expanded(&id));
    assert!(app.rt.state.expansion.expanded(&other));
    assert_eq!(
        Preferences::read(&path)
            .unwrap()
            .workspace_expansion_startup,
        Startup::Collapsed
    );
    app.close();
}

#[test]
fn live_projection_changes_preserve_expansion_and_normalize_missing_locations() {
    let root = tempfile::tempdir().unwrap();
    let (id, other) = two_workspaces(root.path());
    let mut app = Harness::open(root.path());
    app.click(&format!("workspace-{id}"));
    let folder = root.path().join("Saved");
    std::fs::create_dir(&folder).unwrap();
    app.click("choose-folder");
    app.rt.resolve_file_dialog(Some(folder)).unwrap();
    app.wait(|s| !s.busy() && s.workspace().unwrap().folders.len() == 2 && !s.expansion.saving);
    let selected = app.rt.state.selected_folder.clone().unwrap();
    let mut projection = app.rt.state.snapshot.clone().unwrap();
    projection.sequence += 100;
    let workspace = projection
        .workspaces
        .iter_mut()
        .find(|w| w.id == id)
        .unwrap();
    workspace.name = "Renamed".into();
    workspace.folders.reverse();
    projection.workspaces.reverse();
    app.events
        .send(backend::Event::Snapshot(projection.clone()))
        .unwrap();
    app.rt.frame(0.016);
    assert!(app.rt.state.expansion.expanded(&id));
    assert_eq!(app.rt.state.selected_folder.as_ref(), Some(&selected));
    assert_eq!(app.rt.state.breadcrumb(), "Renamed › Saved");
    // Only a newer authoritative projection may retire the selected folder.
    let mut stale = projection.clone();
    stale.sequence -= 1;
    stale
        .workspaces
        .iter_mut()
        .find(|w| w.id == id)
        .unwrap()
        .folders
        .retain(|f| f.id != selected);
    app.events
        .send(backend::Event::Snapshot(stale.clone()))
        .unwrap();
    app.rt.frame(0.016);
    assert_eq!(app.rt.state.selected_folder.as_ref(), Some(&selected));
    stale.sequence = projection.sequence + 1;
    app.events
        .send(backend::Event::Snapshot(stale.clone()))
        .unwrap();
    app.rt.frame(0.016);
    assert_eq!(app.rt.state.location, Location::Workspace(id.clone()));
    assert!(app.rt.find(&format!("disclosure-{id}")).is_none());
    assert!(
        app.rt.state.expansion.expanded(&id),
        "one-folder rows retain their dormant choice"
    );
    stale.sequence += 1;
    stale.workspaces.retain(|w| w.id != id);
    app.events.send(backend::Event::Snapshot(stale)).unwrap();
    app.wait(|s| !s.expansion.saving);
    assert_eq!(app.rt.state.location, Location::All);
    assert_eq!(
        app.rt.state.selected_folder.as_deref(),
        app.rt
            .state
            .workspace()
            .unwrap()
            .folders
            .iter()
            .find(|f| f.is_default)
            .map(|f| f.id.as_str())
    );
    assert!(
        !app.rt
            .state
            .expansion
            .desired
            .workspace_expansion
            .contains_key(&id)
    );
    assert!(
        !Preferences::read(&root.path().join("data/navigation-preferences.json"))
            .unwrap()
            .workspace_expansion
            .contains_key(&id)
    );
    assert!(app.rt.find(&format!("workspace-{other}")).is_some());
    app.close();
}

#[test]
fn deleted_workspaces_are_cleaned_from_real_preferences_on_reopen() {
    let root = tempfile::tempdir().unwrap();
    let (id, other) = two_workspaces(root.path());
    let path = root.path().join("data/navigation-preferences.json");
    Preferences {
        workspace_expansion: [(id.clone(), true), (other.clone(), false)].into(),
        workspace_expansion_startup: Startup::Remember,
    }
    .write(&path)
    .unwrap();
    let paths = paths::Paths {
        data: root.path().join("data"),
        downloads: root.path().join("Downloads"),
        resources: None,
    };
    let machine = paths.open().unwrap();
    machine
        .delete_workspace(
            uuid::Uuid::new_v4().to_string(),
            None,
            id.clone(),
            None,
            false,
        )
        .unwrap();
    machine.begin_shutdown().unwrap();
    drop(machine);
    let mut app = Harness::open(root.path());
    app.wait(|s| s.expansion.loaded && !s.expansion.saving);
    let preferences = Preferences::read(&path).unwrap();
    assert!(!preferences.workspace_expansion.contains_key(&id));
    assert_eq!(preferences.workspace_expansion.get(&other), Some(&false));
    app.close();
}

#[test]
fn menu_outside_click_is_consumed_and_missing_folder_restores_parent_focus() {
    let root = tempfile::tempdir().unwrap();
    let mut app = Harness::open(root.path());
    let id = app.rt.state.workspace().unwrap().id.clone();
    for name in ["Second", "Third"] {
        let folder = root.path().join(name);
        std::fs::create_dir(&folder).unwrap();
        app.click("choose-folder");
        app.rt.resolve_file_dialog(Some(folder)).unwrap();
        app.wait(|s| {
            !s.busy()
                && s.workspace()
                    .unwrap()
                    .folders
                    .iter()
                    .any(|f| f.name == name)
        });
    }
    let location = app.rt.state.location.clone();
    app.click(&format!("workspace-actions-{id}"));
    app.click("nav-completed");
    assert!(app.rt.find("workspace-menu").is_none());
    assert_eq!(
        app.rt.state.location, location,
        "outside dismissal must not navigate the background"
    );
    let removed = app.rt.state.selected_folder.clone().unwrap();
    app.click(&format!("folder-{removed}"));
    let mut snapshot = app.rt.state.snapshot.clone().unwrap();
    snapshot.sequence += 100;
    snapshot.workspaces[0].folders.retain(|f| f.id != removed);
    app.events.send(backend::Event::Snapshot(snapshot)).unwrap();
    app.rt.frame(0.016);
    assert_eq!(app.rt.state.location, Location::Workspace(id.clone()));
    assert!(
        focused(&app, &format!("workspace-{id}")),
        "retiring a focused folder should return focus to its Workspace"
    );
    app.close();
}
