mod core;

use super::{
    AppProjection, AppState, BootId, CheckpointLoad, CheckpointStore, CommandEnvelope,
    CommandReceipt, Decision, EffectId, EffectStatus, Fact, PendingEffect, RecoveryItem,
    RuntimeOverlay, SessionPhase, decide, decide_fact, evolve, import_legacy_library, project,
};
use fs2::FileExt;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub(crate) struct ApplicationActor {
    sender: mpsc::Sender<Message>,
    projection: Arc<ProjectionPublisher>,
    thread: Option<JoinHandle<()>>,
    _lock: StateDirectoryLock,
}

enum Message {
    ProposeWorkspaceAppearance(super::WorkspaceId, mpsc::Sender<super::WorkspaceAppearance>),
    Command(
        Box<CommandEnvelope>,
        mpsc::Sender<Result<CommandReceipt, ActorError>>,
    ),
    Fact(Fact, mpsc::Sender<Result<u64, ActorError>>),
    Telemetry(super::DownloadId, super::TransferTelemetry),
    VideoUpdateAvailable(bool),
    ClaimEffect(mpsc::Sender<Result<Option<PendingEffect>, ActorError>>),
    CompleteEffect {
        effect: EffectId,
        facts: Vec<Fact>,
        response: mpsc::Sender<Result<u64, ActorError>>,
    },
    ExecutionContext(
        super::DownloadId,
        mpsc::Sender<Result<Option<ExecutionContext>, ActorError>>,
    ),
    AllExecutionContexts(mpsc::Sender<Result<Vec<ExecutionContext>, ActorError>>),
    Stop,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecutionContext {
    pub(crate) download: super::Download,
    pub(crate) output_folder: String,
    pub(crate) organization: super::FileOrganization,
}

struct ActorCore {
    state: AppState,
    overlay: RuntimeOverlay,
    store: CheckpointStore,
    publisher: Arc<ProjectionPublisher>,
    writable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ActorError {
    #[error("the application state machine is not running")]
    Stopped,
    #[error("the application state machine is read-only: {0}")]
    ReadOnly(String),
    #[error("the command was rejected: {0}")]
    Rejected(String),
    #[error("the application state could not be committed: {0}")]
    Durability(String),
    #[error("the state directory is already in use")]
    AlreadyOpen,
    #[error("the state machine could not open: {0}")]
    Open(String),
}

impl ApplicationActor {
    pub(crate) fn open(
        support_directory: PathBuf,
        downloads_path: String,
    ) -> Result<Self, ActorError> {
        fs::create_dir_all(&support_directory).map_err(|error| {
            ActorError::Open(format!(
                "could not create {}: {error}",
                support_directory.display()
            ))
        })?;
        let state_lock = StateDirectoryLock::acquire(&support_directory)?;
        let store = CheckpointStore::new(support_directory.join("application-state.json"));
        let (mut state, writable) = match store.load().map_err(ActorError::Open)? {
            CheckpointLoad::Missing => {
                let legacy = support_directory.join("library.json");
                let state = if legacy.exists() {
                    match import_legacy_library(&legacy, downloads_path.clone()) {
                        Ok(state) => state,
                        Err(import_error) => {
                            let corrupt = super::checkpoint::preserve(&legacy).ok();
                            let mut fresh = AppState::fresh(downloads_path);
                            fresh.recovery.push(RecoveryItem {
                                key: "legacy-library-invalid".to_string(),
                                summary: "Legacy download library could not be imported."
                                    .to_string(),
                                detail: match corrupt {
                                    Some(path) => {
                                        format!("{import_error}. Preserved as {}.", path.display())
                                    }
                                    None => import_error,
                                },
                            });
                            fresh
                        }
                    }
                } else {
                    AppState::fresh(downloads_path)
                };
                (state, true)
            }
            CheckpointLoad::Current(state) => (state, true),
            CheckpointLoad::RecoveredFromBackup {
                mut state,
                preserved_primary,
            } => {
                state.recovery.push(RecoveryItem {
                    key: "checkpoint-backup-fallback".to_string(),
                    summary: "Application state was recovered from an older backup.".to_string(),
                    detail: format!(
                        "Recovery evidence is at {}. Transfers remain fenced until recovery is resolved.",
                        preserved_primary.display()
                    ),
                });
                (state, true)
            }
            CheckpointLoad::RecoveryRequired {
                preserved_primary,
                reason,
            } => {
                let mut state = AppState::fresh(downloads_path);
                state.session = SessionPhase::RecoveryRequired;
                state.recovery.push(RecoveryItem {
                    key: "checkpoint-unavailable".to_string(),
                    summary: "Application state needs recovery.".to_string(),
                    detail: match preserved_primary {
                        Some(path) => {
                            format!("{reason}. The primary was preserved at {}.", path.display())
                        }
                        None => reason,
                    },
                });
                (state, true)
            }
        };

        let appearance_migrated = state.migrate_workspace_appearances();
        let reconcile = !matches!(state.session, SessionPhase::RecoveryRequired);
        if writable && reconcile {
            // Opening never resumes blindly. Adapters open quiescent, then explicit
            // reconciliation moves the session to Running.
            state.session = SessionPhase::Reconciling;
            // A process death loses every in-memory effect lease. Re-delivery is safe because
            // adapters use stable Download/attempt identity and platform effects verify output
            // identity before acting.
            for effect in state.outbox.values_mut() {
                effect.status = EffectStatus::Pending;
            }
        }
        if writable && (reconcile || appearance_migrated) {
            store
                .commit(&state)
                .map_err(|error| ActorError::Open(error.to_string()))?;
        }

        let overlay = RuntimeOverlay {
            boot: Some(BootId::new()),
            ..RuntimeOverlay::default()
        };
        let publisher = Arc::new(ProjectionPublisher::new(project(&state, &overlay)));
        let (sender, receiver) = mpsc::channel();
        let actor_publisher = Arc::clone(&publisher);
        let thread = thread::Builder::new()
            .name("opul-application-state".to_string())
            .spawn(move || {
                ActorCore {
                    state,
                    overlay,
                    store,
                    publisher: actor_publisher,
                    writable,
                }
                .run(receiver);
            })
            .map_err(|error| ActorError::Open(format!("could not start state actor: {error}")))?;

        Ok(Self {
            sender,
            projection: publisher,
            thread: Some(thread),
            _lock: state_lock,
        })
    }

