use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Download {
    pub(crate) id: DownloadId,
    pub(crate) folder: FolderId,
    pub(crate) name: String,
    pub(crate) attempt: u32,
    pub(crate) control_generation: u64,
    pub(crate) source: DownloadSource,
    pub(crate) lifecycle: DownloadLifecycle,
    pub(crate) output: Option<OutputArtifact>,
}

impl Eq for Download {}

impl Download {
    pub(crate) fn needs_video_tools(&self) -> bool {
        matches!(&self.source, DownloadSource::Media(media) if matches!(media.origin, MediaOrigin::Page { .. } | MediaOrigin::PendingPage { .. } | MediaOrigin::Guided { requires_video_tools: true, .. }))
    }

    /// Running on the video tools, about to, or waiting for them: what removing them affects.
    pub(crate) fn relies_on_video_tools(&self) -> bool {
        self.needs_video_tools()
            && (self.lifecycle.would_run()
                || matches!(self.lifecycle, DownloadLifecycle::WaitingForVideoTools))
    }

    pub(crate) fn active_info_hash(&self) -> Option<&str> {
        if self.lifecycle.is_retired() {
            return None;
        }
        match &self.source {
            DownloadSource::Torrent(torrent) => torrent.info_hash.as_deref(),
            DownloadSource::Media(_) => None,
        }
    }

    /// The output Opul may move to the Trash on its own evidence: only an artifact whose
    /// identity was recorded when it was produced proves the file at that path is this one.
    pub(crate) fn verified_output(&self) -> Option<&OutputArtifact> {
        self.output
            .as_ref()
            .filter(|output| output.identity.is_some())
    }

    /// Adapter-reported leftovers that the verified output does not already cover.
    pub(crate) fn uncovered_leftovers(&self) -> Vec<OutputArtifact> {
        let DownloadLifecycle::Retiring { leftovers, .. } = &self.lifecycle else {
            return Vec::new();
        };
        let covered = self.verified_output().map(|output| {
            std::path::PathBuf::from(output.content_folder.as_deref().unwrap_or(&output.path))
        });
        leftovers
            .iter()
            .filter(|leftover| {
                covered.as_ref().is_none_or(|covered| {
                    !std::path::Path::new(&leftover.path).starts_with(covered)
                })
            })
            .cloned()
            .collect()
    }

    pub(crate) fn has_trash_targets(&self) -> bool {
        self.verified_output().is_some() || !self.uncovered_leftovers().is_empty()
    }

    /// A torrent whose selected files are all on disk. It never shows progress or offers
    /// pause and resume again until it has something new to download.
    pub(crate) fn torrent_complete(&self) -> bool {
        matches!(&self.source, DownloadSource::Torrent(torrent) if torrent.complete)
    }

    /// A complete torrent reattaching to its files before it seeds again.
    pub(crate) fn checking_files(&self) -> bool {
        self.torrent_complete()
            && matches!(
                self.lifecycle,
                DownloadLifecycle::Queued | DownloadLifecycle::Starting
            )
    }

    pub(crate) fn source_kind(&self) -> SourceKind {
        match self.source {
            DownloadSource::Torrent(_) => SourceKind::Torrent,
            DownloadSource::Media(_) => SourceKind::Media,
        }
    }

