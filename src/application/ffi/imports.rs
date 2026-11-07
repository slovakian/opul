use super::*;

impl OpulStateMachine {
    pub fn add_torrent(
        &self,
        request: MachineTorrentRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let id = DownloadId::new();
        let (origin, name, info_hash) = if let Some(bytes) = request.metainfo {
            let meta = parse_torrent_bytes(&bytes).map_err(engine_error)?;
            let blob = self.stage_torrent_blob(&bytes)?;
            (
                TorrentOrigin::Metainfo {
                    blob,
                    original_name: Path::new(&request.source)
                        .file_name()
                        .and_then(|value| value.to_str())
                        .unwrap_or("download.torrent")
                        .to_string(),
                },
                meta.name,
                Some(meta.info_hash),
            )
        } else if request.source.to_ascii_lowercase().starts_with("magnet:") {
            let meta = parse_magnet_uri(&request.source).map_err(engine_error)?;
            (
                TorrentOrigin::Magnet {
                    uri: request.source.clone(),
                },
                meta.name.unwrap_or_else(|| request.source.clone()),
                Some(meta.info_hash),
            )
        } else {
            let name = Path::new(&request.source)
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&request.source)
                .to_string();
            (
                TorrentOrigin::RemoteMetainfo {
                    url: request.source.clone(),
                },
                name,
                None,
            )
        };
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddDownload {
                workspace: workspace_id(&request.workspace_id)?,
                download: Box::new(Download {
                    id,
                    folder: folder_id(&request.folder_id)?,
                    name,
                    attempt: 0,
                    control_generation: 0,
                    source: DownloadSource::Torrent(TorrentDownload {
                        origin,
                        info_hash,
                        metadata: None,
                        seed_ratio_limit: request.seed_ratio_limit,
                        files: Vec::new(),
                    }),
                    lifecycle: DownloadLifecycle::Queued,
                    output: None,
                }),
            },
        )
    }

    pub fn add_video_page(
        &self,
        request: MachineMediaRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.sync_video_tools()?;
        let item = request
            .items
            .first()
            .ok_or_else(|| StateMachineError::Rejected {
                reason: "a video page requires a link".into(),
            })?;
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddMediaItems {
                workspace: workspace_id(&request.workspace_id)?,
                downloads: vec![Download {
                    id: DownloadId::new(),
                    folder: folder_id(&request.folder_id)?,
                    name: item.url.clone(),
                    attempt: 0,
                    control_generation: 0,
                    source: DownloadSource::Media(MediaDownload {
                        origin: MediaOrigin::PendingPage {
                            url: item.url.clone(),
                        },
                        format: format_from_view(request.format),
                        policy: policy_from_view(&request.policy),
                        legacy_binding: None,
                        thumbnail_url: None,
                        extractor: None,
                    }),
                    lifecycle: DownloadLifecycle::Queued,
                    output: None,
                }],
            },
        )
    }

    pub fn add_media(
        &self,
        request: MachineMediaRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.add_media_items(request, false)
    }

    pub fn add_direct_media(
        &self,
        request: MachineMediaRequest,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        self.add_media_items(request, true)
    }

    pub(super) fn add_media_items(
        &self,
        request: MachineMediaRequest,
        direct: bool,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let folder = folder_id(&request.folder_id)?;
        let downloads = request
            .items
            .into_iter()
            .map(|item| Download {
                id: DownloadId::new(),
                folder,
                name: if item.title.trim().is_empty() {
                    item.url.clone()
                } else {
                    item.title
                },
                attempt: 0,
                control_generation: 0,
                source: DownloadSource::Media(MediaDownload {
                    origin: if direct {
                        MediaOrigin::Direct {
                            url: item.url,
                            referer: None,
                        }
                    } else {
                        MediaOrigin::Page {
                            page_url: item.page_url,
                            item_url: item.url,
                            playlist_index: item.playlist_index,
                        }
                    },
                    format: format_from_view(request.format),
                    policy: policy_from_view(&request.policy),
                    legacy_binding: None,
                    thumbnail_url: item.thumbnail_url,
                    extractor: item.extractor,
                }),
                lifecycle: DownloadLifecycle::Queued,
                output: None,
            })
            .collect();
        self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddMediaItems {
                workspace: workspace_id(&request.workspace_id)?,
                downloads,
            },
        )
    }

    pub fn add_guided_media(
        &self,
        request: MachineMediaRequest,
        session: Arc<GuidedExtractionSession>,
        selection: GuidedDownloadSelection,
    ) -> Result<MachineCommandReceipt, StateMachineError> {
        let item = request
            .items
            .first()
            .ok_or_else(|| StateMachineError::Rejected {
                reason: "guided media requires one selected item".to_string(),
            })?;
        let requires_video_tools = session
            .requires_video_tools(
                &selection.candidate_id,
                &MediaDownloadOptions {
                    format: engine_format(format_from_view(request.format)),
                    download_subtitles: request.policy.download_subtitles,
                    embed_subtitles: request.policy.embed_subtitles,
                    write_thumbnail: request.policy.write_thumbnail,
                    write_metadata: request.policy.write_metadata,
                    ..MediaDownloadOptions::default()
                },
            )
            .map_err(engine_error)?;
        let id = DownloadId::new();
        self.guided
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, GuidedInput { session, selection });
        let result = self.dispatch_request(
            request.request_id,
            request.expected_revision,
            Command::AddDownload {
                workspace: workspace_id(&request.workspace_id)?,
                download: Box::new(Download {
                    id,
                    folder: folder_id(&request.folder_id)?,
                    name: item.title.clone(),
                    attempt: 0,
                    control_generation: 0,
                    source: DownloadSource::Media(MediaDownload {
                        origin: MediaOrigin::Guided {
                            page_url: item.page_url.clone(),
                            url: item.url.clone(),
                            discovery_id: id.to_string(),
                            requires_video_tools,
                        },
                        format: format_from_view(request.format),
                        policy: policy_from_view(&request.policy),
                        legacy_binding: None,
                        thumbnail_url: item.thumbnail_url.clone(),
                        extractor: Some("User-guided extraction".to_string()),
                    }),
                    lifecycle: DownloadLifecycle::Queued,
                    output: None,
                }),
            },
        );
        if result.is_err() {
            self.guided
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .remove(&id);
        }
        result
    }
}
