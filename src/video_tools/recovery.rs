use super::manager::StoredState;
use std::fs;
use std::path::Path;

pub(super) fn clean_temporary(root: &Path) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(".install-") || (name.starts_with(".state-") && name.ends_with(".json"))
        {
            remove(&entry.path())?;
        }
    }
    Ok(())
}

pub(super) fn prune(root: &Path, stored: &StoredState) -> Result<(), String> {
    let runtime = stored
        .installed
        .as_ref()
        .map(|pair| pair.runtime.to_string());
    let previous_runtime = stored.previous_runtime.map(|number| number.to_string());
    let script = stored.installed.as_ref().map(|pair| pair.yt_dlp.as_str());
    for (directory, current, previous) in [
        (
            "python-runtime",
            runtime.as_deref(),
            previous_runtime.as_deref(),
        ),
        ("yt-dlp", script, stored.previous_yt_dlp.as_deref()),
    ] {
        let directory = root.join(directory);
        // A failed placement can leave a file here. It is not a version directory.
        if !directory.is_dir() {
            continue;
        }
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if Some(name.as_ref()) != current && Some(name.as_ref()) != previous {
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
