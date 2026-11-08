use super::layout;
use super::manager::StoredState;
use std::fs;
use std::path::Path;

pub(super) fn clean_temporary(root: &Path) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if layout::is_temporary(&entry.file_name().to_string_lossy()) {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

/// Delete every version directory that state.json neither selects nor retains.
pub(super) fn prune(root: &Path, stored: &StoredState) -> Result<(), String> {
    let pairs = || stored.installed.iter().chain(stored.rollback_pair.iter());
    let runtimes: Vec<String> = pairs()
        .map(|pair| layout::runtime_name(pair.runtime))
        .collect();
    let scripts: Vec<String> = pairs()
        .map(|pair| layout::script_name(pair.yt_dlp_channel, &pair.yt_dlp))
        .chain(stored.previous_yt_dlp.clone())
        .collect();
    for (directory, kept) in [(layout::RUNTIMES, runtimes), (layout::SCRIPTS, scripts)] {
        let directory = root.join(directory);
        // A failed placement can leave a file here. It is not a version directory.
        if !directory.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if !kept.contains(&entry.file_name().to_string_lossy().into_owned()) {
                remove(&entry.path())?;
            }
        }
    }
    Ok(())
}

fn remove(path: &Path) -> Result<(), String> {
    if fs::symlink_metadata(path)
        .map_err(|e| e.to_string())?
        .is_dir()
    {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|e| format!("{}: {e}", path.display()))
}
