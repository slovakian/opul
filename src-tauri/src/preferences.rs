use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub theme: Theme,
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

    pub fn save(&self, preferences: Preferences) -> Result<(), String> {
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
        let temporary = self.path.with_extension("json.tmp");
        let mut options = OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let bytes = serde_json::to_vec_pretty(&preferences).map_err(|error| error.to_string())?;
        let mut file = options
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        file.write_all(&bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| error.to_string())?;
        fs::rename(&temporary, &self.path).map_err(|error| error.to_string())?;
        *value = preferences;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
