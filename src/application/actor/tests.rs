use super::*;
use crate::application::{
    Command, Download, DownloadLifecycle, DownloadSource, MediaDownload, MediaFormat, MediaOrigin,
};
use uuid::Uuid;

fn temp_dir() -> PathBuf {
    let path = std::env::temp_dir().join(format!("opul-actor-{}", Uuid::new_v4()));
    fs::create_dir_all(&path).unwrap();
    path
}

fn envelope(command: Command) -> CommandEnvelope {
    CommandEnvelope {
        id: super::super::CommandId::new(),
        fingerprint: Uuid::new_v4().to_string(),
        expected_revision: None,
        command,
    }
}

fn make_running(actor: &ApplicationActor) {
    actor.dispatch(envelope(Command::MarkReconciled)).unwrap();
}

#[test]
fn recently_saved_emoji_are_durable_bounded_and_deduplicated() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".into()).unwrap();
    make_running(&actor);
    let id = actor.snapshot().workspaces[0].id;
    assert!(actor.snapshot().recent_emoji.is_empty());
    let save = |emoji: &str| {
        envelope(Command::SetWorkspaceAppearance {
            workspace: id,
            emoji: emoji.into(),
            appearance: crate::WorkspaceAppearance::default(),
        })
    };
    actor.dispatch(save("👩🏽‍💻")).unwrap();
    actor.dispatch(save("👍🏿")).unwrap();
    let retry = save("👩🏽‍💻");
    actor.dispatch(retry.clone()).unwrap();
    let checkpoint: serde_json::Value =
        serde_json::from_slice(&fs::read(folder.join("application-state.json")).unwrap()).unwrap();
    assert_eq!(
        checkpoint["state"]["recent_emoji"],
        serde_json::json!(["👩🏽‍💻", "👍🏿"])
    );
    let mut expected = Vec::new();
    for index in 0..30 {
        let emoji = char::from_u32(0x1f600 + index).unwrap().to_string();
        actor.dispatch(save(&emoji)).unwrap();
        expected.insert(0, emoji);
        expected.truncate(super::super::domain::RECENT_EMOJI_LIMIT);
    }
    actor.dispatch(retry.clone()).unwrap();
    assert_eq!(actor.snapshot().recent_emoji, expected);
    drop(actor);
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
    assert_eq!(actor.snapshot().recent_emoji, expected);
    actor.dispatch(retry).unwrap();
    assert_eq!(actor.snapshot().recent_emoji, expected);
    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn creation_draft_commits_icon_and_frozen_appearance_together_and_retries_after_restart() {
    for emoji in ["📁", "👨‍👩‍👧‍👦", "👩🏽‍💻", "👍🏿", "🫱🏻‍🫲🏿"]
    {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".into()).unwrap();
        make_running(&actor);
        let before = actor.snapshot();
        let checkpoint = folder.join("application-state.json");
        let bytes = fs::read(&checkpoint).unwrap();
        // Opening and canceling a proposal never publishes or checkpoints a workspace.
        let canceled_id = super::super::WorkspaceId::new();
        actor.propose_workspace_appearance(canceled_id).unwrap();
        assert_eq!(actor.snapshot(), before);
        assert_eq!(fs::read(&checkpoint).unwrap(), bytes);

        let mut workspace = AppState::fresh("/tmp/Family".into()).workspaces.remove(0);
        workspace.name = "Family".into();
        workspace.icon = super::super::WorkspaceIcon::Emoji(emoji.into());
        workspace.appearance = actor.propose_workspace_appearance(workspace.id).unwrap();
        if emoji != "📁" {
            workspace.appearance.preference = crate::AppearancePreference::Original;
        }
        let id = workspace.id;
        let appearance = workspace.appearance.clone();
        let request = envelope(Command::CreateWorkspace { workspace });

        // A checkpoint failure cannot publish either half of the creation draft.
        let backup = folder.join("application-state.json.bak");
        fs::remove_file(&backup).unwrap();
        fs::create_dir(&backup).unwrap();
        assert!(actor.dispatch(request.clone()).is_err());
        assert_eq!(actor.snapshot().workspaces, before.workspaces);
        assert_eq!(actor.snapshot().revision, before.revision);
        assert_eq!(actor.snapshot().recent_emoji, before.recent_emoji);
        assert_eq!(fs::read(&checkpoint).unwrap(), bytes);
        drop(actor);
        fs::remove_dir(&backup).unwrap();

        // Durability faults put the actor in read-only mode until restart.
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
        make_running(&actor);
        actor.dispatch(request.clone()).unwrap();
        let saved = actor.snapshot();
        let created = saved
            .workspaces
            .iter()
            .find(|workspace| workspace.id == id)
            .unwrap();
        assert_eq!(
            created.icon,
            super::super::WorkspaceIcon::Emoji(emoji.into())
        );
        assert_eq!(created.appearance, appearance);
        assert_eq!(saved.recent_emoji, vec![emoji.to_string()]);
        assert!(
            saved
                .workspaces
                .iter()
                .all(|workspace| workspace.id != canceled_id)
        );
        drop(actor);

        let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
        // An unchanged retry after a lost response creates exactly one workspace.
        actor.dispatch(request).unwrap();
        let reopened = actor.snapshot();
        assert_eq!(reopened.workspaces, saved.workspaces);
        assert_eq!(reopened.recent_emoji, saved.recent_emoji);
        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }
}

