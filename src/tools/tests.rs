use super::*;
use pgp::composed::{KeyType, SecretKeyParamsBuilder, SignedSecretKey};
use pgp::crypto::hash::HashAlgorithm;
use pgp::ser::Serialize as _;
use pgp::types::{KeyDetails, Password};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
struct FakeState {
    latest: Mutex<Option<String>>,
    files: Mutex<HashMap<(String, String), Vec<u8>>>,
    /// Fail the archive download with a network error after this many bytes.
    interrupt_archive_after: Mutex<Option<usize>>,
    calls: AtomicUsize,
}

struct FakeSource(Arc<FakeState>);

impl ReleaseSource for FakeSource {
    fn latest_version(&self) -> Result<String, ToolError> {
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        self.0
            .latest
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| ToolError::Network("offline".to_string()))
    }

    fn asset_size(&self, version: &str, asset: &str) -> Result<u64, ToolError> {
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        self.0
            .files
            .lock()
            .unwrap()
            .get(&(version.to_string(), asset.to_string()))
            .map(|bytes| bytes.len() as u64)
            .ok_or_else(|| ToolError::Network("not found".to_string()))
    }

    fn download(
        &self,
        version: &str,
        asset: &str,
        limit: u64,
        destination: &mut dyn Write,
        progress: &mut dyn FnMut(u64, Option<u64>),
    ) -> Result<(), ToolError> {
        self.0.calls.fetch_add(1, Ordering::SeqCst);
        let bytes = self
            .0
            .files
            .lock()
            .unwrap()
            .get(&(version.to_string(), asset.to_string()))
            .cloned()
            .ok_or_else(|| ToolError::Network(format!("{asset} not found")))?;
        assert!(bytes.len() as u64 <= limit);
        if asset == ARCHIVE
            && let Some(cut) = *self.0.interrupt_archive_after.lock().unwrap()
        {
            destination.write_all(&bytes[..cut]).unwrap();
            progress(cut as u64, Some(bytes.len() as u64));
            return Err(ToolError::Network("connection reset".to_string()));
        }
        destination.write_all(&bytes).unwrap();
        progress(bytes.len() as u64, Some(bytes.len() as u64));
        Ok(())
    }
}

struct Fixture {
    root: PathBuf,
    fake: Arc<FakeState>,
    key: SignedSecretKey,
    tools: Arc<MediaTools>,
}

