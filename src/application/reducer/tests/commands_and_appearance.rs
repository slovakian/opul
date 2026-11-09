use super::*;

#[test]
fn appearance_preferences_preserve_automatic_assignment_and_full_unicode() {
    use crate::AppearancePreference;
    let mut state = running_state();
    let id = state.workspaces[0].id;
    let original = state.workspaces[0].appearance.automatic_folder.clone();
    let manual = AppearancePreference::Custom {
        hue: 180,
        saturation: 150,
        brightness: 110,
    };
    for preference in [
        AppearancePreference::Original,
        AppearancePreference::Automatic,
        manual,
    ] {
        for emoji in ["👩🏽‍💻", "⚪", "⚪\u{fe0f}", "👨‍👩‍👧‍👦", "📁\u{fe0e}", "📁"]
        {
            state = run(
                &state,
                Command::SetWorkspaceAppearance {
                    workspace: id,
                    emoji: emoji.into(),
                    appearance: crate::WorkspaceAppearance {
                        automatic_folder: None,
                        preference: preference.clone(),
                    },
                },
            );
            assert_eq!(state.workspaces[0].icon, WorkspaceIcon::Emoji(emoji.into()));
            assert_eq!(state.workspaces[0].appearance.preference, preference);
            assert_eq!(state.workspaces[0].appearance.automatic_folder, original);
        }
    }
}

#[test]
fn first_folder_change_uses_the_frozen_proposal_and_never_replaces_it() {
    let state = running_state();
    let native = new_workspace(&state, "🐼");
    let id = native.id;
    let proposal = state.propose_folder_appearance(id);
    let mut state = run(&state, Command::CreateWorkspace { workspace: native });
    let mut other = new_workspace(&state, "📁");
    other.appearance = proposal.clone();
    state = run(&state, Command::CreateWorkspace { workspace: other });
    assert_ne!(state.propose_folder_appearance(id), proposal);
    state = run(
        &state,
        Command::SetWorkspaceAppearance {
            workspace: id,
            emoji: "📁".into(),
            appearance: proposal.clone(),
        },
    );
    assert_eq!(state.workspace(id).unwrap().appearance, proposal);
    let other_proposal = state.propose_folder_appearance(id);
    state = run(
        &state,
        Command::SetWorkspaceAppearance {
            workspace: id,
            emoji: "📁".into(),
            appearance: other_proposal,
        },
    );
    assert_eq!(state.workspace(id).unwrap().appearance, proposal);
}

#[test]
fn invalid_appearance_save_rejects_both_fields_without_changing_state() {
    let state = running_state();
    let id = state.workspaces[0].id;
    for (emoji, preference) in [
        (
            "🐼",
            crate::AppearancePreference::Custom {
                hue: 360,
                saturation: 100,
                brightness: 100,
            },
        ),
        (
            "🐼",
            crate::AppearancePreference::Custom {
                hue: 0,
                saturation: 1001,
                brightness: 100,
            },
        ),
        (
            "🐼",
            crate::AppearancePreference::Custom {
                hue: 0,
                saturation: 100,
                brightness: 49,
            },
        ),
        (" ", crate::AppearancePreference::Original),
        ("🐼\n", crate::AppearancePreference::Original),
    ] {
        let before = state.clone();
        assert!(
            decide(
                &state,
                &command(Command::SetWorkspaceAppearance {
                    workspace: id,
                    emoji: emoji.into(),
                    appearance: crate::WorkspaceAppearance {
                        automatic_folder: None,
                        preference
                    },
                })
            )
            .is_err()
        );
        assert_eq!(state, before);
    }
    assert!(
        decide(
            &state,
            &command(Command::SetWorkspaceAppearance {
                workspace: WorkspaceId::new(),
                emoji: "🐼".into(),
                appearance: crate::WorkspaceAppearance::default(),
            })
        )
        .is_err()
    );
}

