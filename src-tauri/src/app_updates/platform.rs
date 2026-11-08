use super::{BoxFuture, Clock, InstallBlocker, Progress, UpdateSource};
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri_plugin_updater::{Error, Update, UpdaterExt};

/// Checks the channel manifest configured in `tauri.conf.json`, verified with its public key.
pub struct TauriSource {
    app: tauri::AppHandle,
    /// The update object from the last check; install needs its URL and signature.
    found: Mutex<Option<Update>>,
}

impl TauriSource {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self {
            app,
            found: Mutex::new(None),
        }
    }

    fn remember(&self, update: Option<Update>) {
        *self
            .found
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = update;
    }
}

/// The updater reports a manifest it can't fetch (such as a 404 before the first release
/// on a channel) as `ReleaseNotFound`. Nothing is published, so Opul is up to date.
pub(super) fn means_up_to_date(error: &Error) -> bool {
    matches!(error, Error::ReleaseNotFound)
}

pub(super) fn describe(error: &Error) -> String {
    match error {
        Error::Reqwest(_) | Error::Network(_) => {
            "Couldn't reach the update server. Check your connection and try again.".into()
        }
        Error::Minisign(_) | Error::Base64(_) | Error::SignatureUtf8(_) => {
            "The update didn't pass signature verification, so Opul didn't install it.".into()
        }
        Error::TargetNotFound(_) | Error::TargetsNotFound(_) => {
            "This update isn't available for your Mac yet.".into()
        }
        other => other.to_string(),
    }
}

impl UpdateSource for TauriSource {
    fn check(&self) -> BoxFuture<'_, Result<Option<String>, String>> {
        Box::pin(async move {
            let updater = self.app.updater().map_err(|error| describe(&error))?;
            match updater.check().await {
                Ok(Some(update)) => {
                    let version = update.version.clone();
                    self.remember(Some(update));
                    Ok(Some(version))
                }
                Ok(None) => {
                    self.remember(None);
                    Ok(None)
                }
                Err(error) if means_up_to_date(&error) => {
                    self.remember(None);
                    Ok(None)
                }
                Err(error) => Err(describe(&error)),
            }
        })
    }

    fn install(
        &self,
        version: String,
        mut progress: Progress,
    ) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            let update = self
                .found
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
                .filter(|update| update.version == version)
                .ok_or("Check for updates again, then download.")?;
            let mut downloaded = 0_u64;
            update
                .download_and_install(
                    |chunk, total| {
                        downloaded += chunk as u64;
                        progress(downloaded, total);
                    },
                    || {},
                )
                .await
                .map_err(|error| describe(&error))
        })
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn unix_now(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    }

    fn sleep(&self, duration: Duration) -> BoxFuture<'static, ()> {
        Box::pin(tokio::time::sleep(duration))
    }
}

/// The updater replaces the running `.app` in place, so it needs a real bundle on a
/// writable volume. A non-writable /Applications is left to the updater's admin prompt.
pub fn detect_install_blocker(
    executable: &Path,
    writable: impl Fn(&Path) -> bool,
) -> Option<InstallBlocker> {
    let bundle = executable
        .parent()
        .filter(|macos| macos.file_name().is_some_and(|name| name == "MacOS"))
        .and_then(Path::parent)
        .filter(|contents| contents.file_name().is_some_and(|name| name == "Contents"))
        .and_then(Path::parent)
        .filter(|bundle| {
            bundle
                .extension()
                .is_some_and(|extension| extension == "app")
        });
    let Some(bundle) = bundle else {
        return Some(InstallBlocker::Development);
    };
    if bundle
        .components()
        .any(|component| component.as_os_str() == "AppTranslocation")
    {
        return Some(InstallBlocker::Translocated);
    }
    let folder = bundle.parent()?;
    if folder.starts_with("/Volumes") && !writable(folder) {
        return Some(InstallBlocker::DiskImage);
    }
    None
}

/// Disk images mount read-only, so a probe file can't be created there.
pub fn folder_is_writable(folder: &Path) -> bool {
    let probe = folder.join(format!(".opul-update-probe-{}", std::process::id()));
    let created = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe)
        .is_ok();
    if created {
        let _ = std::fs::remove_file(&probe);
    }
    created
}
