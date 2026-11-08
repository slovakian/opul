use super::*;

pub(super) fn offer(
    app: &tauri::AppHandle,
    machine: &OpulStateMachine,
    tools: &Arc<VideoTools>,
    config: &Config,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    if !matches!(tools.snapshot().state, VideoToolsState::NotSetUp) || tools.invocation().is_some()
    {
        return Err("Offer must start without video tools or consent".into());
    }
    ui::click(app, "Not now", evidence)?;
    ui::dismissed(app, "Set Up Video Tools", evidence)?;
    if !matches!(tools.snapshot().state, VideoToolsState::NotSetUp) {
        return Err("Not now started an installation".into());
    }
    transfers::independent(machine, config, "not-set-up", evidence)?;
    let video = add_video(machine, format!("{}/single.mp4", config.base_url), true)?;
    let playlist = add_video(machine, format!("{}/playlist.html", config.base_url), true)?;
    if [video.as_str(), playlist.as_str()].iter().any(|id| {
        !item(machine, id).is_ok_and(|d| d.state == MachineDownloadState::WaitingForVideoTools)
    }) {
        return Err("Unconsented video/playlist did not remain two waiting items".into());
    }
    ui::open_setup(app)?;
    ui::click(app, "Don't show this again", evidence)?;
    ui::dismissed(app, "Set Up Video Tools", evidence)?;
    until(
        || Ok(matches!(tools.snapshot().state, VideoToolsState::TurnedOff)),
        30,
    )?;
    transfers::independent(machine, config, "turned-off", evidence)?;
    evidence
        .push(json!({"waiting_video": video, "waiting_playlist": playlist, "turned_off": true}));
    Ok(())
}

pub(super) fn setup(
    app: &tauri::AppHandle,
    machine: &OpulStateMachine,
    tools: &Arc<VideoTools>,
    config: &Config,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    if !matches!(tools.snapshot().state, VideoToolsState::TurnedOff) {
        return Err("Turned off did not survive ordinary restart".into());
    }
    let snapshot = machine.snapshot();
    let waiting: Vec<_> = snapshot.workspaces[0]
        .downloads
        .iter()
        .filter(|d| d.state == MachineDownloadState::WaitingForVideoTools)
        .map(|d| d.id.clone())
        .collect();
    if waiting.len() != 2 {
        return Err("Waiting video/playlist did not survive restart".into());
    }
    transfers::independent(machine, config, "off-after-restart", evidence)?;
    ui::open_setup(app)?;
    ui::download(app, evidence)?;
    until(
        || match tools.snapshot().state {
            VideoToolsState::Ready { .. } => {
                Err("Setup finished before concurrent transfers began".into())
            }
            VideoToolsState::SettingUp { .. } => Ok(true),
            _ => Ok(false),
        },
        30,
    )?;
    let during_setup = transfers::independent(machine, config, "setting-up", evidence)?;
    if !matches!(during_setup, VideoToolsState::SettingUp { .. }) {
        return Err("Independent transfers did not start during Setup".into());
    }
    ui::click(app, "Continue to Opul", evidence)?;
    ui::dismissed(app, "Set Up Video Tools", evidence)?;
    wait_runtime(tools, config.expected_runtime)?;
    for id in &waiting {
        wait_finished(machine, id)?;
    }
    until(
        || {
            Ok(machine.snapshot().workspaces[0]
                .downloads
                .iter()
                .filter(|d| d.name.starts_with("CC0 fixture playlist"))
                .count()
                == 2)
        },
        120,
    )?;
    let snapshot = machine.snapshot();
    let videos: Vec<_> = snapshot.workspaces[0]
        .downloads
        .iter()
        .filter(|d| d.name.starts_with("CC0 fixture playlist") || waiting.contains(&d.id))
        .collect();
    if videos.len() != 3
        || videos.iter().any(|d| {
            d.folder_id != snapshot.workspaces[0].folders[0].id
                || d.format != Some(MachineFormat::Original)
        })
    {
        return Err("Playlist expansion lost destination, format or identities".into());
    }
    for download in &videos {
        wait_finished(machine, &download.id)?;
    }
    evidence.push(
        json!({"waiting_video_and_playlist_expanded": true, "video_count": videos.len(),
        "original_identities": waiting, "destination_and_format_preserved": true}),
    );
    transfers::independent(machine, config, "ready", evidence)?;
    let slow = add_video(machine, format!("{}/slow.mp4", config.base_url), false)?;
    let folder = PathBuf::from(&snapshot.workspaces[0].folders[0].path);
    let mut partials = Vec::new();
    until(
        || {
            partials = std::fs::read_dir(&folder)
                .map_err(|e| e.to_string())?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension().is_some_and(|e| e == "part")
                        && path.metadata().is_ok_and(|m| m.len() > 0)
                })
                .collect();
            Ok(!partials.is_empty())
        },
        90,
    )?;
    let retained: Vec<_> = partials
        .iter()
        .map(std::fs::read)
        .collect::<Result<_, _>>()
        .map_err(|e| e.to_string())?;
    transfers::independent(machine, config, "video-running", evidence)?;
    ui::open_tools(app, evidence)?;
    ui::click(app, "Remove…", evidence)?;
    ui::text(app, "1 video download is in progress.", evidence)?;
    ui::click(app, "Cancel", evidence)?;
    ui::dismissed(app, "Remove video tools?", evidence)?;
    if runtime(tools) != Some(config.expected_runtime) {
        return Err("Cancel removed tools".into());
    }
    ui::click(app, "Remove…", evidence)?;
    ui::click(app, "Remove", evidence)?;
    until(
        || Ok(matches!(tools.snapshot().state, VideoToolsState::NotSetUp)),
        90,
    )?;
    until(
        || Ok(item(machine, &slow)?.state == MachineDownloadState::WaitingForVideoTools),
        90,
    )?;
    if partials.iter().any(|p| !p.exists()) {
        return Err("Removal deleted partial output".into());
    }
    transfers::independent(machine, config, "removed", evidence)?;
    for (path, bytes) in partials.iter().zip(&retained) {
        if !std::fs::read(path)
            .map_err(|e| e.to_string())?
            .starts_with(bytes)
        {
            return Err("Removal lost previously written partial bytes".into());
        }
    }
    std::fs::write(&config.gate, b"resume").map_err(|e| e.to_string())?;
    ui::download(app, evidence)?;
    wait_runtime(tools, config.expected_runtime)?;
    let resumed = wait_finished(machine, &slow)?;
    transfers::independent(machine, config, "re-setup", evidence)?;
    evidence.push(
        json!({"removed_download": slow, "retained_partials": partials,
        "resumed_output": resumed.output_path}),
    );
    Ok(())
}
