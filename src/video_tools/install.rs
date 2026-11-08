use super::layout;
use super::manager::{StoredState, ready};
use super::verification::{checksum, ed25519, health, openpgp, safe_version};
use super::*;
use flate2::read::GzDecoder;
use reqwest::Client;
use semver::Version;
use std::fs::{self, File};
use std::path::{Component as PathComponent, Path};
use std::sync::Arc;
use uuid::Uuid;

const MAX_POINTER: u64 = 128 * 1024;
/// An Ed25519 signature is 64 bytes; an OpenPGP signature is larger.
const MAX_OPUL_SIGNATURE: u64 = 1024;
const MAX_PGP_SIGNATURE: u64 = 64 * 1024;
const MAX_CHECKSUMS: u64 = 256 * 1024;
const MAX_RUNTIME_ARCHIVE: u64 = 100 * 1024 * 1024;
const MAX_RUNTIME_EXPANDED: u64 = 256 * 1024 * 1024;
const MAX_SCRIPT: u64 = 16 * 1024 * 1024;

#[derive(Deserialize)]
struct Pointer {
    schema_version: u32,
    runtime: u64,
    minimum_opul_version: String,
    architectures: std::collections::HashMap<String, RuntimeAsset>,
}

#[derive(Deserialize)]
pub(super) struct RuntimeAsset {
    url: String,
    sha256: String,
    size: u64,
    signature_url: String,
}

pub(super) struct Resolved {
    pub(super) runtime: u64,
    pub(super) minimum_opul_version: String,
    pub(super) runtime_compatible: bool,
    pub(super) asset: RuntimeAsset,
    pub(super) yt_dlp: String,
    pub(super) yt_dlp_channel: YtDlpChannel,
    /// The yt-dlp release's download folder: the script, its checksums and their signature.
    pub(super) release: String,
}

impl Resolved {
    pub(super) fn script_url(&self) -> String {
        format!("{}/yt-dlp", self.release)
    }
}

pub(super) struct Failure {
    pub reason: FailureReason,
    pub details: String,
    pub exhausted_integrity: bool,
}
pub(super) fn download_error(e: impl std::fmt::Display) -> Failure {
    Failure {
        reason: FailureReason::Download,
        details: e.to_string(),
        exhausted_integrity: false,
    }
}
pub(super) fn verification_error(e: impl std::fmt::Display) -> Failure {
    Failure {
        reason: FailureReason::Verification,
        details: e.to_string(),
        exhausted_integrity: false,
    }
}

fn integrity_error(e: impl std::fmt::Display) -> Failure {
    Failure {
        exhausted_integrity: true,
        ..verification_error(e)
    }
}

/// Fetch and verify, and once more after a mismatch: a corrupted transfer gets a second chance,
/// a publication that is consistently wrong does not. The inner error is the last mismatch.
async fn verified<T, F>(mut attempt: impl FnMut() -> F) -> Result<Result<T, String>, Failure>
where
    F: std::future::Future<Output = Result<Result<T, String>, Failure>>,
{
    let mut mismatch = String::new();
    for _ in 0..2 {
        match attempt().await? {
            Ok(value) => return Ok(Ok(value)),
            Err(error) => mismatch = error,
        }
    }
    Ok(Err(mismatch))
}

