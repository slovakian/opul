use super::*;
use ed25519_dalek::{Signature, VerifyingKey};
use pgp::composed::{Deserializable, DetachedSignature, SignedPublicKey};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

pub(super) fn ed25519(bytes: &[u8], signature: &[u8], key: &[u8; 32]) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(key).map_err(|e| e.to_string())?;
    let signature = Signature::from_slice(signature).map_err(|e| e.to_string())?;
    key.verify_strict(bytes, &signature)
        .map_err(|e| format!("Opul signature: {e}"))
}

pub(super) fn openpgp(bytes: &[u8], signature: &[u8], key: &str) -> Result<(), String> {
    let (key, _) = SignedPublicKey::from_string(key).map_err(|e| e.to_string())?;
    key.verify()
        .map_err(|e| format!("yt-dlp public key: {e}"))?;
    let signature =
        DetachedSignature::from_bytes(Cursor::new(signature)).map_err(|e| e.to_string())?;
    if signature.verify(&key, bytes).is_ok() {
        return Ok(());
    }
    for subkey in &key.public_subkeys {
        if signature.verify(&subkey.key, bytes).is_ok() {
            return Ok(());
        }
    }
    Err("yt-dlp checksum signature did not match the embedded key.".into())
}

pub(super) fn checksum(bytes: &[u8], expected: &str) -> Result<(), String> {
    if hex::encode(Sha256::digest(bytes)) == expected.to_ascii_lowercase() {
        Ok(())
    } else {
        Err("The SHA-256 checksum did not match.".into())
    }
}

pub(super) fn safe_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 100
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_')
        && version != "."
        && version != ".."
}

fn health_output(output: &std::process::Output, expected: &str) -> Result<String, String> {
    let text = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let version_debug = text.lines().any(|line| {
        line.starts_with("[debug] yt-dlp version ")
            && line
                .split_whitespace()
                .any(|word| word.split('@').next_back() == Some(expected))
    });
    let lower = text.to_ascii_lowercase();
    let deprecated = lower.contains("deprecat") && lower.contains("python");
    let curl = text
        .lines()
        .filter_map(|line| line.strip_prefix("[debug] Request Handlers: "))
        .any(|line| line.split(',').any(|handler| handler.trim() == "curl_cffi"));
    let quickjs = text
        .lines()
        .filter_map(|line| line.strip_prefix("[debug] JS runtimes: "))
        .any(|line| {
            line.split(',').any(|runtime| {
                runtime.trim().starts_with("quickjs-") && !runtime.contains("(unavailable)")
            })
        });
    let python = text.lines().find_map(|line| {
        line.strip_prefix("[debug] Python ")
            .and_then(|s| s.split_whitespace().next())
    });
    if output.status.success()
        && version_debug
        && !deprecated
        && curl
        && quickjs
        && let Some(python) = python
    {
        return Ok(python.to_string());
    }
    Err(format!(
        "The video tools did not pass their health check. Expected yt-dlp {expected}, supported curl_cffi and QuickJS, without a Python deprecation warning.\n{text}"
    ))
}

#[derive(Debug, thiserror::Error)]
pub(super) enum HealthFailure {
    #[error("{0}")]
    Incompatible(String),
    #[error("{0}")]
    Execution(String),
}

pub(super) async fn health(
    invocation: &VideoToolsInvocation,
    expected: &str,
) -> Result<String, HealthFailure> {
    let mut version = tokio::process::Command::from(invocation.command());
    version.arg("--version").kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), version.output())
        .await
        .map_err(|_| HealthFailure::Execution("Video tools version check timed out.".into()))?
        .map_err(|e| HealthFailure::Execution(e.to_string()))?;
    if !output.status.success() {
        return Err(HealthFailure::Execution(format!(
            "yt-dlp version check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    if String::from_utf8_lossy(&output.stdout).trim() != expected
        || String::from_utf8_lossy(&output.stderr)
            .to_ascii_lowercase()
            .contains("deprecat")
    {
        return Err(HealthFailure::Incompatible(format!(
            "yt-dlp did not report the expected version {expected}: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    // --version exits before yt-dlp initializes its request handlers and JS runtimes.
    let mut command = tokio::process::Command::from(invocation.command());
    command
        .args(["--verbose", "--list-impersonate-targets"])
        .kill_on_drop(true);
    let output = tokio::time::timeout(Duration::from_secs(30), command.output())
        .await
        .map_err(|_| HealthFailure::Execution("Video tools health check timed out.".into()))?
        .map_err(|e| HealthFailure::Execution(e.to_string()))?;
    if !output.status.success() {
        return Err(HealthFailure::Execution(format!(
            "yt-dlp health check failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    health_output(&output, expected).map_err(HealthFailure::Incompatible)
}

pub(super) fn validate_installed(
    config: &VideoToolsConfig,
    pair: &InstalledPair,
) -> Result<(), String> {
    if pair.runtime == 0 || !safe_version(&pair.yt_dlp) {
        return Err("Invalid installed video-tools versions.".into());
    }
    let invocation = super::manager::invocation(config, pair);
    for path in [
        &invocation.interpreter,
        &invocation.script,
        &invocation.quickjs,
    ] {
        let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if !metadata.is_file()
            || (path != &invocation.script && metadata.permissions().mode() & 0o111 == 0)
        {
            return Err(format!("{} is not runnable.", path.display()));
        }
    }
    // Startup validation runs on a dedicated thread, outside the shell's async runtime.
    let expected = pair.yt_dlp.clone();
    std::thread::spawn(move || super::manager::runtime().block_on(health(&invocation, &expected)))
        .join()
        .map_err(|_| "Video tools startup check failed.".to_string())?
        .map(|_| ())
        .map_err(|error| error.to_string())
}
