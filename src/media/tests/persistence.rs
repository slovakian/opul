use super::*;

#[test]
fn load_downloads_corrupt_file_preserves_bad_file_and_recovers_from_backup() {
    let dir = std::env::temp_dir().join(format!("opul-media-test-corrupt-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("media-downloads.json");
    let backup_path = dir.join("media-downloads.json.bak");

    let id = Uuid::new_v4().to_string();
    let mut good_downloads = HashMap::new();
    good_downloads.insert(
        id.clone(),
        StoredMediaDownload {
            id: id.clone(),
            download_id: None,
            attempt: 0,
            source_url: "https://example.com/video.mp4".to_string(),
            page_url: "https://example.com/watch".to_string(),
            title: "Test Video".to_string(),
            output_folder: "/tmp".to_string(),
            content_folder: None,
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
            options: MediaDownloadOptions::default(),
            direct_download: true,
            cookie_file: None,
            request_headers: Vec::new(),
            request_header_file: None,
            guided_kind: None,
            state: MediaTransferState::Finished,
            progress: 1.0,
            total_bytes: 1000,
            downloaded_bytes: 1000,
            download_rate: 0,
            eta_seconds: None,
            output_path: Some("/tmp/video.mp4".to_string()),
            partial_files: Vec::new(),
            error_message: None,
            created_at: 100000,
        },
    );
    fs::write(
        &backup_path,
        serde_json::to_vec_pretty(&good_downloads).unwrap(),
    )
    .unwrap();
    // Write corrupt truncated JSON
    fs::write(&path, b"{\"some-id\": {\"source_url\": ").unwrap();

    let loaded = load_downloads(&path).unwrap();
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items.get(&id).unwrap().title, "Test Video");
    assert!(loaded.recovery_notice.is_some());
    assert_eq!(fs::read(&path).unwrap(), fs::read(&backup_path).unwrap());

    let corrupt_files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
        .collect();
    assert_eq!(corrupt_files.len(), 1);
    let corrupt_content = fs::read(corrupt_files[0].path()).unwrap();
    assert_eq!(corrupt_content, b"{\"some-id\": {\"source_url\": ");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_downloads_incompatible_schema_preserves_bad_file() {
    let dir = std::env::temp_dir().join(format!(
        "opul-media-test-incompatible-{}",
        std::process::id()
    ));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("media-downloads.json");

    fs::write(&path, br#"{"schemaVersion":2,"items":{}}"#).unwrap();

    let loaded = load_downloads(&path).unwrap();
    assert!(loaded.items.is_empty());
    assert!(loaded.recovery_notice.is_some());

    let corrupt_files: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
        .collect();
    assert_eq!(corrupt_files.len(), 1);

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_downloads_preserves_whole_store_when_one_record_is_invalid() {
    let dir = std::env::temp_dir().join(format!(
        "opul-media-test-invalid-record-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("media-downloads.json");
    let id = Uuid::new_v4();
    let invalid = format!(
        r#"{{"{id}":{{"id":"{id}","source_url":"file:///private/video.mp4","output_folder":"/tmp"}}}}"#
    );
    fs::write(&path, invalid.as_bytes()).unwrap();

    let loaded = load_downloads(&path).unwrap();

    assert!(loaded.items.is_empty());
    assert!(loaded.recovery_notice.is_some());
    assert!(path.exists());
    let corrupt_files = fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains("corrupt"))
        .count();
    assert_eq!(corrupt_files, 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn save_downloads_atomic_and_keeps_backup() {
    let dir = std::env::temp_dir().join(format!("opul-media-test-atomic-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("media-downloads.json");
    let backup_path = dir.join("media-downloads.json.bak");

    let id1 = Uuid::new_v4().to_string();
    let mut map1 = HashMap::new();
    map1.insert(
        id1.clone(),
        StoredMediaDownload {
            id: id1.clone(),
            download_id: None,
            attempt: 0,
            source_url: "https://example.com/1.mp4".to_string(),
            page_url: "https://example.com/1".to_string(),
            title: "Video 1".to_string(),
            output_folder: "/tmp".to_string(),
            content_folder: None,
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
            options: MediaDownloadOptions::default(),
            direct_download: false,
            cookie_file: None,
            request_headers: Vec::new(),
            request_header_file: None,
            guided_kind: None,
            state: MediaTransferState::Queued,
            progress: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            download_rate: 0,
            eta_seconds: None,
            output_path: None,
            partial_files: Vec::new(),
            error_message: None,
            created_at: 1,
        },
    );
    save_downloads(&path, &map1);
    assert!(path.exists());
    assert!(!backup_path.exists());

    let id2 = Uuid::new_v4().to_string();
    let mut map2 = HashMap::new();
    map2.insert(
        id2.clone(),
        StoredMediaDownload {
            id: id2.clone(),
            download_id: None,
            attempt: 0,
            source_url: "https://example.com/2.mp4".to_string(),
            page_url: "https://example.com/2".to_string(),
            title: "Video 2".to_string(),
            output_folder: "/tmp".to_string(),
            content_folder: None,
            thumbnail_url: None,
            extractor: None,
            playlist_index: None,
            options: MediaDownloadOptions::default(),
            direct_download: false,
            cookie_file: None,
            request_headers: Vec::new(),
            request_header_file: None,
            guided_kind: None,
            state: MediaTransferState::Queued,
            progress: 0.0,
            total_bytes: 0,
            downloaded_bytes: 0,
            download_rate: 0,
            eta_seconds: None,
            output_path: None,
            partial_files: Vec::new(),
            error_message: None,
            created_at: 2,
        },
    );
    save_downloads(&path, &map2);
    assert!(path.exists());
    assert!(backup_path.exists());

    let loaded = load_downloads(&path).unwrap();
    assert_eq!(loaded.items.get(&id2).unwrap().title, "Video 2");

    let loaded_bak = load_downloads(&backup_path).unwrap();
    assert_eq!(loaded_bak.items.get(&id1).unwrap().title, "Video 1");

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn load_downloads_tolerates_010_and_unknown_fields() {
    let dir = std::env::temp_dir().join(format!("opul-media-test-010-{}", std::process::id()));
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("media-downloads.json");

    let id = Uuid::new_v4().to_string();
    // Early 0.1.0 format with missing optional fields and extra unknown fields.
    let json_010 = format!(
        r#"{{
            "{id}": {{
                "id": "{id}",
                "source_url": "https://example.com/media.mp4",
                "output_folder": "/tmp",
                "title": "Legacy 0.1.0 Media",
                "options": {{
                    "format": "Original",
                    "download_subtitles": false,
                    "subtitle_languages": [],
                    "embed_subtitles": false,
                    "write_thumbnail": false,
                    "write_metadata": false,
                    "filename_template": "%(title)s.%(ext)s"
                }},
                "state": "Finished",
                "progress": 1.0,
                "unknown_field_123": "ignore_me"
            }}
        }}"#
    );
    fs::write(&path, json_010.as_bytes()).unwrap();

    let loaded = load_downloads(&path).unwrap();
    assert_eq!(loaded.items.len(), 1);
    let item = loaded.items.get(&id).unwrap();
    assert_eq!(item.title, "Legacy 0.1.0 Media");
    assert_eq!(item.state, MediaTransferState::Finished);

    let _ = fs::remove_dir_all(&dir);
}
