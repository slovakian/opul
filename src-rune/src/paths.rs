use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Paths {
    pub data: PathBuf,
    pub downloads: PathBuf,
    pub resources: Option<PathBuf>,
}

impl Paths {
    pub fn discover() -> Result<Self, String> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or("HOME is unavailable")?;
        let executable = std::env::current_exe().map_err(|e| e.to_string())?;
        let resources = executable
            .parent()
            .and_then(|p| p.parent())
            .map(|p| p.join("Resources"))
            .filter(|p| p.join("rune-resources.json").is_file());
        Ok(Self {
            // Rune's library is deliberately separate from Stable/Nightly and the prototypes.
            data: std::env::var_os("OPUL_RUNE_DATA_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join("Library/Application Support/Opul Rune")),
            downloads: std::env::var_os("OPUL_RUNE_DOWNLOAD_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join("Downloads")),
            resources,
        })
    }

    pub fn open(&self) -> Result<std::sync::Arc<opul_core::OpulStateMachine>, String> {
        use opul_core::{EngineConfig, MediaEngineConfig, OpulMachineConfig, OpulStateMachine};
        std::fs::create_dir_all(&self.downloads).map_err(|e| e.to_string())?;
        let tools =
            opul_core::video_tools::VideoTools::open(opul_core::video_tools::VideoToolsConfig {
                root: self.data.join("Tools"),
                opul_version: env!("CARGO_PKG_VERSION").into(),
                nightly_available: false,
            })?;
        OpulStateMachine::open(OpulMachineConfig {
            application_support_folder: self.data.to_string_lossy().into_owned(),
            downloads_folder: self.downloads.to_string_lossy().into_owned(),
            torrent: EngineConfig {
                session_folder: self
                    .data
                    .join("torrent-session")
                    .to_string_lossy()
                    .into_owned(),
                listen_port: 0,
                randomize_port: true,
                enable_dht: false,
                max_connections: 128,
                max_download_bps: 0,
                max_upload_bps: 0,
            },
            media: MediaEngineConfig {
                session_folder: self
                    .data
                    .join("media-session")
                    .to_string_lossy()
                    .into_owned(),
                resources_folder: self
                    .resources
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned()),
                video_tools: tools,
                max_concurrent_downloads: 3,
                max_download_bps: 0,
            },
        })
        .map_err(|e| e.to_string())
    }
}
