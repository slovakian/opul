use librqbit::api::TorrentIdOrHash;
use librqbit::limits::LimitsConfig;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, DhtSessionConfig, ListenerMode,
    ListenerOptions, Session, SessionOptions, SessionPersistenceConfig, TorrentStatsState,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::net::{Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::CoreError;
use crate::destination::{DownloadContentLayout, resolve_download_destination};
use crate::meta::{
    FilePriority, FileTransferStatus, TorrentMeta, TorrentState, TorrentTransferSnapshot,
    parse_magnet_uri, parse_torrent_bytes,
};
use crate::persistence::{LoadedState, load_state, protect_json_file, save_state};

#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub session_folder: String,
    pub listen_port: u16,
    pub randomize_port: bool,
    pub enable_dht: bool,
    pub max_connections: u32,
    pub max_download_bps: u64,
    pub max_upload_bps: u64,
}

pub(crate) struct TorrentFileInput {
    pub(crate) download_id: String,
    pub(crate) attempt: u32,
    pub(crate) bytes: Vec<u8>,
    pub(crate) source: String,
    pub(crate) save_folder: String,
    pub(crate) content_layout: DownloadContentLayout,
    pub(crate) seed_ratio_limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTorrent {
    #[serde(default)]
    download_id: Option<String>,
    #[serde(default)]
    attempt: u32,
    source: String,
    name: String,
    save_folder: String,
    #[serde(default)]
    content_folder: Option<String>,
    seed_ratio_limit: Option<f64>,
    stopped_seeding: bool,
    user_paused: bool,
    priorities: HashMap<u32, String>,
    comment: Option<String>,
    last_uploaded: u64,
    last_downloaded: u64,
    error: Option<String>,
}

impl StoredTorrent {
    fn fresh(
        source: String,
        name: String,
        save_folder: String,
        content_folder: Option<String>,
        seed_ratio_limit: Option<f64>,
        comment: Option<String>,
    ) -> Self {
        Self {
            download_id: None,
            attempt: 0,
            source,
            name,
            save_folder,
            content_folder,
            seed_ratio_limit,
            stopped_seeding: false,
            user_paused: false,
            priorities: HashMap::new(),
            comment,
            last_uploaded: 0,
            last_downloaded: 0,
            error: None,
        }
    }
}

pub struct TorrentEngine {
    runtime: tokio::runtime::Runtime,
    session: Arc<Session>,
    extras_path: PathBuf,
    extras: Arc<Mutex<HashMap<String, StoredTorrent>>>,
    ignored: Arc<Mutex<HashSet<String>>>,
    recovery_notice: Option<String>,
}

impl TorrentEngine {
    pub fn new(config: EngineConfig) -> Result<Arc<Self>, CoreError> {
        let session_folder = PathBuf::from(&config.session_folder);
        fs::create_dir_all(&session_folder).map_err(|err| CoreError::Engine {
            reason: format!("could not create the torrent session folder: {err}"),
        })?;
        let default_output = session_folder.join("downloads");
        fs::create_dir_all(&default_output).map_err(|err| CoreError::Engine {
            reason: format!("could not create the default torrent folder: {err}"),
        })?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|err| CoreError::Engine {
                reason: format!("could not start the torrent runtime: {err}"),
            })?;

        let mut recovery_notices = Vec::new();
        if let Some(notice) = protect_librqbit_session(&session_folder)? {
            recovery_notices.push(notice);
        }
        let session = open_session(&runtime, &config, &default_output)?;
        let extras_path = session_folder.join("opul-torrent-extras.json");
        let loaded = load_extras(&extras_path).map_err(|reason| CoreError::Engine { reason })?;
        if let Some(notice) = loaded.recovery_notice {
            recovery_notices.push(notice);
        }
        Ok(Arc::new(Self {
            runtime,
            session,
            extras_path,
            extras: Arc::new(Mutex::new(loaded.items)),
            ignored: Arc::new(Mutex::new(HashSet::new())),
            recovery_notice: (!recovery_notices.is_empty()).then(|| recovery_notices.join(" ")),
        }))
    }

    pub fn recovery_notice(&self) -> Option<String> {
        self.recovery_notice.clone()
    }

    pub fn add_magnet(
        &self,
        uri: String,
        save_folder: String,
        content_layout: DownloadContentLayout,
        seed_ratio_limit: Option<f64>,
    ) -> Result<String, CoreError> {
        let meta = parse_magnet_uri(&uri)?;
        let name = meta.name.clone().unwrap_or_else(|| meta.info_hash.clone());
        self.remember(
            StoredTorrent::fresh(
                uri.clone(),
                name,
                save_folder.clone(),
                None,
                seed_ratio_limit,
                None,
            ),
            &meta.info_hash,
        );
        let session = Arc::clone(&self.session);
        let hash = meta.info_hash.clone();
        let trackers = meta.trackers;
        let extras = Arc::clone(&self.extras);
        let extras_path = self.extras_path.clone();
        let ignored = Arc::clone(&self.ignored);
        let source = uri.clone();
        self.runtime.spawn(async move {
            match add_url_torrent(&session, uri, &save_folder, content_layout, trackers).await {
                Ok(prepared) => {
                    update_prepared_stored(&extras, &extras_path, &hash, &hash, &prepared);
                    let was_removed = ignored
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .iter()
                        .any(|value| value == &source || value == &hash);
                    if was_removed && let Ok(id) = TorrentIdOrHash::parse(&hash) {
                        let _ = session.delete(id, false).await;
                    }
                }
                Err(err) => fail_stored(&extras, &extras_path, &hash, err.to_string()),
            }
        });
        Ok(meta.info_hash)
    }

