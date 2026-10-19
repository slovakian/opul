use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_SCHEMA_VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StateEnvelope<T> {
    schema_version: u32,
    items: T,
}

pub(crate) struct LoadedState<T> {
    pub(crate) items: T,
    pub(crate) recovery_notice: Option<String>,
}

pub(crate) fn load_state<T>(
    path: &Path,
    label: &str,
    validate: impl Fn(&T) -> Result<(), String>,
) -> Result<LoadedState<T>, String>
where
    T: Default + DeserializeOwned,
{
    if !path.exists() {
        return Ok(LoadedState {
            items: T::default(),
            recovery_notice: None,
        });
    }

    let bytes = fs::read(path)
        .map_err(|error| format!("could not read {label} at {}: {error}", path.display()))?;
    match decode_state(&bytes, &validate) {
        Ok(items) => Ok(LoadedState {
            items,
            recovery_notice: None,
        }),
        Err(load_error) => {
            let corrupt_path = preserve_corrupt_file(path)?;
            let backup_path = backup_path(path);
            if let Ok(backup_bytes) = fs::read(&backup_path)
                && let Ok(items) = decode_state(&backup_bytes, &validate)
            {
                atomic_write(path, &backup_bytes)?;
                return Ok(LoadedState {
                    items,
                    recovery_notice: Some(format!(
                        "Recovered {label} from {} after preserving incompatible state as {}.",
                        file_name(&backup_path),
                        file_name(&corrupt_path)
                    )),
                });
            }
            Ok(LoadedState {
                items: T::default(),
                recovery_notice: Some(format!(
                    "Could not load {label} ({load_error}). Preserved it as {} and started with empty state.",
                    file_name(&corrupt_path)
                )),
            })
        }
    }
}

pub(crate) fn save_state<T>(
    path: &Path,
    items: &T,
    validate: impl Fn(&T) -> Result<(), String>,
) -> Result<(), String>
where
    T: Serialize + DeserializeOwned,
{
    validate(items)?;
    let envelope = StateEnvelope {
        schema_version: STATE_SCHEMA_VERSION,
        items,
    };
    let bytes = serde_json::to_vec_pretty(&envelope)
        .map_err(|error| format!("could not encode state for {}: {error}", path.display()))?;

    if path.exists()
        && let Ok(existing) = fs::read(path)
        && decode_state(&existing, &validate).is_ok()
    {
        atomic_write(&backup_path(path), &existing)?;
    }
    atomic_write(path, &bytes)
}

pub(crate) fn protect_json_file(
    path: &Path,
    label: &str,
    validate: impl Fn(&serde_json::Value) -> bool,
    empty_state: &[u8],
) -> Result<Option<String>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read {label} at {}: {error}", path.display()))?;
    if decode_json(&bytes, &validate).is_ok() {
        atomic_write(&backup_path(path), &bytes)?;
        return Ok(None);
    }

    let corrupt_path = preserve_corrupt_file(path)?;
    let backup_path = backup_path(path);
    if let Ok(backup_bytes) = fs::read(&backup_path)
        && decode_json(&backup_bytes, &validate).is_ok()
    {
        atomic_write(path, &backup_bytes)?;
        return Ok(Some(format!(
            "Recovered {label} from {} after preserving incompatible state as {}.",
            file_name(&backup_path),
            file_name(&corrupt_path)
        )));
    }

    atomic_write(path, empty_state)?;
    Ok(Some(format!(
        "Could not recover {label}. Preserved it as {} and started with empty state.",
        file_name(&corrupt_path)
    )))
}

fn decode_state<T>(bytes: &[u8], validate: &impl Fn(&T) -> Result<(), String>) -> Result<T, String>
where
    T: DeserializeOwned,
{
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| format!("invalid JSON: {error}"))?;
    let items = if value.get("schemaVersion").is_some() {
        let envelope: StateEnvelope<T> = serde_json::from_value(value)
            .map_err(|error| format!("invalid state envelope: {error}"))?;
        if envelope.schema_version != STATE_SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema version {}",
                envelope.schema_version
            ));
        }
        envelope.items
    } else {
        serde_json::from_value(value).map_err(|error| format!("invalid legacy state: {error}"))?
    };
    validate(&items)?;
    Ok(items)
}

fn decode_json(bytes: &[u8], validate: &impl Fn(&serde_json::Value) -> bool) -> Result<(), String> {
    let value = serde_json::from_slice(bytes).map_err(|error| format!("invalid JSON: {error}"))?;
    if validate(&value) {
        Ok(())
    } else {
        Err("unexpected JSON structure".to_string())
    }
}

fn preserve_corrupt_file(path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    let corrupt_path = path.with_file_name(format!("{stem}.corrupt.{timestamp}.json"));
    fs::copy(path, &corrupt_path).map_err(|error| {
        format!(
            "could not preserve incompatible state from {} to {}: {error}",
            path.display(),
            corrupt_path.display()
        )
    })?;
    set_file_permissions_0600(&corrupt_path)?;
    File::open(&corrupt_path)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("could not sync {}: {error}", corrupt_path.display()))?;
    Ok(corrupt_path)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_file_name(format!(
        ".{}.tmp.{}.{}",
        file_name(path),
        std::process::id(),
        timestamp
    ));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| format!("could not create {}: {error}", temporary.display()))?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
        set_file_permissions_0600(&temporary)?;
        fs::rename(&temporary, path).map_err(|error| {
            format!(
                "could not replace {} with {}: {error}",
                path.display(),
                temporary.display()
            )
        })?;
        if let Some(parent) = path.parent() {
            File::open(parent)
                .and_then(|directory| directory.sync_all())
                .map_err(|error| format!("could not sync {}: {error}", parent.display()))?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state file")
        .to_string()
}

#[cfg(unix)]
fn set_file_permissions_0600(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("could not secure {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn set_file_permissions_0600(_path: &Path) -> Result<(), String> {
    Ok(())
}
