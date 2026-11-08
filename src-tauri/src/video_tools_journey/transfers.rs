use super::*;
use opul_core::MachineTorrentRequest;

pub(super) fn independent(
    machine: &OpulStateMachine,
    config: &Config,
    phase: &str,
    evidence: &mut Vec<Value>,
) -> Result<VideoToolsState, String> {
    let tools = machine.video_tools().ok_or("No managed tools")?;
    let tools_at_start = tools.snapshot();
    let initial_state = tools_at_start.state.clone();
    let snapshot = machine.snapshot();
    let workspace = &snapshot.workspaces[0];
    let folder = &workspace.folders[0];
    let url = format!("{}/direct/{phase}.txt", config.base_url);
    let request = crate::commands::media_request(
        workspace.id.clone(),
        folder.id.clone(),
        MachineFormat::Original,
        vec![MachineMediaItem {
            url: url.clone(),
            page_url: url,
            title: format!("Independent {phase}"),
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
        }],
    );
    let direct = machine
        .add_direct_media(request)
        .map_err(|e| e.to_string())?
        .created_download_ids[0]
        .clone();
    let torrent = machine
        .add_torrent(MachineTorrentRequest {
            request_id: uuid::Uuid::new_v4().to_string(),
            expected_revision: None,
            workspace_id: workspace.id.clone(),
            folder_id: folder.id.clone(),
            source: format!("{}/torrent/{phase}.torrent", config.base_url),
            metainfo: None,
            seed_ratio_limit: Some(0.0),
        })
        .map_err(|e| e.to_string())?
        .created_download_ids[0]
        .clone();
    let direct = wait_finished(machine, &direct)?;
    let torrent_item = wait_finished(machine, &torrent)?;
    let direct_path = direct.output_path.ok_or("Direct output missing")?;
    let torrent_path = PathBuf::from(&folder.path).join(format!("{phase}.bin"));
    if torrent_item.output_path.as_deref() != torrent_path.to_str() {
        return Err(format!(
            "Completed torrent output identity missing in {phase}"
        ));
    }
    if std::fs::read(&direct_path).map_err(|e| e.to_string())?
        != format!("Opul CC0 direct {phase}\n").as_bytes()
        || std::fs::read(&torrent_path).map_err(|e| e.to_string())?
            != format!("Opul CC0 torrent {phase}\n").repeat(32).as_bytes()
    {
        return Err(format!("Independent transfer bytes differ in {phase}"));
    }
    evidence.push(
        json!({"phase": phase, "direct": direct.id, "torrent": torrent,
        "direct_bytes_verified": true, "torrent_bytes_verified": true,
        "tools_at_start": tools_at_start, "tools_at_completion": tools.snapshot()}),
    );
    Ok(initial_state)
}
