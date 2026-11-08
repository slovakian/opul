use super::install::{Failure, download_error};
use super::*;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub(super) struct Sources {
    pub pointer: String,
    /// Each runtime asset must live under `{prefix}{runtime}/`, its immutable numbered release.
    pub runtime_release_prefix: String,
    pub repository: String,
    pub nightly_repository: String,
    pub opul_key: [u8; 32],
    pub yt_dlp_key: String,
    pub https_only: bool,
    pub exclude_from_backup: bool,
    pub environment: Arc<dyn NetworkEnvironment>,
}

impl Sources {
    pub fn production() -> Self {
        Self {
            pointer: "https://github.com/slovakian/opul/releases/download/python-runtime/python-runtime.json".into(),
            runtime_release_prefix: "https://github.com/slovakian/opul/releases/download/python-runtime-".into(),
            repository: "https://github.com/yt-dlp/yt-dlp".into(),
            nightly_repository: "https://github.com/yt-dlp/yt-dlp-nightly-builds".into(),
            opul_key: [0x06,0x54,0x04,0xda,0xa6,0xf7,0x16,0xa3,0xd2,0x21,0xb6,0xc5,0xe1,0x3a,0x44,0xd5,0xe3,0x1a,0xc9,0xd0,0xf9,0xcd,0xb9,0x59,0x0e,0xb4,0x68,0x2f,0x3d,0x4a,0x1b,0x55],
            yt_dlp_key: include_str!("keys/yt-dlp.asc").into(),
            https_only: true,
            exclude_from_backup: true,
            environment: super::transport::system_network(),
        }
    }

    pub(super) fn client(&self) -> Result<Client, Failure> {
        Client::builder()
            .https_only(self.https_only)
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(180))
            .user_agent("Opul video tools")
            .build()
            .map_err(download_error)
    }
}
