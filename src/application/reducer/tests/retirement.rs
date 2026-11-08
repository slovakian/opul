use super::*;

#[test]
fn finished_download_moves_into_another_saved_folder() {
    let mut state = running_state();
    let workspace = state.workspaces[0].id;
    let target = SavedFolder {
        id: FolderId::new(),
        name: "Films".to_string(),
        path: "/tmp/Films".to_string(),
    };
    state = run(
        &state,
        Command::AddFolder {
            workspace,
            folder: target.clone(),
            make_default: false,
        },
    );
    let mut download = media_download(&state);
    download.output = Some(OutputArtifact {
        path: "/tmp/Downloads/film.mp4".to_string(),
        content_folder: None,
        identity: None,
    });
    state.workspaces[0].downloads.push(download.clone());

    let move_to = |output_path: &str| Command::MoveDownload {
        download: download.id,
        folder: target.id,
        output_path: output_path.to_string(),
        content_folder: None,
        identity: None,
    };
    assert!(matches!(
        decide(&state, &command(move_to("/tmp/Films/film.mp4"))),
        Err(Rejection::InvalidTransition { action: "move", .. })
    ));

    state.workspaces[0].downloads[0].lifecycle = DownloadLifecycle::Finished;
    assert!(matches!(
        decide(&state, &command(move_to("/tmp/Elsewhere/film.mp4"))),
        Err(Rejection::InvalidInput(_))
    ));

    let moved = run(&state, move_to("/tmp/Films/film.mp4"));
    let (_, item) = moved.locate_download(download.id).unwrap();
    assert_eq!(item.folder, target.id);
    assert_eq!(item.output.as_ref().unwrap().path, "/tmp/Films/film.mp4");

    let identity = OutputIdentity {
        byte_length: 42,
        modified_at_millis: 7,
        file_identifier: Some("inode-9".to_string()),
    };
    let measured = run(
        &state,
        Command::MoveDownload {
            download: download.id,
            folder: target.id,
            output_path: "/tmp/Films/film.mp4".to_string(),
            content_folder: None,
            identity: Some(identity.clone()),
        },
    );
    let (_, item) = measured.locate_download(download.id).unwrap();
    assert_eq!(item.verified_output().unwrap().identity, Some(identity));
}

#[test]
fn retirement_keeps_evidence_until_every_dependency_acknowledges() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
    download.output = Some(OutputArtifact {
        path: "/tmp/film.mp4".to_string(),
        content_folder: None,
        identity: Some(OutputIdentity {
            byte_length: 42,
            modified_at_millis: 7,
            file_identifier: Some("inode-9".to_string()),
        }),
    });
    state.workspaces[0].downloads.push(download.clone());
    let retiring = run(
        &state,
        Command::Remove {
            downloads: vec![download.id],
            move_files_to_trash: true,
        },
    );
    assert!(retiring.locate_download(download.id).is_some());
    let writer = decide_fact(
        &retiring,
        &Fact::WriterStopped {
            download: download.id,
            attempt: 0,
            control_generation: 1,
        },
    )
    .unwrap();
    let after_writer = evolve(&retiring, &writer).unwrap();
    assert!(after_writer.locate_download(download.id).is_some());
    let absent = decide_fact(
        &after_writer,
        &Fact::AdapterRecordAbsent {
            download: download.id,
            attempt: 0,
        },
    )
    .unwrap();
    assert!(matches!(
        absent.effects.first().map(|effect| &effect.kind),
        Some(EffectKind::MoveOutputToTrash { .. })
    ));
}