    pub(crate) fn dedupe_key(&self) -> &str {
        match &self.source {
            DownloadSource::Torrent(torrent) => match &torrent.origin {
                TorrentOrigin::Magnet { uri } => uri,
                TorrentOrigin::Metainfo { blob, .. } => blob,
                TorrentOrigin::RemoteMetainfo { url } => url,
            },
            DownloadSource::Media(media) => match &media.origin {
                MediaOrigin::Page { item_url, .. } => item_url,
                MediaOrigin::PendingPage { url } => url,
                MediaOrigin::Direct { url, .. } | MediaOrigin::Guided { url, .. } => url,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DownloadSource {
    Torrent(TorrentDownload),
    Media(MediaDownload),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct TorrentDownload {
    pub(crate) origin: TorrentOrigin,
    pub(crate) info_hash: Option<String>,
    pub(crate) metadata: Option<TorrentMetadata>,
    pub(crate) seed_ratio_limit: Option<f64>,
    pub(crate) files: Vec<TorrentFileSelection>,
    /// Every selected file is on disk, whether or not the torrent is seeding now. Selecting
    /// more files, or a recheck that finds some missing, makes it incomplete again.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) complete: bool,
    /// Seeding stopped on its own because the share ratio reached `seed_ratio_limit`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) reached_seed_ratio: bool,
}

impl Eq for TorrentDownload {}

// Older checkpoints omit these fields; skipping `false` keeps their checksums valid.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TorrentOrigin {
    Magnet { uri: String },
    Metainfo { blob: String, original_name: String },
    RemoteMetainfo { url: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TorrentMetadata {
    pub(crate) total_size: u64,
    pub(crate) piece_length: u64,
    pub(crate) piece_count: u32,
    pub(crate) comment: Option<String>,
    pub(crate) trackers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct TorrentFileSelection {
    pub(crate) index: u32,
    pub(crate) path: String,
    pub(crate) size: u64,
    pub(crate) priority: TorrentFilePriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum TorrentFilePriority {
    Skip,
    Low,
    Normal,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MediaDownload {
    pub(crate) origin: MediaOrigin,
    pub(crate) format: MediaFormat,
    pub(crate) policy: MediaPolicy,
    pub(crate) legacy_binding: Option<String>,
    pub(crate) thumbnail_url: Option<String>,
    pub(crate) extractor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MediaPolicy {
    pub(crate) download_subtitles: bool,
    pub(crate) subtitle_languages: Vec<String>,
    pub(crate) embed_subtitles: bool,
    pub(crate) write_thumbnail: bool,
    pub(crate) write_metadata: bool,
    pub(crate) filename_template: String,
    pub(crate) user_agent: Option<String>,
}

impl Default for MediaPolicy {
    fn default() -> Self {
        Self {
            download_subtitles: false,
            subtitle_languages: vec!["en".to_string()],
            embed_subtitles: false,
            write_thumbnail: false,
            write_metadata: false,
            filename_template: "%(title)s.%(ext)s".to_string(),
            user_agent: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum MediaOrigin {
    PendingPage {
        url: String,
    },
    Page {
        page_url: String,
        item_url: String,
        playlist_index: Option<u32>,
    },
    Direct {
        url: String,
        referer: Option<String>,
    },
    Guided {
        page_url: String,
        url: String,
        discovery_id: String,
        #[serde(default)]
        requires_video_tools: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum DownloadLifecycle {
    WaitingForVideoTools,
    Queued,
    Starting,
    Active(ActivePhase),
    Pausing {
        from: ActivePhase,
        cause: HoldCause,
    },
    Paused {
        resume: ActivePhase,
        cause: HoldCause,
    },
    Finished,
    Failed {
        message: String,
        retryable: bool,
    },
    Cancelled,
    Retiring {
        previous: Box<DownloadLifecycle>,
        move_files_to_trash: bool,
        writer_stopped: bool,
        adapter_absent: bool,
        files_resolved: bool,
        /// Files the transfer adapter owned when its writer stopped: partial data for an
        /// unfinished download, or output the aggregate never verified.
        #[serde(default)]
        leftovers: Vec<OutputArtifact>,
    },
}

impl DownloadLifecycle {
    pub(crate) fn is_retired(&self) -> bool {
        matches!(self, Self::Retiring { .. })
    }

    /// Running, or about to run without the person doing anything. A person's pause, a recovery
    /// hold awaiting acknowledgement, a failure or a cancellation is left as it is.
    pub(crate) fn would_run(&self) -> bool {
        matches!(
            self,
            Self::Queued
                | Self::Starting
                | Self::Active(_)
                | Self::Pausing {
                    cause: HoldCause::Shutdown,
                    ..
                }
                | Self::Paused {
                    cause: HoldCause::Shutdown,
                    ..
                }
        )
    }

    pub(crate) fn valid_for(&self, source: SourceKind) -> bool {
        let phase_is_valid = |phase: ActivePhase| match source {
            SourceKind::Torrent => !matches!(phase, ActivePhase::Processing),
            SourceKind::Media => !matches!(phase, ActivePhase::Verifying | ActivePhase::Seeding),
        };
        match self {
            Self::WaitingForVideoTools => source == SourceKind::Media,
            Self::Active(phase) => phase_is_valid(*phase),
            Self::Pausing { from, .. } | Self::Paused { resume: from, .. } => phase_is_valid(*from),
            Self::Retiring { previous, .. } => previous.valid_for(source),
            Self::Queued
            | Self::Starting
            | Self::Finished
            | Self::Failed { .. }
            | Self::Cancelled => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum ActivePhase {
    Fetching,
    Processing,
    Verifying,
    Seeding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum HoldCause {
    Person,
    Shutdown,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OutputArtifact {
    pub(crate) path: String,
    pub(crate) content_folder: Option<String>,
    pub(crate) identity: Option<OutputIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct OutputIdentity {
    pub(crate) byte_length: u64,
    pub(crate) modified_at_millis: u64,
    pub(crate) file_identifier: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum SourceKind {
    Torrent,
    Media,
}

impl AppState {
    /// Checkpoints written before torrents tracked completion: a seeding or finished torrent
    /// is complete, and a seeding torrent someone paused is simply not seeding any more.
    pub(crate) fn migrate_torrent_completion(&mut self) -> bool {
        let mut changed = false;
        for download in self
            .workspaces
            .iter_mut()
            .flat_map(|workspace| workspace.downloads.iter_mut())
        {
            let DownloadSource::Torrent(torrent) = &mut download.source else {
                continue;
            };
            let seeded = match &download.lifecycle {
                DownloadLifecycle::Active(ActivePhase::Seeding) | DownloadLifecycle::Finished => {
                    true
                }
                DownloadLifecycle::Pausing { from, .. }
                | DownloadLifecycle::Paused { resume: from, .. } => *from == ActivePhase::Seeding,
                _ => false,
            };
            if seeded && !torrent.complete {
                torrent.complete = true;
                changed = true;
            }
            if matches!(
                download.lifecycle,
                DownloadLifecycle::Pausing {
                    from: ActivePhase::Seeding,
                    cause: HoldCause::Person | HoldCause::Recovery,
                } | DownloadLifecycle::Paused {
                    resume: ActivePhase::Seeding,
                    cause: HoldCause::Person | HoldCause::Recovery,
                }
            ) {
                download.lifecycle = DownloadLifecycle::Finished;
                changed = true;
            }
        }
        changed
    }
}