impl Fixture {
    fn new(settings: MediaToolSettings) -> Self {
        let root = std::env::temp_dir().join(format!("opul-tools-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let key = signing_key();
        let fake = Arc::new(FakeState::default());
        let tools = MediaTools::with_source(
            &root.join("Tools"),
            None,
            settings,
            Box::new(FakeSource(Arc::clone(&fake))),
            key.to_public_key(),
        );
        Self {
            root,
            fake,
            key,
            tools,
        }
    }

    fn reopen(&self, settings: MediaToolSettings) -> Arc<MediaTools> {
        MediaTools::with_source(
            &self.root.join("Tools"),
            None,
            settings,
            Box::new(FakeSource(Arc::clone(&self.fake))),
            self.key.to_public_key(),
        )
    }

    /// Publishes a release whose yt-dlp prints `version`, or fails when `works` is false.
    fn publish(&self, version: &str, works: bool) {
        let script = if works {
            format!("#!/bin/sh\necho {version}\n")
        } else {
            "#!/bin/sh\necho 'this Mac cannot run me' >&2\nexit 1\n".to_string()
        };
        let archive = zip_with_executable(&self.root, &script);
        let checksums = format!(
            "{}  {EXECUTABLE}\n{}  {ARCHIVE}\n",
            hex::encode(Sha256::digest(b"single file build")),
            hex::encode(Sha256::digest(&archive))
        );
        let signature = sign(&self.key, checksums.as_bytes());
        let mut files = self.fake.files.lock().unwrap();
        files.insert((version.into(), ARCHIVE.into()), archive);
        files.insert((version.into(), CHECKSUMS.into()), checksums.into_bytes());
        files.insert((version.into(), SIGNATURE.into()), signature);
        *self.fake.latest.lock().unwrap() = Some(version.to_string());
    }

    fn replace(&self, version: &str, asset: &str, bytes: Vec<u8>) {
        self.fake
            .files
            .lock()
            .unwrap()
            .insert((version.into(), asset.into()), bytes);
    }

    fn yt_dlp_root(&self) -> PathBuf {
        self.root.join("Tools").join("yt-dlp")
    }

    fn calls(&self) -> usize {
        self.fake.calls.load(Ordering::SeqCst)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.tools.shutdown();
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn signing_key() -> SignedSecretKey {
    SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519Legacy)
        .can_certify(true)
        .can_sign(true)
        .primary_user_id("Opul test <test@example.invalid>".into())
        .build()
        .unwrap()
        .generate(rand::thread_rng())
        .unwrap()
}

fn sign(key: &SignedSecretKey, data: &[u8]) -> Vec<u8> {
    DetachedSignature::sign_binary_data(
        rand::thread_rng(),
        &key.primary_key,
        &Password::empty(),
        HashAlgorithm::Sha256,
        data,
    )
    .unwrap()
    .to_bytes()
    .unwrap()
}

fn zip_with_executable(scratch: &Path, script: &str) -> Vec<u8> {
    let folder = scratch.join(format!("bundle-{}", Uuid::new_v4()));
    fs::create_dir_all(folder.join("_internal")).unwrap();
    let executable = folder.join(EXECUTABLE);
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    fs::write(folder.join("_internal").join("base_library.zip"), b"python").unwrap();
    let archive = scratch.join(format!("{}.zip", Uuid::new_v4()));
    let status = Command::new("/usr/bin/ditto")
        .args(["-c", "-k"])
        .arg(&folder)
        .arg(&archive)
        .status()
        .unwrap();
    assert!(status.success());
    let bytes = fs::read(&archive).unwrap();
    fs::remove_dir_all(folder).unwrap();
    fs::remove_file(archive).unwrap();
    bytes
}

fn managed() -> MediaToolSettings {
    MediaToolSettings {
        auto_update_yt_dlp: true,
        ..MediaToolSettings::default()
    }
}

fn installed_version(tools: &MediaTools) -> Option<String> {
    match tools.status().yt_dlp {
        YtDlpState::Installed { version, .. } => Some(version),
        _ => None,
    }
}

fn version_folders(root: &Path) -> Vec<String> {
    let mut names = fs::read_dir(root)
        .unwrap()
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| is_version(name))
        .collect::<Vec<_>>();
    names.sort();
    names
}

fn staging_folders(root: &Path) -> usize {
    fs::read_dir(root)
        .unwrap()
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".download-")
        })
        .count()
}

