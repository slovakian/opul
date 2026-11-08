use opul_core::{MachineGrouping, MachineLayout, MachineSort};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub video_tools_setup_shown: bool,
    pub aggregate_views: AggregateViews,
    pub aggregate_destination: AggregateDestination,
    pub workspace_expansion: BTreeMap<String, bool>,
    pub workspace_expansion_startup: WorkspaceExpansionStartup,
    pub max_download_kbps: u32,
    pub max_upload_kbps: u32,
    pub concurrent_downloads: u32,
    pub listen_port: u16,
    pub randomize_port: bool,
    pub enable_dht: bool,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WorkspaceExpansionStartup {
    #[default]
    Remember,
    Collapsed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum AggregateDestination {
    #[default]
    Prompt,
    Workspace {
        workspace_id: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AggregateView {
    All,
    Active,
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DisplayPreferences {
    pub layout: MachineLayout,
    pub sort: MachineSort,
    pub grouping: MachineGrouping,
}

impl Default for DisplayPreferences {
    fn default() -> Self {
        Self {
            layout: MachineLayout::List,
            sort: MachineSort::Order,
            grouping: MachineGrouping::None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AggregateViews {
    pub all: DisplayPreferences,
    pub active: DisplayPreferences,
    pub finished: DisplayPreferences,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            theme: Theme::System,
            video_tools_setup_shown: false,
            aggregate_views: AggregateViews::default(),
            aggregate_destination: AggregateDestination::default(),
            workspace_expansion: BTreeMap::new(),
            workspace_expansion_startup: WorkspaceExpansionStartup::Remember,
            max_download_kbps: 0,
            max_upload_kbps: 0,
            concurrent_downloads: 3,
            listen_port: 6881,
            randomize_port: true,
            enable_dht: true,
            max_connections: 200,
        }
    }
}

pub struct PreferenceStore {
    path: PathBuf,
    value: Mutex<Preferences>,
}

impl PreferenceStore {
    pub fn open(directory: &Path) -> Result<Self, String> {
        let path = directory.join("desktop-preferences.json");
        let value = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("Could not read settings: {error}"))?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Preferences::default(),
            Err(error) => return Err(error.to_string()),
        };
        Ok(Self {
            path,
            value: Mutex::new(value),
        })
    }

    pub fn get(&self) -> Preferences {
        self.value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn save(&self, mut preferences: Preferences) -> Result<Preferences, String> {
        if !(1..=32).contains(&preferences.concurrent_downloads)
            || preferences.max_connections == 0
            || preferences.max_connections > 10_000
            || (!preferences.randomize_port && preferences.listen_port == 0)
        {
            return Err("Check the connection limits and listen port.".into());
        }
        // Serialize writes so two windows cannot interleave the temporary file.
        let mut value = self
            .value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Settings drafts do not own live display or expansion preferences.
        preferences.video_tools_setup_shown = value.video_tools_setup_shown;
        preferences.aggregate_views = value.aggregate_views.clone();
        preferences.workspace_expansion = value.workspace_expansion.clone();
        self.persist(&preferences)?;
        *value = preferences.clone();
        Ok(preferences)
    }

    /// Claims the first-launch offer only after its dismissal memory is durable.
    pub fn mark_video_tools_setup_shown(&self) -> Result<bool, String> {
        let mut value = self
            .value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if value.video_tools_setup_shown {
            return Ok(false);
        }
        let mut next = value.clone();
        next.video_tools_setup_shown = true;
        self.persist(&next)?;
        *value = next;
        Ok(true)
    }

    pub fn save_aggregate_view(
        &self,
        view: AggregateView,
        display: DisplayPreferences,
    ) -> Result<Preferences, String> {
        let mut value = self
            .value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut next = value.clone();
        let target = match view {
            AggregateView::All => &mut next.aggregate_views.all,
            AggregateView::Active => &mut next.aggregate_views.active,
            AggregateView::Finished => &mut next.aggregate_views.finished,
        };
        *target = display;
        self.persist(&next)?;
        *value = next.clone();
        Ok(next)
    }

    pub fn save_workspace_expansion(
        &self,
        choices: BTreeMap<String, Option<bool>>,
    ) -> Result<BTreeMap<String, bool>, String> {
        let mut value = self
            .value
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if choices
            .iter()
            .all(|(id, expanded)| value.workspace_expansion.get(id) == expanded.as_ref())
        {
            return Ok(value.workspace_expansion.clone());
        }
        let mut next = value.clone();
        for (id, expanded) in choices {
            if let Some(expanded) = expanded {
                next.workspace_expansion.insert(id, expanded);
            } else {
                next.workspace_expansion.remove(&id);
            }
        }
        self.persist(&next)?;
        let saved = next.workspace_expansion.clone();
        *value = next;
        Ok(saved)
    }

    // Callers hold the value lock until the atomic write has completed.
    fn persist(&self, preferences: &Preferences) -> Result<(), String> {
        let temporary = self.path.with_extension("json.tmp");
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let bytes = serde_json::to_vec_pretty(preferences).map_err(|error| error.to_string())?;
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())?;
        fs::rename(&temporary, &self.path).map_err(|error| error.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_expansion_defaults_and_live_choices_survive_stale_settings_and_display_saves() {
        let directory =
            std::env::temp_dir().join(format!("opul-expansion-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("desktop-preferences.json"),
            br#"{"theme":"dark"}"#,
        )
        .unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        assert!(store.get().workspace_expansion.is_empty());
        assert_eq!(
            store.get().workspace_expansion_startup,
            WorkspaceExpansionStartup::Remember
        );
        let mut stale_settings = store.get();
        stale_settings.max_download_kbps = 2048;
        stale_settings.workspace_expansion_startup = WorkspaceExpansionStartup::Collapsed;
        store.save(stale_settings.clone()).unwrap();
        let display = DisplayPreferences {
            layout: MachineLayout::Table,
            ..DisplayPreferences::default()
        };
        store
            .save_aggregate_view(AggregateView::Active, display.clone())
            .unwrap();
        store
            .save_workspace_expansion([("research".into(), Some(true))].into())
            .unwrap();
        store
            .save_workspace_expansion([("media".into(), Some(false))].into())
            .unwrap();
        store.save(stale_settings).unwrap();
        store
            .save_aggregate_view(
                AggregateView::All,
                DisplayPreferences {
                    layout: MachineLayout::Table,
                    ..DisplayPreferences::default()
                },
            )
            .unwrap();
        let restored = PreferenceStore::open(&directory).unwrap().get();
        assert_eq!(
            restored.workspace_expansion,
            [("research".into(), true), ("media".into(), false)].into()
        );
        assert_eq!(restored.max_download_kbps, 2048);
        assert_eq!(restored.aggregate_views.all.layout, MachineLayout::Table);
        assert_eq!(restored.aggregate_views.active, display);
        assert!(matches!(restored.theme, Theme::Dark));
        assert_eq!(
            restored.workspace_expansion_startup,
            WorkspaceExpansionStartup::Collapsed
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_workspace_expansion_save_preserves_memory_and_disk_and_retries_latest_choices() {
        let directory =
            std::env::temp_dir().join(format!("opul-expansion-failure-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        store
            .save_workspace_expansion([("research".into(), Some(true))].into())
            .unwrap();
        let before = fs::read(&store.path).unwrap();
        let temporary = store.path.with_extension("json.tmp");
        fs::create_dir(&temporary).unwrap();
        assert!(
            store
                .save_workspace_expansion(
                    [
                        ("research".into(), Some(false)),
                        ("media".into(), Some(true))
                    ]
                    .into()
                )
                .is_err()
        );
        assert_eq!(
            store.get().workspace_expansion,
            [("research".into(), true)].into()
        );
        assert_eq!(fs::read(&store.path).unwrap(), before);
        // A no-op must not touch the filesystem, even while writes are blocked.
        store
            .save_workspace_expansion([("research".into(), Some(true))].into())
            .unwrap();
        fs::remove_dir(&temporary).unwrap();
        store
            .save_workspace_expansion(
                [
                    ("research".into(), Some(false)),
                    ("media".into(), Some(false)),
                ]
                .into(),
            )
            .unwrap();
        assert_eq!(
            PreferenceStore::open(&directory)
                .unwrap()
                .get()
                .workspace_expansion,
            [("research".into(), false), ("media".into(), false)].into()
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn aggregate_destination_defaults_and_round_trips_without_changing_display_preferences() {
        let directory =
            std::env::temp_dir().join(format!("opul-destination-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("desktop-preferences.json"),
            br#"{"theme":"dark"}"#,
        )
        .unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        assert_eq!(
            store.get().aggregate_destination,
            AggregateDestination::Prompt
        );
        let mut preferences = store.get();
        preferences.aggregate_destination = AggregateDestination::Workspace {
            workspace_id: "media".into(),
        };
        store.save(preferences).unwrap();
        store
            .save_aggregate_view(
                AggregateView::All,
                DisplayPreferences {
                    layout: MachineLayout::Table,
                    ..DisplayPreferences::default()
                },
            )
            .unwrap();
        let reopened = PreferenceStore::open(&directory).unwrap();
        assert_eq!(
            reopened.get().aggregate_destination,
            AggregateDestination::Workspace {
                workspace_id: "media".into()
            }
        );
        let mut preferences = reopened.get();
        preferences.aggregate_destination = AggregateDestination::Prompt;
        reopened.save(preferences).unwrap();
        let restored = PreferenceStore::open(&directory).unwrap().get();
        assert_eq!(restored.aggregate_destination, AggregateDestination::Prompt);
        assert_eq!(restored.aggregate_views.all.layout, MachineLayout::Table);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn aggregate_views_persist_independently_and_old_settings_get_defaults() {
        let directory = std::env::temp_dir().join(format!("opul-views-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("desktop-preferences.json"),
            br#"{"theme":"dark"}"#,
        )
        .unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        let mut settings = serde_json::to_value(store.get()).unwrap();
        let default_view = serde_json::json!({"layout":"List","sort":"Order","grouping":"None"});
        for view in ["all", "active", "finished"] {
            assert_eq!(settings["aggregate_views"][view], default_view);
        }
        let views = serde_json::json!({
            "all": {"layout":"Table","sort":"Name","grouping":"Kind"},
            "active": {"layout":"List","sort":"State","grouping":"Folder"},
            "finished": {"layout":"Table","sort":"Source","grouping":"State"}
        });
        settings["max_download_kbps"] = serde_json::json!(2048);
        store
            .save(serde_json::from_value(settings).unwrap())
            .unwrap();
        for (view, name) in [
            (AggregateView::All, "all"),
            (AggregateView::Active, "active"),
            (AggregateView::Finished, "finished"),
        ] {
            store
                .save_aggregate_view(view, serde_json::from_value(views[name].clone()).unwrap())
                .unwrap();
        }
        let mut stale_settings = Preferences::default();
        stale_settings.theme = Theme::Dark;
        stale_settings.max_download_kbps = 2048;
        store.save(stale_settings).unwrap();
        let restored =
            serde_json::to_value(PreferenceStore::open(&directory).unwrap().get()).unwrap();
        assert_eq!(restored["aggregate_views"], views);
        assert_eq!(restored["theme"], "dark");
        assert_eq!(restored["max_download_kbps"], 2048);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_aggregate_save_does_not_replace_memory_or_disk_and_can_retry() {
        let directory =
            std::env::temp_dir().join(format!("opul-view-failure-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        store.save(store.get()).unwrap();
        let before = fs::read(&store.path).unwrap();
        let temporary = store.path.with_extension("json.tmp");
        fs::create_dir(&temporary).unwrap();
        let display = DisplayPreferences {
            layout: MachineLayout::Table,
            ..DisplayPreferences::default()
        };
        assert!(
            store
                .save_aggregate_view(AggregateView::Active, display.clone())
                .is_err()
        );
        assert_eq!(
            store.get().aggregate_views.active,
            DisplayPreferences::default()
        );
        assert_eq!(fs::read(&store.path).unwrap(), before);
        fs::remove_dir(&temporary).unwrap();
        store
            .save_aggregate_view(AggregateView::Active, display.clone())
            .unwrap();
        let restored = PreferenceStore::open(&directory).unwrap().get();
        assert_eq!(restored.aggregate_views.active, display);
        assert_eq!(restored.aggregate_views.all, DisplayPreferences::default());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn preferences_persist_and_invalid_values_do_not_replace_them() {
        let directory =
            std::env::temp_dir().join(format!("opul-preferences-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&directory).unwrap();
        let store = PreferenceStore::open(&directory).unwrap();
        let mut preferences = store.get();
        preferences.max_download_kbps = 2048;
        store.save(preferences.clone()).unwrap();
        assert_eq!(
            PreferenceStore::open(&directory)
                .unwrap()
                .get()
                .max_download_kbps,
            2048
        );
        preferences.concurrent_downloads = 0;
        assert!(store.save(preferences).is_err());
        assert_eq!(store.get().concurrent_downloads, 3);
        fs::remove_dir_all(directory).unwrap();
    }
}

#[cfg(test)]
#[path = "preferences/expansion_tests.rs"]
mod expansion_tests;

#[cfg(test)]
mod setup_tests;