#[test]
fn appearance_save_is_atomic_and_preserves_unrelated_workspace_settings() {
    let mut state = running_state();
    state.workspaces[0].name = "Renamed while choosing an icon".into();
    state.workspaces[0].default_format = MediaFormat::AudioMp3;
    state.workspaces[0].file_organization = FileOrganization::AlwaysCreateFolder;
    let previous = state.workspaces[0].clone();
    let command: Command = serde_json::from_value(serde_json::json!({
        "SetWorkspaceAppearance": {
            "workspace": previous.id,
            "emoji": "👩🏽‍💻",
            "appearance": {
                "automatic_folder": null,
                "preference": { "kind": "Original" }
            }
        }
    }))
    .expect("appearance-only command must be supported");
    let next = run(&state, command);
    let mut expected = previous.clone();
    expected.icon = WorkspaceIcon::Emoji("👩🏽‍💻".into());
    let json = serde_json::to_value(&next.workspaces[0]).unwrap();
    assert_eq!(json["appearance"]["preference"]["kind"], "Original");
    expected.appearance = next.workspaces[0].appearance.clone();
    assert_eq!(next.workspaces[0], expected);
    assert_eq!(
        next.workspaces[0].appearance.automatic_folder,
        previous.appearance.automatic_folder
    );
}

#[test]
fn bootstrap_folder_has_a_persisted_resolved_treatment() {
    let state = running_state();
    let value = serde_json::to_value(&state.workspaces[0]).unwrap();
    let treatment = &value["appearance"]["automatic_folder"];
    assert!(
        treatment.is_object(),
        "bootstrap folder needs a saved treatment"
    );
    assert!(treatment["preset_id"].is_string());
    assert!(treatment["hue"].is_number());
    assert!(treatment["saturation"].is_number());
    assert!(treatment["brightness"].is_number());
}

#[test]
fn create_freezes_the_draft_appearance_even_when_usage_changes() {
    let state = running_state();
    let mut draft = new_workspace(&state, "📁");
    draft.appearance = state.propose_folder_appearance(draft.id);
    let mut other = new_workspace(&state, "📁");
    other.appearance = draft.appearance.clone();
    let changed = run(&state, Command::CreateWorkspace { workspace: other });
    assert_ne!(
        changed.propose_folder_appearance(draft.id),
        draft.appearance
    );
    let created = run(
        &changed,
        Command::CreateWorkspace {
            workspace: draft.clone(),
        },
    );
    assert_eq!(created.workspace(draft.id), Some(&draft));
}

#[test]
fn automatic_assignment_is_only_for_folders_and_survives_rename_reorder_and_deletion() {
    let mut state = running_state();
    let folder = new_workspace(&state, "📁");
    let other = new_workspace(&state, "🎬");
    state = run(
        &state,
        Command::CreateWorkspace {
            workspace: folder.clone(),
        },
    );
    state = run(
        &state,
        Command::CreateWorkspace {
            workspace: other.clone(),
        },
    );
    assert!(
        state
            .workspace(other.id)
            .unwrap()
            .appearance
            .is_unassigned()
    );
    let appearance = state.workspace(folder.id).unwrap().appearance.clone();
    for icon in ["🐼", "📁"] {
        state = run(
            &state,
            Command::RenameWorkspace {
                workspace: folder.id,
                name: "Renamed".into(),
                icon: WorkspaceIcon::Emoji(icon.into()),
                organization: FileOrganization::AlwaysCreateFolder,
            },
        );
        assert_eq!(state.workspace(folder.id).unwrap().appearance, appearance);
    }
    state.workspaces.reverse();
    state = run(
        &state,
        Command::DeleteWorkspace {
            workspace: other.id,
            replacement: None,
            move_files_to_trash: false,
        },
    );
    assert_eq!(state.workspace(folder.id).unwrap().appearance, appearance);
    let replacement = new_workspace(&state, "📁");
    state = run(
        &state,
        Command::CreateWorkspace {
            workspace: replacement,
        },
    );
    assert_eq!(state.workspace(folder.id).unwrap().appearance, appearance);
}

