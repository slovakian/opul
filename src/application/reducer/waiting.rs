use super::*;

pub(super) fn hold_for_tools(state: &AppState, download: &mut Download) {
    if download.needs_video_tools() && state.video_tools != VideoToolsAvailability::Ready {
        download.lifecycle = DownloadLifecycle::WaitingForVideoTools;
    }
}

pub(super) fn start_effect(download: &Download) -> Option<EffectPlan> {
    matches!(download.lifecycle, DownloadLifecycle::Queued).then_some(EffectPlan {
        attempt: Some(download.attempt),
        control_generation: Some(download.control_generation),
        kind: EffectKind::EnsureTransfer {
            download: download.id,
        },
    })
}

pub(super) fn queue_waiting(download: &Download, decision: &mut Decision) {
    decision.events.push(Event::DownloadLifecycleChanged {
        download: download.id,
        lifecycle: DownloadLifecycle::Queued,
        control_generation: download.control_generation,
    });
    decision.effects.push(EffectPlan {
        attempt: Some(download.attempt),
        control_generation: Some(download.control_generation),
        kind: EffectKind::EnsureTransfer {
            download: download.id,
        },
    });
}

pub(super) fn decide_video_tools(
    state: &AppState,
    availability: VideoToolsAvailability,
) -> Decision {
    let mut decision = Decision::none();
    if availability != state.video_tools {
        decision.events.push(Event::VideoToolsChanged(availability));
    }
    if availability == VideoToolsAvailability::Ready
        && matches!(state.session, SessionPhase::Running)
    {
        for download in state
            .workspaces
            .iter()
            .flat_map(|workspace| &workspace.downloads)
        {
            if matches!(download.lifecycle, DownloadLifecycle::WaitingForVideoTools) {
                queue_waiting(download, &mut decision);
            }
        }
    }
    decision
}

pub(super) fn decide_page_resolved(
    state: &AppState,
    id: DownloadId,
    attempt: u32,
    generation: u64,
    items: &[Download],
) -> Result<Decision, Rejection> {
    let Some((workspace_id, pending)) = state.locate_download(id) else {
        return Ok(Decision::none());
    };
    if pending.attempt != attempt
        || pending.control_generation != generation
        || !matches!(
            pending.lifecycle,
            DownloadLifecycle::Queued | DownloadLifecycle::Starting
        )
        || !matches!(&pending.source, DownloadSource::Media(media) if matches!(media.origin, MediaOrigin::PendingPage { .. }))
    {
        return Ok(Decision::none());
    }
    let workspace = state.workspace(workspace_id).unwrap();
    let mut keys = workspace
        .downloads
        .iter()
        .filter(|download| download.id != id)
        .map(|download| download.dedupe_key().to_string())
        .collect::<std::collections::BTreeSet<_>>();
    let mut accepted = Vec::new();
    let mut decision = Decision::none();
    for item in items {
        if !keys.insert(item.dedupe_key().to_string()) {
            continue;
        }
        let mut item = item.clone();
        // The first unique item keeps the accepted Download's identity and position.
        if accepted.is_empty() {
            item.id = id;
        }
        item.folder = pending.folder;
        item.attempt = pending.attempt;
        item.control_generation = pending.control_generation;
        if let (DownloadSource::Media(media), DownloadSource::Media(original)) =
            (&mut item.source, &pending.source)
        {
            media.format = original.format;
            media.policy = original.policy.clone();
        }
        hold_for_tools(state, &mut item);
        decision.effects.extend(start_effect(&item));
        accepted.push(item);
    }
    if accepted.is_empty() {
        // Keep the original identity when every playlist entry already exists in this Workspace.
        decision.events.push(Event::DownloadLifecycleChanged {
            download: id,
            lifecycle: DownloadLifecycle::Cancelled,
            control_generation: generation,
        });
    } else {
        decision.events.push(Event::MediaPageExpanded {
            workspace: workspace_id,
            download: id,
            items: accepted,
        });
    }
    Ok(decision)
}