pub(super) struct Staging(pub PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl VideoTools {
    pub(super) async fn resolve(
        &self,
        transfer: &super::transport::Transfer<'_>,
    ) -> Result<Resolved, Failure> {
        let pointer_url = &self.sources.pointer;
        let pointer_bytes = verified(move || async move {
            let bytes = transfer.bytes(pointer_url, MAX_POINTER).await?;
            let signature = transfer
                .bytes(&format!("{pointer_url}.sig"), MAX_OPUL_SIGNATURE)
                .await?;
            Ok(ed25519(&bytes, &signature, &self.sources.opul_key).map(|()| bytes))
        })
        .await?
        .map_err(|_| verification_error("The runtime pointer signature could not be verified."))?;
        let mut pointer: Pointer =
            serde_json::from_slice(&pointer_bytes).map_err(verification_error)?;
        if pointer.schema_version != 1 || pointer.runtime == 0 {
            return Err(verification_error("Unsupported Python runtime pointer."));
        }
        let minimum = Version::parse(&pointer.minimum_opul_version).map_err(verification_error)?;
        let mut current = Version::parse(&self.config.opul_version).map_err(verification_error)?;
        // Opul Nightly has the capabilities of its base app version.
        if self.config.nightly_available {
            current.pre = semver::Prerelease::EMPTY;
        }
        let architecture = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x64"
        };
        let asset = pointer
            .architectures
            .remove(architecture)
            .ok_or_else(|| verification_error("No Python runtime for this chip type."))?;
        let release = format!(
            "{}{}/",
            self.sources.runtime_release_prefix, pointer.runtime
        );
        if !asset.url.starts_with(&release) || asset.signature_url != format!("{}.sig", asset.url) {
            return Err(verification_error(
                "The runtime asset must belong to its immutable numbered release.",
            ));
        }
        if asset.size == 0 || asset.size > MAX_RUNTIME_ARCHIVE {
            return Err(verification_error("Invalid Python runtime size."));
        }
        let yt_dlp_channel = self.lock().stored.yt_dlp_channel;
        let repository = match yt_dlp_channel {
            YtDlpChannel::Stable => &self.sources.repository,
            YtDlpChannel::Nightly => &self.sources.nightly_repository,
        };
        let latest_url = format!("{repository}/releases/latest");
        let response = transfer.response(reqwest::Method::GET, &latest_url).await?;
        let resolved_url = response.url().as_str();
        let tag_prefix = format!("{repository}/releases/tag/");
        let tag = resolved_url
            .strip_prefix(&tag_prefix)
            .filter(|tag| safe_version(tag))
            .ok_or_else(|| verification_error("Could not resolve a single yt-dlp release tag."))?
            .to_string();
        let release = format!("{repository}/releases/download/{tag}");
        Ok(Resolved {
            runtime: pointer.runtime,
            minimum_opul_version: pointer.minimum_opul_version,
            runtime_compatible: current >= minimum,
            asset,
            yt_dlp: tag,
            yt_dlp_channel,
            release,
        })
    }

    pub(super) async fn published_size(&self) -> Result<u64, Failure> {
        let client = self.sources.client()?;
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: None,
        };
        let resolved = self.resolve(&transfer).await?;
        let response = transfer
            .response(reqwest::Method::HEAD, &resolved.script_url())
            .await?;
        let size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s > 0 && *s <= MAX_SCRIPT)
            .ok_or_else(|| download_error("Published yt-dlp size is unavailable."))?;
        Ok(resolved.asset.size + size)
    }

    pub(super) async fn install(self: Arc<Self>, id: Uuid) {
        let result = self.install_candidate(id).await;
        let mut live = self.lock();
        if !live.is_current(id) {
            return;
        }
        live.job = None;
        if let Err(failure) = result {
            live.snapshot.state = VideoToolsState::NotWorking {
                reason: failure.reason,
                details: failure.details,
            };
            self.publish(&mut live);
        }
    }

    async fn install_candidate(&self, id: Uuid) -> Result<(), Failure> {
        let staging = Staging(layout::staging_dir(&self.config.root, id));
        fs::create_dir(&staging.0).map_err(download_error)?;
        if self.sources.exclude_from_backup {
            exclude_from_backup(&self.config.root).await?;
        }
        let client = self.sources.client()?;
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: Some((id, Component::Python)),
        };
        let resolved = self.resolve(&transfer).await?;
        if !resolved.runtime_compatible {
            return Err(download_error(format!(
                "This Python runtime needs Opul {} or later. Update Opul, then try again.",
                resolved.minimum_opul_version
            )));
        }
        {
            let mut live = self.lock();
            if !live.is_current(id) {
                return Ok(());
            }
            live.candidate = Some((resolved.runtime, resolved.yt_dlp.clone()));
        }
        let runtime_dir = self
            .fetch_runtime(id, &staging.0, &client, &resolved)
            .await?;
        let script_dir = self
            .fetch_script(id, &staging.0, &client, &resolved)
            .await?;
        let candidate = layout::invocation(&runtime_dir, &script_dir);
        let size = fs::metadata(&candidate.script)
            .map_err(download_error)?
            .len();
        let python_version = health(&candidate, &resolved.yt_dlp)
            .await
            .map_err(|details| Failure {
                reason: FailureReason::HealthCheck,
                details: details.to_string(),
                exhausted_integrity: false,
            })?;
        self.progress(id, Component::YtDlp, StepStatus::Done, size, Some(size));
        let mut live = self.lock();
        if !live.is_current(id) {
            return Ok(());
        }
        let installed = InstalledPair {
            runtime: resolved.runtime,
            yt_dlp: resolved.yt_dlp,
            yt_dlp_channel: resolved.yt_dlp_channel,
            python_version,
        };
        place_pair(
            &self.config,
            &installed,
            Some(&runtime_dir),
            Some(&script_dir),
        )
        .map_err(download_error)?;
        // A fresh pair inherits nothing from whatever was installed before.
        live.commit(&self.config, |stored| {
            *stored = StoredState {
                installed: Some(installed.clone()),
                ..stored.preferences_only(false)
            };
        })
        .map_err(download_error)?;
        live.snapshot.published_size = Some(resolved.asset.size + size);
        live.snapshot.state = ready(installed);
        self.publish(&mut live);
        Ok(())
    }

    pub(super) async fn fetch_runtime(
        &self,
        id: Uuid,
        staging: &Path,
        client: &Client,
        resolved: &Resolved,
    ) -> Result<PathBuf, Failure> {
        let transfer = &super::transport::Transfer {
            tools: self,
            client,
            step: Some((id, Component::Python)),
        };
        let asset = &resolved.asset;
        let archive = staging.join("runtime.tar.gz");
        let archive_path = &archive;
        verified(move || async move {
            let archive = archive_path;
            transfer
                .download(&asset.url, archive, Some(asset.size), MAX_RUNTIME_ARCHIVE)
                .await?;
            self.progress(
                id,
                Component::Python,
                StepStatus::Verifying,
                asset.size,
                Some(asset.size),
            );
            let signature = transfer
                .bytes(&asset.signature_url, MAX_OPUL_SIGNATURE)
                .await?;
            let data = fs::read(archive).map_err(download_error)?;
            let verification = ed25519(&data, &signature, &self.sources.opul_key)
                .and_then(|()| checksum(&data, &asset.sha256))
                .and_then(|()| {
                    if data.len() as u64 == asset.size {
                        Ok(())
                    } else {
                        Err("Runtime size did not match the signed pointer.".into())
                    }
                });
            if verification.is_err() {
                fs::remove_file(archive).map_err(download_error)?;
            }
            Ok(verification)
        })
        .await?
        .map_err(integrity_error)?;
        self.progress(
            id,
            Component::Python,
            StepStatus::Installing,
            asset.size,
            Some(asset.size),
        );
        let runtime_dir = staging.join("runtime");
        self.when_idle(id, |_| {
            unpack(&archive, &runtime_dir).map_err(verification_error)
        })
        .await?
        .ok_or_else(|| download_error("Installation cancelled."))?;
        self.progress(
            id,
            Component::Python,
            StepStatus::Done,
            asset.size,
            Some(asset.size),
        );
        Ok(runtime_dir)
    }

    pub(super) async fn fetch_script(
        &self,
        id: Uuid,
        staging: &Path,
        client: &Client,
        resolved: &Resolved,
    ) -> Result<PathBuf, Failure> {
        let base = &resolved.release;
        let transfer = &super::transport::Transfer {
            tools: self,
            client,
            step: Some((id, Component::YtDlp)),
        };
        let sums = verified(move || async move {
            let sums = transfer
                .bytes(&format!("{base}/SHA2-256SUMS"), MAX_CHECKSUMS)
                .await?;
            let signature = transfer
                .bytes(&format!("{base}/SHA2-256SUMS.sig"), MAX_PGP_SIGNATURE)
                .await?;
            Ok(openpgp(&sums, &signature, &self.sources.yt_dlp_key).map(|()| sums))
        })
        .await?
        .map_err(|_| integrity_error("The yt-dlp checksum signature could not be verified."))?;
        let expected_checksum = script_checksum(&sums)?;
        let script_dir = staging.join("script");
        fs::create_dir(&script_dir).map_err(download_error)?;
        let script = layout::script_file(&script_dir);
        let script_path = &script;
        let expected_checksum = &expected_checksum;
        let size = verified(move || async move {
            let script = script_path;
            transfer
                .download(&resolved.script_url(), script, None, MAX_SCRIPT)
                .await?;
            let data = fs::read(script).map_err(download_error)?;
            let size = data.len() as u64;
            self.progress(
                id,
                Component::YtDlp,
                StepStatus::Verifying,
                size,
                Some(size),
            );
            let verification = checksum(&data, expected_checksum).map(|()| size);
            if verification.is_err() {
                fs::remove_file(script).map_err(download_error)?;
            }
            Ok(verification)
        })
        .await?
        .map_err(|_| integrity_error("The yt-dlp checksum did not match."))?;
        self.progress(
            id,
            Component::YtDlp,
            StepStatus::Installing,
            size,
            Some(size),
        );
        Ok(script_dir)
    }
}

