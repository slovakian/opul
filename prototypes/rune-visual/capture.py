#!/usr/bin/env python3
"""Disposable evidence recipe. Real Rune dispatch/rendering, no production test contract."""
import argparse
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
PROTOTYPE = Path(__file__).resolve().parent
BINARY = ROOT / "target/rune-preview/debug/opul-rune-visual"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / ".build/rune-visual/evidence")
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    subprocess.run([
        "cargo", "build", "--locked", "--manifest-path", str(PROTOTYPE / "Cargo.toml"),
        "--target-dir", str(ROOT / "target/rune-preview"), "-j", "2",
    ], cwd=ROOT, check=True)

    def capture(name, scene="cards", theme="light", size="1180x760", script=None, at=1.2):
        command = [
            str(BINARY), "--scene", scene, "--theme", theme, "--size", size,
            "--scale", "1", "--at", str(at),
            "--snap", str(output / f"{name}.png"), "--tree", str(output / f"{name}.json"),
        ]
        if script:
            command.extend(["--script", str(script)])
        subprocess.run(command, cwd=ROOT, check=True, stdout=subprocess.DEVNULL)
        tree = json.loads((output / f"{name}.json").read_text())
        assert not tree["warnings"], tree["warnings"]
        elements = {e["key"]: e for e in tree["elements"] if "key" in e}
        for key in ["previous", "next", "appearance", "motion", "folders", "transfer-state"]:
            control = elements[key]
            assert control["role"] == "button" and (control["focusable"] or control["inert"]), control
            x, y, w, h = control["rect"]
            viewport = elements["root"]["rect"]
            assert w > 0 and h > 0 and x >= 0 and y >= 0, control
            assert x + w <= viewport[2] + 1 and y + h <= viewport[3] + 1, control
        return elements

    def scripted(name, events, **kwargs):
        path = output / f"{name}-input.json"
        path.write_text(json.dumps({"events": events}, indent=2) + "\n")
        return capture(name, script=path, **kwargs)

    def tap(key, t=0.2):
        return {"t": t, "type": "tap", "key": key}

    # Same fixtures across every treatment, appearance and width.
    sheet_script = PROTOTYPE / "scripts/sheet.json"
    for scene in ["cards", "list", "panels"]:
        for theme in ["light", "dark"]:
            initial = capture(f"{scene}-{theme}", scene, theme)
            capture(f"{scene}-{theme}-narrow", scene, theme, "600x720")
            opened = capture(f"{scene}-{theme}-sheet", scene, theme, script=sheet_script)
            assert initial["download-list"]["rect"][2] == opened["download-list"]["rect"][2]
            assert opened["details-sheet"]["role"] == "dialog"
            assert opened["application"]["blur"] > 0
            assert opened["pause"]["inert"] and not opened["close-sheet"]["inert"]

    controls = capture("controls", script=PROTOTYPE / "scripts/controls.json", at=2.4)
    assert controls["variant"]["text"] == "B · Quiet line"
    assert controls["status-0"]["label"] == "Paused · 42%"
    assert "disclosure" not in controls
    assert controls["presentation"]["label"] == "Minimal"
    assert controls["root"]["fill"] != json.loads((output / "cards-light.json").read_text())["elements"][0]["fill"]
    assert "upload-mask" not in controls

    # Distinct transfer states, and restart after failure, through the visible State control.
    for count, expected in [(1, "Paused · 42%"), (2, "Completed"), (3, "Connection lost"), (4, "Downloading · 42%")]:
        state = scripted(f"transfer-{count}", [tap("transfer-state", 0.1 + n * 0.2) for n in range(count)])
        assert state["status-0"]["label"] == expected

    # Modal input is consumed. A second click is required to select a different Download.
    outside = [tap("download-0"), tap("download-1", 0.6)]
    closed = scripted("outside-consumed", outside)
    assert "details-sheet" not in closed
    assert closed["download-0"]["focused"]
    other = scripted("second-click", outside + [tap("download-1", 0.9)], at=1.5)
    assert other["details-sheet"]["label"] == "Details: Field recordings.zip"
    escaped = scripted("escape", [tap("download-0"), {"t": 0.6, "type": "key", "name": "Escape"}])
    assert "details-sheet" not in escaped and escaped["download-0"]["focused"]
    keyed = scripted("keyboard-cycle", [{"t": 0.2, "type": "key", "name": "Left"}])
    assert keyed["variant"]["text"] == "C · Inset panel"
    searched = scripted("search", [tap("search"), {"t": 0.4, "type": "type", "text": "Brand"}, {"t": 0.8, "type": "key", "name": "Left"}], at=1.5)
    assert "row-3" in searched and "row-0" not in searched
    assert searched["variant"]["text"] == "A · Soft edge"
    narrow = scripted("narrow-sidebar", [tap("show-sidebar")], size="600x720")
    assert "narrow-sidebar" in narrow and narrow["next"]["inert"]
    single = scripted("single-folder", [tap("folders")])
    assert "workspace-studio" in single and "disclosure" not in single and "folder-audio" not in single
    collapsed = scripted("collapsed-folders", [tap("disclosure")])
    assert "folder-audio" not in collapsed and "workspace-studio" in collapsed
    focused = scripted("focus", [{"t": 0.2, "type": "key", "name": "Tab"}])
    assert any(element.get("focused") and element.get("focusVisible") for element in focused.values())
    trapped = scripted("focus-trap", [tap("download-0"), *[{"t": 0.5 + n * 0.1, "type": "key", "name": "Tab"} for n in range(7)]], at=1.8)
    assert trapped["close-sheet"]["focused"] or trapped["sheet-pause"]["focused"]

    # Timed native-renderer evidence, including a complete six-second determinate sweep.
    for name, extra in [
        ("effects", []),
        ("sheet-motion", ["--script", str(PROTOTYPE / "scripts/sheet-motion.json")]),
        ("reduced", ["--script", str(output / "reduced-input.json")]),
    ]:
        if name == "reduced":
            (output / "reduced-input.json").write_text(json.dumps({"events": [tap("motion", 0.1)]}))
        subprocess.run([
            str(BINARY), "--theme", "dark", "--scale", "1", "--size", "1180x760",
            "--frames", "105", "--fps", "15", "--video", str(output / f"{name}.mp4"),
            "--trace", str(output / f"{name}.jsonl"), *extra,
        ], cwd=ROOT, check=True, stdout=subprocess.DEVNULL)

    records = [json.loads(line) for line in (output / "effects.jsonl").read_text().splitlines()]
    names = {item["id"]: item["nm"].split("/")[-1] for record in records for item in record.get("m", [])}
    current = {}
    samples = []
    for record in records:
        for element in record.get("e", []):
            current[names[element[0]]] = element[1:5]
        if record.get("t") in [600.0, 1200.0, 2000.0]:
            samples.append({"milliseconds": record["t"], **{
                key: current[key] for key in ["progress-0-fill", "progress-0-empty", "progress-0-sheen", "progress-1-sheen", "upload-mask", "upload-copy"]
            }})
    assert len(samples) == 3
    assert samples[0]["progress-0-sheen"][0] > samples[-1]["progress-0-sheen"][0]
    assert samples[0]["progress-1-sheen"][0] < samples[-1]["progress-1-sheen"][0]
    assert samples[0]["upload-mask"][1] > samples[-1]["upload-mask"][1]
    assert samples[0]["upload-copy"] == samples[-1]["upload-copy"]
    assert samples[0]["progress-0-fill"] == samples[-1]["progress-0-fill"]
    reduced_records = [json.loads(line) for line in (output / "reduced.jsonl").read_text().splitlines()]
    assert not reduced_records[-1]["quality"]["animating"]
    (output / "motion-summary.json").write_text(json.dumps({"samples": samples, "reduced_settled": True}, indent=2) + "\n")
    print(f"Captured treatments, controls, modal interactions and 7s motion: {output}")


if __name__ == "__main__":
    main()
