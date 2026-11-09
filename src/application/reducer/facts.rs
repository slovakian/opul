use super::*;

pub(crate) fn decide_fact(state: &AppState, fact: &Fact) -> Result<Decision, Rejection> {
    match fact {
        Fact::VideoToolsChanged(availability) => Ok(decide_video_tools(state, *availability)),
        Fact::MediaPageFailed {
            download,
            attempt,
            control_generation,
            message,
        } => Ok(decide_page_failed(
            state,
            *download,
            *attempt,
            *control_generation,
            message,
        )),
        Fact::MediaPageResolved {
            download,
            attempt,
            control_generation,
            items,
        } => decide_page_resolved(state, *download, *attempt, *control_generation, items),
        Fact::TransferStarted {
            download,
            attempt,
            control_generation,
            phase,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.control_generation != *control_generation {
                return Ok(Decision::none());
            }
            // A complete torrent keeps checking its files until the engine says it seeds.
            if item.checking_files() {
                return Ok(Decision::none());
            }
            let lifecycle = match &item.lifecycle {
                DownloadLifecycle::Queued | DownloadLifecycle::Starting => {
                    DownloadLifecycle::Active(*phase)
                }
                DownloadLifecycle::Pausing { from, cause } => DownloadLifecycle::Pausing {
                    from: *from,
                    cause: *cause,
                },
                _ => return Ok(Decision::none()),
            };
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle,
                control_generation: *control_generation,
            }))
        }
        Fact::TransferAdvanced {
            download,
            attempt,
            phase,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if !phase_allowed(item.source_kind(), *phase) {
                return Ok(Decision::none());
            }
            if let DownloadSource::Torrent(torrent) = &item.source {
                return Ok(torrent_advanced(item, torrent, *phase));
            }
            if matches!(item.lifecycle, DownloadLifecycle::Active(current) if current == *phase) {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Active(*phase),
                control_generation: item.control_generation,
            }))
        }
        Fact::TransferPaused {
            download,
            attempt,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.control_generation != *control_generation {
                return Ok(Decision::none());
            }
            let DownloadLifecycle::Pausing { from, cause } = item.lifecycle else {
                return Ok(Decision::none());
            };
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Paused {
                    resume: from,
                    cause,
                },
                control_generation: *control_generation,
            }))
        }
        Fact::TransferFinished {
            download,
            attempt,
            output,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if matches!(item.lifecycle, DownloadLifecycle::Finished)
                && item.output.as_ref() == Some(output)
            {
                return Ok(Decision::none());
            }
            // Start seeding asked the engine to run again; its last "stopped" report is stale.
            if item.checking_files() {
                return Ok(Decision::none());
            }
            let mut events = vec![
                Event::DownloadOutputRecorded {
                    download: *download,
                    output: output.clone(),
                },
                Event::DownloadLifecycleChanged {
                    download: *download,
                    lifecycle: DownloadLifecycle::Finished,
                    control_generation: item.control_generation,
                },
            ];
            if let DownloadSource::Torrent(torrent) = &item.source {
                if !torrent.complete {
                    events.push(Event::TorrentCompletionChanged {
                        download: *download,
                        complete: true,
                    });
                }
                // Only the engine stops a seeding torrent on its own: the ratio was reached.
                if matches!(
                    item.lifecycle,
                    DownloadLifecycle::Active(ActivePhase::Seeding)
                ) && torrent.seed_ratio_limit.is_some()
                {
                    events.push(Event::SeedRatioReached {
                        download: *download,
                    });
                }
            }
            Ok(Decision {
                events,
                effects: Vec::new(),
                created_downloads: Vec::new(),
            })
        }
        Fact::OutputFound {
            download,
            attempt,
            output,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt
                || item.lifecycle.is_retired()
                || item.output.as_ref() == Some(output)
            {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadOutputRecorded {
                download: *download,
                output: output.clone(),
            }))
        }
        Fact::TransferFailed {
            download,
            attempt,
            message,
            retryable,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || item.lifecycle.is_retired() {
                return Ok(Decision::none());
            }
            if matches!(
                &item.lifecycle,
                DownloadLifecycle::Failed {
                    message: current,
                    retryable: current_retryable,
                } if current == message && current_retryable == retryable
            ) {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadLifecycleChanged {
                download: *download,
                lifecycle: DownloadLifecycle::Failed {
                    message: message.clone(),
                    retryable: *retryable,
                },
                control_generation: item.control_generation,
            }))
        }
        Fact::TorrentResolved {
            download,
            attempt,
            info_hash,
            metadata,
            files,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.attempt != *attempt || !matches!(item.source, DownloadSource::Torrent(_)) {
                return Ok(Decision::none());
            }
            let normalized = info_hash.to_ascii_lowercase();
            if let DownloadSource::Torrent(torrent) = &item.source
                && torrent.info_hash.as_deref() == Some(normalized.as_str())
                && torrent.metadata.as_ref() == Some(metadata)
                && torrent.files == *files
            {
                return Ok(Decision::none());
            }
            if let Some((_, existing)) = state.workspaces.iter().find_map(|workspace| {
                workspace.downloads.iter().find_map(|candidate| {
                    (candidate.id != *download
                        && candidate
                            .active_info_hash()
                            .is_some_and(|hash| hash.eq_ignore_ascii_case(&normalized)))
                    .then_some((workspace.id, candidate))
                })
            }) {
                return Ok(simple(Event::DownloadLifecycleChanged {
                    download: *download,
                    lifecycle: DownloadLifecycle::Failed {
                        message: format!(
                            "This torrent already exists as download {}.",
                            existing.id
                        ),
                        retryable: false,
                    },
                    control_generation: item.control_generation,
                }));
            }
            Ok(simple(Event::TorrentResolved {
                download: *download,
                info_hash: normalized,
                metadata: metadata.clone(),
                files: files.clone(),
            }))
        }
        Fact::LeftoversFound {
            download,
            attempt,
            leftovers,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            let DownloadLifecycle::Retiring {
                move_files_to_trash: true,
                writer_stopped: false,
                leftovers: current,
                ..
            } = &item.lifecycle
            else {
                return Ok(Decision::none());
            };
            if item.attempt != *attempt || current == leftovers {
                return Ok(Decision::none());
            }
            Ok(simple(Event::RetirementLeftoversRecorded {
                download: *download,
                leftovers: leftovers.clone(),
            }))
        }
        Fact::WriterStopped {
            download,
            attempt,
            control_generation,
        } => retirement_progress(
            state,
            *download,
            *attempt,
            Some(*control_generation),
            Some(true),
            None,
            None,
        ),
        Fact::AdapterRecordAbsent { download, attempt } => {
            retirement_progress(state, *download, *attempt, None, None, Some(true), None)
        }
        Fact::FilesResolved {
            download,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            retirement_progress(
                state,
                *download,
                item.attempt,
                Some(*control_generation),
                None,
                None,
                Some(true),
            )
        }
        Fact::RetirementFinalized {
            download,
            control_generation,
        } => {
            let (_, item) = match state.locate_download(*download) {
                Some(found) => found,
                None => return Ok(Decision::none()),
            };
            if item.control_generation != *control_generation
                || !matches!(
                    item.lifecycle,
                    DownloadLifecycle::Retiring {
                        writer_stopped: true,
                        adapter_absent: true,
                        files_resolved: true,
                        ..
                    }
                )
            {
                return Ok(Decision::none());
            }
            Ok(simple(Event::DownloadRetired {
                download: *download,
            }))
        }
    }
}

