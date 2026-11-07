use super::*;

impl MediaEngine {
    /// Stop only video writers, wait for process exit, then discard adapter records.
    /// Download files, including .part files, are intentionally retained.
    pub(crate) fn stop_video_downloads(&self) -> Result<(), String> {
        let ids = self
            .items
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .values()
            .filter(|item| !item.direct_download && item.state != MediaTransferState::Finished)
            .map(|item| item.id.clone())
            .collect::<Vec<_>>();
        for id in &ids {
            self.cancel(id.clone());
            if let Some(pid) = self.process_id(id) {
                // Paused process groups must wake up to handle TERM.
                signal_process(pid, "CONT");
                signal_process(pid, "TERM");
            }
        }
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let active = self
                .workers
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .active
                .iter()
                .any(|id| ids.contains(id));
            if !active {
                break;
            }
            if Instant::now() >= deadline {
                for id in &ids {
                    if let Some(pid) = self.process_id(id) {
                        signal_process(pid, "KILL");
                    }
                }
                return Err(
                    "Video downloads are still stopping. Try removing the tools again.".into(),
                );
            }
            thread::sleep(Duration::from_millis(10));
        }
        for id in ids {
            self.remove(id);
        }
        Ok(())
    }

    pub fn snapshots(&self) -> Vec<MediaTransferSnapshot> {
        let items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut values = items.values().collect::<Vec<_>>();
        values.sort_by_key(|item| item.created_at);
        values
            .into_iter()
            .map(StoredMediaDownload::snapshot)
            .collect()
    }

    pub fn pause(&self, id: String) {
        if !self.transition_state(
            &id,
            &[
                MediaTransferState::Queued,
                MediaTransferState::Downloading,
                MediaTransferState::Processing,
            ],
            MediaTransferState::Paused,
            None,
        ) {
            return;
        }
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "STOP");
        }
    }

    pub fn resume(&self, id: String) {
        if let Some(pid) = self.process_id(&id) {
            if self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Downloading,
                None,
            ) {
                signal_process(pid, "CONT");
            }
        } else if self.can_start(&id) {
            // Queued and restored transfers already own a worker waiting at the
            // concurrency gate. Waking that worker avoids starting a duplicate.
            self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Queued,
                None,
            );
        } else {
            self.transition_state(
                &id,
                &[MediaTransferState::Paused],
                MediaTransferState::Failed,
                Some("yt-dlp is required to resume this download".to_string()),
            );
        }
        self.gate.1.notify_all();
    }

    pub fn cancel(&self, id: String) {
        if !self.transition_state(
            &id,
            &[
                MediaTransferState::Queued,
                MediaTransferState::Downloading,
                MediaTransferState::Processing,
                MediaTransferState::Paused,
            ],
            MediaTransferState::Cancelled,
            None,
        ) {
            return;
        }
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "TERM");
        }
        self.gate.1.notify_all();
    }

    pub fn retry(&self, id: String) {
        {
            let items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get(&id) else {
                return;
            };
            if !matches!(
                item.state,
                MediaTransferState::Failed | MediaTransferState::Cancelled
            ) {
                return;
            }
        }
        {
            let mut workers = self
                .workers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if workers.active.contains(&id) {
                workers.retry_requested.insert(id);
                return;
            }
        }
        let should_retry = {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let Some(item) = items.get_mut(&id) else {
                return;
            };
            if !matches!(
                item.state,
                MediaTransferState::Failed | MediaTransferState::Cancelled
            ) {
                return;
            }
            item.state = MediaTransferState::Queued;
            item.error_message = None;
            item.progress = 0.0;
            item.download_rate = 0;
            item.eta_seconds = None;
            save_downloads(&self.store_path, &items);
            true
        };
        if should_retry {
            if self.can_start(&id) {
                spawn_worker(self.worker_context(), id);
            } else {
                self.update_state(
                    &id,
                    MediaTransferState::Failed,
                    Some("yt-dlp is required to retry this download".to_string()),
                );
            }
        }
    }

    pub fn remove(&self, id: String) {
        self.workers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retry_requested
            .remove(&id);
        if let Some(pid) = self.process_id(&id) {
            signal_process(pid, "TERM");
        }
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let removed = items.remove(&id);
        save_downloads(&self.store_path, &items);
        if let Some(item) = removed {
            if item.cookie_file.is_some()
                && let Some(cookie_file) = owned_cookie_path(&self.cookie_directory(), &id)
            {
                let _ = fs::remove_file(cookie_file);
            }
            if item.request_header_file.is_some()
                && let Some(header_file) =
                    owned_request_header_path(&self.request_header_directory(), &id)
            {
                let _ = fs::remove_file(header_file);
            }
        }
        self.gate.1.notify_all();
    }

    pub fn set_max_concurrent_downloads(&self, maximum: u32) {
        let mut gate = self
            .gate
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        gate.maximum = maximum.max(1);
        drop(gate);
        self.gate.1.notify_all();
    }

    pub fn set_max_download_bps(&self, bytes_per_second: u64) {
        *self
            .max_download_bps
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = bytes_per_second;
    }

    pub fn set_cookie_jar(&self, contents: String) -> Result<(), CoreError> {
        if contents.len() > 8 * 1024 * 1024 {
            return Err(CoreError::InvalidConfiguration {
                reason: "the browser cookie jar is unexpectedly large".to_string(),
            });
        }
        let trimmed = contents.trim();
        if trimmed.is_empty() {
            if self.cookie_path.exists() {
                fs::remove_file(&self.cookie_path).map_err(|error| CoreError::Engine {
                    reason: format!("could not clear browser cookies: {error}"),
                })?;
            }
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            for item in items.values_mut() {
                if item.cookie_file.take().is_some()
                    && let Some(cookie_file) = owned_cookie_path(&self.cookie_directory(), &item.id)
                {
                    let _ = fs::remove_file(cookie_file);
                }
            }
            save_downloads(&self.store_path, &items);
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(&self.cookie_path)
            .map_err(|error| CoreError::Engine {
                reason: format!("could not store browser cookies: {error}"),
            })?;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not protect browser cookies: {error}"),
            })?;
        file.write_all(trimmed.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|error| CoreError::Engine {
                reason: format!("could not store browser cookies: {error}"),
            })
    }

    pub fn clear_guided_session_data(&self) -> Result<(), CoreError> {
        self.set_cookie_jar(String::new())?;
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        for item in items.values_mut() {
            item.request_headers.clear();
            if item.request_header_file.take().is_some()
                && let Some(path) =
                    owned_request_header_path(&self.request_header_directory(), &item.id)
            {
                let _ = fs::remove_file(path);
            }
        }
        save_downloads(&self.store_path, &items);
        Ok(())
    }
}
