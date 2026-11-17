//! Presentation locations over the authoritative library projection.
use opul_core::{MachineDownload, MachineDownloadState, MachineSnapshot, MachineWorkspace};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Location {
    All,
    InProgress,
    Completed,
    Workspace(String),
    Folder(String, String),
}

impl Location {
    pub fn workspace<'a>(&self, snapshot: &'a MachineSnapshot) -> Option<&'a MachineWorkspace> {
        let id = match self {
            Self::Workspace(id) | Self::Folder(id, _) => id,
            _ => return None,
        };
        snapshot.workspaces.iter().find(|w| &w.id == id)
    }

    pub fn includes(&self, workspace: &MachineWorkspace, download: &MachineDownload) -> bool {
        use MachineDownloadState::*;
        match self {
            Self::All => true,
            Self::InProgress => matches!(
                download.state,
                WaitingForVideoTools
                    | Queued
                    | Starting
                    | Fetching
                    | Processing
                    | Verifying
                    | Pausing
            ),
            Self::Completed => matches!(
                download.state,
                Finished | Seeding | NotSeeding | CheckingFiles
            ),
            Self::Workspace(id) => &workspace.id == id,
            Self::Folder(id, folder) => &workspace.id == id && &download.folder_id == folder,
        }
    }

    pub fn normalize(&mut self, snapshot: &MachineSnapshot) {
        match self {
            Self::Workspace(id) | Self::Folder(id, _)
                if !snapshot.workspaces.iter().any(|w| &w.id == id) =>
            {
                *self = Self::All
            }
            Self::Folder(id, folder) => {
                let workspace = snapshot.workspaces.iter().find(|w| &w.id == id).unwrap();
                if workspace.folders.len() <= 1
                    || !workspace.folders.iter().any(|f| &f.id == folder)
                {
                    *self = Self::Workspace(id.clone());
                }
            }
            _ => {}
        }
    }
}
