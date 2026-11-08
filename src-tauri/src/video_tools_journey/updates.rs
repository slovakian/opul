use super::*;
use opul_core::video_tools::UpdateFlag;

fn wait_updating(tools: &VideoTools, target: u64) -> Result<(), String> {
    until(
        || {
            if runtime(tools) == Some(target) {
                return Err("Update completed before concurrent transfers began".into());
            }
            Ok(matches!(
                tools.snapshot().state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Updating { .. },
                    ..
                }
            ))
        },
        180,
    )
}

fn during_update(
    machine: &OpulStateMachine,
    tools: &VideoTools,
    config: &Config,
    phase: &str,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    wait_updating(tools, config.expected_runtime)?;
    let state = transfers::independent(machine, config, phase, evidence)?;
    if !matches!(
        state,
        VideoToolsState::Ready {
            update_flag: UpdateFlag::Updating { .. },
            ..
        }
    ) {
        return Err("Independent transfers did not begin during an update".into());
    }
    Ok(())
}

pub(super) fn manual(
    app: &tauri::AppHandle,
    machine: &OpulStateMachine,
    tools: &Arc<VideoTools>,
    config: &Config,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    if runtime(tools) != Some(1) {
        return Err("Manual update needs the verified runtime 1 baseline".into());
    }
    ui::open_tools(app, evidence)?;
    ui::keep_updated(app, false, evidence)?;
    transfers::independent(machine, config, "manual-before", evidence)?;
    ui::click(app, "Check for updates", evidence)?;
    until(
        || {
            Ok(matches!(
                tools.snapshot().state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Available { .. },
                    ..
                }
            ))
        },
        180,
    )?;
    if runtime(tools) != Some(1) {
        return Err("Check installed without consent".into());
    }
    transfers::independent(machine, config, "update-available", evidence)?;
    ui::click(app, "Update", evidence)?;
    during_update(machine, tools, config, "manual-updating", evidence)?;
    wait_runtime(tools, config.expected_runtime)?;
    let failed = add_video(
        machine,
        format!("{}/missing-video.mp4", config.base_url),
        false,
    )?;
    until(
        || Ok(item(machine, &failed)?.state == MachineDownloadState::Failed),
        180,
    )?;
    wait_runtime(tools, 1)?;
    transfers::independent(machine, config, "rolled-back", evidence)?;
    let original_error = item(machine, &failed)?.error_message;
    ui::click(app, "Check for updates", evidence)?;
    until(
        || {
            Ok(matches!(
                tools.snapshot().state,
                VideoToolsState::Ready {
                    update_flag: UpdateFlag::Available { .. },
                    ..
                }
            ))
        },
        180,
    )?;
    ui::click(app, "Update", evidence)?;
    wait_runtime(tools, config.expected_runtime)?;
    let success = add_video(machine, format!("{}/success.mp4", config.base_url), false)?;
    wait_finished(machine, &success)?;
    let previous = tools
        .invocation()
        .ok_or("No selected invocation")?
        .interpreter
        .parent()
        .and_then(|p| p.parent())
        .and_then(|p| p.parent())
        .ok_or("Invalid runtime path")?
        .join("1");
    until(|| Ok(!previous.exists()), 30)?;
    transfers::independent(machine, config, "manual-finished", evidence)?;
    evidence.push(json!({"manual_update": [1, config.expected_runtime], "automatic_rollback": [config.expected_runtime, 1],
        "upstream_error_preserved": original_error, "retry_succeeded": true, "previous_runtime_removed_after_success": true}));
    Ok(())
}

pub(super) fn automatic(
    app: &tauri::AppHandle,
    machine: &OpulStateMachine,
    tools: &Arc<VideoTools>,
    config: &Config,
    evidence: &mut Vec<Value>,
) -> Result<(), String> {
    if runtime(tools) != Some(1) {
        return Err("Automatic update needs the verified runtime 1 baseline".into());
    }
    let started = Instant::now();
    ui::open_tools(app, evidence)?;
    ui::keep_updated(app, true, evidence)?;
    transfers::independent(machine, config, "automatic-waiting", evidence)?;
    during_update(machine, tools, config, "automatic-updating", evidence)?;
    wait_runtime(tools, config.expected_runtime)?;
    let elapsed = started.elapsed().as_secs();
    let success = add_video(machine, format!("{}/success.mp4", config.base_url), false)?;
    wait_finished(machine, &success)?;
    transfers::independent(machine, config, "automatic-ready", evidence)?;
    evidence.push(
        json!({"automatic_update": [1, config.expected_runtime], "elapsed_seconds": elapsed,
        "manual_check_or_update_commands": 0, "last_checked": tools.snapshot()}),
    );
    Ok(())
}