#[test]
fn every_unfinished_state_can_trash_its_partial_files() {
    let lifecycles = [
        DownloadLifecycle::Queued,
        DownloadLifecycle::Starting,
        DownloadLifecycle::Active(ActivePhase::Fetching),
        DownloadLifecycle::Active(ActivePhase::Processing),
        DownloadLifecycle::Pausing {
            from: ActivePhase::Fetching,
            cause: HoldCause::Person,
        },
        DownloadLifecycle::Paused {
            resume: ActivePhase::Fetching,
            cause: HoldCause::Person,
        },
        DownloadLifecycle::Failed {
            message: "network".to_string(),
            retryable: true,
        },
        DownloadLifecycle::Cancelled,
    ];
    for lifecycle in lifecycles {
        let mut state = running_state();
        let mut download = media_download(&state);
        download.lifecycle = lifecycle.clone();
        state.workspaces[0].downloads.push(download.clone());
        let retiring = run(
            &state,
            Command::Remove {
                downloads: vec![download.id],
                move_files_to_trash: true,
            },
        );
        let leftovers = vec![partial("/tmp/Downloads/.film.part")];
        let (stopped, effects) = apply_facts(
            &retiring,
            &[
                Fact::LeftoversFound {
                    download: download.id,
                    attempt: 0,
                    leftovers: leftovers.clone(),
                },
                Fact::WriterStopped {
                    download: download.id,
                    attempt: 0,
                    control_generation: 1,
                },
                Fact::AdapterRecordAbsent {
                    download: download.id,
                    attempt: 0,
                },
            ],
        );
        assert_eq!(
            effects,
            vec![EffectKind::MoveOutputToTrash {
                download: download.id,
                artifact: None,
                leftovers,
            }],
            "{lifecycle:?} should trash its partial files"
        );
        let (resolved, effects) = apply_facts(
            &stopped,
            &[Fact::FilesResolved {
                download: download.id,
                control_generation: 1,
            }],
        );
        assert_eq!(
            effects,
            vec![EffectKind::FinalizeRetirement {
                download: download.id
            }]
        );
        let (retired, _) = apply_facts(
            &resolved,
            &[Fact::RetirementFinalized {
                download: download.id,
                control_generation: 1,
            }],
        );
        assert!(retired.locate_download(download.id).is_none());
    }
}

#[test]
fn unverified_output_is_trashed_only_on_the_adapters_evidence() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Finished;
    download.output = Some(OutputArtifact {
        path: "/tmp/Downloads/film.mp4".to_string(),
        content_folder: None,
        identity: None,
    });
    state.workspaces[0].downloads.push(download.clone());
    let remove = Command::Remove {
        downloads: vec![download.id],
        move_files_to_trash: true,
    };
    assert!(decide(&state, &command(remove.clone())).is_ok());
    let retiring = run(&state, remove);

    // The adapter still owned the file, so it is trashed with the identity it has now.
    let leftovers = vec![partial("/tmp/Downloads/film.mp4")];
    let (_, effects) = apply_facts(
        &retiring,
        &[
            Fact::LeftoversFound {
                download: download.id,
                attempt: 0,
                leftovers: leftovers.clone(),
            },
            Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
            Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        ],
    );
    assert_eq!(
        effects,
        vec![EffectKind::MoveOutputToTrash {
            download: download.id,
            artifact: None,
            leftovers,
        }]
    );

    // Without that evidence the file stays, and the removal still finishes.
    let (_, effects) = apply_facts(
        &retiring,
        &[
            Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
            Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        ],
    );
    assert_eq!(
        effects,
        vec![EffectKind::FinalizeRetirement {
            download: download.id
        }]
    );
}

#[test]
fn leftovers_inside_the_verified_output_are_not_trashed_twice() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Finished;
    let output = OutputArtifact {
        path: "/tmp/Downloads/Film/film.mp4".to_string(),
        content_folder: Some("/tmp/Downloads/Film".to_string()),
        identity: Some(OutputIdentity {
            byte_length: 42,
            modified_at_millis: 7,
            file_identifier: Some("inode-9".to_string()),
        }),
    };
    download.output = Some(output.clone());
    state.workspaces[0].downloads.push(download.clone());
    let retiring = run(
        &state,
        Command::Remove {
            downloads: vec![download.id],
            move_files_to_trash: true,
        },
    );
    let (_, effects) = apply_facts(
        &retiring,
        &[
            Fact::LeftoversFound {
                download: download.id,
                attempt: 0,
                leftovers: vec![
                    partial("/tmp/Downloads/Film"),
                    partial("/tmp/Downloads/.film.f137.mp4.part"),
                ],
            },
            Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
            Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        ],
    );
    assert_eq!(
        effects,
        vec![EffectKind::MoveOutputToTrash {
            download: download.id,
            artifact: Some(output),
            leftovers: vec![partial("/tmp/Downloads/.film.f137.mp4.part")],
        }]
    );
}

#[test]
fn removing_from_the_list_ignores_leftovers() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Paused {
        resume: ActivePhase::Fetching,
        cause: HoldCause::Person,
    };
    state.workspaces[0].downloads.push(download.clone());
    let retiring = run(
        &state,
        Command::Remove {
            downloads: vec![download.id],
            move_files_to_trash: false,
        },
    );
    let (_, effects) = apply_facts(
        &retiring,
        &[
            Fact::LeftoversFound {
                download: download.id,
                attempt: 0,
                leftovers: vec![partial("/tmp/Downloads/.film.part")],
            },
            Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
            Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        ],
    );
    assert_eq!(
        effects,
        vec![EffectKind::FinalizeRetirement {
            download: download.id
        }]
    );
}

