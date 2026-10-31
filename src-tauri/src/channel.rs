use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseChannel {
    Stable,
    Nightly,
}

impl ReleaseChannel {
    #[must_use]
    pub fn detect(id: &str, product_name: &str, env_var: Option<&str>) -> Self {
        if let Some(env) = env_var {
            let env_lower = env.trim().to_ascii_lowercase();
            if env_lower == "nightly" {
                return Self::Nightly;
            }
            if env_lower == "stable" {
                return Self::Stable;
            }
        }

        if id.ends_with(".nightly") || id == "dev.opul.app.nightly" {
            return Self::Nightly;
        }

        if product_name.to_ascii_lowercase().contains("nightly") {
            return Self::Nightly;
        }

        Self::Stable
    }

    #[must_use]
    pub fn app_name(self) -> &'static str {
        match self {
            Self::Stable => "Opul",
            Self::Nightly => "Opul Nightly",
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn bundle_identifier(self) -> &'static str {
        match self {
            Self::Stable => "dev.opul.app",
            Self::Nightly => "dev.opul.app.nightly",
        }
    }

    #[must_use]
    #[allow(dead_code)]
    pub fn default_updater_endpoint(self) -> &'static str {
        match self {
            Self::Stable => {
                "https://github.com/slovakian/opul/releases/latest/download/latest.json"
            }
            Self::Nightly => {
                "https://github.com/slovakian/opul/releases/download/nightly/nightly.json"
            }
        }
    }

    #[must_use]
    pub fn data_directory_name(self) -> &'static str {
        self.app_name()
    }
}

#[must_use]
pub fn resolve_data_directory(
    channel: ReleaseChannel,
    base_dir: &Path,
    env_override: Option<&OsStr>,
) -> PathBuf {
    if let Some(path) = env_override {
        return PathBuf::from(path);
    }
    base_dir.join(channel.data_directory_name())
}

#[cfg(test)]
mod tests {
    use super::*;
    use opul_core::{EngineConfig, MediaEngineConfig, OpulMachineConfig, OpulStateMachine};

    #[test]
    fn channel_detects_stable_by_default() {
        let channel = ReleaseChannel::detect("dev.opul.app", "Opul", None);
        assert_eq!(channel, ReleaseChannel::Stable);
        assert_eq!(channel.app_name(), "Opul");
        assert_eq!(channel.bundle_identifier(), "dev.opul.app");
        assert_eq!(channel.data_directory_name(), "Opul");
        assert_eq!(
            channel.default_updater_endpoint(),
            "https://github.com/slovakian/opul/releases/latest/download/latest.json"
        );
    }

    #[test]
    fn channel_detects_nightly_by_bundle_id_or_product_name() {
        let channel_id = ReleaseChannel::detect("dev.opul.app.nightly", "Opul", None);
        assert_eq!(channel_id, ReleaseChannel::Nightly);
        assert_eq!(channel_id.app_name(), "Opul Nightly");
        assert_eq!(channel_id.bundle_identifier(), "dev.opul.app.nightly");
        assert_eq!(channel_id.data_directory_name(), "Opul Nightly");
        assert_eq!(
            channel_id.default_updater_endpoint(),
            "https://github.com/slovakian/opul/releases/download/nightly/nightly.json"
        );

        let channel_name = ReleaseChannel::detect("dev.opul.app", "Opul Nightly", None);
        assert_eq!(channel_name, ReleaseChannel::Nightly);
    }

    #[test]
    fn channel_env_override_takes_precedence() {
        let nightly_env = ReleaseChannel::detect("dev.opul.app", "Opul", Some("nightly"));
        assert_eq!(nightly_env, ReleaseChannel::Nightly);

        let stable_env =
            ReleaseChannel::detect("dev.opul.app.nightly", "Opul Nightly", Some("stable"));
        assert_eq!(stable_env, ReleaseChannel::Stable);
    }

    #[test]
    fn resolves_isolated_data_directories() {
        let base = Path::new("/Users/test/Library/Application Support");
        let stable_path = resolve_data_directory(ReleaseChannel::Stable, base, None);
        let nightly_path = resolve_data_directory(ReleaseChannel::Nightly, base, None);

        assert_eq!(stable_path, base.join("Opul"));
        assert_eq!(nightly_path, base.join("Opul Nightly"));

        // Env override takes precedence
        let override_path = OsStr::new("/tmp/custom-state");
        let custom_path = resolve_data_directory(ReleaseChannel::Stable, base, Some(override_path));
        assert_eq!(custom_path, PathBuf::from("/tmp/custom-state"));
    }