#[test]
fn appearance_commit_is_durable_idempotent_and_does_not_overwrite_settings() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".into()).unwrap();
    make_running(&actor);
    let before = actor.snapshot().workspaces[0].clone();
    let id = before.id;
    let request = envelope(Command::SetWorkspaceAppearance {
        workspace: id,
        emoji: "👩🏽‍💻".into(),
        appearance: crate::WorkspaceAppearance {
            automatic_folder: None,
            preference: crate::AppearancePreference::Preset {
                treatment: crate::application::appearance::FOLDER_TREATMENTS[1].clone(),
            },
        },
    });
    actor
        .dispatch(envelope(Command::SetDefaultFormat {
            workspace: id,
            format: MediaFormat::AudioMp3,
        }))
        .unwrap();
    actor.dispatch(request.clone()).unwrap();
    let saved = actor.snapshot().workspaces[0].clone();
    assert_eq!(saved.default_format, MediaFormat::AudioMp3);
    assert_eq!(
        saved.appearance.automatic_folder,
        before.appearance.automatic_folder
    );
    drop(actor);
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
    assert_eq!(actor.snapshot().workspaces[0], saved);
    // A retry after a lost response is recognized even after restart.
    actor.dispatch(request).unwrap();
    assert_eq!(actor.snapshot().workspaces[0], saved);
    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn resolved_custom_treatment_survives_restart_for_eligible_and_excluded_emoji() {
    for emoji in ["📁", "🌈", "⚪️", "👩🏽‍💻"] {
        let folder = temp_dir();
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".into()).unwrap();
        make_running(&actor);
        let before = actor.snapshot().workspaces[0].clone();
        let preference = crate::AppearancePreference::Custom {
            hue: 359,
            saturation: 1000,
            brightness: 50,
        };
        let request = envelope(Command::SetWorkspaceAppearance {
            workspace: before.id,
            emoji: emoji.into(),
            appearance: crate::WorkspaceAppearance {
                automatic_folder: None,
                preference: preference.clone(),
            },
        });
        actor.dispatch(request.clone()).unwrap();
        let saved = actor.snapshot().workspaces[0].clone();
        assert_eq!(saved.appearance.preference, preference);
        assert_eq!(
            saved.appearance.automatic_folder,
            before.appearance.automatic_folder
        );
        assert_eq!(saved.icon, super::super::WorkspaceIcon::Emoji(emoji.into()));
        drop(actor);
        let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
        assert_eq!(actor.snapshot().workspaces[0], saved);
        actor.dispatch(request).unwrap();
        assert_eq!(actor.snapshot().workspaces[0], saved);
        drop(actor);
        fs::remove_dir_all(folder).unwrap();
    }
}

