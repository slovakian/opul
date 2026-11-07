use super::*;

pub(super) fn execute_effect(
    actor: &ApplicationActor,
    torrent: &TorrentEngine,
    media: &MediaEngine,
    guided: &Mutex<HashMap<DownloadId, GuidedInput>>,
    platform: &Mutex<Vec<MachinePlatformEffect>>,
    effect: PendingEffect,
) {
    let result = match &effect.kind {
        EffectKind::EnsureTransfer { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                ensure_transfer(torrent, media, guided, &context)
                    .map_err(|error| ActorError::Rejected(error.to_string()))?;
                actor.complete_effect(
                    effect.id,
                    vec![Fact::TransferStarted {
                        download: context.download.id,
                        attempt: context.download.attempt,
                        control_generation: context.download.control_generation,
                        phase: ActivePhase::Fetching,
                    }],
                )
            }),
        EffectKind::ConvergeControl { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                converge_control(torrent, media, &context);
                actor.complete_effect(effect.id, Vec::new())
            }),
        EffectKind::RemoveTransfer { download } => actor
            .execution_context(*download)
            .and_then(|context| {
                context.ok_or_else(|| ActorError::Rejected("download disappeared".to_string()))
            })
            .and_then(|context| {
                // Ask the adapter what it wrote before its record goes away: that is the only
                // evidence of partial files, and of output the aggregate never verified.
                let owned = if matches!(
                    context.download.lifecycle,
                    DownloadLifecycle::Retiring {
                        move_files_to_trash: true,
                        ..
                    }
                ) {
                    owned_paths(torrent, media, &context)
                } else {
                    Vec::new()
                };
                remove_transfer(torrent, media, &context);
                let mut facts = Vec::new();
                if !owned.is_empty() {
                    facts.push(Fact::LeftoversFound {
                        download: context.download.id,
                        attempt: context.download.attempt,
                        leftovers: leftover_artifacts(owned),
                    });
                }
                facts.push(Fact::WriterStopped {
                    download: context.download.id,
                    attempt: context.download.attempt,
                    control_generation: context.download.control_generation,
                });
                facts.push(Fact::AdapterRecordAbsent {
                    download: context.download.id,
                    attempt: context.download.attempt,
                });
                actor.complete_effect(effect.id, facts)
            }),
        EffectKind::MoveOutputToTrash {
            download,
            artifact,
            leftovers,
        } => {
            let platform_effect = MachinePlatformEffect {
                effect_id: effect.id.to_string(),
                download_id: download.to_string(),
                targets: artifact
                    .iter()
                    .chain(leftovers)
                    .map(|artifact| MachineTrashTarget {
                        path: artifact
                            .content_folder
                            .clone()
                            .unwrap_or_else(|| artifact.path.clone()),
                        expected_byte_length: artifact
                            .identity
                            .as_ref()
                            .map(|identity| identity.byte_length),
                        expected_modified_at_millis: artifact
                            .identity
                            .as_ref()
                            .map(|identity| identity.modified_at_millis),
                        expected_file_identifier: artifact
                            .identity
                            .as_ref()
                            .and_then(|identity| identity.file_identifier.clone()),
                    })
                    .collect(),
            };
            let mut effects = platform
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if !effects
                .iter()
                .any(|pending| pending.effect_id == platform_effect.effect_id)
            {
                effects.push(platform_effect);
            }
            Ok(0)
        }
        EffectKind::FinalizeRetirement { download } => actor.complete_effect(
            effect.id,
            vec![Fact::RetirementFinalized {
                download: *download,
                control_generation: effect.control_generation.unwrap_or_default(),
            }],
        ),
        EffectKind::StopAllTransfers => {
            torrent.stop_all();
            media.stop_all();
            actor.complete_effect(effect.id, Vec::new())
        }
    };
    if let Err(error) = result {
        // Engine failure becomes a typed fact when the target still exists. The effect is
        // acknowledged so it cannot busy-loop; Retry creates a fresh incarnation.
        if let Some(download) = effect.target()
            && let Ok(Some(context)) = actor.execution_context(download)
        {
            let _ = actor.complete_effect(
                effect.id,
                vec![Fact::TransferFailed {
                    download,
                    attempt: context.download.attempt,
                    message: error.to_string(),
                    retryable: true,
                }],
            );
        }
    }
}