#[test]
fn deleting_a_workspace_can_trash_its_downloads_files() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Paused {
        resume: ActivePhase::Fetching,
        cause: HoldCause::Person,
    };
    state.workspaces[0].downloads.push(download.clone());
    let mut remaining = state.workspaces[0].clone();
    remaining.id = WorkspaceId::new();
    remaining.default_folder = FolderId::new();
    remaining.folders[0].id = remaining.default_folder;
    remaining.downloads.clear();
    state.workspaces.push(remaining.clone());
    let deleting = run(
        &state,
        Command::DeleteWorkspace {
            workspace: state.workspaces[0].id,
            replacement: None,
            move_files_to_trash: true,
        },
    );
    let leftovers = vec![partial("/tmp/Downloads/.film.part")];
    let (stopped, effects) = apply_facts(
        &deleting,
        &[
            Fact::LeftoversFound {
                download: download.id,
                attempt: 0,
                leftovers: leftovers.clone(),
            },
            Fact::WriterStopped {
                download: download.id,
                attempt: 0,
                control_generation: 1,
            },
            Fact::AdapterRecordAbsent {
                download: download.id,
                attempt: 0,
            },
        ],
    );
    assert_eq!(
        effects,
        vec![EffectKind::MoveOutputToTrash {
            download: download.id,
            artifact: None,
            leftovers,
        }]
    );
    let (finished, _) = apply_facts(
        &stopped,
        &[
            Fact::FilesResolved {
                download: download.id,
                control_generation: 1,
            },
            Fact::RetirementFinalized {
                download: download.id,
                control_generation: 1,
            },
        ],
    );
    assert_eq!(finished.workspaces, vec![remaining]);
    assert!(finished.structural_changes.workspace_deletions.is_empty());
}

#[test]
fn workspace_deletion_intent_survives_until_retirement_finishes() {
    let mut state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Active(ActivePhase::Fetching);
    state.workspaces[0].downloads.push(download.clone());
    let replacement_folder = FolderId::new();
    let mut replacement = Workspace {
        id: WorkspaceId::new(),
        name: "Downloads".to_string(),
        icon: WorkspaceIcon::Symbol("arrow.down.circle".to_string()),
        appearance: crate::application::WorkspaceAppearance::default(),
        folders: vec![SavedFolder {
            id: replacement_folder,
            name: "Downloads".to_string(),
            path: "/tmp/NewDownloads".to_string(),
        }],
        default_folder: replacement_folder,
        default_format: MediaFormat::Best,
        file_organization: FileOrganization::Automatic,
        view: WorkspaceView::default(),
        downloads: Vec::new(),
    };
    state.assign_folder_appearance(&mut replacement);
    let deleting = run(
        &state,
        Command::DeleteWorkspace {
            workspace: state.workspaces[0].id,
            replacement: Some(replacement.clone()),
            move_files_to_trash: false,
        },
    );
    assert_eq!(deleting.structural_changes.workspace_deletions.len(), 1);
    assert!(deleting.locate_download(download.id).is_some());

    let writer = decide_fact(
        &deleting,
        &Fact::WriterStopped {
            download: download.id,
            attempt: 0,
            control_generation: 1,
        },
    )
    .unwrap();
    let after_writer = evolve(&deleting, &writer).unwrap();
    let absent = decide_fact(
        &after_writer,
        &Fact::AdapterRecordAbsent {
            download: download.id,
            attempt: 0,
        },
    )
    .unwrap();
    let ready = evolve(&after_writer, &absent).unwrap();
    let retired = decide_fact(
        &ready,
        &Fact::RetirementFinalized {
            download: download.id,
            control_generation: 1,
        },
    )
    .unwrap();
    let finished = evolve(&ready, &retired).unwrap();
    assert_eq!(finished.workspaces, vec![replacement]);
    assert!(finished.structural_changes.workspace_deletions.is_empty());
}

#[test]
fn state_rejects_a_folder_from_another_workspace() {
    let mut state = running_state();
    let foreign = FolderId::new();
    state.workspaces.push(Workspace {
        id: WorkspaceId::new(),
        name: "Other".to_string(),
        icon: WorkspaceIcon::Emoji("🎬".to_string()),
        appearance: crate::application::WorkspaceAppearance::default(),
        folders: vec![SavedFolder {
            id: foreign,
            name: "Other".to_string(),
            path: "/tmp/Other".to_string(),
        }],
        default_folder: foreign,
        default_format: MediaFormat::Best,
        file_organization: FileOrganization::Automatic,
        view: WorkspaceView::default(),
        downloads: Vec::new(),
    });
    let mut download = media_download(&state);
    download.folder = foreign;
    state.workspaces[0].downloads.push(download);
    assert!(matches!(
        state.validate(),
        Err(InvariantViolation::FolderOutsideWorkspace { .. })
    ));
}