/// Waits for the scheduler thread to settle on a state.
fn eventually(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn the_bundled_key_is_yt_dlps_release_key() {
    let (key, _) = SignedPublicKey::from_string(YT_DLP_PUBLIC_KEY).unwrap();
    assert_eq!(
        hex::encode_upper(key.primary_key.fingerprint().as_bytes()),
        "AC0CBBE6848D6A873464AF4E57CF65933B5A7581"
    );
}

#[test]
fn yt_dlps_real_signature_verifies_and_a_changed_byte_does_not() {
    let (key, _) = SignedPublicKey::from_string(YT_DLP_PUBLIC_KEY).unwrap();
    let checksums = include_bytes!("fixtures/SHA2-256SUMS");
    let signature = include_bytes!("fixtures/SHA2-256SUMS.sig");
    verify_signature(&key, checksums, signature).unwrap();
    assert_eq!(
        expected_checksum(checksums, ARCHIVE).unwrap(),
        "2b5251788b36fcbc476568b4eea8cb239f0a775759456f2ff9344a437c52f000"
    );
    let mut tampered = checksums.to_vec();
    tampered[0] ^= 1;
    assert!(matches!(
        verify_signature(&key, &tampered, signature),
        Err(ToolError::Verification(_))
    ));
}

#[test]
fn installs_a_verified_release_and_points_current_at_it() {
    let fixture = Fixture::new(managed());
    fixture.publish("2026.10.21", true);
    assert_eq!(fixture.tools.status().yt_dlp, YtDlpState::NotInstalled);
    assert_eq!(fixture.tools.unavailable(), Some(Unavailable::Missing));

    fixture.tools.request_install();
    fixture.tools.install(true).unwrap();

    assert_eq!(
        installed_version(&fixture.tools).as_deref(),
        Some("2026.10.21")
    );
    let root = fixture.yt_dlp_root();
    assert_eq!(
        fs::read_link(root.join(CURRENT)).unwrap(),
        Path::new("2026.10.21")
    );
    assert_eq!(
        fs::metadata(&root).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(root.join("2026.10.21"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(staging_folders(&root), 0);
    let process = fixture.tools.resolve().unwrap();
    assert_eq!(process.yt_dlp, root.join("2026.10.21").join(EXECUTABLE));
    assert!(fixture.tools.status().yt_dlp_ready);

    // The choice survives a relaunch.
    drop(process);
    let reopened = fixture.reopen(managed());
    assert_eq!(installed_version(&reopened).as_deref(), Some("2026.10.21"));
}

#[test]
fn a_bad_checksum_is_rejected_and_the_previous_copy_stays() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.01", true);
    fixture.tools.install(true).unwrap();
    fixture.publish("2026.10.21", true);
    let mut archive = fixture.fake.files.lock().unwrap()
        [&("2026.10.21".to_string(), ARCHIVE.to_string())]
        .clone();
    let last = archive.len() - 1;
    archive[last] ^= 0xff;
    fixture.replace("2026.10.21", ARCHIVE, archive);

    let error = fixture.tools.install(false).unwrap_err();

    assert!(matches!(error, ToolError::Verification(_)), "{error:?}");
    assert_eq!(
        installed_version(&fixture.tools).as_deref(),
        Some("2026.10.01")
    );
    let status = fixture.tools.status();
    assert!(status.update_error.unwrap().contains("verification"));
    assert_eq!(status.activity, YtDlpActivity::Idle);
    assert_eq!(staging_folders(&fixture.yt_dlp_root()), 0);
    // Never retried silently: no retry is scheduled and automatic checks skip the release.
    assert!(fixture.tools.lock().retry.is_none());
    let calls = fixture.calls();
    fixture.tools.install(false).unwrap();
    assert_eq!(
        fixture.calls(),
        calls + 1,
        "only the latest-version lookup ran"
    );
}

#[test]
fn a_bad_signature_is_rejected() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.21", true);
    let checksums = fixture.fake.files.lock().unwrap()
        [&("2026.10.21".to_string(), CHECKSUMS.to_string())]
        .clone();
    fixture.replace("2026.10.21", SIGNATURE, sign(&signing_key(), &checksums));

    let error = fixture.tools.install(true).unwrap_err();

    assert!(matches!(error, ToolError::Verification(_)), "{error:?}");
    assert!(matches!(
        fixture.tools.status().yt_dlp,
        YtDlpState::Failed { .. }
    ));
    assert!(!fixture.yt_dlp_root().join("2026.10.21").exists());
}

#[test]
fn tampered_checksums_are_rejected() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.21", true);
    let archive = fixture.fake.files.lock().unwrap()
        [&("2026.10.21".to_string(), ARCHIVE.to_string())]
        .clone();
    let mut altered = archive.clone();
    altered.extend_from_slice(b"extra");
    fixture.replace("2026.10.21", ARCHIVE, altered.clone());
    fixture.replace(
        "2026.10.21",
        CHECKSUMS,
        format!("{}  {ARCHIVE}\n", hex::encode(Sha256::digest(&altered))).into_bytes(),
    );

    let error = fixture.tools.install(true).unwrap_err();

    assert!(matches!(error, ToolError::Verification(_)), "{error:?}");
    assert!(!fixture.tools.status().yt_dlp_ready);
}

#[test]
fn a_release_that_fails_version_rolls_back_and_is_never_tried_again() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.01", true);
    fixture.tools.install(true).unwrap();
    fixture.publish("2026.10.21", false);

    let error = fixture.tools.install(false).unwrap_err();

    assert!(matches!(error, ToolError::Broken(_)), "{error:?}");
    assert_eq!(
        installed_version(&fixture.tools).as_deref(),
        Some("2026.10.01")
    );
    assert_eq!(version_folders(&fixture.yt_dlp_root()), ["2026.10.01"]);
    assert!(
        fixture
            .tools
            .lock()
            .record
            .broken_versions
            .contains(&"2026.10.21".to_string())
    );

    let calls = fixture.calls();
    fixture.tools.install(true).unwrap();
    assert_eq!(
        fixture.calls(),
        calls + 1,
        "the broken release was not downloaded again"
    );
    assert_eq!(
        installed_version(&fixture.tools).as_deref(),
        Some("2026.10.01")
    );
}