#[test]
fn failed_appearance_checkpoint_never_publishes_a_partial_icon() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".into()).unwrap();
    make_running(&actor);
    let before = actor.snapshot().workspaces[0].clone();
    // A file where the backup must go forces failure before installing the new checkpoint.
    let backup = folder.join("application-state.json.bak");
    fs::remove_file(&backup).unwrap();
    fs::create_dir(&backup).unwrap();
    assert!(
        actor
            .dispatch(envelope(Command::SetWorkspaceAppearance {
                workspace: before.id,
                emoji: "👩🏽‍💻".into(),
                appearance: crate::WorkspaceAppearance {
                    automatic_folder: None,
                    preference: crate::AppearancePreference::Original
                },
            }))
            .is_err()
    );
    assert_eq!(actor.snapshot().workspaces[0], before);
    assert!(actor.snapshot().recent_emoji.is_empty());
    drop(actor);
    fs::remove_dir(&backup).unwrap();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
    assert_eq!(actor.snapshot().workspaces[0], before);
    assert!(actor.snapshot().recent_emoji.is_empty());
    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn folder_migration_checkpoints_once_and_survives_restart_without_writes_on_reads() {
    let folder = temp_dir();
    let path = folder.join("application-state.json");
    let store = CheckpointStore::new(path.clone());
    let mut old = AppState::fresh("/tmp/Downloads".into());
    old.workspaces[0].appearance = crate::WorkspaceAppearance::default();
    old.workspaces[0].icon = super::super::WorkspaceIcon::Symbol("arrow.down.circle".into());
    // Empty appearance is omitted, reproducing the old serialized shape and checksum.
    store.commit(&old).unwrap();
    let original_bytes = fs::read(&path).unwrap();
    assert!(!String::from_utf8_lossy(&original_bytes).contains("appearance"));
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
    let first = actor.snapshot();
    assert_eq!(first.workspaces[0].id, old.workspaces[0].id);
    assert!(!first.workspaces[0].appearance.is_unassigned());
    assert_eq!(
        fs::read(path.with_extension("json.bak")).unwrap(),
        original_bytes
    );
    let migrated_bytes = fs::read(&path).unwrap();
    for _ in 0..5 {
        assert_eq!(actor.snapshot(), first);
        let proposal = actor
            .propose_workspace_appearance(super::super::WorkspaceId::new())
            .unwrap();
        assert!(!proposal.is_unassigned());
    }
    assert_eq!(fs::read(&path).unwrap(), migrated_bytes);
    drop(actor);
    let reopened = ApplicationActor::open(folder.clone(), "/tmp/Unused".into()).unwrap();
    assert_eq!(reopened.snapshot().workspaces, first.workspaces);
    drop(reopened);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn actor_commits_intent_and_outbox_before_effect_claim() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    make_running(&actor);
    let projection = actor.snapshot();
    let workspace = projection.workspaces[0].id;
    let saved_folder = projection.workspaces[0].default_folder;
    let download = Download {
        id: super::super::DownloadId::new(),
        folder: saved_folder,
        name: "Film".to_string(),
        attempt: 0,
        control_generation: 0,
        source: DownloadSource::Media(MediaDownload {
            origin: MediaOrigin::Direct {
                url: "https://example.com/film.mp4".to_string(),
                referer: None,
            },
            format: MediaFormat::Original,
            policy: crate::application::MediaPolicy::default(),
            legacy_binding: None,
            thumbnail_url: None,
            extractor: None,
        }),
        lifecycle: DownloadLifecycle::Queued,
        output: None,
    };
    let receipt = actor
        .dispatch(envelope(Command::AddDownload {
            workspace,
            download: Box::new(download.clone()),
        }))
        .unwrap();
    assert_eq!(receipt.created_downloads, vec![download.id]);
    drop(actor);

    let reopened = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    // Starts stay fenced on reopen until reconciliation completes.
    assert!(reopened.claim_effect().unwrap().is_none());
    make_running(&reopened);
    assert!(matches!(
        reopened.claim_effect().unwrap().map(|effect| effect.kind),
        Some(super::super::EffectKind::EnsureTransfer { download: id }) if id == download.id
    ));
    drop(reopened);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn process_restart_releases_in_flight_effect_leases() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    make_running(&actor);
    let projection = actor.snapshot();
    let download = Download {
        id: super::super::DownloadId::new(),
        folder: projection.workspaces[0].default_folder,
        name: "Restartable".to_string(),
        attempt: 0,
        control_generation: 0,
        source: DownloadSource::Media(MediaDownload {
            origin: MediaOrigin::Direct {
                url: "https://example.com/restart.mp4".to_string(),
                referer: None,
            },
            format: MediaFormat::Original,
            policy: crate::application::MediaPolicy::default(),
            legacy_binding: None,
            thumbnail_url: None,
            extractor: None,
        }),
        lifecycle: DownloadLifecycle::Queued,
        output: None,
    };
    actor
        .dispatch(envelope(Command::AddDownload {
            workspace: projection.workspaces[0].id,
            download: Box::new(download.clone()),
        }))
        .unwrap();
    assert!(actor.claim_effect().unwrap().is_some());
    drop(actor);

    let reopened = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    make_running(&reopened);
    assert!(matches!(
        reopened.claim_effect().unwrap().map(|effect| effect.kind),
        Some(super::super::EffectKind::EnsureTransfer { download: id }) if id == download.id
    ));
    drop(reopened);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn command_retry_returns_the_original_receipt() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    make_running(&actor);
    let projection = actor.snapshot();
    let command_id = super::super::CommandId::new();
    let workspace = crate::application::Workspace {
        id: super::super::WorkspaceId::new(),
        name: "Study".to_string(),
        icon: crate::application::WorkspaceIcon::Emoji("📚".to_string()),
        appearance: crate::application::WorkspaceAppearance::default(),
        folders: vec![crate::application::SavedFolder {
            id: super::super::FolderId::new(),
            name: "Study".to_string(),
            path: "/tmp/Study".to_string(),
        }],
        default_folder: super::super::FolderId::new(),
        default_format: MediaFormat::Best,
        file_organization: crate::application::FileOrganization::Automatic,
        view: crate::application::WorkspaceView::default(),
        downloads: Vec::new(),
    };
    let mut workspace = workspace;
    workspace.default_folder = workspace.folders[0].id;
    let request = CommandEnvelope {
        id: command_id,
        fingerprint: "same".to_string(),
        expected_revision: Some(projection.revision),
        command: Command::CreateWorkspace { workspace },
    };
    let first = actor.dispatch(request.clone()).unwrap();
    let repeated = actor.dispatch(request).unwrap();
    assert_eq!(first, repeated);
    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn state_directory_allows_only_one_actor() {
    let folder = temp_dir();
    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    assert!(matches!(
        ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()),
        Err(ActorError::AlreadyOpen)
    ));
    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn corrupt_primary_with_backup_recovers_and_acknowledges_cleanly() {
    let folder = temp_dir();
    let path = folder.join("application-state.json");
    let store = CheckpointStore::new(path.clone());
    let mut first = AppState::fresh("/tmp/Downloads".to_string());
    first.session = SessionPhase::Running;
    store.commit(&first).unwrap();
    let mut second = first.clone();
    second.revision = 1;
    store.commit(&second).unwrap();

    // Corrupt primary
    fs::write(&path, b"corrupted JSON").unwrap();

    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    assert_eq!(actor.snapshot().session, SessionPhase::RecoveryRequired);
    assert!(!actor.snapshot().recovery.is_empty());

    // Acknowledging recovery must succeed, transition to Reconciling, and clear recovery items
    let receipt = actor
        .dispatch(envelope(Command::AcknowledgeRecovery))
        .unwrap();
    assert_eq!(receipt.revision, first.revision + 1);
    assert_eq!(actor.snapshot().session, SessionPhase::Reconciling);
    assert!(actor.snapshot().recovery.is_empty());

    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn corrupt_primary_without_backup_starts_fresh_and_acknowledges_cleanly() {
    let folder = temp_dir();
    let path = folder.join("application-state.json");
    fs::write(&path, b"corrupted JSON without backup").unwrap();

    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    assert_eq!(actor.snapshot().session, SessionPhase::RecoveryRequired);
    assert!(!actor.snapshot().recovery.is_empty());

    // Must not be locked into read-only mode; acknowledging recovery must transition to Reconciling
    actor
        .dispatch(envelope(Command::AcknowledgeRecovery))
        .unwrap();
    assert_eq!(actor.snapshot().session, SessionPhase::Reconciling);
    assert!(actor.snapshot().recovery.is_empty());

    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}

#[test]
fn corrupt_legacy_library_starts_fresh_without_failing() {
    let folder = temp_dir();
    let legacy = folder.join("library.json");
    fs::write(&legacy, b"invalid legacy JSON").unwrap();

    let actor = ApplicationActor::open(folder.clone(), "/tmp/Downloads".to_string()).unwrap();
    let snapshot = actor.snapshot();
    assert_eq!(snapshot.workspaces.len(), 1);
    assert_eq!(snapshot.workspaces[0].name, "Downloads");
    assert!(
        snapshot
            .recovery
            .iter()
            .any(|r| r.key == "legacy-library-invalid")
    );

    drop(actor);
    fs::remove_dir_all(folder).unwrap();
}
