//! Rune presentation preferences. The command worker is the sole disk writer.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Startup {
    #[default]
    Remember,
    Collapsed,
}

#[cfg(test)]
#[path = "preferences_tests.rs"]
mod tests;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub workspace_expansion: BTreeMap<String, bool>,
    pub workspace_expansion_startup: Startup,
}

impl Preferences {
    pub fn read(path: &Path) -> Result<Self, String> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| e.to_string()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        use std::io::Write;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        let temporary = path.with_extension("json.tmp");
        let mut file = std::fs::File::create(&temporary).map_err(|e| e.to_string())?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())
    }
}

#[derive(Default)]
pub struct Expansion {
    pub session: BTreeMap<String, bool>,
    pub desired: Preferences,
    pub loaded: bool,
    pub error: Option<String>,
    pub saving: bool,
    generation: u64,
    edits: BTreeMap<String, Option<bool>>,
    startup_edit: Option<Startup>,
}

impl Expansion {
    pub fn expanded(&self, id: &str) -> bool {
        self.session.get(id).copied().unwrap_or(false)
    }

    pub fn toggle(&mut self, id: String) {
        self.set(id.clone(), !self.expanded(&id));
    }

    pub fn set(&mut self, id: String, expanded: bool) {
        self.session.insert(id.clone(), expanded);
        self.desired
            .workspace_expansion
            .insert(id.clone(), expanded);
        self.edits.insert(id, Some(expanded));
        self.generation += 1;
    }

    pub fn startup(&mut self, startup: Startup) {
        self.desired.workspace_expansion_startup = startup;
        self.startup_edit = Some(startup);
        self.generation += 1;
    }

    /// Only the first successful read initializes session expansion. Early local choices win.
    pub fn loaded(&mut self, result: Result<Preferences, String>) {
        if self.loaded {
            return;
        }
        let mut saved = match result {
            Ok(saved) => saved,
            Err(error) => {
                self.error = Some(format!("Couldn't read workspace expansion: {error}"));
                return;
            }
        };
        let mut initial = if saved.workspace_expansion_startup == Startup::Remember {
            saved.workspace_expansion.clone()
        } else {
            BTreeMap::new()
        };
        for (id, choice) in &self.edits {
            match choice {
                Some(expanded) => {
                    saved.workspace_expansion.insert(id.clone(), *expanded);
                }
                None => {
                    saved.workspace_expansion.remove(id);
                    initial.remove(id);
                }
            }
        }
        initial.append(&mut self.session);
        self.session = initial;
        if let Some(startup) = self.startup_edit {
            saved.workspace_expansion_startup = startup;
        }
        self.desired = saved;
        self.loaded = true;
        self.error = None;
    }

    pub fn retain(&mut self, ids: &std::collections::BTreeSet<String>) {
        let known: std::collections::BTreeSet<_> = self
            .session
            .keys()
            .chain(self.desired.workspace_expansion.keys())
            .chain(self.edits.keys())
            .cloned()
            .collect();
        for id in known {
            if !ids.contains(&id) && self.edits.get(&id) != Some(&None) {
                self.session.remove(&id);
                self.desired.workspace_expansion.remove(&id);
                self.edits.insert(id, None);
                self.generation += 1;
            }
        }
    }

    pub fn write(&mut self) -> Option<(u64, Preferences)> {
        if !self.loaded || (self.edits.is_empty() && self.startup_edit.is_none()) {
            return None;
        }
        self.saving = true;
        self.error = None;
        Some((self.generation, self.desired.clone()))
    }

    pub fn saved(&mut self, generation: u64, result: Result<(), String>) {
        // Acknowledgements never replace session intent, including deletion tombstones.
        if generation != self.generation {
            return;
        }
        self.saving = false;
        match result {
            Ok(()) => {
                self.edits.clear();
                self.startup_edit = None;
                self.error = None;
            }
            Err(error) => self.error = Some(format!("Couldn't save workspace expansion: {error}")),
        }
    }
}
