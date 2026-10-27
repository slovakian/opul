use super::{AppState, Workspace, WorkspaceIcon, WorkspaceId};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;
use uuid::Uuid;

/// Resolved values are saved, so editing a palette never changes a saved icon.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderTreatment {
    pub preset_id: String,
    pub version: u16,
    pub hue: u16,
    pub saturation: u16,
    pub brightness: u16,
}

pub(crate) static FOLDER_TREATMENTS: LazyLock<[FolderTreatment; 12]> = LazyLock::new(|| {
    serde_json::from_str(include_str!("folder-treatments.json"))
        .expect("the bundled folder palette must contain twelve valid treatments")
});

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceAppearance {
    /// Retained when a different emoji is selected, but applied only to folders.
    pub automatic_folder: Option<FolderTreatment>,
}

impl WorkspaceAppearance {
    pub(crate) fn is_unassigned(&self) -> bool {
        self.automatic_folder.is_none()
    }

    pub(crate) fn valid(&self) -> bool {
        self.automatic_folder.as_ref().is_none_or(|treatment| {
            !treatment.preset_id.is_empty()
                && treatment.preset_id.len() <= 80
                && treatment.version > 0
                && treatment.hue < 360
                && treatment.saturation <= 1000
                && (50..=150).contains(&treatment.brightness)
        })
    }
}

impl WorkspaceIcon {
    pub(crate) fn renders_as_folder(&self) -> bool {
        match self {
            Self::Symbol(_) => true,
            Self::Emoji(value) => value.trim().is_empty() || value == "📁" || value == "📁\u{fe0f}",
        }
    }
}

impl AppState {
    pub(crate) fn propose_folder_appearance(&self, id: WorkspaceId) -> WorkspaceAppearance {
        let treatment = FOLDER_TREATMENTS
            .iter()
            .min_by_key(|treatment| {
                let count = self
                    .workspaces
                    .iter()
                    .chain(
                        self.structural_changes
                            .workspace_deletions
                            .iter()
                            .filter_map(|p| p.replacement.as_ref()),
                    )
                    .filter_map(|workspace| workspace.appearance.automatic_folder.as_ref())
                    .filter(|saved| saved.preset_id == treatment.preset_id)
                    .count();
                // UUID v5 is stable across processes and platforms, unlike a runtime hasher.
                (count, Uuid::new_v5(&id.0, treatment.preset_id.as_bytes()))
            })
            .expect("the folder palette is nonempty")
            .clone();
        WorkspaceAppearance {
            automatic_folder: Some(treatment),
        }
    }

    pub(crate) fn assign_folder_appearance(&self, workspace: &mut Workspace) {
        if workspace.icon.renders_as_folder() && workspace.appearance.is_unassigned() {
            workspace.appearance = self.propose_folder_appearance(workspace.id);
        }
    }

