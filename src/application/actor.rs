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
    Command(
        Box<CommandEnvelope>,
        mpsc::Sender<Result<CommandReceipt, ActorError>>,
    ),
    Fact(Fact, mpsc::Sender<Result<u64, ActorError>>),
    Telemetry(super::DownloadId, super::TransferTelemetry),
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
                store
                    .commit(&state)
                    .map_err(|error| ActorError::Open(error.to_string()))?;
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

        if writable && !matches!(state.session, SessionPhase::RecoveryRequired) {
            // Opening never resumes blindly. Adapters open quiescent, then explicit
            // reconciliation moves the session to Running.
            state.session = SessionPhase::Reconciling;
            // A process death loses every in-memory effect lease. Re-delivery is safe because
            // adapters use stable Download/attempt identity and platform effects verify output
            // identity before acting.
            for effect in state.outbox.values_mut() {
                effect.status = EffectStatus::Pending;
            }
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

impl ActorCore {
    fn run(mut self, receiver: mpsc::Receiver<Message>) {
        while let Ok(message) = receiver.recv() {
            match message {
                Message::Command(command, response) => {
                    let _ = response.send(self.handle_command(*command));
                }
                Message::Fact(fact, response) => {
                    let _ = response.send(self.handle_fact(fact));
                }
                Message::Telemetry(download, telemetry) => {
                    if self.state.locate_download(download).is_some() {
                        let changed = self.overlay.telemetry.get(&download) != Some(&telemetry);
                        self.overlay.telemetry.insert(download, telemetry);
                        if changed {
                            self.publish();
                        }
                    }
                }
                Message::ClaimEffect(response) => {
                    let _ = response.send(self.handle_claim());
                }
                Message::CompleteEffect {
                    effect,
                    facts,
                    response,
                } => {
                    let _ = response.send(self.handle_completion(effect, facts));
                }
                Message::ExecutionContext(download, response) => {
                    let context =
                        self.state
                            .locate_download(download)
                            .and_then(|(workspace_id, item)| {
                                let workspace = self.state.workspace(workspace_id)?;
                                let folder = workspace
                                    .folders
                                    .iter()
                                    .find(|folder| folder.id == item.folder)?;
                                Some(ExecutionContext {
                                    download: item.clone(),
                                    output_folder: folder.path.clone(),
                                    organization: workspace.file_organization,
                                })
                            });
                    let _ = response.send(Ok(context));
                }
                Message::AllExecutionContexts(response) => {
                    let contexts = self
                        .state
                        .workspaces
                        .iter()
                        .flat_map(|workspace| {
                            workspace.downloads.iter().filter_map(|item| {
                                let folder = workspace
                                    .folders
                                    .iter()
                                    .find(|folder| folder.id == item.folder)?;
                                Some(ExecutionContext {
                                    download: item.clone(),
                                    output_folder: folder.path.clone(),
                                    organization: workspace.file_organization,
                                })
                            })
                        })
                        .collect();
                    let _ = response.send(Ok(contexts));
                }
                Message::Stop => break,
            }
        }
    }

    fn handle_command(&mut self, envelope: CommandEnvelope) -> Result<CommandReceipt, ActorError> {
        self.require_writable()?;
        if let Some(receipt) = self.state.receipts.get(&envelope.id).cloned() {
            return if receipt.fingerprint == envelope.fingerprint {
                Ok(receipt)
            } else {
                Err(ActorError::Rejected(
                    "the command ID was reused with different input".to_string(),
                ))
            };
        }
        let decision = decide(&self.state, &envelope)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        let created = decision.created_downloads.clone();
        let mut next = self.prepare_next(&decision)?;
        let receipt = CommandReceipt {
            command_id: envelope.id,
            fingerprint: envelope.fingerprint,
            revision: next.revision,
            created_downloads: created,
        };
        next.remember_receipt(receipt.clone());
        self.commit(next)?;
        Ok(receipt)
    }

    fn handle_fact(&mut self, fact: Fact) -> Result<u64, ActorError> {
        self.require_writable()?;
        let decision = decide_fact(&self.state, &fact)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        if decision.events.is_empty() && decision.effects.is_empty() {
            return Ok(self.state.revision);
        }
        let next = self.prepare_next(&decision)?;
        let revision = next.revision;
        self.commit(next)?;
        Ok(revision)
    }

    fn handle_claim(&mut self) -> Result<Option<PendingEffect>, ActorError> {
        self.require_writable()?;
        self.discard_obsolete_pending()?;
        let candidate = self
            .state
            .outbox
            .values()
            .find(|effect| {
                effect.status == EffectStatus::Pending
                    && super::effect_is_eligible(&self.state, effect)
                    && !self.target_in_flight(effect)
            })
            .cloned();
        let Some(candidate) = candidate else {
            return Ok(None);
        };
        let mut next = self.state.clone();
        if let Some(effect) = next.outbox.get_mut(&candidate.id) {
            effect.status = EffectStatus::InFlight;
        }
        next.revision += 1;
        self.commit(next)?;
        Ok(Some(candidate))
    }

    fn handle_completion(
        &mut self,
        effect_id: EffectId,
        facts: Vec<Fact>,
    ) -> Result<u64, ActorError> {
        self.require_writable()?;
        let Some(completed) = self.state.outbox.get(&effect_id).cloned() else {
            return Ok(self.state.revision);
        };
        let mut next = self.state.clone();
        next.outbox.remove(&effect_id);
        for fact in facts {
            let decision = decide_fact(&next, &fact)
                .map_err(|error| ActorError::Rejected(error.to_string()))?;
            for event in &decision.events {
                super::apply(&mut next, event);
            }
            append_effects(&mut next, decision.effects);
        }

        // If a control operation completed after being superseded, schedule convergence to the
        // latest desired state. This prevents an old pause or start from winning externally even
        // though its result is correctly ignored by the reducer.
        if let Some(target) = completed.target()
            && let Some((_, item)) = next.locate_download(target)
            && completed.control_generation != Some(item.control_generation)
        {
            let already_planned = next.outbox.values().any(|effect| {
                effect.target() == Some(target)
                    && effect.control_generation == Some(item.control_generation)
            });
            if !already_planned {
                let effect = PendingEffect {
                    id: EffectId::new(),
                    created_revision: next.revision + 1,
                    attempt: Some(item.attempt),
                    control_generation: Some(item.control_generation),
                    status: EffectStatus::Pending,
                    kind: super::EffectKind::ConvergeControl { download: target },
                };
                next.outbox.insert(effect.id, effect);
            }
        }
        next.revision += 1;
        let revision = next.revision;
        self.commit(next)?;
        Ok(revision)
    }

    fn prepare_next(&self, decision: &Decision) -> Result<AppState, ActorError> {
        let mut next = evolve(&self.state, decision)
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        append_effects(&mut next, decision.effects.clone());
        next.validate()
            .map_err(|error| ActorError::Rejected(error.to_string()))?;
        Ok(next)
    }

    fn commit(&mut self, next: AppState) -> Result<(), ActorError> {
        if let Err(error) = self.store.commit(&next) {
            if error.installed() {
                // The rename made `next` authoritative even though syncing the directory failed.
                // Keep memory aligned with the visible checkpoint before fencing further writes.
                self.state = next;
            }
            self.writable = false;
            self.state.session = SessionPhase::ReadOnlyDurabilityFault;
            self.state.recovery.push(RecoveryItem {
                key: "durability-fault".to_string(),
                summary: "Application state is read-only.".to_string(),
                detail: error.to_string(),
            });
            self.publish();
            return Err(ActorError::Durability(error.to_string()));
        }
        self.state = next;
        self.publish();
        Ok(())
    }

    fn publish(&mut self) {
        self.overlay.sequence = self.overlay.sequence.saturating_add(1);
        self.publisher.publish(project(&self.state, &self.overlay));
    }

    fn require_writable(&self) -> Result<(), ActorError> {
        if self.writable {
            Ok(())
        } else {
            Err(ActorError::ReadOnly(
                self.state
                    .recovery
                    .last()
                    .map(|item| item.detail.clone())
                    .unwrap_or_else(|| "recovery is required".to_string()),
            ))
        }
    }

    fn target_in_flight(&self, candidate: &PendingEffect) -> bool {
        let Some(target) = candidate.target() else {
            return false;
        };
        self.state.outbox.values().any(|effect| {
            effect.id != candidate.id
                && effect.target() == Some(target)
                && effect.status == EffectStatus::InFlight
        })
    }

    fn discard_obsolete_pending(&mut self) -> Result<(), ActorError> {
        let obsolete = self
            .state
            .outbox
            .values()
            .filter(|effect| {
                effect.status == EffectStatus::Pending
                    && super::effect_is_obsolete(&self.state, effect)
            })
            .map(|effect| effect.id)
            .collect::<Vec<_>>();
        if obsolete.is_empty() {
            return Ok(());
        }
        let mut next = self.state.clone();
        for id in obsolete {
            next.outbox.remove(&id);
        }
        next.revision += 1;
        self.commit(next)
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
mod tests {
    use super::*;
    use crate::application::{
        Command, Download, DownloadLifecycle, DownloadSource, MediaDownload, MediaFormat,
        MediaOrigin,
    };
    use uuid::Uuid;

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!("opul-actor-{}", Uuid::new_v4()));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn envelope(command: Command) -> CommandEnvelope {
        CommandEnvelope {
            id: super::super::CommandId::new(),
            fingerprint: Uuid::new_v4().to_string(),
            expected_revision: None,
            command,
        }
    }

    fn make_running(actor: &ApplicationActor) {
        actor.dispatch(envelope(Command::MarkReconciled)).unwrap();
    }

    #[test]
    fn actor_commits_intent_and_outbox_before_effect_claim() {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        make_running(&actor);
        let projection = actor.snapshot();
        let workspace = projection.workspaces[0].id;
        let saved_folder = projection.workspaces[0].default_folder;
        let download = Download {
            id: super::super::DownloadId::new(),
            folder: saved_folder,
            name: "Film".to_string(),
            attempt: 0,
            control_generation: 0,
            source: DownloadSource::Media(MediaDownload {
                origin: MediaOrigin::Direct {
                    url: "https://example.com/film.mp4".to_string(),
                    referer: None,
                },
                format: MediaFormat::Original,
                policy: crate::application::MediaPolicy::default(),
                legacy_binding: None,
                thumbnail_url: None,
                extractor: None,
            }),
            lifecycle: DownloadLifecycle::Queued,
            output: None,
        };
        let receipt = actor
            .dispatch(envelope(Command::AddDownload {
                workspace,
                download: Box::new(download.clone()),
            }))
            .unwrap();
        assert_eq!(receipt.created_downloads, vec![download.id]);
        drop(actor);

        let reopened =
            ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        // Starts stay fenced on reopen until reconciliation completes.
        assert!(reopened.claim_effect().unwrap().is_none());
        make_running(&reopened);
        assert!(matches!(
            reopened.claim_effect().unwrap().map(|effect| effect.kind),
            Some(super::super::EffectKind::EnsureTransfer { download: id }) if id == download.id
        ));
        drop(reopened);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn process_restart_releases_in_flight_effect_leases() {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        make_running(&actor);
        let projection = actor.snapshot();
        let download = Download {
            id: super::super::DownloadId::new(),
            folder: projection.workspaces[0].default_folder,
            name: "Restartable".to_string(),
            attempt: 0,
            control_generation: 0,
            source: DownloadSource::Media(MediaDownload {
                origin: MediaOrigin::Direct {
                    url: "https://example.com/restart.mp4".to_string(),
                    referer: None,
                },
                format: MediaFormat::Original,
                policy: crate::application::MediaPolicy::default(),
                legacy_binding: None,
                thumbnail_url: None,
                extractor: None,
            }),
            lifecycle: DownloadLifecycle::Queued,
            output: None,
        };
        actor
            .dispatch(envelope(Command::AddDownload {
                workspace: projection.workspaces[0].id,
                download: Box::new(download.clone()),
            }))
            .unwrap();
        assert!(actor.claim_effect().unwrap().is_some());
        drop(actor);

        let reopened =
            ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        make_running(&reopened);
        assert!(matches!(
            reopened.claim_effect().unwrap().map(|effect| effect.kind),
            Some(super::super::EffectKind::EnsureTransfer { download: id }) if id == download.id
        ));
        drop(reopened);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn command_retry_returns_the_original_receipt() {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        make_running(&actor);
        let projection = actor.snapshot();
        let command_id = super::super::CommandId::new();
        let workspace = crate::application::Workspace {
            id: super::super::WorkspaceId::new(),
            name: "Study".to_string(),
            icon: crate::application::WorkspaceIcon::Emoji("📚".to_string()),
            folders: vec![crate::application::SavedFolder {
                id: super::super::FolderId::new(),
                name: "Study".to_string(),
                path: "/tmp/Study".to_string(),
            }],
            default_folder: super::super::FolderId::new(),
            default_format: MediaFormat::Best,
            file_organization: crate::application::FileOrganization::Automatic,
            view: crate::application::WorkspaceView::default(),
            downloads: Vec::new(),
        };
        let mut workspace = workspace;
        workspace.default_folder = workspace.folders[0].id;
        let request = CommandEnvelope {
            id: command_id,
            fingerprint: "same".to_string(),
            expected_revision: Some(projection.revision),
            command: Command::CreateWorkspace { workspace },
        };
        let first = actor.dispatch(request.clone()).unwrap();
        let repeated = actor.dispatch(request).unwrap();
        assert_eq!(first, repeated);
        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn state_directory_allows_only_one_actor() {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        assert!(matches!(
            ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()),
            Err(ActorError::AlreadyOpen)
        ));
        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn corrupt_primary_with_backup_recovers_and_acknowledges_cleanly() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        let store = CheckpointStore::new(path.clone());
        let mut first = AppState::fresh("/tmp/Downloads".to_string());
        first.session = SessionPhase::Running;
        store.commit(&first).unwrap();
        let mut second = first.clone();
        second.revision = 1;
        store.commit(&second).unwrap();

        // Corrupt primary
        fs::write(&path, b"corrupted JSON").unwrap();

        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        assert_eq!(actor.snapshot().session, SessionPhase::RecoveryRequired);
        assert!(!actor.snapshot().recovery.is_empty());

        // Acknowledging recovery must succeed, transition to Reconciling, and clear recovery items
        let receipt = actor
            .dispatch(envelope(Command::AcknowledgeRecovery))
            .unwrap();
        assert_eq!(receipt.revision, first.revision + 1);
        assert_eq!(actor.snapshot().session, SessionPhase::Reconciling);
        assert!(actor.snapshot().recovery.is_empty());

        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn corrupt_primary_without_backup_starts_fresh_and_acknowledges_cleanly() {
        let folder = temp_dir();
        let path = folder.join("application-state.json");
        fs::write(&path, b"corrupted JSON without backup").unwrap();

        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        assert_eq!(actor.snapshot().session, SessionPhase::RecoveryRequired);
        assert!(!actor.snapshot().recovery.is_empty());

        // Must not be locked into read-only mode; acknowledging recovery must transition to Reconciling
        actor
            .dispatch(envelope(Command::AcknowledgeRecovery))
            .unwrap();
        assert_eq!(actor.snapshot().session, SessionPhase::Reconciling);
        assert!(actor.snapshot().recovery.is_empty());

        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn corrupt_legacy_library_starts_fresh_without_failing() {
        let folder = temp_dir();
        let legacy = folder.join("library.json");
        fs::write(&legacy, b"invalid legacy JSON").unwrap();

        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
        let snapshot = actor.snapshot();
        assert_eq!(snapshot.workspaces.len(), 1);
        assert_eq!(snapshot.workspaces[0].name, "Downloads");
        assert!(
            snapshot
                .recovery
                .iter()
                .any(|r| r.key == "legacy-library-invalid")
        );

        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }
}
