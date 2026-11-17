use crate::backend::{Client, Command, DownloadAction, Event};
use crate::theme::Look;
use opul_core::{
    MachineCommandReceipt, MachineDownloadCommand, MachineFolderRequest, MachineFormat,
    MachineMediaItem, MachineMediaPolicy, MachineMediaRequest, MachineSessionState,
    MachineSnapshot, MachineWorkspace,
};
use rune::{TextEdit, Tx};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc::Receiver};
use uuid::Uuid;

pub type Wake = Arc<Mutex<Option<rune::Waker>>>;

enum Purpose {
    Submit,
    Folder(String),
    Control,
}
struct Pending {
    id: String,
    purpose: Purpose,
    receipt: Option<MachineCommandReceipt>,
}

pub struct State {
    pub client: Client,
    pub events: Receiver<Event>,
    pub wake: Wake,
    pub snapshot: Option<MachineSnapshot>,
    pub url: TextEdit,
    pub selected_folder: Option<String>,
    pub error: Option<String>,
    pub look: Look,
    pub reduced_motion: bool,
    pub quitting: bool,
    pending: Option<Pending>,
}

impl State {
    pub fn new(client: Client, events: Receiver<Event>, wake: Wake) -> Self {
        Self {
            client,
            events,
            wake,
            snapshot: None,
            url: TextEdit::new(""),
            selected_folder: None,
            error: None,
            pending: None,
            look: Look::default(),
            reduced_motion: false,
            quitting: false,
        }
    }

    pub fn workspace(&self) -> Option<&MachineWorkspace> {
        // The core creates the ordinary default Workspace on first open; never create a fixture.
        self.snapshot.as_ref()?.workspaces.first()
    }