    pub fn add_torrent_file(
        &self,
        bytes: Vec<u8>,
        source: String,
        save_folder: String,
        content_layout: DownloadContentLayout,
        seed_ratio_limit: Option<f64>,
    ) -> Result<TorrentMeta, CoreError> {
        let meta = parse_torrent_bytes(&bytes)?;
        let destination = resolve_download_destination(
            Path::new(&save_folder),
            &meta.name,
            content_layout,
            meta.is_multi_file,
        );
        let output_folder = destination.output_folder.to_string_lossy().into_owned();
        let content_folder = destination
            .content_folder
            .map(|path| path.to_string_lossy().into_owned());
        let source = if source.is_empty() {
            format!("magnet:?xt=urn:btih:{}", meta.info_hash)
        } else {
            source
        };
        let source_for_removal = source.clone();
        self.remember(
            StoredTorrent::fresh(
                source,
                meta.name.clone(),
                output_folder.clone(),
                content_folder,
                seed_ratio_limit,
                meta.comment.clone(),
            ),
            &meta.info_hash,
        );
        let session = Arc::clone(&self.session);
        let hash = meta.info_hash.clone();
        let extras = Arc::clone(&self.extras);
        let extras_path = self.extras_path.clone();
        let ignored = Arc::clone(&self.ignored);
        self.runtime.spawn(async move {
            let opts = add_options(output_folder, Vec::new());
            match session
                .add_torrent(AddTorrent::from_bytes(bytes), Some(opts))
                .await
            {
                Ok(_) => {
                    let was_removed = ignored
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .iter()
                        .any(|value| value == &source_for_removal || value == &hash);
                    if was_removed && let Ok(id) = TorrentIdOrHash::parse(&hash) {
                        let _ = session.delete(id, false).await;
                    }
                }
                Err(err) => fail_stored(&extras, &extras_path, &hash, err.to_string()),
            }
        });
        Ok(meta)
    }

    /// Fetch a `.torrent` from an http(s) URL. The info-hash arrives on a later snapshot.
    pub fn add_remote_torrent(
        &self,
        url: String,
        save_folder: String,
        content_layout: DownloadContentLayout,
        seed_ratio_limit: Option<f64>,
    ) -> Result<(), CoreError> {
        let key = format!("url:{url}");
        self.remember(
            StoredTorrent::fresh(
                url.clone(),
                url.clone(),
                save_folder.clone(),
                None,
                seed_ratio_limit,
                None,
            ),
            &key,
        );
        let session = Arc::clone(&self.session);
        let extras = Arc::clone(&self.extras);
        let extras_path = self.extras_path.clone();
        let ignored = Arc::clone(&self.ignored);
        let source = url.clone();
        self.runtime.spawn(async move {
            match add_url_torrent(&session, url, &save_folder, content_layout, Vec::new()).await {
                Ok(prepared) => {
                    let hash = prepared.info_hash.clone();
                    update_prepared_stored(&extras, &extras_path, &key, &hash, &prepared);
                    let was_removed = ignored
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .contains(&source);
                    if was_removed && let Ok(id) = TorrentIdOrHash::parse(&hash) {
                        let _ = session.delete(id, false).await;
                    }
                }
                Err(err) => fail_stored(&extras, &extras_path, &key, err.to_string()),
            }
        });
        Ok(())
    }

    pub fn pause(&self, info_hash: String) {
        self.update_stored(&info_hash, |stored| {
            stored.user_paused = true;
            stored.stopped_seeding = false;
        });
        self.spawn_with_handle(info_hash, |session, handle| async move {
            let _ = session.pause(&handle).await;
        });
    }

    pub fn resume(&self, info_hash: String) {
        self.update_stored(&info_hash, |stored| {
            stored.user_paused = false;
            stored.stopped_seeding = false;
            stored.error = None;
        });
        self.spawn_with_handle(info_hash, |session, handle| async move {
            let _ = session.unpause(&handle).await;
        });
    }

    pub fn stop_seeding(&self, info_hash: String) {
        self.update_stored(&info_hash, |stored| {
            stored.stopped_seeding = true;
            stored.user_paused = false;
        });
        self.spawn_with_handle(info_hash, |session, handle| async move {
            let _ = session.pause(&handle).await;
        });
    }

    pub fn remove(&self, info_hash: String) {
        let key = info_hash.to_ascii_lowercase();
        self.ignored
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .insert(key.clone());
        {
            let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
            extras.remove(&key);
            self.save_extras(&extras);
        }
        let Ok(id) = TorrentIdOrHash::parse(&key) else {
            return;
        };
        let session = Arc::clone(&self.session);
        self.runtime.spawn(async move {
            let _ = session.delete(id, false).await;
        });
    }