#[test]
fn changing_a_native_emoji_to_a_folder_allocates_once() {
    let state = running_state();
    let other = new_workspace(&state, "📂");
    let state = run(
        &state,
        Command::CreateWorkspace {
            workspace: other.clone(),
        },
    );
    assert!(
        state
            .workspace(other.id)
            .unwrap()
            .appearance
            .is_unassigned()
    );
    let state = run(
        &state,
        Command::RenameWorkspace {
            workspace: other.id,
            name: other.name,
            icon: WorkspaceIcon::Emoji("📁".into()),
            organization: other.file_organization,
        },
    );
    assert!(
        !state
            .workspace(other.id)
            .unwrap()
            .appearance
            .is_unassigned()
    );
}

#[test]
fn replacement_downloads_receives_a_treatment_and_invalid_creation_is_rejected() {
    let state = running_state();
    let replacement = new_workspace(&state, "📁");
    let next = run(
        &state,
        Command::DeleteWorkspace {
            workspace: state.workspaces[0].id,
            replacement: Some(replacement.clone()),
            move_files_to_trash: false,
        },
    );
    assert_eq!(next.workspaces.len(), 1);
    let saved = &next.workspaces[0];
    assert_eq!(saved.id, replacement.id);
    assert_eq!(saved.folders, replacement.folders);
    assert!(!saved.appearance.is_unassigned());
    let mut invalid = new_workspace(&state, "📁");
    invalid.appearance = state.propose_folder_appearance(invalid.id);
    invalid.appearance.automatic_folder.as_mut().unwrap().hue = 360;
    assert!(matches!(
        decide(
            &state,
            &command(Command::CreateWorkspace { workspace: invalid })
        ),
        Err(Rejection::InvalidInput(_))
    ));
}

#[test]
fn new_download_is_committed_with_a_stable_effect_identity_target() {
    let state = running_state();
    let download = media_download(&state);
    let decision = decide(
        &state,
        &command(Command::AddDownload {
            workspace: state.workspaces[0].id,
            download: Box::new(download.clone()),
        }),
    )
    .unwrap();
    assert_eq!(decision.created_downloads, vec![download.id]);
    assert_eq!(
        decision.effects[0].kind,
        EffectKind::EnsureTransfer {
            download: download.id
        }
    );
    let next = evolve(&state, &decision).unwrap();
    assert_eq!(next.locate_download(download.id).unwrap().1.id, download.id);
}