    #[test]
    fn concurrent_stable_and_nightly_instances_coexist_without_lock_collision() {
        let temp = tempfile::tempdir().expect("tempdir");
        let base = temp.path();

        let stable_dir = resolve_data_directory(ReleaseChannel::Stable, base, None);
        let nightly_dir = resolve_data_directory(ReleaseChannel::Nightly, base, None);

        std::fs::create_dir_all(&stable_dir).expect("create stable dir");
        std::fs::create_dir_all(&nightly_dir).expect("create nightly dir");

        let downloads = temp.path().join("downloads");
        std::fs::create_dir_all(&downloads).expect("create downloads dir");

        let make_config = |dir: &Path| OpulMachineConfig {
            application_support_folder: dir.to_string_lossy().into_owned(),
            downloads_folder: downloads.to_string_lossy().into_owned(),
            torrent: EngineConfig {
                session_folder: dir.join("torrent-session").to_string_lossy().into_owned(),
                listen_port: 0,
                randomize_port: true,
                enable_dht: false,
                max_connections: 10,
                max_download_bps: 0,
                max_upload_bps: 0,
            },
            media: MediaEngineConfig {
                session_folder: dir.join("media-session").to_string_lossy().into_owned(),
                resources_folder: None,
                yt_dlp_path: None,
                ffmpeg_path: None,
                max_concurrent_downloads: 2,
                max_download_bps: 0,
            },
        };

        // Open Stable machine
        let stable_machine =
            OpulStateMachine::open(make_config(&stable_dir)).expect("stable machine should open");

        // Concurrently open Nightly machine
        let nightly_machine = OpulStateMachine::open(make_config(&nightly_dir))
            .expect("nightly machine should open concurrently without lock contention");

        // Attempting to open another Stable machine should fail with lock contention
        let duplicate_stable = OpulStateMachine::open(make_config(&stable_dir));
        assert!(
            duplicate_stable.is_err(),
            "second stable instance must be rejected by lock"
        );

        // Both running instances can shut down cleanly
        stable_machine.begin_shutdown().expect("shutdown stable");
        nightly_machine.begin_shutdown().expect("shutdown nightly");
    }

    #[test]
    fn nightly_manifest_parses_valid_tauri_updater_format() {
        let manifest_json = r#"{
            "version": "0.1.0-nightly.20261031.1",
            "notes": "Opul Nightly pre-release",
            "pub_date": "2026-10-31T15:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://github.com/slovakian/opul/releases/download/nightly/Opul-Nightly-arm64.app.tar.gz",
                    "signature": "sig-arm64-test"
                },
                "darwin-x86_64": {
                    "url": "https://github.com/slovakian/opul/releases/download/nightly/Opul-Nightly-x64.app.tar.gz",
                    "signature": "sig-x64-test"
                }
            }
        }"#;

        let release: tauri_plugin_updater::RemoteRelease =
            serde_json::from_str(manifest_json).expect("manifest should deserialize");

        assert_eq!(release.version.to_string(), "0.1.0-nightly.20261031.1");
        assert_eq!(
            release
                .download_url("darwin-aarch64")
                .expect("darwin-aarch64 URL")
                .as_str(),
            "https://github.com/slovakian/opul/releases/download/nightly/Opul-Nightly-arm64.app.tar.gz"
        );
        assert_eq!(
            release
                .signature("darwin-aarch64")
                .expect("darwin-aarch64 signature"),
            "sig-arm64-test"
        );

        assert_eq!(
            release
                .download_url("darwin-x86_64")
                .expect("darwin-x86_64 URL")
                .as_str(),
            "https://github.com/slovakian/opul/releases/download/nightly/Opul-Nightly-x64.app.tar.gz"
        );
        assert_eq!(
            release
                .signature("darwin-x86_64")
                .expect("darwin-x86_64 signature"),
            "sig-x64-test"
        );
    }

    #[test]
    fn stable_manifest_parses_valid_tauri_updater_format() {
        let manifest_json = r#"{
            "version": "0.2.0",
            "notes": "Opul Release 0.2.0",
            "pub_date": "2026-10-31T15:00:00Z",
            "platforms": {
                "darwin-aarch64": {
                    "url": "https://github.com/slovakian/opul/releases/download/v0.2.0/Opul-arm64.app.tar.gz",
                    "signature": "sig-arm64-test"
                },
                "darwin-x86_64": {
                    "url": "https://github.com/slovakian/opul/releases/download/v0.2.0/Opul-x64.app.tar.gz",
                    "signature": "sig-x64-test"
                }
            }
        }"#;

        let release: tauri_plugin_updater::RemoteRelease =
            serde_json::from_str(manifest_json).expect("stable manifest should deserialize");

        assert_eq!(release.version.to_string(), "0.2.0");
        assert_eq!(
            release
                .download_url("darwin-aarch64")
                .expect("darwin-aarch64 URL")
                .as_str(),
            "https://github.com/slovakian/opul/releases/download/v0.2.0/Opul-arm64.app.tar.gz"
        );
        assert_eq!(
            release
                .signature("darwin-aarch64")
                .expect("darwin-aarch64 signature"),
            "sig-arm64-test"
        );

        assert_eq!(
            release
                .download_url("darwin-x86_64")
                .expect("darwin-x86_64 URL")
                .as_str(),
            "https://github.com/slovakian/opul/releases/download/v0.2.0/Opul-x64.app.tar.gz"
        );
        assert_eq!(
            release
                .signature("darwin-x86_64")
                .expect("darwin-x86_64 signature"),
            "sig-x64-test"
        );
    }
}