    pub fn remove_source(&self, source: String) {
        let keys = {
            let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
            let keys = extras
                .iter()
                .filter(|(_, stored)| stored.source == source)
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>();
            for key in &keys {
                extras.remove(key);
            }
            self.save_extras(&extras);
            keys
        };
        {
            let mut ignored = self.ignored.lock().unwrap_or_else(|p| p.into_inner());
            ignored.insert(source);
            ignored.extend(keys.iter().cloned());
        }
        for key in keys {
            let Ok(id) = TorrentIdOrHash::parse(&key) else {
                continue;
            };
            let session = Arc::clone(&self.session);
            self.runtime.spawn(async move {
                let _ = session.delete(id, false).await;
            });
        }
    }

    pub fn set_seed_ratio_limit(&self, info_hash: String, limit: Option<f64>) {
        self.update_stored(&info_hash, |stored| {
            stored.seed_ratio_limit = limit.filter(|value| *value > 0.0);
            if stored.seed_ratio_limit.is_none() {
                stored.stopped_seeding = false;
            }
        });
    }

    pub fn set_file_priorities(&self, info_hash: String, priorities: Vec<FilePriority>) {
        let included: HashSet<usize> = priorities
            .iter()
            .enumerate()
            .filter(|(_, priority)| priority.included())
            .map(|(index, _)| index)
            .collect();
        self.update_stored(&info_hash, |stored| {
            stored.priorities = priorities
                .iter()
                .enumerate()
                .map(|(index, priority)| (index as u32, priority.as_label().to_string()))
                .collect();
        });
        self.spawn_with_handle(info_hash, |session, handle| async move {
            let _ = session.update_only_files(&handle, &included).await;
        });
    }

    pub fn set_speed_limits(&self, download_bps: u64, upload_bps: u64) {
        self.session
            .ratelimits
            .set_download_bps(nonzero_bps(download_bps));
        self.session
            .ratelimits
            .set_upload_bps(nonzero_bps(upload_bps));
    }

    pub fn snapshots(&self) -> Vec<TorrentTransferSnapshot> {
        self.adopt_pending();
        self.enforce_seed_ratios();
        let handles = self.session.with_torrents(|torrents| {
            torrents
                .map(|(_, handle)| Arc::clone(handle))
                .collect::<Vec<_>>()
        });
        let extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        let mut seen = HashSet::new();
        let mut snaps = Vec::new();
        for handle in &handles {
            let hash = handle.info_hash().as_string().to_ascii_lowercase();
            let Some(stored) = extras.get(&hash) else {
                continue;
            };
            seen.insert(hash.clone());
            snaps.push(snapshot_of(handle, stored, &hash));
        }
        for (key, stored) in extras.iter() {
            if seen.contains(key) {
                continue;
            }
            snaps.push(pending_snapshot(key, stored));
        }
        snaps
    }
}

impl TorrentEngine {
    pub(crate) fn new_quiescent(config: EngineConfig) -> Result<Arc<Self>, CoreError> {
        let engine = Self::new(config)?;
        engine.stop_all();
        Ok(engine)
    }

