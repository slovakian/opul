//! Only the worker calls the core. The UI consumes its existing projection publisher.
use crate::paths::Paths;
use crate::preferences::Preferences;
use opul_core::{
    MachineCommandReceipt, MachineDownloadCommand, MachineFolderRequest, MachineMediaRequest,
    MachineSessionState, MachineSnapshot, OpulStateMachine,
};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver, Sender},
};
use std::thread::{self, JoinHandle};

#[derive(Clone, Copy, Debug)]
pub enum DownloadAction {
    Pause,
    Resume,
    Cancel,
    Retry,
    Remove,
}

pub enum Command {
    Add(MachineMediaRequest),
    Folder(MachineFolderRequest),
    Download(MachineDownloadCommand, DownloadAction),
    Recover,
}

impl Command {
    fn execute(self, machine: &OpulStateMachine) -> Result<MachineCommandReceipt, String> {
        let result = match self {
            Self::Add(request) => machine.add_direct_media(request),
            Self::Folder(request) => machine.add_folder(request, false),
            Self::Recover => machine.acknowledge_recovery(),
            Self::Download(request, action) => match action {
                DownloadAction::Pause => machine.pause_download(request),
                DownloadAction::Resume => machine.resume_download(request),
                DownloadAction::Cancel => machine.cancel_download(request),
                DownloadAction::Retry => machine.retry_download(request),
                // Baseline Remove from list retains files. Trash is a separate future action.
                DownloadAction::Remove => machine.remove_downloads(
                    request.request_id,
                    request.expected_revision,
                    vec![request.download_id],
                    false,
                ),
            },
        };
        result.map_err(|e| e.to_string())
    }
}

pub enum Event {
    Snapshot(MachineSnapshot),
    Settled(String, Result<MachineCommandReceipt, String>),
    Failed(String),
    Preferences(Result<Preferences, String>),
    PreferencesSaved(u64, Result<(), String>),
}

enum Work {
    Execute(String, Box<Command>),
    Shutdown,
    ReadPreferences,
    SavePreferences(u64, Preferences),
}

type ShutdownResult = Arc<(Mutex<Option<Result<MachineSnapshot, String>>>, Condvar)>;

#[derive(Clone)]
pub struct Client {
    commands: Sender<Work>,
    stopped: ShutdownResult,
}

impl Client {
    pub fn read_preferences(&self) -> Result<(), String> {
        self.commands
            .send(Work::ReadPreferences)
            .map_err(|_| "The library worker stopped.".into())
    }
    pub fn save_preferences(
        &self,
        generation: u64,
        preferences: Preferences,
    ) -> Result<(), String> {
        self.commands
            .send(Work::SavePreferences(generation, preferences))
            .map_err(|_| "The library worker stopped.".into())
    }
    pub fn execute(&self, id: String, command: Command) -> Result<(), String> {
        self.commands
            .send(Work::Execute(id, Box::new(command)))
            .map_err(|_| "The library worker stopped.".into())
    }
    pub fn shutdown(&self) {
        let _ = self.commands.send(Work::Shutdown);
    }
    pub fn wait_for_shutdown(&self) -> Result<MachineSnapshot, String> {
        self.shutdown();
        let mut result = self.stopped.0.lock().unwrap_or_else(|e| e.into_inner());
        while result.is_none() {
            result = self
                .stopped
                .1
                .wait(result)
                .unwrap_or_else(|e| e.into_inner());
        }
        result.as_ref().expect("completed shutdown").clone()
    }
}

pub struct Backend {
    pub client: Client,
    worker: JoinHandle<Result<MachineSnapshot, String>>,
}

impl Backend {
    pub fn start(
        paths: Paths,
        events: Sender<Event>,
        wake: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Self, String> {
        let (sender, receiver) = mpsc::channel();
        let stopped: ShutdownResult = Arc::new((Mutex::new(None), Condvar::new()));
        let client = Client {
            commands: sender,
            stopped: stopped.clone(),
        };
        let worker = thread::Builder::new()
            .name("opul-rune-commands".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    run(paths, &events, &wake, receiver)
                }))
                .unwrap_or_else(|_| Err("The library worker panicked.".into()))
                .and_then(|snapshot| {
                    if let Some(path) = std::env::var_os("OPUL_RUNE_STATE_OUTPUT") {
                        let bytes =
                            serde_json::to_vec_pretty(&snapshot).map_err(|e| e.to_string())?;
                        std::fs::write(path, bytes).map_err(|e| e.to_string())?;
                    }
                    Ok(snapshot)
                });
                if let Err(error) = &result {
                    let _ = events.send(Event::Failed(error.clone()));
                    wake();
                }
                *stopped.0.lock().unwrap_or_else(|e| e.into_inner()) = Some(result.clone());
                stopped.1.notify_all();
                result
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { client, worker })
    }

    /// Called after Rune's orderly event-loop return, for Quit AND last-window close.
    /// The process cannot terminate until the core and projection worker have joined.
    pub fn finish(self) -> Result<MachineSnapshot, String> {
        self.client.shutdown();
        self.worker
            .join()
            .map_err(|_| "The library worker panicked.".to_string())?
    }
}

fn run(
    paths: Paths,
    events: &Sender<Event>,
    wake: &Arc<dyn Fn() + Send + Sync>,
    commands: Receiver<Work>,
) -> Result<MachineSnapshot, String> {
    let machine = paths.open()?;
    let stop = Arc::new(AtomicBool::new(false));
    let publisher = {
        let machine = machine.clone();
        let stop = stop.clone();
        let events = events.clone();
        let wake = wake.clone();
        thread::Builder::new()
            .name("opul-rune-projections".into())
            .spawn(move || {
                let mut current = machine.snapshot();
                loop {
                    let boot = current.boot_id.clone();
                    let sequence = current.sequence;
                    if events.send(Event::Snapshot(current)).is_err() {
                        break;
                    }
                    wake();
                    loop {
                        if stop.load(Ordering::Acquire) {
                            return;
                        }
                        current = machine.wait_for_snapshot(boot.clone(), sequence, 250);
                        if current.boot_id != boot || current.sequence > sequence {
                            break;
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?
    };
    let preferences_path = paths.data.join("navigation-preferences.json");
    let _ = events.send(Event::Preferences(Preferences::read(&preferences_path)));
    wake();
    while let Ok(work) = commands.recv() {
        let event = match work {
            Work::Execute(id, command) => Event::Settled(id, command.execute(&machine)),
            Work::ReadPreferences => Event::Preferences(Preferences::read(&preferences_path)),
            Work::SavePreferences(generation, preferences) => {
                Event::PreferencesSaved(generation, preferences.write(&preferences_path))
            }
            Work::Shutdown => break,
        };
        let _ = events.send(event);
        wake();
    }
    let shutdown = machine.begin_shutdown().map_err(|e| e.to_string());
    stop.store(true, Ordering::Release);
    publisher
        .join()
        .map_err(|_| "The projection publisher panicked.".to_string())?;
    shutdown?;
    let snapshot = machine.snapshot();
    if snapshot.session != MachineSessionState::Stopped {
        return Err(format!("Shutdown did not settle: {:?}", snapshot.session));
    }
    Ok(snapshot)
}
