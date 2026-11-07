use super::*;

impl MediaEngine {
    pub(crate) fn new_quiescent(config: MediaEngineConfig) -> Result<Arc<Self>, CoreError> {
        open_media_engine(config, false)
    }

    pub(crate) fn claim_legacy_transfer(
        &self,
        transfer_id: &str,
        download_id: &str,
        attempt: u32,
    ) -> Result<(), CoreError> {
        let mut items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let Some(item) = items.get_mut(transfer_id) else {
            return Err(CoreError::Engine {
                reason: format!("legacy media transfer {transfer_id} is missing"),
            });
        };
        if let Some(existing) = item.download_id.as_deref()
            && (existing != download_id || item.attempt != attempt)
        {
            return Err(CoreError::Engine {
                reason: format!(
                    "media transfer {transfer_id} is already bound to download {existing}"
                ),
            });
        }
        item.download_id = Some(download_id.to_string());
        item.attempt = attempt;
        save_downloads(&self.store_path, &items);
        Ok(())
    }

    pub(crate) fn ensure_download(
        &self,
        download_id: &str,
        attempt: u32,
        direct_download: bool,
        item: MediaProbeItem,
        output_folder: String,
        options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        self.enqueue_download(
            item,
            output_folder,
            options,
            GuidedTransferContext {
                direct_download,
                ..GuidedTransferContext::default()
            },
            Some((download_id, attempt)),
        )
    }

    pub(crate) fn ensure_guided_download(
        &self,
        download_id: &str,
        attempt: u32,
        session: Arc<GuidedExtractionSession>,
        selection: GuidedDownloadSelection,
        output_folder: String,
        mut options: MediaDownloadOptions,
    ) -> Result<String, CoreError> {
        if let Some(existing) = self.transfer_id_for(download_id, attempt) {
            return Ok(existing);
        }
        let resolved = session.resolve_candidate(&selection.candidate_id)?;
        let candidate = resolved.candidate;
        let candidate_url = Url::parse(&candidate.url).map_err(|error| CoreError::InvalidUrl {
            reason: format!("invalid guided media URL: {error}"),
        })?;
        options.user_agent = selection.user_agent;
        let direct_download = direct_download_compatible(candidate.kind, &options);
        options.referer = if direct_download {
            direct_referer_for_target(&candidate.referer, &candidate_url)
        } else {
            origin_referer_for_target(&candidate.referer, &candidate_url)
        };
        if !direct_download && resolved.headers.iter().any(is_sensitive_guided_header) {
            return Err(CoreError::InvalidConfiguration {
                reason: "Authenticated request headers cannot be forwarded safely through yt-dlp across media hosts. Use an original direct download or cookie authentication."
                    .to_string(),
            });
        }
        self.enqueue_download(
            MediaProbeItem {
                webpage_url: candidate.url,
                title: candidate.title,
                thumbnail_url: None,
                extractor: Some("User-guided extraction".to_string()),
                duration_seconds: None,
                playlist_index: None,
            },
            output_folder,
            options,
            GuidedTransferContext {
                direct_download,
                cookie_contents: Some(selection.cookies),
                request_headers: resolved.headers,
                guided_kind: Some(candidate.kind),
            },
            Some((download_id, attempt)),
        )
    }

    pub(crate) fn transfer_id_for(&self, download_id: &str, attempt: u32) -> Option<String> {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .find(|item| {
                item.download_id.as_deref() == Some(download_id) && item.attempt == attempt
            })
            .map(|item| item.id.clone())
    }