    pub(crate) fn stop_all(&self) {
        let keys = self
            .extras
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .keys()
            .filter(|key| !key.starts_with("url:"))
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            self.pause(key);
        }
    }

    pub(crate) fn ensure_magnet(
        &self,
        download_id: &str,
        attempt: u32,
        uri: String,
        save_folder: String,
        content_layout: DownloadContentLayout,
        seed_ratio_limit: Option<f64>,
    ) -> Result<String, CoreError> {
        let meta = parse_magnet_uri(&uri)?;
        if self.claim_existing(&meta.info_hash, download_id, attempt, &uri)? {
            return Ok(meta.info_hash);
        }
        let hash = self.add_magnet(uri, save_folder, content_layout, seed_ratio_limit)?;
        self.set_identity(&hash, download_id, attempt);
        Ok(hash)
    }

    pub(crate) fn ensure_torrent_file(
        &self,
        input: TorrentFileInput,
    ) -> Result<TorrentMeta, CoreError> {
        let TorrentFileInput {
            download_id,
            attempt,
            bytes,
            source,
            save_folder,
            content_layout,
            seed_ratio_limit,
        } = input;
        let parsed = parse_torrent_bytes(&bytes)?;
        if self.claim_existing(&parsed.info_hash, &download_id, attempt, &source)? {
            return Ok(parsed);
        }
        let meta =
            self.add_torrent_file(bytes, source, save_folder, content_layout, seed_ratio_limit)?;
        self.set_identity(&meta.info_hash, &download_id, attempt);
        Ok(meta)
    }

    pub(crate) fn ensure_remote_torrent(
        &self,
        download_id: &str,
        attempt: u32,
        url: String,
        save_folder: String,
        content_layout: DownloadContentLayout,
        seed_ratio_limit: Option<f64>,
    ) -> Result<(), CoreError> {
        let key = format!("url:{url}");
        if self.claim_existing(&key, download_id, attempt, &url)? {
            return Ok(());
        }
        self.add_remote_torrent(url, save_folder, content_layout, seed_ratio_limit)?;
        self.set_identity(&key, download_id, attempt);
        Ok(())
    }

    pub(crate) fn remove_download(&self, download_id: &str, attempt: u32) {
        let keys = {
            let extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
            extras
                .iter()
                .filter(|(_, stored)| {
                    stored.download_id.as_deref() == Some(download_id) && stored.attempt == attempt
                })
                .map(|(key, _)| key.clone())
                .collect::<Vec<_>>()
        };
        for key in keys {
            self.remove(key);
        }
    }

    /// Everything this Download's torrents write to disk: the content folder when there is
    /// one, otherwise each file of the torrent, finished or not.
    pub(crate) fn owned_paths(&self, download_id: &str) -> Vec<PathBuf> {
        let extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        let mut paths = Vec::new();
        for (key, stored) in extras.iter() {
            if stored.download_id.as_deref() != Some(download_id) {
                continue;
            }
            if let Some(content_folder) = &stored.content_folder {
                paths.push(PathBuf::from(content_folder));
                continue;
            }
            let Some(handle) = TorrentIdOrHash::parse(key)
                .ok()
                .and_then(|id| self.session.get(id))
            else {
                continue;
            };
            let folder = handle.output_folder().to_path_buf();
            let _ = handle.with_metadata(|metadata| {
                for details in metadata.info.iter_file_details() {
                    paths.push(folder.join(details.filename.to_pathbuf()));
                }
            });
        }
        paths
    }

    pub(crate) fn binding_for(&self, download_id: &str, attempt: u32) -> Option<String> {
        self.extras
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .iter()
            .find(|(_, stored)| {
                stored.download_id.as_deref() == Some(download_id) && stored.attempt == attempt
            })
            .map(|(key, _)| key.clone())
    }

    fn claim_existing(
        &self,
        key: &str,
        download_id: &str,
        attempt: u32,
        source: &str,
    ) -> Result<bool, CoreError> {
        let normalized = key.to_ascii_lowercase();
        let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        let existing_key = if extras.contains_key(&normalized) {
            Some(normalized)
        } else {
            extras
                .iter()
                .find(|(_, stored)| stored.source == source)
                .map(|(key, _)| key.clone())
        };
        let Some(existing_key) = existing_key else {
            return Ok(false);
        };
        let stored = extras
            .get_mut(&existing_key)
            .expect("key came from the map");
        match stored.download_id.as_deref() {
            Some(existing) if existing != download_id || stored.attempt != attempt => {
                Err(CoreError::Engine {
                    reason: format!("this torrent is already bound to download {existing}"),
                })
            }
            _ => {
                stored.download_id = Some(download_id.to_string());
                stored.attempt = attempt;
                self.save_extras(&extras);
                Ok(true)
            }
        }
    }

    fn set_identity(&self, key: &str, download_id: &str, attempt: u32) {
        self.update_stored(key, |stored| {
            stored.download_id = Some(download_id.to_string());
            stored.attempt = attempt;
        });
    }

    fn remember(&self, stored: StoredTorrent, info_hash: &str) {
        {
            let mut ignored = self.ignored.lock().unwrap_or_else(|p| p.into_inner());
            ignored.remove(&stored.source);
            ignored.remove(&info_hash.to_ascii_lowercase());
        }
        let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        extras.insert(info_hash.to_ascii_lowercase(), stored);
        self.save_extras(&extras);
    }

    fn update_stored(&self, info_hash: &str, body: impl FnOnce(&mut StoredTorrent)) {
        let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        let Some(stored) = extras.get_mut(&info_hash.to_ascii_lowercase()) else {
            return;
        };
        body(stored);
        self.save_extras(&extras);
    }

    fn save_extras(&self, extras: &HashMap<String, StoredTorrent>) {
        write_extras(&self.extras_path, extras);
    }

    fn adopt_pending(&self) {
        let handles = self.session.with_torrents(|torrents| {
            torrents
                .map(|(_, handle)| Arc::clone(handle))
                .collect::<Vec<_>>()
        });
        let ignored = self
            .ignored
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .clone();
        let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
        let mut changed = false;
        for handle in &handles {
            let hash = handle.info_hash().as_string().to_ascii_lowercase();
            if extras.contains_key(&hash) || ignored.contains(&hash) {
                continue;
            }
            // A remote .torrent waits under `url:` until its info-hash is known.
            // Claim that placeholder so the Download the person added is not duplicated.
            let placeholder = extras
                .iter()
                .find(|(key, stored)| key.starts_with("url:") && stored.error.is_none())
                .map(|(key, _)| key.clone());
            if let Some(placeholder) = placeholder
                && let Some(mut stored) = extras.remove(&placeholder)
            {
                if stored.name == stored.source
                    && let Some(name) = handle.name()
                {
                    stored.name = name;
                }
                extras.insert(hash.clone(), stored);
                changed = true;
                continue;
            }
            let name = handle.name().unwrap_or_else(|| hash.clone());
            let save_folder = handle.output_folder().to_string_lossy().into_owned();
            extras.insert(
                hash.clone(),
                StoredTorrent::fresh(
                    format!("magnet:?xt=urn:btih:{hash}"),
                    name,
                    save_folder,
                    None,
                    None,
                    None,
                ),
            );
            changed = true;
        }
        if changed {
            self.save_extras(&extras);
        }
    }

    fn enforce_seed_ratios(&self) {
        let mut to_stop = Vec::new();
        {
            let handles = self.session.with_torrents(|torrents| {
                torrents
                    .map(|(_, handle)| Arc::clone(handle))
                    .collect::<Vec<_>>()
            });
            let mut extras = self.extras.lock().unwrap_or_else(|p| p.into_inner());
            let mut changed = false;
            for handle in &handles {
                let hash = handle.info_hash().as_string().to_ascii_lowercase();
                let Some(stored) = extras.get_mut(&hash) else {
                    continue;
                };
                let stats = handle.stats();
                if stats.uploaded_bytes > stored.last_uploaded {
                    stored.last_uploaded = stats.uploaded_bytes;
                    changed = true;
                }
                if stats.progress_bytes > stored.last_downloaded {
                    stored.last_downloaded = stats.progress_bytes;
                    changed = true;
                }
                let limit = stored.seed_ratio_limit;
                if stored.stopped_seeding || stored.user_paused || !stats.finished {
                    continue;
                }
                if let Some(limit) = limit {
                    let downloaded = stored.last_downloaded.max(stats.progress_bytes);
                    if downloaded > 0 {
                        let ratio = stored.last_uploaded as f64 / downloaded as f64;
                        if ratio >= limit {
                            stored.stopped_seeding = true;
                            changed = true;
                            to_stop.push(hash);
                        }
                    }
                }
            }
            if changed {
                self.save_extras(&extras);
            }
        }
        for hash in to_stop {
            self.spawn_with_handle(hash, |session, handle| async move {
                let _ = session.pause(&handle).await;
            });
        }
    }

    fn spawn_with_handle<F, Fut>(&self, info_hash: String, body: F)
    where
        F: FnOnce(Arc<Session>, Arc<librqbit::ManagedTorrent>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let Ok(id) = TorrentIdOrHash::parse(&info_hash) else {
            return;
        };
        let Some(handle) = self.session.get(id) else {
            return;
        };
        let session = Arc::clone(&self.session);
        self.runtime.spawn(async move {
            body(session, handle).await;
        });
    }
}