    /// Called only during import/startup, in ID order rather than sidebar order.
    pub(crate) fn migrate_workspace_appearances(&mut self) -> bool {
        let mut ids = self.workspaces.iter().map(|w| w.id).collect::<Vec<_>>();
        ids.sort();
        let mut changed = false;
        for id in ids {
            let workspace = self.workspace(id).expect("workspace ID came from state");
            if workspace.icon.renders_as_folder() && workspace.appearance.is_unassigned() {
                let appearance = self.propose_folder_appearance(id);
                self.workspace_mut(id)
                    .expect("workspace ID came from state")
                    .appearance = appearance;
                changed = true;
            }
        }
        let mut pending = self.structural_changes.workspace_deletions.clone();
        pending.sort_by_key(|p| p.replacement.as_ref().map(|w| w.id));
        for deletion in pending {
            if let Some(mut replacement) = deletion.replacement {
                let previous = replacement.appearance.clone();
                self.assign_folder_appearance(&mut replacement);
                if replacement.appearance != previous {
                    self.structural_changes
                        .workspace_deletions
                        .iter_mut()
                        .find(|p| p.workspace == deletion.workspace)
                        .expect("pending deletion came from state")
                        .replacement = Some(replacement);
                    changed = true;
                }
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn library(count: u128) -> AppState {
        let mut state = AppState::fresh("/tmp/Downloads".into());
        let template = state.workspaces[0].clone();
        state.workspaces = (1..=count)
            .map(|id| {
                let mut workspace = template.clone();
                workspace.id = WorkspaceId(Uuid::from_u128(id));
                workspace.appearance = WorkspaceAppearance::default();
                workspace
            })
            .collect();
        state
    }

    #[test]
    fn twelve_unique_valid_resolved_treatments() {
        assert_eq!(
            FOLDER_TREATMENTS
                .iter()
                .map(|t| &t.preset_id)
                .collect::<BTreeSet<_>>()
                .len(),
            12
        );
        assert_eq!(
            FOLDER_TREATMENTS
                .iter()
                .map(|t| (t.hue, t.saturation, t.brightness))
                .collect::<BTreeSet<_>>()
                .len(),
            12
        );
        for treatment in FOLDER_TREATMENTS.iter() {
            assert!(
                WorkspaceAppearance {
                    automatic_folder: Some(treatment.clone())
                }
                .valid()
            );
        }
    }

    #[test]
    fn migration_is_balanced_deterministic_and_preserves_workspace_data() {
        let mut state = library(26);
        state.workspaces[0].icon = WorkspaceIcon::Symbol("arrow.down.circle".into());
        state.workspaces[1].icon = WorkspaceIcon::Emoji(" ".into());
        state.workspaces[2].icon = WorkspaceIcon::Emoji("📁\u{fe0f}".into());
        state.workspaces[25].icon = WorkspaceIcon::Emoji("🗂️".into());
        let before = state.clone();
        let mut reordered = state.clone();
        reordered.workspaces.reverse();
        assert!(state.migrate_workspace_appearances());
        assert!(reordered.migrate_workspace_appearances());
        for workspace in &state.workspaces {
            assert_eq!(Some(workspace), reordered.workspace(workspace.id));
        }
        let counts = FOLDER_TREATMENTS
            .iter()
            .map(|t| {
                state
                    .workspaces
                    .iter()
                    .filter(|w| {
                        w.appearance
                            .automatic_folder
                            .as_ref()
                            .is_some_and(|saved| saved.preset_id == t.preset_id)
                    })
                    .count()
            })
            .collect::<Vec<_>>();
        assert_eq!(counts.iter().min(), Some(&2));
        assert_eq!(counts.iter().max(), Some(&3));
        assert!(state.workspaces[25].appearance.is_unassigned());
        let assigned = state.clone();
        assert!(!state.migrate_workspace_appearances());
        assert_eq!(state, assigned);
        for workspace in &mut state.workspaces {
            workspace.appearance = WorkspaceAppearance::default();
        }
        assert_eq!(state, before);
    }

    #[test]
    fn allocation_prefers_the_least_used_identity_and_does_not_write() {
        let mut state = library(12);
        state.migrate_workspace_appearances();
        let unused = state.workspaces.pop().unwrap().appearance;
        let before = state.clone();
        let id = WorkspaceId(Uuid::from_u128(123));
        assert_eq!(state.propose_folder_appearance(id), unused);
        assert_eq!(state.propose_folder_appearance(id), unused);
        assert_eq!(state, before);
    }

    #[test]
    fn saved_parameters_survive_palette_changes_and_non_folder_icons() {
        let mut state = library(1);
        let mut saved = FOLDER_TREATMENTS[0].clone();
        saved.hue = 17;
        saved.version = 7;
        state.workspaces[0].appearance.automatic_folder = Some(saved.clone());
        state.workspaces[0].icon = WorkspaceIcon::Emoji("🐼".into());
        assert!(!state.migrate_workspace_appearances());
        assert_eq!(state.workspaces[0].appearance.automatic_folder, Some(saved));
    }

    #[test]
    fn invalid_treatments_are_rejected_by_state_validation() {
        let mut state = AppState::fresh("/tmp/Downloads".into());
        for (hue, saturation, brightness) in
            [(360, 100, 100), (0, 1001, 100), (0, 100, 49), (0, 100, 151)]
        {
            let saved = state.workspaces[0]
                .appearance
                .automatic_folder
                .as_mut()
                .unwrap();
            saved.hue = hue;
            saved.saturation = saturation;
            saved.brightness = brightness;
            assert!(state.validate().is_err());
        }
    }
}
