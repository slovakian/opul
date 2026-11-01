use crate::preferences::{PreferenceStore, Preferences};
use opul_core::OpulStateMachine;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct Desktop {
    state: Mutex<DesktopState>,
}

enum DesktopState {
    PreferencesUnavailable(String),
    Initialized {
        preferences: Arc<PreferenceStore>,
        machine: Result<Arc<OpulStateMachine>, String>,
    },
}

impl DesktopState {
    fn open(
        directory: Result<PathBuf, String>,
        open_machine: impl FnOnce(&Preferences) -> Result<Arc<OpulStateMachine>, String>,
    ) -> Self {
        let preferences = directory.and_then(|directory| {
            std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
            PreferenceStore::open(&directory).map(Arc::new)
        });
        match preferences {
            Ok(preferences) => Self::Initialized {
                machine: open_machine(&preferences.get()),
                preferences,
            },
            Err(error) => Self::PreferencesUnavailable(error),
        }
    }

    fn preferences(&self) -> Result<Arc<PreferenceStore>, String> {
        match self {
            Self::PreferencesUnavailable(error) => Err(error.clone()),
            Self::Initialized { preferences, .. } => Ok(Arc::clone(preferences)),
        }
    }
}

impl Desktop {
    pub fn open(
        directory: Result<PathBuf, String>,
        open_machine: impl FnOnce(&Preferences) -> Result<Arc<OpulStateMachine>, String>,
    ) -> Self {
        Self {
            state: Mutex::new(DesktopState::open(directory, open_machine)),
        }
    }

    pub fn machine(&self) -> Result<Arc<OpulStateMachine>, String> {
        let state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        match &*state {
            DesktopState::PreferencesUnavailable(error) => Err(error.clone()),
            DesktopState::Initialized { machine, .. } => machine.clone(),
        }
    }

    pub fn preferences(&self) -> Result<Arc<PreferenceStore>, String> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .preferences()
    }

    pub fn load_preferences(
        &self,
        directory: Result<PathBuf, String>,
        open_machine: impl FnOnce(&Preferences) -> Result<Arc<OpulStateMachine>, String>,
    ) -> Result<Preferences, String> {
        // Serialize initialization across windows and concurrent startup reads.
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if matches!(*state, DesktopState::PreferencesUnavailable(_)) {
            *state = DesktopState::open(directory, open_machine);
        }
        // Once opened, the live store owns subsequent mutations. Do not reload it.
        Ok(state.preferences()?.get())
    }
}

#[cfg(test)]
#[path = "desktop_tests.rs"]
mod tests;