    pub(crate) fn pause_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            self.pause(id);
        }
    }

    pub(crate) fn resume_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            let state = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .get(&id)
                .map(|item| item.state);
            match state {
                Some(MediaTransferState::Queued) => spawn_worker(self.worker_context(), id),
                Some(MediaTransferState::Paused) => self.resume(id),
                _ => {}
            }
        }
    }

    pub(crate) fn cancel_download(&self, download_id: &str, attempt: u32) {
        if let Some(id) = self.transfer_id_for(download_id, attempt) {
            self.cancel(id);
        }
    }

    /// Removes every attempt of the Download: a retry leaves earlier attempts behind, and
    /// none of them may outlive the Download.
    pub(crate) fn remove_download(&self, download_id: &str, _attempt: u32) {
        for id in self.transfer_ids_for(download_id) {
            self.remove(id);
        }
    }

    /// Everything this Download's attempts write to disk: the content folder when there is
    /// one, otherwise the finished file and whatever partial files are still around.
    pub(crate) fn owned_paths(&self, download_id: &str) -> Vec<PathBuf> {
        let items = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut paths = Vec::new();
        for item in items
            .values()
            .filter(|item| item.download_id.as_deref() == Some(download_id))
        {
            if let Some(content_folder) = &item.content_folder {
                paths.push(PathBuf::from(content_folder));
                continue;
            }
            paths.extend(item.output_path.iter().map(PathBuf::from));
            if item.direct_download
                && let Ok(source) = Url::parse(&item.source_url)
            {
                paths.push(direct_temporary_path(
                    &item.output_folder,
                    &source,
                    &item.title,
                    &item.id,
                ));
            }
            for partial in &item.partial_files {
                paths.extend(yt_dlp_partial_paths(Path::new(partial)));
            }
        }
        paths
    }

    pub(super) fn transfer_ids_for(&self, download_id: &str) -> Vec<String> {
        self.items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .filter(|item| item.download_id.as_deref() == Some(download_id))
            .map(|item| item.id.clone())
            .collect()
    }

    pub(crate) fn stop_all(&self) {
        let ids = self
            .items
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        for id in ids {
            self.pause(id);
        }
    }

    pub(super) fn enqueue_download(
        &self,
        item: MediaProbeItem,
        output_folder: String,
        options: MediaDownloadOptions,
        guided: GuidedTransferContext,
        identity: Option<(&str, u32)>,
    ) -> Result<String, CoreError> {
        let GuidedTransferContext {
            direct_download,
            cookie_contents,
            request_headers,
            guided_kind,
        } = guided;
        validate_http_url(&item.webpage_url)?;
        validate_filename_template(&options.filename_template)?;
        validate_output_folder(&output_folder)?;
        if !direct_download {
            if self.ffmpeg.is_none() {
                return Err(CoreError::DependencyUnavailable {
                    reason: "Opul’s built-in media tools are unavailable. Reinstall Opul to restore them.".to_string(),
                });
            }
            self.require_yt_dlp()?;
        }
        let title = if item.title.trim().is_empty() {
            item.webpage_url.clone()
        } else {
            item.title.clone()
        };
        let naturally_grouped = options.download_subtitles
            || options.embed_subtitles
            || options.write_thumbnail
            || options.write_metadata;
        let destination = resolve_download_destination(
            Path::new(&output_folder),
            &title,
            options.content_layout,
            naturally_grouped,
        );
        fs::create_dir_all(&destination.output_folder).map_err(|error| CoreError::Engine {
            reason: format!("could not create download folder: {error}"),
        })?;
        if let Some((download_id, attempt)) = identity {
            let items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(existing) = items.values().find(|existing| {
                existing.download_id.as_deref() == Some(download_id) && existing.attempt == attempt
            }) {
                return Ok(existing.id.clone());
            }
        }
        let id = identity
            .map(|(download_id, attempt)| {
                Uuid::new_v5(
                    &Uuid::NAMESPACE_OID,
                    format!("opul:{download_id}:{attempt}").as_bytes(),
                )
                .to_string()
            })
            .unwrap_or_else(|| Uuid::new_v4().to_string());
        let cookie_file = self.snapshot_cookie_jar(&id, cookie_contents.as_deref())?;
        let request_header_file = match self.snapshot_request_headers(&id, &request_headers) {
            Ok(path) => path,
            Err(error) => {
                if cookie_file.is_some()
                    && let Some(path) = owned_cookie_path(&self.cookie_directory(), &id)
                {
                    let _ = fs::remove_file(path);
                }
                return Err(error);
            }
        };
        let stored = StoredMediaDownload {
            id: id.clone(),
            download_id: identity.map(|(download_id, _)| download_id.to_string()),
            attempt: identity.map(|(_, attempt)| attempt).unwrap_or(0),
            source_url: item.webpage_url.clone(),
            page_url: options
                .referer
                .clone()
                .unwrap_or_else(|| item.webpage_url.clone()),
            title,
            output_folder: destination.output_folder.to_string_lossy().into_owned(),
            content_folder: destination
                .content_folder
                .map(|path| path.to_string_lossy().into_owned()),
            thumbnail_url: item.thumbnail_url,
            extractor: item.extractor,
            playlist_index: item.playlist_index,
            options,
            direct_download,
            cookie_file,
            request_headers,
            request_header_file,
            guided_kind,
            state: MediaTransferState::Queued,
            progress: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            download_rate: 0,
            eta_seconds: None,
            output_path: None,
            partial_files: Vec::new(),
            error_message: None,
            created_at: now_millis(),
        };
        {
            let mut items = self
                .items
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            items.insert(id.clone(), stored);
            save_downloads(&self.store_path, &items);
        }
        spawn_worker(self.worker_context(), id.clone());
        Ok(id)
    }
}
