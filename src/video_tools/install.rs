use super::manager::{is_current, persist, ready};
use super::verification::{checksum, ed25519, health, openpgp, safe_version};
use super::*;
use flate2::read::GzDecoder;
use reqwest::Client;
use semver::Version;
use std::fs::{self, File};
use std::path::{Component as PathComponent, Path};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Clone)]
pub(super) struct Sources {
    pub pointer: String,
    pub repository: String,
    pub opul_key: [u8; 32],
    pub yt_dlp_key: String,
    pub allow_fixture_http: bool,
    pub environment: Arc<dyn NetworkEnvironment>,
}

impl Sources {
    pub fn production() -> Self {
        Self {
            pointer: "https://github.com/slovakian/opul/releases/download/python-runtime/python-runtime.json".into(),
            repository: "https://github.com/yt-dlp/yt-dlp".into(),
            opul_key: [0x06,0x54,0x04,0xda,0xa6,0xf7,0x16,0xa3,0xd2,0x21,0xb6,0xc5,0xe1,0x3a,0x44,0xd5,0xe3,0x1a,0xc9,0xd0,0xf9,0xcd,0xb9,0x59,0x0e,0xb4,0x68,0x2f,0x3d,0x4a,0x1b,0x55],
            yt_dlp_key: include_str!("keys/yt-dlp.asc").into(),
            allow_fixture_http: false,
            environment: super::transport::system_network(),
        }
    }

    fn client(&self) -> Result<Client, Failure> {
        Client::builder()
            .https_only(!self.allow_fixture_http)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(180))
            .user_agent("Opul video tools")
            .build()
            .map_err(download_error)
    }
}

#[derive(Deserialize)]
struct Pointer {
    schema_version: u32,
    runtime: u64,
    minimum_opul_version: String,
    architectures: std::collections::HashMap<String, RuntimeAsset>,
}

#[derive(Deserialize)]
struct RuntimeAsset {
    url: String,
    sha256: String,
    size: u64,
    signature_url: String,
}

struct Resolved {
    runtime: u64,
    asset: RuntimeAsset,
    yt_dlp: String,
    script_url: String,
    checksum: String,
}