/// The one checksum the signed manifest lists for the bare yt-dlp script.
fn script_checksum(sums: &[u8]) -> Result<String, Failure> {
    let manifest = std::str::from_utf8(sums).map_err(verification_error)?;
    let mut matches = manifest.lines().filter_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        let name = fields.next()?.trim_start_matches('*');
        (name == "yt-dlp"
            && fields.next().is_none()
            && hash.len() == 64
            && hash.bytes().all(|b| b.is_ascii_hexdigit()))
        .then_some(hash)
    });
    let expected = matches
        .next()
        .ok_or_else(|| verification_error("No checksum for the bare yt-dlp script."))?
        .to_string();
    if matches.next().is_some() {
        return Err(verification_error("Ambiguous yt-dlp checksum."));
    }
    Ok(expected)
}

pub(super) fn place_pair(
    config: &VideoToolsConfig,
    installed: &InstalledPair,
    runtime: Option<&Path>,
    script: Option<&Path>,
) -> std::io::Result<()> {
    if let Some(runtime) = runtime {
        place(
            runtime,
            &layout::runtime_dir(&config.root, installed.runtime),
        )?;
    }
    if let Some(script) = script {
        place(
            script,
            &layout::script_dir(&config.root, installed.yt_dlp_channel, &installed.yt_dlp),
        )?;
    }
    Ok(())
}

