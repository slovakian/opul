use super::*;

use ed25519_dalek::{Signer, SigningKey};
use flate2::{Compression, write::GzEncoder};
use pgp::composed::{
    ArmorOptions, DetachedSignature, KeyType, SecretKeyParamsBuilder, SignedPublicKey,
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
}
pub(super) struct Fixture {
    pub(super) config: VideoToolsConfig,
    pub(super) sources: super::install::Sources,
    pub(super) routes: Arc<Mutex<HashMap<String, Response>>>,
    pub(super) hits: Arc<Mutex<Vec<String>>>,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) server: Option<thread::JoinHandle<()>>,
    pub(super) _temp: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new(health: &str) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config = VideoToolsConfig {
            root: temp.path().join("Tools"),
            opul_version: "0.2.2".into(),
            built_in_tools: temp.path().join("media-tools"),
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
        let script = b"official fixture script".to_vec();
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
            "#!/bin/sh\nprintf '%s\\n' \"$@\" >> '{}'\ncase \"$*\" in\n *--version*|*--list-impersonate-targets*) printf '%s\\n' '2026.10.26'; cat >&2 <<'HEALTH'\n{}\nHEALTH\n;;\n *) printf '%s\\n' '{{\"title\":\"Fixture video\",\"webpage_url\":\"https://example.test/video\"}}' ;;\nesac\n",
            record.display(),
            health
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
            "architectures": (["arm64", "x64"].into_iter().map(|arch| (arch, serde_json::json!({ "url": format!("{base}/runtime"), "signature_url": format!("{base}/runtime.sig"), "sha256": hex::encode(Sha256::digest(&runtime)), "size": runtime.len() }))).collect::<HashMap<_,_>>())
        })).unwrap();
        let data = [
            ("/pointer", pointer.clone()),
            ("/pointer.sig", signing.sign(&pointer).to_bytes().to_vec()),
            ("/runtime", runtime.clone()),
            ("/runtime.sig", signing.sign(&runtime).to_bytes().to_vec()),
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
            },
        );
        routes.insert(
            "/yt/releases/tag/2026.10.26".into(),
            Response {
                bytes: vec![],
                redirect: None,
                delay: Duration::ZERO,
            },
        );
        let routes = Arc::new(Mutex::new(routes));
        let hits = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let server = {
            let (routes, hits, stop) = (Arc::clone(&routes), Arc::clone(&hits), Arc::clone(&stop));
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
                    let response = routes.lock().unwrap().get(&path).cloned();
                    if let Some(response) = response {
                        thread::sleep(response.delay);
                        let status = if response.redirect.is_some() {
                            "302 Found"
                        } else {
                            "200 OK"
                        };
                        let location = response
                            .redirect
                            .map(|url| format!("Location: {url}\r\n"))
                            .unwrap_or_default();
                        let header = format!(
                            "HTTP/1.1 {status}\r\n{location}Content-Length: {}\r\nConnection: close\r\n\r\n",
                            response.bytes.len()
                        );
                        let _ = stream.write_all(header.as_bytes());
                        if method != "HEAD" {
                            let _ = stream.write_all(&response.bytes);
                        }
                    } else {
                        let _ = stream
                            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                    }
                }
            })
        };
        Self {
            config,
            sources: super::install::Sources {
                pointer: format!("{base}/pointer"),
                repository: format!("{base}/yt"),
                opul_key: signing.verifying_key().to_bytes(),
                yt_dlp_key: public,
                allow_fixture_http: true,
            },
            routes,
            hits,
            stop,
            server: Some(server),
            _temp: temp,
        }
    }
    pub(super) fn open(&self) -> Arc<VideoTools> {
        VideoTools::open_with_sources(self.config.clone(), self.sources.clone()).unwrap()
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