#[test]
fn an_interrupted_download_leaves_nothing_behind_and_retries_with_backoff() {
    let fixture = Fixture::new(managed());
    fixture.publish("2026.10.21", true);
    *fixture.fake.interrupt_archive_after.lock().unwrap() = Some(100);
    fixture.tools.request_install();

    let error = fixture.tools.install(true).unwrap_err();

    assert!(matches!(error, ToolError::Network(_)), "{error:?}");
    assert_eq!(staging_folders(&fixture.yt_dlp_root()), 0);
    assert!(version_folders(&fixture.yt_dlp_root()).is_empty());
    let status = fixture.tools.status();
    assert!(
        status.yt_dlp_pending,
        "links keep waiting while a retry is scheduled"
    );
    assert!(matches!(status.activity, YtDlpActivity::WaitingToRetry { seconds } if seconds <= 30));
    assert_eq!(fixture.tools.unavailable(), Some(Unavailable::Pending));

    let _ = fixture.tools.install(false);
    let (at, attempts) = fixture.tools.lock().retry.unwrap();
    assert_eq!(attempts, 2);
    assert!(
        at > Instant::now() + Duration::from_secs(60),
        "the second retry waits two minutes"
    );

    *fixture.fake.interrupt_archive_after.lock().unwrap() = None;
    fixture.tools.install(false).unwrap();
    assert_eq!(
        installed_version(&fixture.tools).as_deref(),
        Some("2026.10.21")
    );
    assert!(fixture.tools.lock().retry.is_none());
}

#[test]
fn nothing_is_requested_from_the_network_without_consent_or_with_updates_off() {
    // Not installed and never agreed to: starting Opul makes no request at all.
    let fixture = Fixture::new(managed());
    fixture.publish("2026.10.21", true);
    fixture.tools.start();
    thread::sleep(Duration::from_millis(300));
    assert_eq!(fixture.calls(), 0);
    drop(fixture);

    // Installed with automatic updates off: still no request.
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.01", true);
    fixture.tools.install(true).unwrap();
    let calls = fixture.calls();
    let tools = fixture.reopen(MediaToolSettings {
        auto_update_yt_dlp: false,
        ..MediaToolSettings::default()
    });
    tools.start();
    thread::sleep(Duration::from_millis(300));
    assert_eq!(fixture.calls(), calls);
    tools.shutdown();

    // With automatic updates on, Opul checks at launch.
    fixture.publish("2026.10.21", true);
    let tools = fixture.reopen(managed());
    tools.start();
    assert!(eventually(
        || installed_version(&tools).as_deref() == Some("2026.10.21")
    ));
    tools.shutdown();
}

#[test]
fn a_custom_path_means_the_managed_copy_is_never_touched() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.10.01", true);
    fixture.tools.install(true).unwrap();
    fixture.publish("2026.10.21", true);
    let custom = fixture.root.join("my-yt-dlp");
    fs::write(&custom, "#!/bin/sh\necho 2025.01.01-custom\n").unwrap();
    fs::set_permissions(&custom, fs::Permissions::from_mode(0o755)).unwrap();
    let settings = MediaToolSettings {
        custom_yt_dlp_path: Some(custom.to_string_lossy().into_owned()),
        ..managed()
    };
    let calls = fixture.calls();

    let tools = fixture.reopen(settings);
    tools.start();
    assert!(eventually(|| matches!(
        tools.status().yt_dlp,
        YtDlpState::Custom {
            version: Some(_),
            ..
        }
    )));
    thread::sleep(Duration::from_millis(200));

    assert_eq!(
        fixture.calls(),
        calls,
        "no update check while a custom path is set"
    );
    assert_eq!(version_folders(&fixture.yt_dlp_root()), ["2026.10.01"]);
    assert_eq!(tools.resolve().unwrap().yt_dlp, custom);

    // A custom copy that stops working is reported, never replaced by the managed one.
    fs::write(&custom, "#!/bin/sh\nexit 3\n").unwrap();
    assert_eq!(
        tools.resolve().err(),
        Some(Unavailable::CustomBroken {
            path: custom.to_string_lossy().into_owned()
        })
    );
    assert!(matches!(
        tools.status().yt_dlp,
        YtDlpState::CustomBroken { .. }
    ));
    tools.shutdown();
}