    pub(crate) fn snapshot(&self) -> AppProjection {
        self.projection.snapshot()
    }

    pub(crate) fn propose_workspace_appearance(
        &self,
        id: super::WorkspaceId,
    ) -> Result<super::WorkspaceAppearance, ActorError> {
        let (response, receiver) = mpsc::channel();
        self.sender
            .send(Message::ProposeWorkspaceAppearance(id, response))
            .map_err(|_| ActorError::Stopped)?;
        receiver.recv().map_err(|_| ActorError::Stopped)
    }

    pub(crate) fn wait_for_projection(
        &self,
        after_boot: Option<BootId>,
        after_sequence: u64,
        timeout: Duration,
    ) -> AppProjection {
        self.projection.wait(after_boot, after_sequence, timeout)
    }

    pub(crate) fn dispatch(&self, command: CommandEnvelope) -> Result<CommandReceipt, ActorError> {
        request(&self.sender, |response| {
            Message::Command(Box::new(command), response)
        })
    }

    pub(crate) fn observe(&self, fact: Fact) -> Result<u64, ActorError> {
        request(&self.sender, |response| Message::Fact(fact, response))
    }

    pub(crate) fn observe_video_update(&self, available: bool) -> Result<(), ActorError> {
        self.sender
            .send(Message::VideoUpdateAvailable(available))
            .map_err(|_| ActorError::Stopped)
    }

    pub(crate) fn observe_telemetry(
        &self,
        download: super::DownloadId,
        telemetry: super::TransferTelemetry,
    ) -> Result<(), ActorError> {
        self.sender
            .send(Message::Telemetry(download, telemetry))
            .map_err(|_| ActorError::Stopped)
    }

    pub(crate) fn claim_effect(&self) -> Result<Option<PendingEffect>, ActorError> {
        request(&self.sender, Message::ClaimEffect)
    }

    pub(crate) fn complete_effect(
        &self,
        effect: EffectId,
        facts: Vec<Fact>,
    ) -> Result<u64, ActorError> {
        request(&self.sender, |response| Message::CompleteEffect {
            effect,
            facts,
            response,
        })
    }

    pub(crate) fn execution_context(
        &self,
        download: super::DownloadId,
    ) -> Result<Option<ExecutionContext>, ActorError> {
        request(&self.sender, |response| {
            Message::ExecutionContext(download, response)
        })
    }

    pub(crate) fn all_execution_contexts(&self) -> Result<Vec<ExecutionContext>, ActorError> {
        request(&self.sender, Message::AllExecutionContexts)
    }
}

impl Drop for ApplicationActor {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn append_effects(state: &mut AppState, plans: Vec<super::EffectPlan>) {
    for plan in plans {
        let effect = PendingEffect {
            id: EffectId::new(),
            created_revision: state.revision,
            attempt: plan.attempt,
            control_generation: plan.control_generation,
            status: EffectStatus::Pending,
            kind: plan.kind,
        };
        state.outbox.insert(effect.id, effect);
    }
}

fn request<T>(
    sender: &mpsc::Sender<Message>,
    message: impl FnOnce(mpsc::Sender<Result<T, ActorError>>) -> Message,
) -> Result<T, ActorError> {
    let (response_sender, response_receiver) = mpsc::channel();
    sender
        .send(message(response_sender))
        .map_err(|_| ActorError::Stopped)?;
    response_receiver.recv().map_err(|_| ActorError::Stopped)?
}

struct ProjectionPublisher {
    value: Mutex<AppProjection>,
    changed: Condvar,
}

impl ProjectionPublisher {
    fn new(value: AppProjection) -> Self {
        Self {
            value: Mutex::new(value),
            changed: Condvar::new(),
        }
    }

    fn snapshot(&self) -> AppProjection {
        self.value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    fn publish(&self, value: AppProjection) {
        *self
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
        self.changed.notify_all();
    }

    fn wait(
        &self,
        after_boot: Option<BootId>,
        after_sequence: u64,
        timeout: Duration,
    ) -> AppProjection {
        let current = self
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if current.boot != after_boot || current.sequence > after_sequence {
            return current.clone();
        }
        let (current, _) = self
            .changed
            .wait_timeout_while(current, timeout, |projection| {
                projection.boot == after_boot && projection.sequence <= after_sequence
            })
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        current.clone()
    }
}

struct StateDirectoryLock {
    file: File,
}

impl StateDirectoryLock {
    fn acquire(directory: &Path) -> Result<Self, ActorError> {
        let path = directory.join("application-state.lock");
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| {
                ActorError::Open(format!("could not open {}: {error}", path.display()))
            })?;
        file.try_lock_exclusive()
            .map_err(|_| ActorError::AlreadyOpen)?;
        Ok(Self { file })
    }
}

impl Drop for StateDirectoryLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests;
