//! The one rule: a download that needs the video tools runs only while they are Ready.

use super::*;

#[test]
fn removal_keeps_what_the_person_paused_cancelled_or_saw_fail() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    state.video_tools = VideoToolsAvailability::Ready;
    let settled = [
        DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        },
        DownloadLifecycle::Pausing {
            from: ActivePhase::Fetching,
            cause: HoldCause::Person,
        },
        DownloadLifecycle::Cancelled,
        DownloadLifecycle::Failed {
            message: "gone".into(),
            retryable: false,
        },
    ];
    state.workspaces[0].downloads = settled
        .iter()
        .map(|lifecycle| Download {
            lifecycle: lifecycle.clone(),
            ..page(&state)
        })
        .collect();
    let next = apply_fact(
        &state,
        Fact::VideoToolsChanged(VideoToolsAvailability::NotSetUp),
    );
    let next = apply_fact(
        &next,
        Fact::VideoToolsChanged(VideoToolsAvailability::Ready),
    );
    let lifecycles: Vec<_> = next.workspaces[0]
        .downloads
        .iter()
        .map(|download| download.lifecycle.clone())
        .collect();
    assert_eq!(lifecycles, settled);
}

#[test]
fn tools_that_stopped_working_hold_unfinished_videos_at_startup() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Reconciling;
    state.video_tools = VideoToolsAvailability::NotWorking;
    let mut video = page(&state);
    video.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
    state.workspaces[0].downloads = vec![video.clone()];
    let decision = decide(&state, &command(Command::MarkReconciled)).unwrap();
    assert!(decision.effects.is_empty());
    let next = evolve(&state, &decision).unwrap();
    assert_eq!(
        next.workspaces[0].downloads[0].lifecycle,
        DownloadLifecycle::WaitingForVideoTools
    );
}

#[test]
fn resuming_a_video_without_tools_waits_for_them() {
    let mut state = AppState::fresh("/tmp/Films".into());
    state.session = SessionPhase::Running;
    state.video_tools = VideoToolsAvailability::TurnedOff;
    let mut video = page(&state);
    video.lifecycle = DownloadLifecycle::Paused {
        resume: ActivePhase::Fetching,
        cause: HoldCause::Person,
    };
    state.workspaces[0].downloads = vec![video.clone()];
    let decision = decide(&state, &command(Command::Resume { download: video.id })).unwrap();
    assert!(decision.effects.is_empty());
    let next = evolve(&state, &decision).unwrap();
    assert_eq!(
        next.workspaces[0].downloads[0].lifecycle,
        DownloadLifecycle::WaitingForVideoTools
    );
}