#[test]
fn removal_keeps_a_version_in_use_until_its_process_ends() {
    let fixture = Fixture::new(managed());
    fixture.publish("2026.10.21", true);
    fixture.tools.request_install();
    fixture.tools.install(true).unwrap();
    let running = fixture.tools.resolve().unwrap();

    fixture.tools.remove();

    assert_eq!(fixture.tools.status().yt_dlp, YtDlpState::NotInstalled);
    assert_eq!(fixture.tools.unavailable(), Some(Unavailable::Missing));
    assert!(
        running.yt_dlp.exists(),
        "the running download keeps its yt-dlp"
    );
    assert!(!fixture.tools.lock().record.wanted);
    drop(running);
    assert!(version_folders(&fixture.yt_dlp_root()).is_empty());
}

#[test]
fn updates_keep_the_previous_version_and_any_version_in_use() {
    let fixture = Fixture::new(managed());
    fixture.tools.request_install();
    fixture.publish("2026.09.01", true);
    fixture.tools.install(true).unwrap();
    let running = fixture.tools.resolve().unwrap();
    fixture.publish("2026.10.01", true);
    fixture.tools.install(false).unwrap();
    fixture.publish("2026.10.21", true);
    fixture.tools.install(false).unwrap();

    assert_eq!(
        version_folders(&fixture.yt_dlp_root()),
        ["2026.09.01", "2026.10.01", "2026.10.21"]
    );
    // A new process gets the new version; the running one keeps its own.
    assert!(
        fixture
            .tools
            .resolve()
            .unwrap()
            .yt_dlp
            .ends_with(format!("2026.10.21/{EXECUTABLE}"))
    );
    drop(running);
    assert_eq!(
        version_folders(&fixture.yt_dlp_root()),
        ["2026.10.01", "2026.10.21"]
    );
}

#[test]
fn yt_dlp_always_gets_opuls_javascript_runtime_and_ffmpeg_folder() {
    let arguments = yt_dlp_tool_arguments(
        Some(Path::new("/Applications/Opul.app/Contents/Helpers/qjs")),
        Some(Path::new("/Applications/Opul.app/Contents/Helpers")),
    );
    assert_eq!(
        arguments,
        [
            "--no-js-runtimes",
            "--js-runtimes",
            "quickjs:/Applications/Opul.app/Contents/Helpers/qjs",
            "--ffmpeg-location",
            "/Applications/Opul.app/Contents/Helpers",
        ]
    );
    // Without the driver no other runtime is allowed either.
    assert_eq!(yt_dlp_tool_arguments(None, None), ["--no-js-runtimes"]);
}

#[test]
fn a_custom_ffmpeg_is_passed_as_its_folder_when_named_ffmpeg() {
    assert_eq!(
        ffmpeg_location(Path::new("/opt/ffmpeg/bin/ffmpeg")),
        Path::new("/opt/ffmpeg/bin")
    );
    assert_eq!(
        ffmpeg_location(Path::new("/opt/tools/ffmpeg-7")),
        Path::new("/opt/tools/ffmpeg-7")
    );
}

#[test]
fn release_names_are_validated_and_ordered() {
    assert!(is_version("2026.10.21"));
    assert!(is_version("2026.10.21.1"));
    assert!(!is_version("../../evil"));
    assert!(!is_version("2026.10"));
    assert!(!is_version("nightly"));
    assert!(compare_versions("2026.10.21.1", "2026.10.21").is_gt());
    assert!(compare_versions("2026.09.30", "2026.10.01").is_lt());
}

/// The real thing: download, verify and run the latest stable yt-dlp from GitHub.
#[test]
#[ignore = "downloads about 54 MB from GitHub"]
fn installs_the_latest_release_from_github() {
    let root = std::env::temp_dir().join(format!("opul-tools-github-{}", Uuid::new_v4()));
    let tools = MediaTools::new(&root, None, managed());
    tools.request_install();
    tools.install(true).unwrap();
    let status = tools.status();
    assert!(status.yt_dlp_ready, "{status:?}");
    let process = tools.resolve().unwrap();
    let output = Command::new(&process.yt_dlp)
        .arg("--version")
        .output()
        .unwrap();
    assert!(output.status.success());
    drop(process);
    let _ = fs::remove_dir_all(root);
}
