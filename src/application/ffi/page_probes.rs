use super::*;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};

pub(super) fn is_page_probe(actor: &ApplicationActor, effect: &PendingEffect) -> bool {
    matches!(effect.kind, EffectKind::EnsureTransfer { .. })
        && effect.target().and_then(|id| actor.execution_context(id).ok().flatten())
            .is_some_and(|context| matches!(&context.download.source, DownloadSource::Media(media) if matches!(media.origin, MediaOrigin::PendingPage { .. })))
}

pub(super) fn start_worker(
    actor: Arc<ApplicationActor>,
    media: Arc<MediaEngine>,
    stop: Arc<AtomicBool>,
) -> Result<(Sender<PendingEffect>, JoinHandle<()>), StateMachineError> {
    let (sender, receiver) = mpsc::channel::<PendingEffect>();
    let worker = thread::Builder::new()
        .name("opul-page-probes".into())
        .spawn(move || {
            while !stop.load(Ordering::Acquire) {
                match receiver.recv_timeout(Duration::from_millis(100)) {
                    Ok(effect) => probe_page(&actor, &media, &stop, effect),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        })
        .map_err(|error| StateMachineError::Open {
            reason: format!("could not start page probe worker: {error}"),
        })?;
    Ok((sender, worker))
}

fn probe_page(
    actor: &ApplicationActor,
    media: &MediaEngine,
    stop: &AtomicBool,
    effect: PendingEffect,
) {
    let Some(id) = effect.target() else {
        return;
    };
    let Ok(Some(context)) = actor.execution_context(id) else {
        let _ = actor.complete_effect(effect.id, Vec::new());
        return;
    };
    let DownloadSource::Media(source) = &context.download.source else {
        return;
    };
    let MediaOrigin::PendingPage { url } = &source.origin else {
        return;
    };
    let current = || {
        !stop.load(Ordering::Acquire)
            && matches!(actor.snapshot().session, SessionPhase::Running)
            && actor
                .execution_context(id)
                .ok()
                .flatten()
                .is_some_and(|context| {
                    effect.attempt == Some(context.download.attempt)
                        && effect.control_generation == Some(context.download.control_generation)
                        && matches!(
                            context.download.lifecycle,
                            DownloadLifecycle::Queued | DownloadLifecycle::Starting
                        )
                })
    };
    let result = media.probe_url_while(url.clone(), current);
    let facts = if !current() {
        Vec::new()
    } else {
        match result {
            Ok(probe) => {
                let items = probe
                    .items
                    .into_iter()
                    .map(|item| {
                        let mut download = context.download.clone();
                        download.id = DownloadId::new();
                        download.name = item.title;
                        let mut source = source.clone();
                        source.origin = MediaOrigin::Page {
                            page_url: probe.webpage_url.clone(),
                            item_url: item.webpage_url,
                            playlist_index: item.playlist_index,
                        };
                        source.thumbnail_url = item.thumbnail_url;
                        source.extractor = item.extractor;
                        download.source = DownloadSource::Media(source);
                        download
                    })
                    .collect();
                vec![Fact::MediaPageResolved {
                    download: id,
                    attempt: context.download.attempt,
                    control_generation: context.download.control_generation,
                    items,
                }]
            }
            Err(error) => vec![Fact::MediaPageFailed {
                download: id,
                attempt: context.download.attempt,
                control_generation: context.download.control_generation,
                message: error.to_string(),
            }],
        }
    };
    let _ = actor.complete_effect(effect.id, facts);
}