#[test]
fn pause_is_pending_until_the_matching_generation_acknowledges() {
    let state = running_state();
    let download = media_download(&state);
    let state = run(
        &state,
        Command::AddDownload {
            workspace: state.workspaces[0].id,
            download: Box::new(download.clone()),
        },
    );
    let state = evolve(
        &state,
        &decide(
            &state,
            &command(Command::Pause {
                download: download.id,
            }),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(matches!(
        state.locate_download(download.id).unwrap().1.lifecycle,
        DownloadLifecycle::Pausing {
            cause: HoldCause::Person,
            ..
        }
    ));
    let stale = decide_fact(
        &state,
        &Fact::TransferPaused {
            download: download.id,
            attempt: 0,
            control_generation: 0,
        },
    )
    .unwrap();
    assert!(stale.events.is_empty());
    let matching = decide_fact(
        &state,
        &Fact::TransferPaused {
            download: download.id,
            attempt: 0,
            control_generation: 1,
        },
    )
    .unwrap();
    let paused = evolve(&state, &matching).unwrap();
    assert!(matches!(
        paused.locate_download(download.id).unwrap().1.lifecycle,
        DownloadLifecycle::Paused {
            cause: HoldCause::Person,
            ..
        }
    ));
}

#[test]
fn obsolete_pause_effect_is_ineligible_after_resume() {
    let state = running_state();
    let mut download = media_download(&state);
    download.lifecycle = DownloadLifecycle::Paused {
        resume: ActivePhase::Fetching,
        cause: HoldCause::Person,
    };
    download.control_generation = 4;
    let workspace = state.workspaces[0].id;
    let mut state = state;
    state.workspaces[0].downloads.push(download.clone());
    let decision = decide(
        &state,
        &command(Command::Resume {
            download: download.id,
        }),
    )
    .unwrap();
    let next = evolve(&state, &decision).unwrap();
    let stale = PendingEffect {
        id: EffectId::new(),
        created_revision: state.revision,
        attempt: Some(0),
        control_generation: Some(4),
        status: EffectStatus::Pending,
        kind: EffectKind::ConvergeControl {
            download: download.id,
        },
    };
    assert!(!effect_is_eligible(&next, &stale));
    assert_eq!(next.workspace(workspace).unwrap().downloads.len(), 1);
}

#[test]
fn backup_recovery_cannot_be_unfenced_by_mark_reconciled() {
    let mut state = running_state();
    state.session = SessionPhase::RecoveryRequired;
    let result = decide(&state, &command(Command::MarkReconciled));
    assert!(matches!(result, Err(Rejection::InvalidInput(_))));
    let effect = PendingEffect {
        id: EffectId::new(),
        created_revision: state.revision,
        attempt: None,
        control_generation: None,
        status: EffectStatus::Pending,
        kind: EffectKind::StopAllTransfers,
    };
    assert!(!effect_is_eligible(&state, &effect));
    let acknowledged = run(&state, Command::AcknowledgeRecovery);
    assert_eq!(acknowledged.session, SessionPhase::Reconciling);
}

#[test]
fn reconciliation_reissues_queued_and_shutdown_interrupted_intent() {
    let mut state = running_state();
    state.session = SessionPhase::Reconciling;
    let queued = media_download(&state);
    let mut interrupted = media_download(&state);
    interrupted.id = DownloadId::new();
    interrupted.control_generation = 3;
    interrupted.lifecycle = DownloadLifecycle::Paused {
        resume: ActivePhase::Fetching,
        cause: HoldCause::Shutdown,
    };
    state.workspaces[0].downloads = vec![queued.clone(), interrupted.clone()];

    let decision = decide(&state, &command(Command::MarkReconciled)).unwrap();
    assert!(
        decision
            .events
            .contains(&Event::SessionChanged(SessionPhase::Running))
    );
    assert!(decision.effects.iter().any(|effect| {
        effect.control_generation == Some(0)
            && effect.kind
                == EffectKind::EnsureTransfer {
                    download: queued.id,
                }
    }));
    assert!(decision.effects.iter().any(|effect| {
        effect.control_generation == Some(4)
            && effect.kind
                == EffectKind::EnsureTransfer {
                    download: interrupted.id,
                }
    }));
    let next = evolve(&state, &decision).unwrap();
    assert!(matches!(
        next.locate_download(interrupted.id).unwrap().1.lifecycle,
        DownloadLifecycle::Starting
    ));
}

#[test]
fn duplicate_torrent_hash_is_rejected_globally() {
    let mut state = running_state();
    let folder = state.workspaces[0].default_folder;
    let first = Download {
        id: DownloadId::new(),
        folder,
        name: "One".to_string(),
        attempt: 0,
        control_generation: 0,
        source: DownloadSource::Torrent(TorrentDownload {
            origin: TorrentOrigin::Magnet {
                uri: "magnet:?xt=urn:btih:abc".to_string(),
            },
            info_hash: Some("abc".to_string()),
            metadata: None,
            seed_ratio_limit: Some(1.0),
            files: Vec::new(),
            complete: false,
            reached_seed_ratio: false,
        }),
        lifecycle: DownloadLifecycle::Queued,
        output: None,
    };
    state.workspaces[0].downloads.push(first.clone());
    let mut second = first.clone();
    second.id = DownloadId::new();
    let result = decide(
        &state,
        &command(Command::AddDownload {
            workspace: state.workspaces[0].id,
            download: Box::new(second),
        }),
    );
    assert_eq!(result.unwrap_err(), Rejection::DuplicateTorrent(first.id));
}
