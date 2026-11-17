#!/usr/bin/env python3
"""Exercise the real macOS bundle through accessibility, native dialogs and fresh processes."""
import argparse
import hashlib
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import threading
import time

ROOT = Path(__file__).resolve().parent.parent
PAYLOAD = bytes(range(251)) * 32768


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        offset = int(self.headers.get("Range", "bytes=0-").split("=")[1].split("-")[0])
        self.send_response(206 if offset else 200)
        self.send_header("Content-Type", "video/mp4")
        self.send_header("Content-Length", str(len(PAYLOAD) - offset))
        self.send_header("Accept-Ranges", "bytes")
        if offset:
            self.send_header("Content-Range", f"bytes {offset}-{len(PAYLOAD) - 1}/{len(PAYLOAD)}")
        self.end_headers()
        try:
            for start in range(offset, len(PAYLOAD), 16384):
                self.wfile.write(PAYLOAD[start:start + 16384])
                self.wfile.flush()
                time.sleep(0.02)
        except (BrokenPipeError, ConnectionResetError):
            pass

    def log_message(self, *_):
        pass


def wait(check, description, seconds=30):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        value = check()
        if value:
            return value
        time.sleep(0.15)
    raise AssertionError(f"Timed out: {description}")


class App:
    def __init__(self, bundle, root, name):
        self.output = root / f"{name}-stopped.json"
        self.log = (root / f"{name}.log").open("w")
        env = {
            "HOME": os.environ["HOME"], "PATH": "/usr/bin:/bin:/usr/sbin:/sbin",
            "OPUL_RUNE_DATA_DIR": str(root / "data"),
            "OPUL_RUNE_DOWNLOAD_DIR": str(root / "Downloads"),
            "OPUL_RUNE_STATE_OUTPUT": str(self.output),
        }
        self.process = subprocess.Popen([str(bundle / "Contents/MacOS/opul-rune")],
                                        cwd=root, env=env, stdout=self.log, stderr=self.log)
        try:
            wait(lambda: self.script("return exists window 1", check=False) == "true", "native window")
            self.script("set frontmost to true")
            wait(lambda: "Ready" in self.tree(), "library ready")
        except Exception:
            self.script('set frontmost to true\nkeystroke "q" using {command down}', check=False)
            self.process.wait(timeout=30)
            self.log.close()
            raise

    def script(self, body, check=True):
        script = f'''tell application "System Events"
tell (first application process whose unix id is {self.process.pid})
{body}
end tell
end tell'''
        result = subprocess.run(["osascript", "-e", script], text=True, capture_output=True, timeout=15)
        if check and result.returncode:
            raise AssertionError(result.stderr)
        return result.stdout.strip()

    def tree(self):
        return self.script("return entire contents of window 1", check=False)

    def press(self, name):
        literal = json.dumps(name, ensure_ascii=False)
        def attempt():
            return self.script(f'''set axElements to get entire contents of window 1
repeat with axElement in axElements
    try
        set node to contents of axElement
        if role of node is "AXButton" and name of node is {literal} then
            perform action "AXPress" of node
            return "pressed"
        end if
    end try
end repeat
return "missing"''', check=False) == "pressed"
        wait(attempt, f"button {name}", seconds=5)

    def submit(self, url):
        self.script(f'''set frontmost to true
set axElements to get entire contents of window 1
repeat with axElement in axElements
    set node to contents of axElement
    if role of node is "AXTextField" then
        set focused of node to true
        keystroke {json.dumps(url)}
        exit repeat
    end if
end repeat''')
        self.press("Download")
        wait(lambda: "Downloading" in self.tree(), "live progress")

    def choose_folder(self, folder):
        self.press("Add Saved Folder…")
        # Exercise the actual NSOpenPanel. The Rune fixture dialog is covered by Rust tests.
        time.sleep(1)
        self.script(f'''keystroke "g" using {{command down, shift down}}
delay 0.5
keystroke {json.dumps(str(folder))}
keystroke return
delay 1''')
        self.press("Open")
        wait(lambda: str(folder) in self.tree(), "native folder selection committed")

    def close(self, last_window=False):
        if last_window:
            self.script('perform action "AXPress" of (first button of window 1 whose subrole is "AXCloseButton")')
        else:
            self.script('set frontmost to true\nkeystroke "q" using {command down}')
        assert self.process.wait(timeout=30) == 0
        self.log.close()
        snapshot = json.loads(self.output.read_text())
        assert snapshot["session"] == "Stopped", snapshot
        return snapshot


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    if not args.skip_build:
        subprocess.run([str(ROOT / "Scripts/package-rune.sh")], cwd=ROOT, check=True)
    evidence = ROOT / ".build/rune/evidence"
    evidence.mkdir(parents=True, exist_ok=True)
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    root = Path(tempfile.mkdtemp(prefix="opul-rune-journey-")).resolve()
    bundle = root / "Opul Rune.app"
    shutil.copytree(ROOT / ".build/rune/Opul Rune.app", bundle)
    folder = root / "Saved"
    folder.mkdir()
    app = None
    try:
        app = App(bundle, root, "first")
        app.choose_folder(folder)
        app.submit(f"http://127.0.0.1:{server.server_port}/held.mp4")
        app.press("Pause")
        wait(lambda: "Resume" in app.tree(), "deliberate pause")
        app.submit(f"http://127.0.0.1:{server.server_port}/running.mp4")
        (evidence / "before-quit.txt").write_text(app.tree())
        stopped = app.close()
        app = None
        downloads = stopped["workspaces"][0]["downloads"]
        held_id = next(d["id"] for d in downloads if "held.mp4" in d["source_url"])
        running_id = next(d["id"] for d in downloads if "running.mp4" in d["source_url"])
        assert next(d for d in downloads if d["id"] == held_id)["hold_cause"] == "Person"
        assert next(d for d in downloads if d["id"] == running_id)["hold_cause"] == "Shutdown"
        app = App(bundle, root, "second")
        wait(lambda: "Completed" in app.tree(), "running intent resumes in fresh process")
        assert "Resume" in app.tree(), "deliberate hold was lost"
        (evidence / "restored.txt").write_text(app.tree())
        app.press("Resume")
        wait(lambda: app.tree().count("group Completed") == 2, "held transfer completes")
        final = app.close(last_window=True)
        app = None
        downloads = final["workspaces"][0]["downloads"]
        assert {d["id"] for d in downloads} == {held_id, running_id}
        for download in downloads:
            assert download["state"] == "Finished", download
            output = Path(download["output_path"])
            assert output.is_relative_to(folder), output
            assert output.read_bytes() == PAYLOAD
        assert "Resources" in (root / "first.log").read_text()
        for file in root.glob("*.json"):
            shutil.copy(file, evidence / file.name)
        for file in root.glob("*.log"):
            shutil.copy(file, evidence / file.name)
        (evidence / "result.json").write_text(json.dumps({
            "result": "passed", "isolated_root": str(root), "download_ids": [held_id, running_id],
            "payload_sha256": hashlib.sha256(PAYLOAD).hexdigest(), "bytes_per_file": len(PAYLOAD),
            "quit_routes": ["Command-Q", "last window close"],
        }, indent=2))
        print(f"Native packaged journey passed. Evidence: {evidence}. Isolated files: {root}")
    finally:
        if app and app.process.poll() is None:
            app.close()
        server.shutdown()
        server.server_close()


if __name__ == "__main__":
    main()
