use super::*;

pub(super) fn observe_adapters(
    actor: &ApplicationActor,
    torrent: &TorrentEngine,
    media: &MediaEngine,
) {
    for snapshot in torrent.snapshots() {
        let Some(download_id_value) = snapshot.download_id.as_deref() else {
            continue;
        };
        let Ok(download) = download_id(download_id_value) else {
            continue;
        };
        // Where a finished torrent's files are: its content folder, or its one file.
        let output_path = || {
            snapshot.content_folder.clone().or_else(|| {
                let [file] = snapshot.files.as_slice() else {
                    return None;
                };
                Some(
                    Path::new(&snapshot.save_folder)
                        .join(&file.path)
                        .to_string_lossy()
                        .into_owned(),
                )
            })
        };
        let phase = match snapshot.state {
            // A recheck, or a torrent still waiting for its file list, says nothing about
            // whether the files are there.
            TorrentState::Queued | TorrentState::Fetching
                if snapshot.verifying || snapshot.total_size == 0 =>
            {
                ActivePhase::Verifying
            }
            TorrentState::Queued | TorrentState::Fetching => ActivePhase::Fetching,
            TorrentState::Paused => {
                if let Ok(Some(context)) = actor.execution_context(download) {
                    let _ = actor.observe(Fact::TransferPaused {
                        download,
                        attempt: snapshot.attempt,
                        control_generation: context.download.control_generation,
                    });
                }
                ActivePhase::Fetching
            }
            TorrentState::Seeding => {
                if let Some(path) = output_path() {
                    let _ = actor.observe(Fact::OutputFound {
                        download,
                        attempt: snapshot.attempt,
                        output: output_artifact(path, snapshot.content_folder.clone()),
                    });
                }
                ActivePhase::Seeding
            }
            TorrentState::Finished => {
                if let Some(path) = output_path() {
                    let _ = actor.observe(Fact::TransferFinished {
                        download,
                        attempt: snapshot.attempt,
                        output: output_artifact(path, snapshot.content_folder.clone()),
                    });
                }
                ActivePhase::Seeding
            }
            TorrentState::Failed => {
                let _ = actor.observe(Fact::TransferFailed {
                    download,
                    attempt: snapshot.attempt,
                    message: snapshot
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "The torrent transfer failed.".to_string()),
                    retryable: true,
                });
                ActivePhase::Fetching
            }
        };
        if !snapshot.info_hash.is_empty() {
            let _ = actor.observe(Fact::TorrentResolved {
                download,
                attempt: snapshot.attempt,
                info_hash: snapshot.info_hash.clone(),
                metadata: TorrentMetadata {
                    total_size: snapshot.total_size,
                    piece_length: snapshot.piece_length,
                    piece_count: snapshot.piece_count,
                    comment: snapshot.comment.clone(),
                    trackers: snapshot.trackers.clone(),
                },
                files: snapshot
                    .files
                    .iter()
                    .map(|file| TorrentFileSelection {
                        index: file.file_index,
                        path: file.path.clone(),
                        size: file.size,
                        priority: match file.priority.as_str() {
                            "Skip" => TorrentFilePriority::Skip,
                            "Low" => TorrentFilePriority::Low,
                            "High" => TorrentFilePriority::High,
                            _ => TorrentFilePriority::Normal,
                        },
                    })
                    .collect(),
            });
        }
        if !matches!(
            snapshot.state,
            TorrentState::Paused | TorrentState::Finished | TorrentState::Failed
        ) {
            let _ = actor.observe(Fact::TransferAdvanced {
                download,
                attempt: snapshot.attempt,
                phase,
            });
        }
        let _ = actor.observe_telemetry(
            download,
            TransferTelemetry {
                downloaded_bytes: snapshot.bytes_downloaded,
                uploaded_bytes: snapshot.bytes_uploaded,
                total_bytes: snapshot.selected_size,
                download_rate: snapshot.download_rate,
                upload_rate: snapshot.upload_rate,
                eta_seconds: snapshot.eta_seconds,
                peers: snapshot.peers,
                seeds: snapshot.seeds,
                progress_millionths: fraction_to_millionths(snapshot.progress),
            },
        );
    }
    for snapshot in media.snapshots() {
        let Some(download_id_value) = snapshot.download_id.as_deref() else {
            continue;
        };
        let Ok(download) = download_id(download_id_value) else {
            continue;
        };
        match snapshot.state {
            MediaTransferState::Queued | MediaTransferState::Downloading => {
                let _ = actor.observe(Fact::TransferAdvanced {
                    download,
                    attempt: snapshot.attempt,
                    phase: ActivePhase::Fetching,
                });
            }
            MediaTransferState::Processing => {
                let _ = actor.observe(Fact::TransferAdvanced {
                    download,
                    attempt: snapshot.attempt,
                    phase: ActivePhase::Processing,
                });
            }
            MediaTransferState::Paused => {
                if let Ok(Some(context)) = actor.execution_context(download) {
                    let _ = actor.observe(Fact::TransferPaused {
                        download,
                        attempt: snapshot.attempt,
                        control_generation: context.download.control_generation,
                    });
                }
            }
            MediaTransferState::Finished => {
                if let Some(path) = snapshot.output_path.clone() {
                    let _ = actor.observe(Fact::TransferFinished {
                        download,
                        attempt: snapshot.attempt,
                        output: output_artifact(path, snapshot.content_folder.clone()),
                    });
                }
            }
            MediaTransferState::Failed => {
                let _ = actor.observe(Fact::TransferFailed {
                    download,
                    attempt: snapshot.attempt,
                    message: snapshot
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "The media transfer failed.".to_string()),
                    retryable: true,
                });
            }
            MediaTransferState::Cancelled => {}
        }
        let _ = actor.observe_telemetry(
            download,
            TransferTelemetry {
                downloaded_bytes: snapshot.downloaded_bytes,
                uploaded_bytes: 0,
                total_bytes: snapshot.total_bytes,
                download_rate: snapshot.download_rate,
                upload_rate: 0,
                eta_seconds: snapshot.eta_seconds,
                peers: 0,
                seeds: 0,
                progress_millionths: fraction_to_millionths(snapshot.progress),
            },
        );
    }
}
