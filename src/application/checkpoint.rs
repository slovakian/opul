use super::domain::{AppState, EffectStatus};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const CHECKPOINT_FORMAT: u32 = 1;

#[derive(Debug)]
pub(crate) struct CheckpointStore {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum CheckpointCommitError {
    #[error("{0}")]
    BeforeInstall(String),
    #[error("{0}")]
    AfterInstall(String),
}

impl CheckpointCommitError {
    pub(crate) fn installed(&self) -> bool {
        matches!(self, Self::AfterInstall(_))
    }
}

impl From<String> for CheckpointCommitError {
    fn from(message: String) -> Self {
        Self::BeforeInstall(message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CheckpointLoad {
    Missing,
    Current(AppState),
    RecoveredFromBackup {
        state: AppState,
        preserved_primary: PathBuf,
    },
    RecoveryRequired {
        preserved_primary: Option<PathBuf>,
        reason: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Envelope {
    format: u32,
    checksum: String,
    state: AppState,
}

impl CheckpointStore {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) fn load(&self) -> Result<CheckpointLoad, String> {
        if !self.path.exists() {
            let backup = self.backup_path();
            if !backup.exists() {
                return Ok(CheckpointLoad::Missing);
            }
            return match read_checkpoint(&backup) {
                Ok(mut state) => {
                    state.session = super::domain::SessionPhase::RecoveryRequired;
                    for effect in state.outbox.values_mut() {
                        effect.status = EffectStatus::Pending;
                    }
                    Ok(CheckpointLoad::RecoveredFromBackup {
                        state,
                        preserved_primary: backup,
                    })
                }
                Err(error) => Ok(CheckpointLoad::RecoveryRequired {
                    preserved_primary: preserve(&backup).ok(),
                    reason: format!(
                        "the application checkpoint is missing and its backup is invalid ({error})"
                    ),
                }),
            };
        }
        match read_checkpoint(&self.path) {
            Ok(state) => Ok(CheckpointLoad::Current(state)),
            Err(primary_error) => {
                let preserved_primary = preserve(&self.path).ok();
                let backup = self.backup_path();
                match read_checkpoint(&backup) {
                    Ok(mut state) => {
                        // Backup fallback can roll back acknowledged effects. Keep every effect
                        // fenced until adapter and output reconciliation has re-established truth.
                        state.session = super::domain::SessionPhase::RecoveryRequired;
                        for effect in state.outbox.values_mut() {
                            effect.status = EffectStatus::Pending;
                        }
                        Ok(CheckpointLoad::RecoveredFromBackup {
                            state,
                            preserved_primary: preserved_primary
                                .unwrap_or_else(|| self.path.clone()),
                        })
                    }
                Err(backup_error) => {
                    let preserved_backup = if backup.exists() {
                        preserve(&backup).ok()
                    } else {
                        None
                    };
                    Ok(CheckpointLoad::RecoveryRequired {
                        preserved_primary: preserved_primary.or(preserved_backup),
                        reason: format!(
                            "the application checkpoint is invalid ({primary_error}) and its backup is unavailable or invalid ({backup_error})"
                        ),
                    })
                }
                }
            }
        }
    }

    pub(crate) fn commit(&self, state: &AppState) -> Result<(), CheckpointCommitError> {
        state.validate().map_err(|error| error.to_string())?;
        let state_bytes = serde_json::to_vec(state)
            .map_err(|error| format!("could not encode the application state: {error}"))?;
        let envelope = Envelope {
            format: CHECKPOINT_FORMAT,
            checksum: checksum(&state_bytes),
            state: state.clone(),
        };
        let bytes = serde_json::to_vec_pretty(&envelope)
            .map_err(|error| format!("could not encode the checkpoint: {error}"))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }

        let temporary = temporary_path(&self.path);
        write_new(&temporary, &bytes)?;

        // Rotate the old validated primary first. A returned error must mean the authoritative
        // primary did not change; otherwise the actor's in-memory state could disagree with disk.
        // If the existing primary is invalid/corrupt, we do not rotate it into backup and do not fail;
        // instead we preserve existing backup and proceed to overwrite the corrupt file with valid state.
        let old_primary = if self.path.exists() {
            let bytes = fs::read(&self.path)
                .map_err(|error| format!("could not read {}: {error}", self.path.display()))?;
            if read_checkpoint_bytes(&bytes).is_ok() {
                Some(bytes)
            } else {
                None
            }
        } else {
            None
        };

        if let Some(old_primary) = old_primary
            && let Err(error) = atomic_replace(&self.backup_path(), &old_primary)
        {
            let _ = fs::remove_file(&temporary);
            return Err(error.into());
        }

        fs::rename(&temporary, &self.path).map_err(|error| {
            let _ = fs::remove_file(&temporary);
            format!(
                "could not install checkpoint {}: {error}",
                self.path.display()
            )
        })?;
        sync_parent(&self.path).map_err(CheckpointCommitError::AfterInstall)
    }

    fn backup_path(&self) -> PathBuf {
        self.path.with_extension("json.bak")
    }
}

fn read_checkpoint(path: &Path) -> Result<AppState, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("could not read {}: {error}", path.display()))?;
    read_checkpoint_bytes(&bytes)
}

fn read_checkpoint_bytes(bytes: &[u8]) -> Result<AppState, String> {
    let envelope: Envelope = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid checkpoint JSON: {error}"))?;
    if envelope.format != CHECKPOINT_FORMAT {
        return Err(format!("unsupported checkpoint format {}", envelope.format));
    }
    let state_bytes = serde_json::to_vec(&envelope.state)
        .map_err(|error| format!("could not verify checkpoint: {error}"))?;
    if checksum(&state_bytes) != envelope.checksum {
        return Err("checkpoint checksum mismatch".to_string());
    }
    envelope
        .state
        .validate()
        .map_err(|error| format!("checkpoint invariant failed: {error}"))?;
    Ok(envelope.state)
}

fn checksum(bytes: &[u8]) -> String {
    hex::encode(Sha1::digest(bytes))
}

pub(crate) fn preserve(path: &Path) -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("application-state.json");
    let target = path.with_file_name(format!("{name}.corrupt.{stamp}"));
    fs::copy(path, &target).map_err(|error| {
        format!(
            "could not preserve {} as {}: {error}",
            path.display(),
            target.display()
        )
    })?;
    secure(&target)?;
    File::open(&target)
        .and_then(|file| file.sync_all())
        .map_err(|error| format!("could not sync {}: {error}", target.display()))?;
    Ok(target)
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = temporary_path(path);
    write_new(&temporary, bytes)?;
    fs::rename(&temporary, path).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        format!("could not replace {}: {error}", path.display())
    })?;
    sync_parent(path)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create_new(true).write(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    secure(path)
}

