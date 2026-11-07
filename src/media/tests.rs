use super::{
    GuidedDownloadSelection, GuidedExtractionSession, GuidedHttpHeader, GuidedMediaKind,
    GuidedObservation, GuidedObservationSource, MediaDownloadOptions, MediaEngine,
    MediaEngineConfig, MediaFormat, MediaProbeItem, MediaTransferState, StoredMediaDownload,
    apply_format_args, apply_progress_line, classify_guided_url, direct_cookie_header,
    direct_download_compatible, direct_filename, direct_referer_for_target, expiry_from_url,
    load_downloads, numeric_field, owned_cookie_path, parse_probe, parse_utc_seconds,
    save_downloads, validate_filename_template, validate_output_folder, yt_dlp_partial_paths,
};
use crate::{CoreError, DownloadContentLayout};
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write as _};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use url::Url;
use uuid::Uuid;

pub(super) fn built_in_resources(root: &Path) -> PathBuf {
    let resources = root.join("resources");
    let tools = resources.join("media-tools");
    fs::create_dir_all(&tools).unwrap();
    for name in ["ffmpeg", "ffprobe"] {
        let path = tools.join(name);
        fs::write(
            &path,
            format!("#!/bin/sh\nprintf '%s\\n' '{name} version test'\n"),
        )
        .unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }
    resources
}

mod guided;
mod persistence;
mod probe_and_progress;
mod tools;
mod transfers;

fn guided_observation(
    url: &str,
    initiator: &str,
    source: GuidedObservationSource,
) -> GuidedObservation {
    GuidedObservation {
        url: url.to_string(),
        page_url: "https://app.example/watch".to_string(),
        frame_url: None,
        title: "A film".to_string(),
        initiator: initiator.to_string(),
        mime_type: None,
        status_code: None,
        content_length: None,
        observed_at_ms: 1_700_000_000_000,
        source,
        is_main_frame: true,
        element_key: None,
        width: None,
        height: None,
        headers: Vec::new(),
    }
}