fn fail_stored(
    extras: &Mutex<HashMap<String, StoredTorrent>>,
    path: &Path,
    key: &str,
    err: String,
) {
    let mut map = extras
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(stored) = map.get_mut(key) {
        stored.error = Some(err);
    }
    write_extras(path, &map);
}

fn write_extras(path: &Path, extras: &HashMap<String, StoredTorrent>) {
    if let Err(error) = save_state(path, extras, |_| Ok(())) {
        eprintln!("Warning: could not save torrent extras: {error}");
    }
}

struct PreparedTorrent {
    info_hash: String,
    name: String,
    save_folder: String,
    content_folder: Option<String>,
    comment: Option<String>,
}

async fn add_url_torrent(
    session: &Arc<Session>,
    url: String,
    save_folder: &str,
    content_layout: DownloadContentLayout,
    trackers: Vec<String>,
) -> Result<PreparedTorrent, String> {
    let mut inspect_options = add_options(save_folder.to_string(), trackers.clone());
    inspect_options.list_only = true;
    let response = session
        .add_torrent(AddTorrent::from_url(url), Some(inspect_options))
        .await
        .map_err(|error| error.to_string())?;
    let listed = match response {
        AddTorrentResponse::ListOnly(listed) => listed,
        AddTorrentResponse::AlreadyManaged(_, handle) | AddTorrentResponse::Added(_, handle) => {
            return Ok(PreparedTorrent {
                info_hash: handle.info_hash().as_string(),
                name: handle.name().unwrap_or_else(|| "Torrent".to_string()),
                save_folder: handle.output_folder().to_string_lossy().into_owned(),
                content_folder: None,
                comment: None,
            });
        }
    };
    let meta = parse_torrent_bytes(&listed.torrent_bytes).map_err(|error| error.to_string())?;
    let destination = resolve_download_destination(
        Path::new(save_folder),
        &meta.name,
        content_layout,
        meta.is_multi_file,
    );
    let output_folder = destination.output_folder.to_string_lossy().into_owned();
    let content_folder = destination
        .content_folder
        .map(|path| path.to_string_lossy().into_owned());
    let mut options = add_options(output_folder.clone(), trackers);
    if !listed.seen_peers.is_empty() {
        options.initial_peers = Some(listed.seen_peers);
    }
    session
        .add_torrent(AddTorrent::from_bytes(listed.torrent_bytes), Some(options))
        .await
        .map_err(|error| error.to_string())?;
    Ok(PreparedTorrent {
        info_hash: meta.info_hash,
        name: meta.name,
        save_folder: output_folder,
        content_folder,
        comment: meta.comment,
    })
}

