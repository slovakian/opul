use crate::backend::{Client, Command, DownloadAction, Event};
use crate::navigation::Location;
use crate::preferences::{Expansion, Startup};
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
    Folder(String, String),
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
    pub location: Location,
    pub expansion: Expansion,
    pub navigation_open: bool,
    pub workspace_menu: Option<String>,
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
            location: Location::All,
            expansion: Expansion::default(),
            navigation_open: false,
            workspace_menu: None,
            error: None,
            pending: None,
            look: Look::default(),
            reduced_motion: false,
            quitting: false,
        }
    }

    pub fn workspace(&self) -> Option<&MachineWorkspace> {
        let snapshot = self.snapshot.as_ref()?;
        self.location.workspace(snapshot).or_else(|| {
            // The baseline has an unambiguous contextual destination with one Workspace.
            (snapshot.workspaces.len() == 1).then(|| &snapshot.workspaces[0])
        })
    }

    pub fn navigate(&mut self, location: Location) {
        self.location = location;
        if let Some(snapshot) = &self.snapshot {
            self.location.normalize(snapshot);
        }
        self.sync_destination();
        self.navigation_open = false;
        self.workspace_menu = None;
    }

    fn sync_destination(&mut self) {
        self.selected_folder = self
            .workspace()
            .and_then(|w| {
                if let Location::Folder(_, folder) = &self.location {
                    w.folders.iter().find(|f| &f.id == folder)
                } else {
                    w.folders.iter().find(|f| f.is_default)
                }
            })
            .map(|f| f.id.clone());
    }

    pub fn count(&self, location: &Location) -> usize {
        self.snapshot.as_ref().map_or(0, |s| {
            s.workspaces
                .iter()
                .map(|w| {
                    w.downloads
                        .iter()
                        .filter(|d| location.includes(w, d))
                        .count()
                })
                .sum()
        })
    }

    pub fn breadcrumb(&self) -> String {
        match &self.location {
            Location::All => "All".into(),
            Location::InProgress => "In progress".into(),
            Location::Completed => "Completed".into(),
            Location::Workspace(_) => self
                .workspace()
                .map_or_else(|| "All".into(), |w| w.name.clone()),
            Location::Folder(_, id) => self.workspace().map_or_else(
                || "All".into(),
                |w| {
                    format!(
                        "{} › {}",
                        w.name,
                        w.folders
                            .iter()
                            .find(|f| &f.id == id)
                            .map_or("", |f| &f.name)
                    )
                },
            ),
        }
    }

    pub fn toggle_expansion(&mut self, id: String) {
        self.expansion.toggle(id);
        self.save_expansion();
    }

    pub fn set_startup(&mut self, startup: Startup) {
        self.expansion.startup(startup);
        self.save_expansion();
    }

    fn save_expansion(&mut self) {
        if let Some((generation, preferences)) = self.expansion.write()
            && let Err(error) = self.client.save_preferences(generation, preferences)
        {
            self.expansion.saved(generation, Err(error));
        }
    }

    pub fn retry_expansion(&mut self) {
        if self.expansion.loaded {
            self.save_expansion();
        } else if let Err(error) = self.client.read_preferences() {
            self.expansion.error = Some(error);
        }
    }

    fn clean_expansion(&mut self) {
        if let Some(snapshot) = &self.snapshot {
            self.expansion
                .retain(&snapshot.workspaces.iter().map(|w| w.id.clone()).collect());
        }
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
                Event::Preferences(result) => {
                    self.expansion.loaded(result);
                    self.clean_expansion();
                    self.save_expansion();
                }
                Event::PreferencesSaved(generation, result) => {
                    self.expansion.saved(generation, result)
                }
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
        let snapshot = self.snapshot.as_ref().unwrap();
        self.location.normalize(snapshot);
        if self
            .workspace_menu
            .as_ref()
            .is_some_and(|id| !snapshot.workspaces.iter().any(|w| &w.id == id))
        {
            self.workspace_menu = None;
        }
        self.sync_destination();
        // Telemetry does not initiate writes. Only confirmed membership changes do.
        let before = self.expansion.desired.clone();
        self.clean_expansion();
        if before != self.expansion.desired {
            self.save_expansion();
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
            Purpose::Folder(workspace, id) => {
                if self.snapshot.as_ref().is_some_and(|s| {
                    s.workspaces
                        .iter()
                        .any(|w| w.id == workspace && w.folders.iter().any(|f| f.id == id))
                }) {
                    self.navigate(Location::Folder(workspace.clone(), id));
                    self.expansion.set(workspace, true);
                    self.save_expansion();
                }
            }
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
            self.error = Some("Choose a Workspace or Saved Folder before downloading.".into());
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
        let Some(workspace) = self.workspace() else {
            return;
        };
        let workspace_id = workspace.id.clone();
        self.choose_folder_for(workspace_id, tx);
    }

    pub fn choose_folder_for(&mut self, workspace_id: String, tx: &mut Tx) {
        if self.busy() {
            return;
        }
        tx.open_folder(
            rune::FileDialogOptions {
                title: "Add a Saved Folder".into(),
                ..Default::default()
            },
            move |state: &mut Self, result, _| match result {
                Ok(Some(path)) => state.add_folder(&workspace_id, path),
                Ok(None) => {}
                Err(error) => state.error = Some(error.to_string()),
            },
        );
    }

    fn add_folder(&mut self, workspace_id: &str, path: PathBuf) {
        if self.busy() {
            return;
        }
        let Some(workspace) = self
            .snapshot
            .as_ref()
            .and_then(|s| s.workspaces.iter().find(|w| w.id == workspace_id))
        else {
            return;
        };
        if let Some(folder) = workspace
            .folders
            .iter()
            .find(|f| std::path::Path::new(&f.path) == path)
        {
            self.navigate(Location::Folder(workspace.id.clone(), folder.id.clone()));
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
        self.send(
            request_id,
            Purpose::Folder(workspace_id.into(), id),
            Command::Folder(request),
        );
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