pub(super) fn ensure_transfer(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    guided: &Mutex<HashMap<DownloadId, GuidedInput>>,
    context: &ExecutionContext,
) -> Result<(), crate::CoreError> {
    let id = context.download.id.to_string();
    let layout = content_layout(context.organization);
    match &context.download.source {
        DownloadSource::Torrent(download) => match &download.origin {
            TorrentOrigin::Magnet { uri } => {
                torrent.ensure_magnet(
                    &id,
                    context.download.attempt,
                    uri.clone(),
                    context.output_folder.clone(),
                    layout,
                    download.seed_ratio_limit,
                )?;
            }
            TorrentOrigin::Metainfo { blob, .. } => {
                let path = blob.strip_prefix("legacy-path:").unwrap_or(blob);
                let bytes = fs::read(path).map_err(|error| crate::CoreError::Engine {
                    reason: format!("could not read torrent source {path}: {error}"),
                })?;
                torrent.ensure_torrent_file(TorrentFileInput {
                    download_id: id,
                    attempt: context.download.attempt,
                    bytes,
                    source: path.to_string(),
                    save_folder: context.output_folder.clone(),
                    content_layout: layout,
                    seed_ratio_limit: download.seed_ratio_limit,
                })?;
            }
            TorrentOrigin::RemoteMetainfo { url } => torrent.ensure_remote_torrent(
                &id,
                context.download.attempt,
                url.clone(),
                context.output_folder.clone(),
                layout,
                download.seed_ratio_limit,
            )?,
        },
        DownloadSource::Media(download) => {
            let options = media_options(download, layout);
            if matches!(download.origin, MediaOrigin::Guided { .. }) {
                let input = guided
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .remove(&context.download.id)
                    .ok_or_else(|| crate::CoreError::Engine {
                        reason:
                            "guided credentials expired before the transfer started; add it again"
                                .to_string(),
                    })?;
                media.ensure_guided_download(
                    &id,
                    context.download.attempt,
                    input.session,
                    input.selection,
                    context.output_folder.clone(),
                    options,
                )?;
            } else {
                let (url, page_url, playlist_index) = match &download.origin {
                    MediaOrigin::Page {
                        page_url,
                        item_url,
                        playlist_index,
                    } => (item_url.clone(), page_url.clone(), *playlist_index),
                    MediaOrigin::Direct { url, referer } => (
                        url.clone(),
                        referer.clone().unwrap_or_else(|| url.clone()),
                        None,
                    ),
                    MediaOrigin::Guided { .. } => unreachable!(),
                };
                media.ensure_download(
                    &id,
                    context.download.attempt,
                    matches!(download.origin, MediaOrigin::Direct { .. }),
                    MediaProbeItem {
                        webpage_url: url,
                        title: context.download.name.clone(),
                        thumbnail_url: download.thumbnail_url.clone(),
                        extractor: download.extractor.clone(),
                        duration_seconds: None,
                        playlist_index,
                    },
                    context.output_folder.clone(),
                    MediaDownloadOptions {
                        referer: Some(page_url),
                        ..options
                    },
                )?;
            }
            media.resume_download(&id, context.download.attempt);
        }
    }
    Ok(())
}

pub(super) fn converge_control(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    context: &ExecutionContext,
) {
    let id = context.download.id.to_string();
    match &context.download.source {
        DownloadSource::Torrent(download) => {
            let binding = torrent
                .binding_for(&id, context.download.attempt)
                .or_else(|| download.info_hash.clone());
            let Some(binding) = binding else { return };
            torrent.set_seed_ratio_limit(binding.clone(), download.seed_ratio_limit);
            if !download.files.is_empty() {
                let length = download
                    .files
                    .iter()
                    .map(|file| file.index as usize)
                    .max()
                    .unwrap_or_default()
                    + 1;
                let mut priorities = vec![crate::meta::FilePriority::Normal; length];
                for file in &download.files {
                    priorities[file.index as usize] = match file.priority {
                        TorrentFilePriority::Skip => crate::meta::FilePriority::Skip,
                        TorrentFilePriority::Low => crate::meta::FilePriority::Low,
                        TorrentFilePriority::Normal => crate::meta::FilePriority::Normal,
                        TorrentFilePriority::High => crate::meta::FilePriority::High,
                    };
                }
                torrent.set_file_priorities(binding.clone(), priorities);
            }
            match context.download.lifecycle {
                DownloadLifecycle::Starting | DownloadLifecycle::Active(_) => {
                    torrent.resume(binding)
                }
                DownloadLifecycle::Finished => torrent.stop_seeding(binding),
                _ => torrent.pause(binding),
            }
        }
        DownloadSource::Media(_) => match context.download.lifecycle {
            DownloadLifecycle::Starting | DownloadLifecycle::Active(_) => {
                media.resume_download(&id, context.download.attempt)
            }
            DownloadLifecycle::Cancelled => media.cancel_download(&id, context.download.attempt),
            _ => media.pause_download(&id, context.download.attempt),
        },
    }
}

pub(super) fn remove_transfer(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    context: &ExecutionContext,
) {
    let id = context.download.id.to_string();
    match context.download.source {
        DownloadSource::Torrent(_) => torrent.remove_download(&id, context.download.attempt),
        DownloadSource::Media(_) => media.remove_download(&id, context.download.attempt),
    }
}

/// Every path the adapters wrote for this Download, across all of its attempts, kept only
/// when it lies strictly inside the Download's saved folder.
pub(super) fn owned_paths(
    torrent: &TorrentEngine,
    media: &MediaEngine,
    context: &ExecutionContext,
) -> Vec<PathBuf> {
    let id = context.download.id.to_string();
    let paths = match context.download.source {
        DownloadSource::Torrent(_) => torrent.owned_paths(&id),
        DownloadSource::Media(_) => media.owned_paths(&id),
    };
    let folder = Path::new(&context.output_folder);
    let mut owned = Vec::<PathBuf>::new();
    for path in paths {
        let inside = path != folder
            && path.starts_with(folder)
            && !path
                .components()
                .any(|component| matches!(component, std::path::Component::ParentDir));
        // A folder already listed covers everything in it.
        if inside && !owned.iter().any(|known| path.starts_with(known)) {
            owned.retain(|known| !known.starts_with(&path));
            owned.push(path);
        }
    }
    owned
}

/// Leftovers that exist once the writer has stopped, with the identity they have now.
pub(super) fn leftover_artifacts(paths: Vec<PathBuf>) -> Vec<OutputArtifact> {
    paths
        .into_iter()
        .filter_map(|path| {
            let metadata = fs::symlink_metadata(&path).ok()?;
            let path = path.to_string_lossy().into_owned();
            let content_folder = metadata.is_dir().then(|| path.clone());
            Some(output_artifact(path, content_folder))
        })
        .collect()
}
