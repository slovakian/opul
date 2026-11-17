//! Scripted Rune input, real core, loopback HTTP and filesystem assertions. No engine mocks.
use super::*;
use opul_core::{MachineDownloadState, MachineHoldCause, MachineSessionState, MachineSnapshot};
use rune::{Env, Key, Runtime};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

struct Harness {
    rt: Runtime<State>,
    backend: Backend,
    events: mpsc::Sender<backend::Event>,
}

impl Harness {
    fn open(root: &Path) -> Self {
        let paths = paths::Paths {
            data: root.join("data"),
            downloads: root.join("Downloads"),
            resources: None,
        };
        let (events, receiver) = mpsc::channel();
        let backend = Backend::start(paths, events.clone(), Arc::new(|| {})).unwrap();
        let state = State::new(backend.client.clone(), receiver, Arc::new(Mutex::new(None)));
        let mut rt = Runtime::new(state, view::view, (1180.0, 760.0), Env::new(false, 1.0));
        rt.poll = Some(|s, _, _| s.poll());
        let mut harness = Self {
            rt,
            backend,
            events,
        };
        harness.wait(|s| {
            s.snapshot
                .as_ref()
                .is_some_and(|s| s.session == MachineSessionState::Running)
        });
        harness
    }

    fn wait(&mut self, ready: impl Fn(&State) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            self.rt.frame(1.0 / 60.0);
            if ready(&self.rt.state) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "condition timed out: {:?}, error {:?}",
                self.rt.state.snapshot,
                self.rt.state.error
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn click(&mut self, key: &str) {
        self.rt.frame(1.0 / 60.0);
        let (x, y) = self.rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        self.rt.pointer_down(x, y);
        self.rt.pointer_up(x, y);
        self.rt.frame(1.0 / 60.0);
    }

    fn submit(&mut self, url: &str) -> String {
        let count = self.rt.state.workspace().unwrap().downloads.len();
        self.click("direct-url");
        self.rt.paste(url);
        self.rt.frame(1.0 / 60.0);
        self.click("submit-download");
        self.wait(|s| !s.busy() && s.workspace().unwrap().downloads.len() > count);
        self.rt
            .state
            .workspace()
            .unwrap()
            .downloads
            .last()
            .unwrap()
            .id
            .clone()
    }

    fn state(&self, id: &str) -> MachineDownloadState {
        self.rt
            .state
            .workspace()
            .unwrap()
            .downloads
            .iter()
            .find(|d| d.id == id)
            .unwrap()
            .state
    }

    fn wait_state(&mut self, id: &str, expected: MachineDownloadState) {
        self.wait(|s| {
            !s.busy()
                && s.workspace()
                    .unwrap()
                    .downloads
                    .iter()
                    .any(|d| d.id == id && d.state == expected)
        });
        let element = self
            .rt
            .elements()
            .into_iter()
            .find(|e| e.key.as_deref() == Some(&format!("status-{id}")))
            .unwrap();
        assert_eq!(element.role, "status");
        assert!(element.rect[2] > 0.0);
    }

    fn close(self) -> MachineSnapshot {
        self.backend.finish().unwrap()
    }
}

struct Server {
    address: std::net::SocketAddr,
    stop: Arc<AtomicBool>,
    fail: Arc<AtomicBool>,
    payload: Arc<Vec<u8>>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Server {
    fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let fail = Arc::new(AtomicBool::new(false));
        let payload = Arc::new(
            (0..4 * 1024 * 1024)
                .map(|i| (i % 251) as u8)
                .collect::<Vec<_>>(),
        );
        let (stopping, failing, bytes) = (stop.clone(), fail.clone(), payload.clone());
        let thread = thread::spawn(move || {
            for stream in listener.incoming() {
                let stream = stream.unwrap();
                if stopping.load(Ordering::Acquire) {
                    break;
                }
                let payload = bytes.clone();
                let fail = failing.load(Ordering::Acquire);
                thread::spawn(move || {
                    let _ = serve(stream, &payload, fail);
                });
            }
        });
        Self {
            address,
            stop,
            fail,
            payload,
            thread: Some(thread),
        }
    }
    fn url(&self, name: &str) -> String {
        format!("http://{}/{name}.mp4", self.address)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn serve(mut stream: TcpStream, payload: &[u8], fail: bool) -> std::io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    let mut offset = 0;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        if let Some(value) = line.to_lowercase().strip_prefix("range: bytes=") {
            offset = value.split('-').next().unwrap().parse::<usize>().unwrap();
        }
    }
    if fail {
        return write!(
            stream,
            "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
    }
    if offset > 0 {
        write!(
            stream,
            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {offset}-{}/{}\r\n",
            payload.len() - 1,
            payload.len()
        )?;
    } else {
        write!(stream, "HTTP/1.1 200 OK\r\n")?;
    }
    write!(
        stream,
        "Content-Type: video/mp4\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nETag: \"fixture-v1\"\r\nConnection: close\r\n\r\n",
        payload.len() - offset
    )?;
    for chunk in payload[offset..].chunks(8192) {
        stream.write_all(chunk)?;
        thread::sleep(Duration::from_millis(4));
    }
    Ok(())
}

#[test]
fn rune_folder_transfer_controls_output_and_restore() {
    let root = tempfile::tempdir().unwrap();
    let folder = root.path().join("Saved");
    std::fs::create_dir(&folder).unwrap();
    let server = Server::start();
    let mut app = Harness::open(root.path());
    assert_eq!(app.rt.state.workspace().unwrap().folders.len(), 1);
    app.click("choose-folder");
    assert!(app.rt.file_dialog().is_some());
    app.rt.resolve_file_dialog(None).unwrap();
    app.rt.frame(0.016);
    assert_eq!(app.rt.state.workspace().unwrap().folders.len(), 1);
    app.click("choose-folder");
    app.rt.resolve_file_dialog(Some(folder.clone())).unwrap();
    app.wait(|s| !s.busy() && s.workspace().unwrap().folders.len() == 2);
    let folder_id = app.rt.state.selected_folder.clone().unwrap();
    let id = app.submit(&server.url("journey"));
    assert!(app.rt.state.url.text.is_empty());
    app.wait(|s| {
        s.workspace()
            .unwrap()
            .downloads
            .iter()
            .any(|d| d.id == id && d.bytes_downloaded > 0)
    });
    app.click(&format!("pause-{id}"));
    app.wait_state(&id, MachineDownloadState::Paused);
    let held = app.rt.state.workspace().unwrap().downloads[0].clone();
    assert_eq!(held.hold_cause, Some(MachineHoldCause::Person));
    assert_eq!(held.folder_id, folder_id);
    assert!(held.bytes_downloaded < server.payload.len() as u64);
    let stopped = app.close();
    assert_eq!(stopped.session, MachineSessionState::Stopped);
    let mut app = Harness::open(root.path());
    assert_eq!(app.state(&id), MachineDownloadState::Paused);
    app.click(&format!("resume-{id}"));
    app.wait_state(&id, MachineDownloadState::Finished);
    let download = app.rt.state.workspace().unwrap().downloads[0].clone();
    let output = download.output_path.unwrap();
    assert!(Path::new(&output).starts_with(&folder));
    assert_eq!(std::fs::read(&output).unwrap(), *server.payload);
    app.click(&format!("remove-{id}"));
    app.wait(|s| !s.busy() && s.workspace().unwrap().downloads.is_empty());
    assert!(
        Path::new(&output).is_file(),
        "Remove from list must retain output"
    );
    app.close();
}

#[test]
fn retry_cancel_and_shutdown_preserve_identity_and_running_intent() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::start();
    server.fail.store(true, Ordering::Release);
    let mut app = Harness::open(root.path());
    let id = app.submit(&server.url("retry"));
    app.wait_state(&id, MachineDownloadState::Failed);
    server.fail.store(false, Ordering::Release);
    app.click(&format!("retry-{id}"));
    app.wait(|s| {
        s.workspace()
            .unwrap()
            .downloads
            .iter()
            .any(|d| d.id == id && d.bytes_downloaded > 0)
    });
    let stopped = app.close();
    assert_eq!(stopped.session, MachineSessionState::Stopped);
    let mut app = Harness::open(root.path());
    app.wait_state(&id, MachineDownloadState::Finished);
    assert_eq!(app.rt.state.workspace().unwrap().downloads.len(), 1);
    let cancelled = app.submit(&server.url("cancel"));
    app.wait_state(&cancelled, MachineDownloadState::Fetching);
    app.click(&format!("cancel-{cancelled}"));
    app.wait_state(&cancelled, MachineDownloadState::Cancelled);
    assert!(app.rt.find(&format!("retry-{cancelled}")).is_none());
    assert_eq!(app.rt.state.workspace().unwrap().downloads.len(), 2);
    app.close();
}

#[test]
fn stale_projections_and_receipts_cannot_replace_newer_ui_state() {
    let root = tempfile::tempdir().unwrap();
    let mut app = Harness::open(root.path());
    let current = app.rt.state.snapshot.clone().unwrap();
    let mut stale = current.clone();
    stale.sequence = current.sequence.saturating_sub(1);
    stale.workspaces.clear();
    app.events.send(backend::Event::Snapshot(stale)).unwrap();
    app.events
        .send(backend::Event::Settled(
            "unrelated-request".into(),
            Err("stale failure".into()),
        ))
        .unwrap();
    app.rt.frame(0.016);
    assert_eq!(
        app.rt.state.snapshot.as_ref().unwrap().workspaces,
        current.workspaces
    );
    assert!(app.rt.state.error.is_none());
    let mut other_boot = current;
    other_boot.boot_id = Some(uuid::Uuid::new_v4().to_string());
    other_boot.sequence += 100;
    other_boot.workspaces.clear();
    app.events
        .send(backend::Event::Snapshot(other_boot))
        .unwrap();
    app.rt.frame(0.016);
    assert!(app.rt.state.workspace().is_some());
    app.click("direct-url");
    app.rt.paste("file:///etc/passwd");
    app.rt.frame(0.016);
    app.rt.key(Key::Enter.into());
    app.rt.frame(0.016);
    assert_eq!(app.rt.state.url.text, "file:///etc/passwd");
    assert!(app.rt.state.error.is_some());
    assert!(app.rt.state.workspace().unwrap().downloads.is_empty());
    app.close();
}
