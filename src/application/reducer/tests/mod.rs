use super::*;
use uuid::Uuid;

pub(super) fn new_workspace(state: &AppState, icon: &str) -> Workspace {
    let mut workspace = AppState::fresh("/tmp/NewDownloads".into())
        .workspaces
        .remove(0);
    workspace.icon = WorkspaceIcon::Emoji(icon.into());
    workspace.appearance = crate::application::WorkspaceAppearance::default();
    assert!(state.workspace(workspace.id).is_none());
    workspace
}

pub(super) fn running_state() -> AppState {
    let mut state = AppState::fresh("/tmp/Downloads".to_string());
    state.session = SessionPhase::Running;
    state
}

pub(super) fn media_download(state: &AppState) -> Download {
    Download {
        id: DownloadId::new(),
        folder: state.workspaces[0].default_folder,
        name: "Film".to_string(),
        attempt: 0,
        control_generation: 0,
        source: DownloadSource::Media(MediaDownload {
            origin: MediaOrigin::Page {
                page_url: "https://example.com/watch".to_string(),
                item_url: "https://example.com/watch?v=1".to_string(),
                playlist_index: None,
            },
            format: MediaFormat::Best,
            policy: MediaPolicy::default(),
            legacy_binding: None,
            thumbnail_url: None,
            extractor: None,
        }),
        lifecycle: DownloadLifecycle::Queued,
        output: None,
    }
}

pub(super) fn command(command: Command) -> CommandEnvelope {
    CommandEnvelope {
        id: CommandId::new(),
        fingerprint: Uuid::new_v4().to_string(),
        expected_revision: None,
        command,
    }
}

pub(super) fn run(state: &AppState, command: Command) -> AppState {
    let decision = decide(state, &self::command(command)).unwrap();
    evolve(state, &decision).unwrap()
}

pub(super) fn apply_facts(state: &AppState, facts: &[Fact]) -> (AppState, Vec<EffectKind>) {
    let mut next = state.clone();
    let mut effects = Vec::new();
    for fact in facts {
        let decision = decide_fact(&next, fact).unwrap();
        effects.extend(decision.effects.iter().map(|effect| effect.kind.clone()));
        next = evolve(&next, &decision).unwrap();
    }
    (next, effects)
}

pub(super) fn partial(path: &str) -> OutputArtifact {
    OutputArtifact {
        path: path.to_string(),
        content_folder: None,
        identity: Some(OutputIdentity {
            byte_length: 10,
            modified_at_millis: 1,
            file_identifier: Some(format!("inode-{path}")),
        }),
    }
}

mod commands_and_appearance;
mod retirement;
