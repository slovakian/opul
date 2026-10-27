use opul_core::{MachineGrouping, MachineLayout, MachineSort};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
    pub aggregate_views: AggregateViews,
    pub aggregate_destination: AggregateDestination,
    pub max_download_kbps: u32,
    pub max_upload_kbps: u32,
    pub concurrent_downloads: u32,
    pub listen_port: u16,
    pub randomize_port: bool,
    pub enable_dht: bool,
    pub max_connections: u32,
    pub yt_dlp_path: Option<String>,
    pub ffmpeg_path: Option<String>,
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
            aggregate_views: AggregateViews::default(),
            aggregate_destination: AggregateDestination::default(),
            max_download_kbps: 0,
            max_upload_kbps: 0,
            concurrent_downloads: 3,
            listen_port: 6881,
            randomize_port: true,
            enable_dht: true,
            max_connections: 200,
            yt_dlp_path: None,
            ffmpeg_path: None,
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
        // Settings drafts do not own aggregate display preferences.
        preferences.aggregate_views = value.aggregate_views.clone();
        self.persist(&preferences)?;
        *value = preferences.clone();
        Ok(preferences)
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

    // Both callers hold the value lock until the atomic write has completed.
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