fn update_prepared_stored(
    extras: &Mutex<HashMap<String, StoredTorrent>>,
    path: &Path,
    from: &str,
    info_hash: &str,
    prepared: &PreparedTorrent,
) {
    let mut map = extras
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(mut stored) = map.remove(&from.to_ascii_lowercase()) {
        stored.name = prepared.name.clone();
        stored.save_folder = prepared.save_folder.clone();
        stored.content_folder = prepared.content_folder.clone();
        stored.comment = prepared.comment.clone();
        stored.error = None;
        map.insert(info_hash.to_ascii_lowercase(), stored);
        write_extras(path, &map);
    }
}

fn add_options(save_folder: String, trackers: Vec<String>) -> AddTorrentOptions {
    AddTorrentOptions {
        output_folder: Some(save_folder),
        overwrite: true,
        trackers: if trackers.is_empty() {
            None
        } else {
            Some(trackers)
        },
        ..Default::default()
    }
}

fn nonzero_bps(bps: u64) -> Option<NonZeroU32> {
    let bps = u32::try_from(bps).unwrap_or(u32::MAX);
    NonZeroU32::new(bps)
}

fn open_session(
    runtime: &tokio::runtime::Runtime,
    config: &EngineConfig,
    default_output: &Path,
) -> Result<Arc<Session>, CoreError> {
    let port = if config.randomize_port {
        0
    } else {
        config.listen_port
    };
    match start_session(runtime, config, default_output, port) {
        Ok(session) => Ok(session),
        Err(err) if port != 0 => start_session(runtime, config, default_output, 0).or(Err(err)),
        Err(err) => Err(err),
    }
}

fn start_session(
    runtime: &tokio::runtime::Runtime,
    config: &EngineConfig,
    default_output: &Path,
    port: u16,
) -> Result<Arc<Session>, CoreError> {
    let listen = ListenerOptions {
        mode: ListenerMode::TcpAndUtp,
        listen_addr: SocketAddr::from((Ipv6Addr::UNSPECIFIED, port)),
        enable_upnp_port_forwarding: false,
        ..Default::default()
    };
    let dht = if config.enable_dht {
        Some(DhtSessionConfig {
            port: if port == 0 { None } else { Some(port) },
            persistence: None,
            bootstrap_addrs: None,
        })
    } else {
        None
    };
    let peer_limit = if config.max_connections == 0 {
        None
    } else {
        Some(config.max_connections as usize)
    };
    let opts = SessionOptions {
        dht,
        fastresume: true,
        persistence: Some(SessionPersistenceConfig::Json {
            folder: Some(PathBuf::from(&config.session_folder)),
        }),
        listen: Some(listen),
        ratelimits: LimitsConfig {
            download_bps: nonzero_bps(config.max_download_bps),
            upload_bps: nonzero_bps(config.max_upload_bps),
        },
        peer_limit,
        client_name_and_version: Some("Opul 0.1.0".to_string()),
        ..Default::default()
    };
    runtime
        .block_on(Session::new_with_opts(default_output.to_path_buf(), opts))
        .map_err(|err| CoreError::Engine {
            reason: format!("could not open the torrent session: {err:#}"),
        })
}

fn load_extras(path: &Path) -> Result<LoadedState<HashMap<String, StoredTorrent>>, String> {
    load_state(path, "torrent metadata", |_| Ok(()))
}

fn protect_librqbit_session(session_folder: &Path) -> Result<Option<String>, CoreError> {
    let path = session_folder.join("session.json");
    protect_json_file(
        &path,
        "torrent session",
        |value| {
            value
                .get("torrents")
                .is_some_and(serde_json::Value::is_object)
        },
        br#"{"torrents":{}}"#,
    )
    .map_err(|reason| CoreError::Engine { reason })
}

fn pending_snapshot(key: &str, stored: &StoredTorrent) -> TorrentTransferSnapshot {
    let info_hash = if key.len() == 40 && key.chars().all(|c| c.is_ascii_hexdigit()) {
        key.to_string()
    } else {
        String::new()
    };
    let state = if stored.error.is_some() {
        TorrentState::Failed
    } else {
        TorrentState::Fetching
    };
    TorrentTransferSnapshot {
        download_id: stored.download_id.clone(),
        attempt: stored.attempt,
        info_hash,
        source: stored.source.clone(),
        name: stored.name.clone(),
        save_folder: stored.save_folder.clone(),
        content_folder: stored.content_folder.clone(),
        state,
        total_size: 0,
        selected_size: 0,
        bytes_downloaded: stored.last_downloaded,
        bytes_uploaded: stored.last_uploaded,
        download_rate: 0,
        upload_rate: 0,
        peers: 0,
        seeds: 0,
        seed_ratio: 0.0,
        seed_ratio_limit: stored.seed_ratio_limit,
        progress: 0.0,
        eta_seconds: None,
        piece_length: 0,
        piece_count: 0,
        comment: stored.comment.clone(),
        trackers: Vec::new(),
        files: Vec::new(),
        verifying: false,
        error_message: stored.error.clone(),
    }
}

