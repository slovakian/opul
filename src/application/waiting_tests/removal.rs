use super::*;

#[test]
fn removal_affects_running_and_waiting_video_downloads_only() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    let lifecycles = [
        (DownloadLifecycle::Queued, true),
        (DownloadLifecycle::Active(ActivePhase::Fetching), true),
        (DownloadLifecycle::WaitingForVideoTools, true),
        (
            DownloadLifecycle::Paused {
                cause: HoldCause::Shutdown,
                resume: ActivePhase::Fetching,
            },
            true,
        ),
        (
            DownloadLifecycle::Paused {
                cause: HoldCause::Person,
                resume: ActivePhase::Fetching,
            },
            false,
        ),
        (DownloadLifecycle::Finished, false),
        (DownloadLifecycle::Cancelled, false),
    ];
    for (lifecycle, _) in &lifecycles {
        let mut download = page(&state);
        download.lifecycle = lifecycle.clone();
        state.workspaces[0].downloads.push(download);
    }
    let mut direct = page(&state);
    if let DownloadSource::Media(media) = &mut direct.source {
        media.origin = MediaOrigin::Direct {
            url: "https://example.test/file.mp4".into(),
            referer: None,
        };
    }
    direct.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
    state.workspaces[0].downloads.push(direct);

    let projected = project(&state, &RuntimeOverlay::default());
    let relies = projected.workspaces[0]
        .downloads
        .iter()
        .map(|download| download.relies_on_video_tools)
        .collect::<Vec<_>>();
    let mut expected = lifecycles
        .iter()
        .map(|(_, relies)| *relies)
        .collect::<Vec<_>>();
    expected.push(false);
    assert_eq!(relies, expected);
}
