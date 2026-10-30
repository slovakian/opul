use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DownloadContentLayout {
    #[default]
    Original,
    Subfolder,
    NoSubfolder,
}

pub(crate) struct DownloadDestination {
    pub output_folder: PathBuf,
    pub content_folder: Option<PathBuf>,
}

pub(crate) fn resolve_download_destination(
    save_folder: &Path,
    title: &str,
    layout: DownloadContentLayout,
    naturally_grouped: bool,
) -> DownloadDestination {
    let use_subfolder = match layout {
        DownloadContentLayout::Original => naturally_grouped,
        DownloadContentLayout::Subfolder => true,
        DownloadContentLayout::NoSubfolder => false,
    };
    if use_subfolder {
        let folder = save_folder.join(safe_folder_name(title));
        DownloadDestination {
            output_folder: folder.clone(),
            content_folder: Some(folder),
        }
    } else {
        DownloadDestination {
            output_folder: save_folder.to_path_buf(),
            content_folder: None,
        }
    }
}

fn safe_folder_name(title: &str) -> String {
    let mut value = String::new();
    for character in title.trim().chars() {
        let character = if character == '/' || character == ':' || character.is_control() {
            '_'
        } else {
            character
        };
        if value.len() + character.len_utf8() > 180 {
            break;
        }
        value.push(character);
    }
    let value = value.trim_matches([' ', '.']);
    if value.is_empty() || matches!(value, "." | "..") {
        "Download".to_string()
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{DownloadContentLayout, resolve_download_destination, safe_folder_name};
    use std::path::Path;

    #[test]
    fn original_only_groups_natural_bundles() {
        let single = resolve_download_destination(
            Path::new("/Downloads"),
            "Movie",
            DownloadContentLayout::Original,
            false,
        );
        assert_eq!(single.output_folder, Path::new("/Downloads"));
        assert!(single.content_folder.is_none());

        let bundle = resolve_download_destination(
            Path::new("/Downloads"),
            "Movie",
            DownloadContentLayout::Original,
            true,
        );
        assert_eq!(bundle.output_folder, Path::new("/Downloads/Movie"));
        assert_eq!(
            bundle.content_folder.as_deref(),
            Some(Path::new("/Downloads/Movie"))
        );
    }

    #[test]
    fn explicit_layout_overrides_natural_shape() {
        let grouped = resolve_download_destination(
            Path::new("/Downloads"),
            "One file",
            DownloadContentLayout::Subfolder,
            false,
        );
        assert_eq!(grouped.output_folder, Path::new("/Downloads/One file"));

        let flat = resolve_download_destination(
            Path::new("/Downloads"),
            "Many files",
            DownloadContentLayout::NoSubfolder,
            true,
        );
        assert_eq!(flat.output_folder, Path::new("/Downloads"));
        assert!(flat.content_folder.is_none());
    }

    #[test]
    fn folder_names_cannot_escape_the_save_folder() {
        let destination = resolve_download_destination(
            Path::new("/Downloads"),
            " ../A/B: C ",
            DownloadContentLayout::Subfolder,
            false,
        );
        assert_eq!(destination.output_folder, Path::new("/Downloads/_A_B_ C"));
        assert!(safe_folder_name(&"🎬".repeat(100)).len() <= 180);
    }
}
