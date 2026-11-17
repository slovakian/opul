#!/usr/bin/env python3
"""Exercise the disposable preview through Rune's renderer and semantic tree."""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / ".build/rune-preview/evidence"


def capture(name, script=None):
    command = [
        str(ROOT / "Scripts/rune-preview.sh"),
        "--tree", str(OUTPUT / f"{name}.json"),
        "--snap", str(OUTPUT / f"{name}.png"),
        "--at", "1.2", "--scale", "1",
    ]
    if script:
        command.extend(["--script", str(ROOT / "prototypes/rune-baseline/scripts" / script)])
    subprocess.run(command, cwd=ROOT, check=True)
    tree = json.loads((OUTPUT / f"{name}.json").read_text())
    return {element["key"]: element for element in tree["elements"] if "key" in element}


def check_status(elements, status, action):
    assert elements["transfer-status"]["text"] == status, elements["transfer-status"]
    button = elements["toggle-transfer"]
    assert button["role"] == "button" and button["label"] == action, button
    assert button["focusable"] and not button["inert"], button
    assert button["rect"][2] > 0 and button["rect"][3] > 0, button


def main():
    OUTPUT.mkdir(parents=True, exist_ok=True)
    initial = capture("initial")
    check_status(initial, "Downloading", "Pause fixture")
    check_status(capture("paused", "pause.json"), "Paused", "Resume fixture")
    resumed = capture("resumed-dark", "resume.json")
    check_status(resumed, "Downloading", "Pause fixture")
    assert initial["toggle-transfer"]["fill"] != resumed["toggle-transfer"]["fill"]
    for directory in ["data", "Downloads"]:
        assert (OUTPUT.parent / directory).is_dir()
        assert not list((OUTPUT.parent / directory).iterdir()), "Fixtures must not write a real library or transfer."
    print(f"PASS: initial, pause, resume and appearance through Rune; PNGs and trees: {OUTPUT}")


if __name__ == "__main__":
    main()
