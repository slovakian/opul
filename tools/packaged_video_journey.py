"""Run combined acceptance in an actual packaged app with production signed tools.

Media/tracker/peer fixtures are local CC0, except slow.mp4, which is the CC BY 3.0
Blender trailer (https://peach.blender.org/about/). Updates use unmodified runtime 1
installations reconstructed from its immutable signed release; no tool-source or trust overrides.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import time

from journey_fixture import JourneyFixture


def inventory(state):
    checkpoint = state / 'application-state.json'
    if not checkpoint.exists():
        return {}
    result = {}
    for workspace in json.loads(checkpoint.read_text())['state']['workspaces']:
        for download in workspace['downloads']:
            entry = {'folder': download['folder'],
                     'format': download['source'].get('Media', {}).get('format')}
            if download['lifecycle'] == 'Finished':
                path = Path(download['output']['path'])
                entry['output'] = str(path)
                entry['sha256'] = hashlib.sha256(path.read_bytes()).hexdigest()
            result[download['id']] = entry
    return result


def launch(app, root, library, mode, fixture, runtime):
    state = root / library / 'state'
    downloads = root / library / 'downloads'
    state.mkdir(parents=True, exist_ok=True)
    downloads.mkdir(parents=True, exist_ok=True)
    preferences = state / 'desktop-preferences.json'
    previous = inventory(state)
    if not preferences.exists():
        preferences.write_text(json.dumps({'enable_dht': False, 'randomize_port': True}))
    config = root / f'{library}-{mode}-config.json'
    config.write_text(json.dumps({'mode': mode, 'base_url': fixture.base,
        'gate': str(fixture.gate), 'expected_runtime': runtime}))
    report = root / f'{library}-{mode}-report.json'
    ready = root / f'{library}-{mode}-ready.json'
    info = json.loads(subprocess.check_output(['plutil', '-convert', 'json', '-o', '-',
                                               str(app / 'Contents/Info.plist')]))
    executable = app / 'Contents/MacOS' / info['CFBundleExecutable']
    environment = {key: value for key, value in os.environ.items()
                   if key not in {'OPUL_YT_DLP', 'OPUL_SMOKE_VIDEO_RESTART'}}
    environment.update(OPUL_DATA_DIR=str(state), OPUL_DOWNLOAD_DIR=str(downloads),
        OPUL_SMOKE_READY_FILE=str(ready), OPUL_SMOKE_JOURNEY_CONFIG=str(config),
        OPUL_SMOKE_VIDEO_REPORT=str(report), OPUL_SMOKE_VIDEO_URL=fixture.base + '/single.mp4')
    with (root / f'{library}-{mode}.log').open('w') as log:
        process = subprocess.Popen([str(executable)], env=environment, stdout=log, stderr=log)
        try:
            deadline = time.monotonic() + 1200
            while not (report.exists() and ready.exists()):
                if process.poll() is not None or time.monotonic() >= deadline:
                    raise RuntimeError(f'{mode} exited/timed out before reporting: {root}')
                time.sleep(0.1)
            result = json.loads(report.read_text())
            if not result['ok']:
                raise RuntimeError(json.dumps(result))
            if process.wait(timeout=90) != 0:
                raise RuntimeError(f'{mode} did not exit normally')
            checkpoint = json.loads((state / 'application-state.json').read_text())
            if checkpoint['state']['session'] != 'Stopped':
                raise RuntimeError(f'{mode} did not commit ordinary shutdown')
            current = inventory(state)
            for identity, prior in previous.items():
                if identity not in current or any(current[identity].get(key) != value
                                                   for key, value in prior.items()):
                    raise RuntimeError(f'Restart lost Download identity/destination/format/output: {identity}')
            result['prior_downloads_preserved'] = previous
            verified_videos = []
            for workspace in checkpoint['state']['workspaces']:
                for download in workspace['downloads']:
                    media = download['source'].get('Media', {})
                    page = media.get('origin', {}).get('Page', {})
                    path = page.get('item_url', '').removeprefix(fixture.base)
                    if download['lifecycle'] != 'Finished' or path not in {
                            '/single.mp4', '/first.mp4', '/second.mp4', '/success.mp4', '/slow.mp4'}:
                        continue
                    expected = fixture.slow_media if path == '/slow.mp4' else fixture.media
                    digest = hashlib.sha256(expected).hexdigest()
                    if current[download['id']]['sha256'] != digest:
                        raise RuntimeError(f'Completed video bytes differ: {download["id"]}')
                    verified_videos.append({'id': download['id'], 'sha256': digest})
            result['video_bytes_verified'] = verified_videos
            backup = subprocess.check_output(['tmutil', 'isexcluded', str(state / 'Tools')], text=True).strip()
            if result['tools']['state']['kind'] == 'ready' and not backup.startswith('[Excluded]'):
                raise RuntimeError('Installed Tools is not excluded from Time Machine')
            result['time_machine'] = backup
            report.write_text(json.dumps(result, indent=2) + '\n')
            print(f'PASS {library}/{mode}: real app, transfers and ordinary quit', flush=True)
            return result
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


def run(args):
    root = args.output.resolve()
    root.mkdir(parents=True, exist_ok=False)
    fixture = JourneyFixture(root, args.slow_media)
    reports = []
    try:
        if args.scenario in {'all', 'lifecycle', 'offer'}:
            reports.append(launch(args.app, root, 'lifecycle', 'offer', fixture, args.runtime))
            if args.scenario != 'offer':
                reports.append(launch(args.app, root, 'lifecycle', 'setup', fixture, args.runtime))
                reports.append(launch(args.app, root, 'lifecycle', 'restart', fixture, args.runtime))
                if not any(r['path'] == '/slow.mp4' and r['start'] > 0 for r in fixture.ranges):
                    raise RuntimeError('Re-setup did not resume the retained partial with HTTP Range')
        for mode in ['manual', 'automatic']:
            if args.scenario not in {'all', mode}:
                continue
            if args.baseline_tools is None:
                raise ValueError('Updates require --baseline-tools from prepare_journey_baseline.py')
            tools = root / mode / 'state/Tools'
            shutil.copytree(args.baseline_tools, tools, symlinks=True, ignore=shutil.ignore_patterns('tools.lock'))
            subprocess.run(['tmutil', 'addexclusion', str(tools)], check=True)
            state_file = tools / 'state.json'
            state = json.loads(state_file.read_text())
            if state['installed']['runtime'] != 1:
                raise ValueError('Baseline must select the unmodified, previously verified runtime 1')
            state['keep_up_to_date'] = False
            state_file.write_text(json.dumps(state))
            reports.append(launch(args.app, root, mode, mode, fixture, args.runtime))
            reports.append(launch(args.app, root, mode, 'restart', fixture, args.runtime))
        result = {'ok': True, 'app': str(args.app), 'reports': reports,
                  'real_peer_pieces': fixture.pieces_sent, 'http_ranges': fixture.ranges}
        if not fixture.pieces_sent:
            raise RuntimeError('No actual BitTorrent peer traffic was observed')
        (root / 'result.json').write_text(json.dumps(result, indent=2) + '\n')
        print(f'Packaged acceptance passed: {root / "result.json"}', flush=True)
    finally:
        (root / 'fixture-traffic.json').write_text(json.dumps(
            {'pieces': fixture.pieces_sent, 'ranges': fixture.ranges}, indent=2) + '\n')
        fixture.close()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('app', type=lambda p: Path(p).resolve())
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--baseline-tools', type=Path)
    parser.add_argument('--slow-media', type=Path, required=True)
    parser.add_argument('--runtime', type=int, default=2)
    parser.add_argument('--scenario', choices=['all', 'offer', 'lifecycle', 'manual', 'automatic'], default='all')
    run(parser.parse_args())