/// What a torrent engine report means for the Download. Completion comes only from the
/// engine: seeding proves every selected file is there, and fetching outside a recheck
/// proves something is missing (more files were selected, or a recheck found a gap).
fn torrent_advanced(item: &Download, torrent: &TorrentDownload, phase: ActivePhase) -> Decision {
    // Not seeding until the person starts it again; a late "seeding" report is stale.
    if matches!(item.lifecycle, DownloadLifecycle::Finished) {
        return Decision::none();
    }
    let phase = match phase {
        // Rechecking proves nothing either way. A complete torrent keeps what it shows;
        // an incomplete one reads as downloading, as it always has.
        ActivePhase::Verifying if torrent.complete => return Decision::none(),
        ActivePhase::Verifying => ActivePhase::Fetching,
        phase => phase,
    };
    let complete = phase == ActivePhase::Seeding;
    let mut events = Vec::new();
    if torrent.complete != complete {
        events.push(Event::TorrentCompletionChanged {
            download: item.id,
            complete,
        });
    }
    if !matches!(item.lifecycle, DownloadLifecycle::Active(current) if current == phase) {
        events.push(Event::DownloadLifecycleChanged {
            download: item.id,
            lifecycle: DownloadLifecycle::Active(phase),
            control_generation: item.control_generation,
        });
    }
    Decision {
        events,
        effects: Vec::new(),
        created_downloads: Vec::new(),
    }
}