pub(super) struct Failure {
    pub reason: FailureReason,
    pub details: String,
}
pub(super) fn download_error(e: impl std::fmt::Display) -> Failure {
    Failure {
        reason: FailureReason::Download,
        details: e.to_string(),
    }
}
pub(super) fn verification_error(e: impl std::fmt::Display) -> Failure {
    Failure {
        reason: FailureReason::Verification,
        details: e.to_string(),
    }
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl VideoTools {
    async fn resolve(
        &self,
        transfer: &super::transport::Transfer<'_>,
    ) -> Result<Resolved, Failure> {
        let mut pointer_bytes = Vec::new();
        let mut verified = false;
        for _ in 0..2 {
            pointer_bytes = transfer.bytes(&self.sources.pointer, 128 * 1024).await?;
            let signature = transfer
                .bytes(&format!("{}.sig", self.sources.pointer), 1024)
                .await?;
            if ed25519(&pointer_bytes, &signature, &self.sources.opul_key).is_ok() {
                verified = true;
                break;
            }
        }
        if !verified {
            return Err(verification_error(
                "The runtime pointer signature could not be verified.",
            ));
        }
        let mut pointer: Pointer =
            serde_json::from_slice(&pointer_bytes).map_err(verification_error)?;
        if pointer.schema_version != 1 || pointer.runtime == 0 {
            return Err(verification_error("Unsupported Python runtime pointer."));
        }
        let minimum = Version::parse(&pointer.minimum_opul_version).map_err(verification_error)?;
        let current = Version::parse(&self.config.opul_version).map_err(verification_error)?;
        if current < minimum {
            return Err(download_error(format!(
                "This Python runtime needs Opul {minimum} or later. Update Opul, then try again."
            )));
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
        if !self.sources.allow_fixture_http {
            let prefix = format!(
                "https://github.com/slovakian/opul/releases/download/python-runtime-{}/",
                pointer.runtime
            );
            if !asset.url.starts_with(&prefix)
                || asset.signature_url != format!("{}.sig", asset.url)
            {
                return Err(verification_error(
                    "The runtime asset must belong to its immutable numbered release.",
                ));
            }
        }
        if asset.size == 0 || asset.size > 100 * 1024 * 1024 {
            return Err(verification_error("Invalid Python runtime size."));
        }
        let latest_url = format!("{}/releases/latest", self.sources.repository);
        let response = transfer.response(reqwest::Method::GET, &latest_url).await?;
        let resolved_url = response.url().as_str();
        let tag_prefix = format!("{}/releases/tag/", self.sources.repository);
        let tag = resolved_url
            .strip_prefix(&tag_prefix)
            .filter(|tag| safe_version(tag))
            .ok_or_else(|| verification_error("Could not resolve a single yt-dlp release tag."))?
            .to_string();
        let base = format!("{}/releases/download/{tag}", self.sources.repository);
        let mut sums = Vec::new();
        let mut sums_verified = false;
        for _ in 0..2 {
            sums = transfer
                .bytes(&format!("{base}/SHA2-256SUMS"), 256 * 1024)
                .await?;
            let signature = transfer
                .bytes(&format!("{base}/SHA2-256SUMS.sig"), 64 * 1024)
                .await?;
            if openpgp(&sums, &signature, &self.sources.yt_dlp_key).is_ok() {
                sums_verified = true;
                break;
            }
        }
        if !sums_verified {
            return Err(verification_error(
                "The yt-dlp checksum signature could not be verified.",
            ));
        }
        let manifest = std::str::from_utf8(&sums).map_err(verification_error)?;
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
        let checksum = matches
            .next()
            .ok_or_else(|| verification_error("No checksum for the bare yt-dlp script."))?
            .to_string();
        if matches.next().is_some() {
            return Err(verification_error("Ambiguous yt-dlp checksum."));
        }
        Ok(Resolved {
            runtime: pointer.runtime,
            asset,
            yt_dlp: tag,
            script_url: format!("{base}/yt-dlp"),
            checksum,
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
            .response(reqwest::Method::HEAD, &resolved.script_url)
            .await?;
        let size = response
            .headers()
            .get(reqwest::header::CONTENT_LENGTH)
            .and_then(|s| s.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s > 0 && *s <= 16 * 1024 * 1024)
            .ok_or_else(|| download_error("Published yt-dlp size is unavailable."))?;
        Ok(resolved.asset.size + size)
    }

    pub(super) async fn install(self: Arc<Self>, id: Uuid) {
        let result = self.install_candidate(id).await;
        let mut live = self.lock();
        if !is_current(&live, id) {
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
        let staging = Staging(self.config.root.join(format!(".install-{id}")));
        fs::create_dir(&staging.0).map_err(download_error)?;
        exclude_from_backup(&self.config.root, self.sources.allow_fixture_http)?;
        let client = self.sources.client()?;
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: Some((id, Component::Python)),
        };
        let resolved = self.resolve(&transfer).await?;
        {
            let mut live = self.lock();
            if !is_current(&live, id) {
                return Ok(());
            }
            live.candidate = Some((resolved.runtime, resolved.yt_dlp.clone()));
        }
        let archive = staging.0.join("runtime.tar.gz");
        for attempt in 0..2 {
            transfer
                .download(
                    &resolved.asset.url,
                    &archive,
                    Some(resolved.asset.size),
                    100 * 1024 * 1024,
                )
                .await?;
            self.progress(
                id,
                Component::Python,
                StepStatus::Verifying,
                resolved.asset.size,
                Some(resolved.asset.size),
            );
            let signature = transfer.bytes(&resolved.asset.signature_url, 1024).await?;
            let data = fs::read(&archive).map_err(download_error)?;
            let verification = ed25519(&data, &signature, &self.sources.opul_key)
                .and_then(|()| checksum(&data, &resolved.asset.sha256))
                .and_then(|()| {
                    if data.len() as u64 == resolved.asset.size {
                        Ok(())
                    } else {
                        Err("Runtime size did not match the signed pointer.".into())
                    }
                });
            if verification.is_ok() {
                break;
            }
            fs::remove_file(&archive).map_err(download_error)?;
            if attempt == 1 {
                return Err(verification_error(verification.unwrap_err()));
            }
        }
        self.progress(
            id,
            Component::Python,
            StepStatus::Installing,
            resolved.asset.size,
            Some(resolved.asset.size),
        );
        let runtime_dir = staging.0.join("runtime");
        unpack(&archive, &runtime_dir).map_err(verification_error)?;
        self.progress(
            id,
            Component::Python,
            StepStatus::Done,
            resolved.asset.size,
            Some(resolved.asset.size),
        );
        let script_dir = staging.0.join("script");
        fs::create_dir(&script_dir).map_err(download_error)?;
        let script = script_dir.join("yt-dlp");
        let transfer = super::transport::Transfer {
            tools: self,
            client: &client,
            step: Some((id, Component::YtDlp)),
        };
        for attempt in 0..2 {
            transfer
                .download(&resolved.script_url, &script, None, 16 * 1024 * 1024)
                .await?;
            let data = fs::read(&script).map_err(download_error)?;
            self.progress(
                id,
                Component::YtDlp,
                StepStatus::Verifying,
                data.len() as u64,
                Some(data.len() as u64),
            );
            if checksum(&data, &resolved.checksum).is_ok() {
                break;
            }
            fs::remove_file(&script).map_err(download_error)?;
            if attempt == 1 {
                return Err(verification_error("The yt-dlp checksum did not match."));
            }
        }
        let size = fs::metadata(&script).map_err(download_error)?.len();
        self.progress(
            id,
            Component::YtDlp,
            StepStatus::Installing,
            size,
            Some(size),
        );
        let candidate = VideoToolsInvocation {
            interpreter: runtime_dir.join("bin/python3"),
            quickjs: runtime_dir.join("bin/qjs"),
            script,
            built_in_tools: self.config.built_in_tools.clone(),
        };
        let python_version = health(&candidate, &resolved.yt_dlp)
            .await
            .map_err(|details| Failure {
                reason: FailureReason::HealthCheck,
                details,
            })?;
        self.progress(id, Component::YtDlp, StepStatus::Done, size, Some(size));
        let mut live = self.lock();
        if !is_current(&live, id) {
            return Ok(());
        }
        let installed = InstalledPair {
            runtime: resolved.runtime,
            yt_dlp: resolved.yt_dlp,
            python_version,
        };
        let selected = super::manager::invocation(&self.config, &installed);
        place(
            &runtime_dir,
            selected.interpreter.parent().unwrap().parent().unwrap(),
        )
        .map_err(download_error)?;
        place(&script_dir, selected.script.parent().unwrap()).map_err(download_error)?;
        let mut stored = live.stored.clone();
        stored.installed = Some(installed.clone());
        stored.turned_off = false;
        persist(&self.config, &stored).map_err(download_error)?;
        live.stored = stored;
        live.snapshot.published_size = Some(resolved.asset.size + size);
        live.snapshot.state = ready(installed);
        self.publish(&mut live);
        Ok(())
    }
}

fn place(staged: &Path, destination: &Path) -> std::io::Result<()> {
    fs::create_dir_all(destination.parent().unwrap())?;
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
        if size > 256 * 1024 * 1024 {
            return Err("Expanded runtime exceeds its size limit.".into());
        }
        if !entry.unpack_in(destination).map_err(|e| e.to_string())? {
            return Err("Runtime archive path escaped staging.".into());
        }
    }
    Ok(())
}

fn exclude_from_backup(path: &Path, fixture: bool) -> Result<(), Failure> {
    if cfg!(target_os = "macos") && !fixture {
        let output = Command::new("/usr/bin/tmutil")
            .arg("addexclusion")
            .arg(path)
            .output()
            .map_err(download_error)?;
        if !output.status.success() {
            return Err(download_error(format!(
                "Could not exclude Tools from Time Machine: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
    }
    Ok(())
}
