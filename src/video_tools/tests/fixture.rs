use super::network::FakeNetwork;
use super::*;

use ed25519_dalek::{Signer, SigningKey};
use flate2::{Compression, write::GzEncoder};
use pgp::composed::{
    ArmorOptions, DetachedSignature, KeyType, SecretKeyParamsBuilder, SignedPublicKey,
    SignedSecretKey,
};
use pgp::crypto::hash::HashAlgorithm;
use pgp::ser::Serialize as _;
use pgp::types::Password;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub(super) struct Response {
    pub(super) bytes: Vec<u8>,
    pub(super) redirect: Option<String>,
    pub(super) delay: Duration,
    pub(super) interruptions: usize,
    pub(super) ignore_range: bool,
    pub(super) failures: usize,
    pub(super) failure_status: u16,
    pub(super) corruptions: usize,
    pub(super) range_header: Option<String>,
}
pub(super) struct Fixture {
    key: SignedSecretKey,
    pub(super) config: VideoToolsConfig,
    pub(super) sources: super::sources::Sources,
    pub(super) routes: Arc<Mutex<HashMap<String, Response>>>,
    pub(super) hits: Arc<Mutex<Vec<String>>>,
    pub(super) network: Arc<FakeNetwork>,
    pub(super) ranges: Arc<Mutex<Vec<(String, u64)>>>,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) server: Option<thread::JoinHandle<()>>,
    pub(super) _temp: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new(health: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config = VideoToolsConfig {
            nightly_available: false,
            root: temp.path().join("Tools"),
            opul_version: "0.2.2".into(),
        };
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let signing = SigningKey::from_bytes(&[9; 32]);
        let key = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519Legacy)
            .can_sign(true)
            .can_certify(true)
            .primary_user_id("Opul fixture <fixture@example.test>".into())
            .build()
            .unwrap()
            .generate(rand::thread_rng())
            .unwrap()
            .sign(&mut rand::thread_rng(), &Password::empty())
            .unwrap();
        let public = SignedPublicKey::from(key.clone())
            .to_armored_string(ArmorOptions::default())
            .unwrap();
        let script = b"2026.10.26".to_vec();
        let sums = format!("{}  yt-dlp\n", hex::encode(Sha256::digest(&script))).into_bytes();
        let signature = DetachedSignature::sign_binary_data(
            rand::thread_rng(),
            &key.primary_key,
            &Password::empty(),
            HashAlgorithm::Sha256,
            sums.as_slice(),
        )
        .unwrap()
        .to_bytes()
        .unwrap();
        let record = temp.path().join("executed");
        let interpreter = format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\ncase \"$*\" in\n *--version*|*--list-impersonate-targets*) printf '%s\\n' \"$(cat \"$2\")\"; cat >&2 <<HEALTH\n{}\nHEALTH\n;;\n *) printf '%s\\n' '{{\"title\":\"Fixture video\",\"webpage_url\":\"https://example.test/video\"}}' ;;\nesac\n",
            record.display(),
            health.replace("stable@2026.10.26", "stable@$(cat \"$2\")")
        );
        let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        for (path, bytes) in [
            ("bin/python3", interpreter.as_bytes()),
            ("bin/qjs", b"#!/bin/sh\nexit 0\n".as_slice()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, path, bytes).unwrap();
        }
        let runtime = archive.into_inner().unwrap().finish().unwrap();
        let pointer = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1, "runtime": 1, "minimum_opul_version": "0.2.2",
            "architectures": (["arm64", "x64"].into_iter().map(|arch| (arch, serde_json::json!({ "url": format!("{base}/runtime-1/runtime"), "signature_url": format!("{base}/runtime-1/runtime.sig"), "sha256": hex::encode(Sha256::digest(&runtime)), "size": runtime.len() }))).collect::<HashMap<_,_>>())
        })).unwrap();
        let data = [
            ("/pointer", pointer.clone()),
            ("/pointer.sig", signing.sign(&pointer).to_bytes().to_vec()),
            ("/runtime-1/runtime", runtime.clone()),
            (
                "/runtime-1/runtime.sig",
                signing.sign(&runtime).to_bytes().to_vec(),
            ),
            ("/yt/releases/download/2026.10.26/yt-dlp", script),
            ("/yt/releases/download/2026.10.26/SHA2-256SUMS", sums),
            (
                "/yt/releases/download/2026.10.26/SHA2-256SUMS.sig",
                signature,
            ),
        ];
        let mut routes: HashMap<_, _> = data
            .into_iter()
            .map(|(path, bytes)| {
                (
                    path.to_string(),
                    Response {
                        bytes,
                        redirect: None,
                        delay: Duration::ZERO,
                        interruptions: 0,
                        ignore_range: false,
                        failures: 0,
                        failure_status: 503,
                        corruptions: 0,
                        range_header: None,
                    },
                )
            })
            .collect();
        routes.insert(
            "/yt/releases/latest".into(),
            Response {
                bytes: vec![],
                redirect: Some(format!("{base}/yt/releases/tag/2026.10.26")),
                delay: Duration::ZERO,
                interruptions: 0,
                ignore_range: false,
                failures: 0,
                failure_status: 503,
                corruptions: 0,
                range_header: None,
            },
        );
        routes.insert(
            "/yt/releases/tag/2026.10.26".into(),
            Response {
                bytes: vec![],
                redirect: None,
                delay: Duration::ZERO,
                interruptions: 0,
                ignore_range: false,
                failures: 0,
                failure_status: 503,
                corruptions: 0,
                range_header: None,
            },
        );
        let routes = Arc::new(Mutex::new(routes));
        let hits = Arc::new(Mutex::new(Vec::new()));
        let network = Arc::new(FakeNetwork::new());
        let ranges = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let server = {
            let (routes, hits, stop) = (Arc::clone(&routes), Arc::clone(&hits), Arc::clone(&stop));
            let ranges = Arc::clone(&ranges);
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let Ok((mut stream, _)) = listener.accept() else {
                        thread::sleep(Duration::from_millis(2));
                        continue;
                    };
                    stream.set_nonblocking(false).unwrap();
                    stream
                        .set_read_timeout(Some(Duration::from_secs(2)))
                        .unwrap();
                    let mut buffer = Vec::new();
                    loop {
                        let mut chunk = [0; 4096];
                        let Ok(length) = stream.read(&mut chunk) else {
                            break;
                        };
                        if length == 0 {
                            break;
                        }
                        buffer.extend_from_slice(&chunk[..length]);
                        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                            break;
                        }
                    }
                    if buffer.is_empty() {
                        continue;
                    }
                    let request = String::from_utf8_lossy(&buffer);
                    let mut parts = request.split_whitespace();
                    let method = parts.next().unwrap_or("");
                    let path = parts.next().unwrap_or("").to_string();
                    hits.lock().unwrap().push(format!("{method} {path}"));
                    let offset = request
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("range: bytes=")
                                .and_then(|s| s.strip_suffix('-'))
                                .and_then(|s| s.parse::<u64>().ok())
                        })
                        .unwrap_or(0);
                    if offset > 0 {
                        ranges.lock().unwrap().push((path.clone(), offset));
                    }
                    let response = {
                        let mut routes = routes.lock().unwrap();
                        let response = routes.get_mut(&path);
                        response.map(|r| {
                            let copy = r.clone();
                            r.interruptions = r.interruptions.saturating_sub(1);
                            r.failures = r.failures.saturating_sub(1);
                            r.corruptions = r.corruptions.saturating_sub(1);
                            copy
                        })
                    };
                    if let Some(mut response) = response {
                        if response.corruptions > 0 && !response.bytes.is_empty() {
                            response.bytes[0] ^= 1;
                        }
                        if response.failures > 0 {
                            let _ = stream.write_all(format!("HTTP/1.1 {} Fixture failure\r\nContent-Length: 0\r\nConnection: close\r\n\r\n", response.failure_status).as_bytes());
                            continue;
                        }
                        thread::sleep(response.delay);
                        let offset = if response.ignore_range {
                            0
                        } else {
                            offset as usize
                        };
                        let status = if offset > 0 {
                            "206 Partial Content"
                        } else if response.redirect.is_some() {
                            "302 Found"
                        } else {
                            "200 OK"
                        };
                        let location = response
                            .redirect
                            .map(|url| format!("Location: {url}\r\n"))
                            .unwrap_or_default();
                        let range = if let Some(value) = response.range_header {
                            format!("Content-Range: {value}\r\n")
                        } else if offset > 0 {
                            format!(
                                "Content-Range: bytes {offset}-{}/{}\r\n",
                                response.bytes.len() - 1,
                                response.bytes.len()
                            )
                        } else {
                            String::new()
                        };
                        let header = format!(
                            "HTTP/1.1 {status}\r\n{location}{range}Content-Length: {}\r\nConnection: close\r\n\r\n",
                            response.bytes.len() - offset
                        );
                        let _ = stream.write_all(header.as_bytes());
                        if method != "HEAD" {
                            let end = if response.interruptions > 0 {
                                offset + (response.bytes.len() - offset) / 2
                            } else {
                                response.bytes.len()
                            };
                            let _ = stream.write_all(&response.bytes[offset..end]);
                        }
                    } else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    }
                }
            })
        };
        Self {
            key,
            config,
            sources: super::sources::Sources {
                pointer: format!("{base}/pointer"),
                repository: format!("{base}/yt"),
                nightly_repository: format!("{base}/nightly"),
                opul_key: signing.verifying_key().to_bytes(),
                yt_dlp_key: public,
                runtime_release_prefix: format!("{base}/runtime-"),
                https_only: false,
                exclude_from_backup: false,
                environment: network.clone(),
            },
            routes,
            hits,
            ranges,
            network,
            stop,
            server: Some(server),
            _temp: temp,
        }
    }
    pub(super) fn publish(&self, number: u64, version: &str, health: &str) {
        let base = self.sources.pointer.strip_suffix("/pointer").unwrap();
        let script = version.as_bytes().to_vec();
        let sums = format!("{}  yt-dlp\n", hex::encode(Sha256::digest(&script))).into_bytes();
        let signature = DetachedSignature::sign_binary_data(
            rand::thread_rng(),
            &self.key.primary_key,
            &Password::empty(),
            HashAlgorithm::Sha256,
            sums.as_slice(),
        )
        .unwrap()
        .to_bytes()
        .unwrap();
        // The fixture interpreter reads the candidate script to report its version.
        let interpreter = format!(
            "#!/bin/sh\nscript=\"$2\"\nversion=$(cat \"$script\")\ncase \"$*\" in\n *--version*|*--list-impersonate-targets*) printf '%s\\n' \"$version\"; cat >&2 <<HEALTH\n{}\nHEALTH\n;;\n *) exit 0 ;;\nesac\n",
            health.replace(version, "$version")
        );
        let mut archive = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::fast()));
        for (path, bytes) in [
            ("bin/python3", interpreter.as_bytes()),
            ("bin/qjs", b"#!/bin/sh\nexit 0\n".as_slice()),
        ] {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            archive.append_data(&mut header, path, bytes).unwrap();
        }
        let runtime = archive.into_inner().unwrap().finish().unwrap();
        let pointer = serde_json::to_vec(&serde_json::json!({
            "schema_version": 1, "runtime": number, "minimum_opul_version": "0.2.2",
            "architectures": (["arm64", "x64"].into_iter().map(|arch| (arch, serde_json::json!({
                "url": format!("{base}/runtime-{number}/runtime"), "signature_url": format!("{base}/runtime-{number}/runtime.sig"),
                "sha256": hex::encode(Sha256::digest(&runtime)), "size": runtime.len()
            }))).collect::<HashMap<_,_>>())
        })).unwrap();
        let signing = SigningKey::from_bytes(&[9; 32]);
        let mut routes = self.routes.lock().unwrap();
        let template = routes["/runtime-1/runtime"].clone();
        for (path, bytes) in [
            ("/pointer".into(), pointer.clone()),
            (
                "/pointer.sig".into(),
                signing.sign(&pointer).to_bytes().to_vec(),
            ),
            (format!("/runtime-{number}/runtime"), runtime.clone()),
            (
                format!("/runtime-{number}/runtime.sig"),
                signing.sign(&runtime).to_bytes().to_vec(),
            ),
            (format!("/yt/releases/download/{version}/yt-dlp"), script),
            (
                format!("/yt/releases/download/{version}/SHA2-256SUMS"),
                sums,
            ),
            (
                format!("/yt/releases/download/{version}/SHA2-256SUMS.sig"),
                signature,
            ),
            (format!("/yt/releases/tag/{version}"), vec![]),
        ] {
            routes.insert(
                path,
                Response {
                    bytes,
                    ..template.clone()
                },
            );
        }
        routes.get_mut("/yt/releases/latest").unwrap().redirect =
            Some(format!("{base}/yt/releases/tag/{version}"));
    }
    pub(super) fn open(&self) -> Arc<VideoTools> {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match VideoTools::open_with_sources(self.config.clone(), self.sources.clone()) {
                Ok(tools) => return tools,
                Err(err)
                    if err == "The video tools directory is already in use."
                        && Instant::now() < deadline =>
                {
                    thread::sleep(Duration::from_millis(5));
                }
                Err(err) => panic!("{err}"),
            }
        }
    }
    pub(super) fn close(&self, tools: Arc<VideoTools>) {
        let deadline = Instant::now() + Duration::from_secs(10);
        // Simulated process restart must wait for the prior task to release its directory lock.
        while Arc::strong_count(&tools) != 1 {
            assert!(Instant::now() < deadline, "tools owner did not finish");
            thread::sleep(Duration::from_millis(2));
        }
        drop(tools);
    }
    pub(super) fn wait(&self, tools: &VideoTools) -> VideoToolsSnapshot {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let snapshot = tools.snapshot();
            if !matches!(snapshot.state, VideoToolsState::SettingUp { .. })
                && tools.lock().job.is_none()
            {
                return snapshot;
            }
            assert!(Instant::now() < deadline, "setup timed out: {snapshot:?}");
            thread::sleep(Duration::from_millis(5));
        }
    }
    pub(super) fn tamper(&self, path: &str) {
        let mut routes = self.routes.lock().unwrap();
        let bytes = &mut routes.get_mut(path).unwrap().bytes;
        if path.ends_with(".sig") {
            let middle = bytes.len() / 2;
            bytes[middle] ^= 1;
        } else {
            bytes.push(0);
        }
    }
    pub(super) fn count(&self, path: &str) -> usize {
        self.hits
            .lock()
            .unwrap()
            .iter()
            .filter(|s| *s == path)
            .count()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        self.server.take().unwrap().join().unwrap();
    }
}