fn temporary_path(path: &Path) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("state");
    path.with_file_name(format!(".{name}.tmp.{}.{stamp}", std::process::id()))
}

fn sync_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("could not sync {}: {error}", parent.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn secure(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| format!("could not secure {}: {error}", path.display()))
}

#[cfg(not(unix))]
fn secure(_path: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{EffectId, PendingEffect, SessionPhase};

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("opul-checkpoint-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn checkpoint_round_trips_with_checksum_and_backup() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path);
        let mut first = AppState::fresh("/tmp/Downloads".to_string());
        first.session = SessionPhase::Running;
        store.commit(&first).unwrap();
        let mut second = first.clone();
        second.revision = 1;
        store.commit(&second).unwrap();
        assert_eq!(store.load().unwrap(), CheckpointLoad::Current(second));
        assert!(store.backup_path().exists());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn backup_fallback_fences_all_execution() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path.clone());
        let mut first = AppState::fresh("/tmp/Downloads".to_string());
        first.outbox.insert(
            EffectId::new(),
            PendingEffect {
                id: EffectId::new(),
                created_revision: 0,
                attempt: None,
                control_generation: None,
                status: EffectStatus::InFlight,
                kind: super::super::domain::EffectKind::StopAllTransfers,
            },
        );
        // Fix the intentionally independently generated key before validation.
        let effect = first.outbox.values().next().unwrap().clone();
        first.outbox.clear();
        first.outbox.insert(effect.id, effect);
        store.commit(&first).unwrap();
        let mut second = first.clone();
        second.revision = 1;
        store.commit(&second).unwrap();
        fs::write(&path, b"not json").unwrap();
        let CheckpointLoad::RecoveredFromBackup { state, .. } = store.load().unwrap() else {
            panic!("expected backup recovery");
        };
        assert_eq!(state.session, SessionPhase::RecoveryRequired);
        assert!(
            state
                .outbox
                .values()
                .all(|effect| effect.status == EffectStatus::Pending)
        );
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn missing_primary_recovers_backup_instead_of_creating_fresh_state() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path.clone());
        let first = AppState::fresh("/tmp/Downloads".to_string());
        store.commit(&first).unwrap();
        let mut second = first.clone();
        second.revision = 1;
        store.commit(&second).unwrap();
        fs::remove_file(&path).unwrap();
        let CheckpointLoad::RecoveredFromBackup { state, .. } = store.load().unwrap() else {
            panic!("expected backup recovery");
        };
        assert_eq!(state.revision, first.revision);
        assert_eq!(state.session, SessionPhase::RecoveryRequired);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn commit_succeeds_and_overwrites_corrupt_primary() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path.clone());
        let first = AppState::fresh("/tmp/Downloads".to_string());
        store.commit(&first).unwrap();

        // Corrupt the primary file on disk.
        fs::write(&path, b"{ invalid json").unwrap();

        // A new commit must succeed and cleanly overwrite the corrupt primary,
        // without attempting to rotate corrupt bytes to backup.
        let mut second = first.clone();
        second.revision = 2;
        store.commit(&second).unwrap();

        let loaded = store.load().unwrap();
        assert_eq!(loaded, CheckpointLoad::Current(second));
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn corrupt_primary_and_corrupt_backup_requires_recovery() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path.clone());
        let backup = store.backup_path();

        fs::write(&path, b"corrupted primary").unwrap();
        fs::write(&backup, b"corrupted backup").unwrap();

        let outcome = store.load().unwrap();
        assert!(matches!(outcome, CheckpointLoad::RecoveryRequired { .. }));
        fs::remove_dir_all(folder).unwrap();
    }
}
