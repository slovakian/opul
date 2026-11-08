use super::*;

impl MediaEngine {
    pub(super) fn cookie_path_if_present(&self) -> Option<PathBuf> {
        self.cookie_path.exists().then(|| self.cookie_path.clone())
    }

    pub(super) fn worker_context(&self) -> WorkerContext {
        WorkerContext {
            yt_dlp: self.yt_dlp.clone(),
            ffmpeg: self.ffmpeg.clone(),
            cookie_directory: self.cookie_directory(),
            request_header_directory: self.request_header_directory(),
            store_path: self.store_path.clone(),
            items: Arc::clone(&self.items),
            processes: Arc::clone(&self.processes),
            workers: Arc::clone(&self.workers),
            gate: Arc::clone(&self.gate),
            max_download_bps: Arc::clone(&self.max_download_bps),
        }
    }

    pub(super) fn snapshot_cookie_jar(
        &self,
        id: &str,
        supplied_contents: Option<&str>,
    ) -> Result<Option<String>, CoreError> {
        let contents = if let Some(contents) = supplied_contents {
            if contents.len() > 8 * 1024 * 1024 {
                return Err(CoreError::InvalidConfiguration {
                    reason: "the browser cookie jar is unexpectedly large".to_string(),
                });
            }
            if contents.trim().is_empty() {
                return Ok(None);
            }
            contents.as_bytes().to_vec()
        } else {
            let Ok(contents) = fs::read(&self.cookie_path) else {
                return Ok(None);
            };
            contents
        };
        let directory = self.cookie_directory();
        fs::create_dir_all(&directory).map_err(|error| CoreError::Engine {
            reason: format!("could not create the cookie folder: {error}"),
        })?;
        let path = directory.join(format!("{id}.txt"));
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download cookies: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect download cookies: {error}"),
            })?;
        file.write_all(&contents)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download cookies: {error}"),
            })?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    pub(super) fn cookie_directory(&self) -> PathBuf {
        self.cookie_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("cookies")
    }

    pub(super) fn snapshot_request_headers(
        &self,
        id: &str,
        headers: &[GuidedHttpHeader],
    ) -> Result<Option<String>, CoreError> {
        if headers.is_empty() {
            return Ok(None);
        }
        let directory = self.request_header_directory();
        fs::create_dir_all(&directory).map_err(|error| CoreError::Engine {
            reason: format!("could not create the request-header folder: {error}"),
        })?;
        let path = directory.join(format!("{id}.json"));
        let contents = serde_json::to_vec(headers).map_err(|error| CoreError::Engine {
            reason: format!("could not encode download request headers: {error}"),
        })?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not prepare download request headers: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .and_then(|_| file.write_all(&contents))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect download request headers: {error}"),
            })?;
        Ok(Some(path.to_string_lossy().into_owned()))
    }

    pub(super) fn request_header_directory(&self) -> PathBuf {
        self.cookie_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("request-headers")
    }

    pub(super) fn restore_pending(&self) {
        let ids = {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut ids = Vec::new();
            for item in items.values_mut() {
                if matches!(
                    item.state,
                    MediaTransferState::Downloading | MediaTransferState::Processing
                ) {
                    item.state = MediaTransferState::Queued;
                }
                if matches!(
                    item.state,
                    MediaTransferState::Queued | MediaTransferState::Paused
                ) && (item.direct_download || self.yt_dlp.available())
                {
                    ids.push(item.id.clone());
                }
            }
            save_downloads(&self.store_path, &items);
            ids
        };
        for id in ids {
            spawn_worker(self.worker_context(), id);
        }
    }

    pub(super) fn can_start(&self, id: &str) -> bool {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .is_some_and(|item| item.direct_download || self.yt_dlp.available())
    }

    pub(super) fn transition_state(
        &self,
        id: &str,
        allowed: &[MediaTransferState],
        state: MediaTransferState,
        error_message: Option<String>,
    ) -> bool {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(id) else {
            return false;
        };
        if !allowed.contains(&item.state) {
            return false;
        }
        item.state = state;
        item.error_message = error_message;
        if state != MediaTransferState::Downloading {
            item.download_rate = 0;
        }
        save_downloads(&self.store_path, &items);
        true
    }

    pub(super) fn update_state(
        &self,
        id: &str,
        state: MediaTransferState,
        error_message: Option<String>,
    ) {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(id) else {
            return;
        };
        item.state = state;
        item.error_message = error_message;
        if state != MediaTransferState::Downloading {
            item.download_rate = 0;
        }
        save_downloads(&self.store_path, &items);
    }

    pub(super) fn process_id(&self, id: &str) -> Option<u32> {
        self.processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(id)
            .copied()
    }
}

impl Drop for MediaEngine {
    fn drop(&mut self) {
        {
            let mut gate = self
                .gate
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            gate.shutdown = true;
        }
        self.gate.1.notify_all();
        let pids = self
            .processes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .copied()
            .collect::<Vec<_>>();
        for pid in pids {
            signal_process(pid, "TERM");
        }
    }
}