fn place(staged: &Path, destination: &Path) -> std::io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    if destination.exists() {
        fs::remove_dir_all(destination)?;
    }
    fs::rename(staged, destination)
}

fn unpack(archive: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination).map_err(|e| e.to_string())?;
    let mut archive = tar::Archive::new(GzDecoder::new(
        File::open(archive).map_err(|e| e.to_string())?,
    ));
    let mut size = 0_u64;
    for entry in archive.entries().map_err(|e| e.to_string())? {
        let mut entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path().map_err(|e| e.to_string())?.into_owned();
        if path.is_absolute()
            || path
                .components()
                .any(|c| matches!(c, PathComponent::ParentDir | PathComponent::Prefix(_)))
        {
            return Err("Unsafe runtime archive path.".into());
        }
        let kind = entry.header().entry_type();
        if !(kind.is_file() || kind.is_dir() || kind.is_symlink() || kind.is_hard_link()) {
            return Err("Unsupported runtime archive entry.".into());
        }
        if let Some(link) = entry.link_name().map_err(|e| e.to_string())? {
            // Python's bin/python3 is a relative symlink; no link may escape the runtime.
            let mut depth = if kind.is_symlink() {
                path.parent().map_or(0, |p| p.components().count())
            } else {
                0
            };
            if link.is_absolute() {
                return Err("Absolute runtime symlink.".into());
            }
            for component in link.components() {
                match component {
                    PathComponent::ParentDir if depth == 0 => {
                        return Err("Runtime link escapes its directory.".into());
                    }
                    PathComponent::ParentDir => depth -= 1,
                    PathComponent::Normal(_) => depth += 1,
                    _ => {}
                }
            }
        }
        size += entry.size();
        if size > MAX_RUNTIME_EXPANDED {
            return Err("Expanded runtime exceeds its size limit.".into());
        }
        if !entry.unpack_in(destination).map_err(|e| e.to_string())? {
            return Err("Runtime archive path escaped staging.".into());
        }
    }
    Ok(())
}

async fn exclude_from_backup(path: &Path) -> Result<(), Failure> {
    if !cfg!(target_os = "macos") {
        return Ok(());
    }
    let output = tokio::process::Command::new("/usr/bin/tmutil")
        .arg("addexclusion")
        .arg(path)
        .kill_on_drop(true)
        .output()
        .await
        .map_err(download_error)?;
    if !output.status.success() {
        return Err(download_error(format!(
            "Could not exclude Tools from Time Machine: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}