    pub fn busy(&self) -> bool {
        self.quitting
            || self.pending.is_some()
            || self
                .snapshot
                .as_ref()
                .is_none_or(|s| s.session != MachineSessionState::Running)
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            changed = true;
            match event {
                Event::Snapshot(snapshot) => self.accept_snapshot(snapshot),
                Event::Failed(error) => {
                    self.error = Some(error);
                    self.pending = None;
                }
                Event::Settled(id, result) => {
                    if let Some(pending) = &mut self.pending
                        && pending.id == id
                    {
                        match result {
                            Ok(receipt) => pending.receipt = Some(receipt),
                            Err(error) => {
                                self.error = Some(error);
                                self.pending = None;
                            }
                        }
                    }
                }
            }
            self.settle();
        }
        changed
    }

    fn accept_snapshot(&mut self, snapshot: MachineSnapshot) {
        if let Some(current) = &self.snapshot
            && (snapshot.boot_id != current.boot_id || snapshot.sequence <= current.sequence)
        {
            return;
        }
        self.snapshot = Some(snapshot);
        if self.selected_folder.as_ref().is_none_or(|id| {
            self.workspace()
                .is_none_or(|w| !w.folders.iter().any(|f| &f.id == id))
        }) {
            self.selected_folder = self
                .workspace()
                .and_then(|w| {
                    w.folders
                        .iter()
                        .find(|f| f.is_default)
                        .or_else(|| w.folders.first())
                })
                .map(|f| f.id.clone());
        }
    }

    fn settle(&mut self) {
        let committed = self
            .pending
            .as_ref()
            .and_then(|p| p.receipt.as_ref())
            .zip(self.snapshot.as_ref())
            .is_some_and(|(r, s)| s.revision >= r.revision);
        if !committed {
            return;
        }
        match self
            .pending
            .take()
            .expect("committed pending command")
            .purpose
        {
            Purpose::Submit => self.url = TextEdit::new(""),
            Purpose::Folder(id) => self.selected_folder = Some(id),
            Purpose::Control => {}
        }
    }

    fn send(&mut self, id: String, purpose: Purpose, command: Command) {
        if self.quitting || self.pending.is_some() {
            return;
        }
        match self.client.execute(id.clone(), command) {
            Ok(()) => {
                self.error = None;
                self.pending = Some(Pending {
                    id,
                    purpose,
                    receipt: None,
                });
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn submit(&mut self) {
        if self.busy() {
            return;
        }
        let value = self.url.text.trim();
        let url = match url::Url::parse(value) {
            Ok(url)
                if matches!(url.scheme(), "http" | "https")
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none() =>
            {
                url
            }
            _ => {
                self.error = Some("Enter a direct HTTP or HTTPS file link.".into());
                return;
            }
        };
        let Some(workspace) = self.workspace() else {
            return;
        };
        let Some(folder_id) = &self.selected_folder else {
            return;
        };
        let request_id = Uuid::new_v4().to_string();
        let title = url
            .path_segments()
            .and_then(|mut parts| parts.next_back())
            .filter(|s| !s.is_empty())
            .unwrap_or("Download")
            .to_string();
        let request = MachineMediaRequest {
            request_id: request_id.clone(),
            expected_revision: None,
            workspace_id: workspace.id.clone(),
            folder_id: folder_id.clone(),
            format: MachineFormat::Original,
            policy: MachineMediaPolicy {
                download_subtitles: false,
                subtitle_languages: Vec::new(),
                embed_subtitles: false,
                write_thumbnail: false,
                write_metadata: false,
                filename_template: "%(title)s.%(ext)s".into(),
                user_agent: None,
            },
            items: vec![MachineMediaItem {
                url: url.to_string(),
                title,
                page_url: url.to_string(),
                thumbnail_url: None,
                extractor: None,
                playlist_index: None,
            }],
        };
        self.send(request_id, Purpose::Submit, Command::Add(request));
    }

    pub fn choose_folder(&mut self, tx: &mut Tx) {
        if self.busy() {
            return;
        }
        tx.open_folder(
            rune::FileDialogOptions {
                title: "Add a Saved Folder".into(),
                ..Default::default()
            },
            |state: &mut Self, result, _| match result {
                Ok(Some(path)) => state.add_folder(path),
                Ok(None) => {}
                Err(error) => state.error = Some(error.to_string()),
            },
        );
    }

    fn add_folder(&mut self, path: PathBuf) {
        if self.busy() {
            return;
        }
        let Some(workspace) = self.workspace() else {
            return;
        };
        if let Some(folder) = workspace
            .folders
            .iter()
            .find(|f| std::path::Path::new(&f.path) == path)
        {
            self.selected_folder = Some(folder.id.clone());
            return;
        }
        let id = Uuid::new_v4().to_string();
        let request_id = Uuid::new_v4().to_string();
        let request = MachineFolderRequest {
            request_id: request_id.clone(),
            expected_revision: None,
            workspace_id: workspace.id.clone(),
            folder_id: id.clone(),
            name: path
                .file_name()
                .unwrap_or(path.as_os_str())
                .to_string_lossy()
                .into_owned(),
            path: path.to_string_lossy().into_owned(),
        };
        self.send(request_id, Purpose::Folder(id), Command::Folder(request));
    }

    pub fn control(&mut self, download_id: String, action: DownloadAction) {
        if self.busy() {
            return;
        }
        let id = Uuid::new_v4().to_string();
        let request = MachineDownloadCommand {
            request_id: id.clone(),
            expected_revision: None,
            download_id,
        };
        self.send(id, Purpose::Control, Command::Download(request, action));
    }

    pub fn recover(&mut self) {
        self.send(
            Uuid::new_v4().to_string(),
            Purpose::Control,
            Command::Recover,
        );
    }

    pub fn shutdown(&mut self) {
        if !self.quitting {
            self.quitting = true;
            // AppKit may terminate after this hook. Await the worker's durable Stopped
            // projection here; doing it only after App::run returns misses native Quit.
            if let Err(error) = self.client.wait_for_shutdown() {
                eprintln!("Rune shutdown failed: {error}");
                std::process::exit(1);
            }
        }
    }
}