fn snapshot_of(
    handle: &librqbit::ManagedTorrent,
    stored: &StoredTorrent,
    hash: &str,
) -> TorrentTransferSnapshot {
    let stats = handle.stats();
    let downloaded = stored.last_downloaded.max(stats.progress_bytes);
    let uploaded = stored.last_uploaded.max(stats.uploaded_bytes);
    let ratio = if downloaded == 0 {
        0.0
    } else {
        uploaded as f64 / downloaded as f64
    };
    let meta = handle.with_metadata(|metadata| {
        let lengths = metadata.info.lengths();
        let progress = stats.file_progress.clone();
        let only = handle.only_files();
        let files = metadata
            .info
            .iter_file_details()
            .enumerate()
            .map(|(index, details)| {
                file_status(
                    stored,
                    &progress,
                    &only,
                    index,
                    details.filename.to_string(),
                    details.len,
                )
            })
            .collect();
        (
            lengths.total_length(),
            lengths.default_piece_length() as u64,
            lengths.total_pieces(),
            files,
        )
    });
    let (total_size, piece_length, piece_count, files) = meta.unwrap_or((0, 0, 0, Vec::new()));
    let selected_size = if files.is_empty() {
        total_size
    } else {
        files
            .iter()
            .filter(|file| file.is_included)
            .map(|file| file.size)
            .sum()
    };
    let selected_size = if selected_size == 0 {
        total_size
    } else {
        selected_size
    };
    let live = stats.live.as_ref();
    let download_rate = live.map(|item| item.download_speed.as_bytes()).unwrap_or(0);
    let upload_rate = live.map(|item| item.upload_speed.as_bytes()).unwrap_or(0);
    let peers = live.map(|item| item.snapshot.peer_stats.live).unwrap_or(0);
    let seeds = live
        .map(|item| item.snapshot.peer_stats.not_needed)
        .unwrap_or(0);
    let remaining = selected_size.saturating_sub(downloaded);
    let eta_seconds = if download_rate > 0 && remaining > 0 {
        Some(remaining / download_rate)
    } else {
        None
    };
    let progress = if selected_size == 0 {
        0.0
    } else {
        (downloaded as f64 / selected_size as f64).min(1.0)
    };
    let verifying = matches!(stats.state, TorrentStatsState::Initializing { .. });
    let failed = matches!(stats.state, TorrentStatsState::Error) || stored.error.is_some();
    let state = if failed {
        TorrentState::Failed
    } else if stored.stopped_seeding && stats.finished {
        TorrentState::Finished
    } else if stored.user_paused {
        TorrentState::Paused
    } else if stats.finished {
        TorrentState::Seeding
    } else if verifying
        || matches!(
            stats.state,
            TorrentStatsState::Live | TorrentStatsState::Initializing { .. }
        )
    {
        TorrentState::Fetching
    } else if matches!(stats.state, TorrentStatsState::Paused) {
        TorrentState::Paused
    } else {
        TorrentState::Fetching
    };
    let mut trackers: Vec<String> = handle
        .shared()
        .trackers
        .iter()
        .map(|url| url.to_string())
        .collect();
    trackers.sort();
    let name = handle.name().unwrap_or_else(|| stored.name.clone());
    TorrentTransferSnapshot {
        download_id: stored.download_id.clone(),
        attempt: stored.attempt,
        info_hash: hash.to_string(),
        source: stored.source.clone(),
        name,
        save_folder: handle.output_folder().to_string_lossy().into_owned(),
        content_folder: stored.content_folder.clone(),
        state,
        total_size,
        selected_size,
        bytes_downloaded: downloaded,
        bytes_uploaded: uploaded,
        download_rate,
        upload_rate,
        peers,
        seeds,
        seed_ratio: ratio,
        seed_ratio_limit: stored.seed_ratio_limit,
        progress,
        eta_seconds,
        piece_length,
        piece_count,
        comment: stored.comment.clone(),
        trackers,
        files,
        verifying,
        error_message: stored.error.clone().or(stats.error),
    }
}

