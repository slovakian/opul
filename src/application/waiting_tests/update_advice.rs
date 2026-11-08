use super::*;

#[test]
fn failed_managed_video_projects_update_advice_without_changing_the_error() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    let mut download = page(&state);
    let failure = DownloadLifecycle::Failed {
        message: "ERROR: original yt-dlp details, unchanged".into(),
        retryable: true,
    };
    download.lifecycle = failure.clone();
    state.workspaces[0].downloads.push(download);
    let mut overlay = RuntimeOverlay {
        video_update_available: true,
        ..RuntimeOverlay::default()
    };
    let projected = project(&state, &overlay);
    let item = &projected.workspaces[0].downloads[0];
    assert!(item.capabilities.can_update_video_tools);
    assert_eq!(item.lifecycle, failure);
    overlay.video_update_available = false;
    assert!(
        !project(&state, &overlay).workspaces[0].downloads[0]
            .capabilities
            .can_update_video_tools
    );
    overlay.video_update_available = true;
    state.workspaces[0].downloads[0].lifecycle = DownloadLifecycle::Finished;
    assert!(
        !project(&state, &overlay).workspaces[0].downloads[0]
            .capabilities
            .can_update_video_tools
    );
    let download = &mut state.workspaces[0].downloads[0];
    download.lifecycle = failure;
    if let DownloadSource::Media(media) = &mut download.source {
        media.origin = MediaOrigin::Direct {
            url: "https://example.test/file.mp4".into(),
            referer: None,
        };
    }
    assert!(
        !project(&state, &overlay).workspaces[0].downloads[0]
            .capabilities
            .can_update_video_tools
    );
}
