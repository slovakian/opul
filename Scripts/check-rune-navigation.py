#!/usr/bin/env python3
"""Verify #104 in the signed bundle: native navigation, narrow windows and relaunch."""
import argparse
import importlib.util
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent.parent
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("rune_app", ROOT / "Scripts/check-rune-app.py")
journey = importlib.util.module_from_spec(spec)
spec.loader.exec_module(journey)


def preferences(root):
    return json.loads((root / "data/navigation-preferences.json").read_text())


def capture(bundle, root, evidence):
    """Renderer captures of the same library; native actions are verified separately above."""
    script = evidence / "open-navigation.json"
    script.write_text(json.dumps({"events": [{"t": 0.5, "type": "tap", "key": "open-navigation"}]}))
    env = os.environ.copy()
    env.update(OPUL_RUNE_DATA_DIR=str(root / "data"), OPUL_RUNE_DOWNLOAD_DIR=str(root / "Downloads"))
    for theme in ("light", "dark"):
        for name, size in (("wide", "1180x760"), ("narrow", "680x650")):
            command = [str(bundle / "Contents/MacOS/opul-rune"), "--snap", str(evidence / f"{name}-{theme}.png"),
                       "--tree", str(evidence / f"{name}-{theme}.json"), "--at", "1.2", "--scale", "1", "--theme", theme, "--size", size]
            if name == "narrow":
                command.extend(["--script", str(script)])
            subprocess.run(command, cwd=root, env=env, check=True, capture_output=True, text=True, timeout=60)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    if not args.skip_build:
        subprocess.run([str(ROOT / "Scripts/package-rune.sh")], cwd=ROOT, check=True)
    evidence = ROOT / ".build/rune/navigation-evidence"
    evidence.mkdir(parents=True, exist_ok=True)
    root = Path(tempfile.mkdtemp(prefix="opul-rune-navigation-")).resolve()
    bundle = root / "Opul Rune.app"
    shutil.copytree(ROOT / ".build/rune/Opul Rune.app", bundle)
    saved = root / "Saved"
    saved.mkdir()
    app = None
    try:
        app = journey.App(bundle, root, "remember")
        assert "Toggle Downloads folders" not in app.tree()
        app.choose_folder(saved)
        journey.wait(lambda: "Saved · 0" in app.tree(), "second folder expands")
        (evidence / "wide.txt").write_text(app.tree())
        app.press("Toggle Downloads folders")
        assert "Saved · 0" not in app.tree()
        assert str(saved) in app.tree(), "disclosure changed contextual destination"
        app.press("Toggle Downloads folders")
        app.script("set size of window 1 to {680, 650}")
        journey.wait(lambda: "button Library" in app.tree(), "narrow navigation trigger")
        app.press("Library")
        journey.wait(lambda: "Saved · 0" in app.tree(), "narrow navigation opens")
        app.press("Toggle Downloads folders")
        assert "Close navigation" in app.tree(), "disclosure closed navigation"
        app.press("Toggle Downloads folders")
        app.press("Downloads actions")
        app.script("key code 53")
        assert "Close navigation" in app.tree(), "Escape dismissed outer navigation before menu"
        app.script("key code 53")
        journey.wait(lambda: "Close navigation" not in app.tree(), "Escape closes navigation")
        app.press("Library")
        app.press("Saved · 0")
        assert "Close navigation" not in app.tree(), "selection did not close navigation"
        assert str(saved) in app.tree()
        (evidence / "narrow.txt").write_text(app.tree())
        app.script("set size of window 1 to {1180, 760}")
        journey.wait(lambda: "Saved · 0" in app.tree(), "wide branch remains expanded")
        app.press("All · 0")
        assert str(root / "Downloads") in app.tree()
        app.press("In progress · 0")
        app.press("Completed · 0")
        app.press("Downloads workspace")
        snapshot = app.close()
        app = None
        workspace_id = snapshot["workspaces"][0]["id"]
        assert preferences(root)["workspace_expansion"][workspace_id]

        app = journey.App(bundle, root, "collapsed-policy")
        journey.wait(lambda: "Saved · 0" in app.tree(), "Remember survives fresh process")
        app.press("Startup: Remember")
        journey.wait(lambda: preferences(root)["workspace_expansion_startup"] == "collapsed", "startup policy saved")
        assert "Saved · 0" in app.tree(), "startup policy changed current expansion"
        app.close()
        app = None

        app = journey.App(bundle, root, "collapsed-start")
        assert "Saved · 0" not in app.tree()
        assert preferences(root)["workspace_expansion"][workspace_id], "collapsed startup destroyed remembered choice"
        temporary = root / "data/navigation-preferences.json.tmp"
        temporary.mkdir()
        app.press("Toggle Downloads folders")
        app.press("Toggle Downloads folders")
        journey.wait(lambda: "Couldn't save workspace expansion" in app.tree(), "real write failure")
        assert preferences(root)["workspace_expansion"][workspace_id]
        temporary.rmdir()
        app.press("Retry navigation preferences")
        journey.wait(lambda: not preferences(root)["workspace_expansion"][workspace_id], "Retry saves latest choice")
        app.press("Toggle Downloads folders")
        app.press("Startup: Collapsed")
        journey.wait(lambda: preferences(root)["workspace_expansion_startup"] == "remember", "remember policy restored")
        (evidence / "recovered.txt").write_text(app.tree())
        app.close(last_window=True)
        app = None

        app = journey.App(bundle, root, "restored")
        journey.wait(lambda: "Saved · 0" in app.tree(), "final remembered expansion")
        final = app.close()
        app = None
        assert final["workspaces"][0]["id"] == workspace_id
        capture(bundle, root, evidence)
        for path in root.glob("*.log"):
            shutil.copy(path, evidence / path.name)
        shutil.copy(root / "data/navigation-preferences.json", evidence / "preferences.json")
        (evidence / "result.json").write_text(json.dumps({
            "result": "passed", "isolated_root": str(root), "workspace_id": workspace_id,
            "native_widths": [680, 1180], "fresh_processes": 4,
            "checks": ["native picker", "independent disclosure", "nested Escape", "narrow selection", "Remember", "Start collapsed", "atomic failure and latest-choice Retry", "Quit and last-window close"],
        }, indent=2))
        print(f"Native navigation passed. Evidence: {evidence}. Isolated library: {root}")
    finally:
        if app and app.process.poll() is None:
            app.close()


if __name__ == "__main__":
    main()