fn file_status(
    stored: &StoredTorrent,
    progress: &[u64],
    only: &Option<Vec<usize>>,
    index: usize,
    path: String,
    size: u64,
) -> FileTransferStatus {
    let included = only
        .as_ref()
        .map(|ids| ids.contains(&index))
        .unwrap_or(true);
    let priority = if !included {
        FilePriority::Skip.as_label().to_string()
    } else {
        stored
            .priorities
            .get(&(index as u32))
            .cloned()
            .filter(|label| label != FilePriority::Skip.as_label())
            .unwrap_or_else(|| FilePriority::Normal.as_label().to_string())
    };
    FileTransferStatus {
        file_index: index as u32,
        path,
        size,
        bytes_completed: progress.get(index).copied().unwrap_or(0),
        is_included: included,
        priority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::BValue;
    use crate::meta::encode_meta_for_test;
    use std::collections::BTreeMap;

    fn sample_bytes() -> Vec<u8> {
        let mut info = BTreeMap::new();
        info.insert(b"name".to_vec(), BValue::Bytes(b"hello.txt".to_vec()));
        info.insert(b"length".to_vec(), BValue::Int(11));
        info.insert(b"piece length".to_vec(), BValue::Int(16384));
        info.insert(b"pieces".to_vec(), BValue::Bytes(vec![0u8; 20]));
        encode_meta_for_test(info, None)
    }

    #[test]
    fn torrent_file_joins_the_session_without_a_tracker() {
        let dir = std::env::temp_dir().join(format!("opul-torrent-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let engine = TorrentEngine::new(EngineConfig {
            session_folder: dir.to_string_lossy().into_owned(),
            listen_port: 0,
            randomize_port: true,
            enable_dht: false,
            max_connections: 8,
            max_download_bps: 0,
            max_upload_bps: 0,
        })
        .expect("session");
        let meta = engine
            .add_torrent_file(
                sample_bytes(),
                "memory://hello.torrent".to_string(),
                dir.join("out").to_string_lossy().into_owned(),
                DownloadContentLayout::Original,
                Some(1.0),
            )
            .expect("meta");
        let mut matched = None;
        for _ in 0..40 {
            if let Some(snapshot) = engine.snapshots().into_iter().find(|snap| {
                snap.info_hash == meta.info_hash
                    && snap.piece_count > 0
                    && snap.state != TorrentState::Failed
            }) {
                matched = Some(snapshot);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = fs::remove_dir_all(&dir);
        let snapshot = matched.expect("torrent never left the failed state");
        assert_eq!(snapshot.save_folder, dir.join("out").to_string_lossy());
        assert!(snapshot.content_folder.is_none());
    }

    #[test]
    fn load_extras_corrupt_file_preserves_bad_file_and_recovers_from_backup() {
        let dir =
            std::env::temp_dir().join(format!("opul-extras-test-corrupt-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("opul-torrent-extras.json");
        let backup_path = dir.join("opul-torrent-extras.json.bak");

        let mut good_extras = HashMap::new();
        good_extras.insert(
            "hash1".to_string(),
            StoredTorrent::fresh(
                "magnet:?xt=urn:btih:hash1".to_string(),
                "Good Torrent".to_string(),
                "/Downloads".to_string(),
                None,
                Some(1.5),
                None,
            ),
        );
        fs::write(
            &backup_path,
            serde_json::to_vec_pretty(&good_extras).unwrap(),
        )
        .unwrap();
        // Write corrupt truncated JSON
        fs::write(&path, b"{\"hash1\": {\"name\": ").unwrap();

        let loaded = load_extras(&path).unwrap();
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items.get("hash1").unwrap().name, "Good Torrent");
        assert!(loaded.recovery_notice.is_some());
        assert_eq!(fs::read(&path).unwrap(), fs::read(&backup_path).unwrap());

        // Verify a corrupt backup was created
        let corrupt_files: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("corrupt"))
            .collect();
        assert_eq!(corrupt_files.len(), 1);
        let corrupt_content = fs::read(corrupt_files[0].path()).unwrap();
        assert_eq!(corrupt_content, b"{\"hash1\": {\"name\": ");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_extras_incompatible_schema_preserves_bad_file() {
        let dir = std::env::temp_dir().join(format!(
            "opul-extras-test-incompatible-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("opul-torrent-extras.json");

        fs::write(&path, br#"{"schemaVersion":2,"items":{}}"#).unwrap();

        let loaded = load_extras(&path).unwrap();
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
    fn corrupt_librqbit_session_is_preserved_and_restored_from_backup() {
        let dir = std::env::temp_dir().join(format!(
            "opul-native-session-test-corrupt-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.json");
        let backup_path = dir.join("session.json.bak");
        let backup = br#"{"torrents":{}}"#;
        fs::write(&backup_path, backup).unwrap();
        fs::write(&path, b"{\"torrents\":").unwrap();

        let notice = protect_librqbit_session(&dir).unwrap();

        assert!(notice.is_some());
        assert_eq!(fs::read(&path).unwrap(), backup);
        let corrupt_files = fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("corrupt"))
            .count();
        assert_eq!(corrupt_files, 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_extras_atomic_and_keeps_backup() {
        let dir =
            std::env::temp_dir().join(format!("opul-extras-test-atomic-{}", std::process::id()));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("opul-torrent-extras.json");
        let backup_path = dir.join("opul-torrent-extras.json.bak");

        let mut extras1 = HashMap::new();
        extras1.insert(
            "hash1".to_string(),
            StoredTorrent::fresh(
                "src1".to_string(),
                "Torrent 1".to_string(),
                "/dir".to_string(),
                None,
                None,
                None,
            ),
        );
        write_extras(&path, &extras1);
        assert!(path.exists());
        assert!(!backup_path.exists());

        let mut extras2 = HashMap::new();
        extras2.insert(
            "hash2".to_string(),
            StoredTorrent::fresh(
                "src2".to_string(),
                "Torrent 2".to_string(),
                "/dir".to_string(),
                None,
                None,
                None,
            ),
        );
        write_extras(&path, &extras2);
        assert!(path.exists());
        assert!(backup_path.exists());

        let loaded_primary = load_extras(&path).unwrap();
        assert_eq!(loaded_primary.items.get("hash2").unwrap().name, "Torrent 2");

        let loaded_bak = load_extras(&backup_path).unwrap();
        assert_eq!(loaded_bak.items.get("hash1").unwrap().name, "Torrent 1");

        let _ = fs::remove_dir_all(&dir);
    }
}
