"""Audit a copied or mounted macOS app; emit measured package evidence as JSON."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def run(*command):
    return subprocess.check_output(command, text=True).strip()


def version(text):
    parts = tuple(int(part) for part in text.split('.'))
    return parts + (0,) * (3 - len(parts))


def audit(app, arch, dmg=None):
    app = app.resolve(strict=True)
    cpu = 'arm64' if arch == 'arm64' else 'x86_64'
    info = json.loads(run('plutil', '-convert', 'json', '-o', '-', str(app / 'Contents/Info.plist')))
    if version(info['LSMinimumSystemVersion']) > (13, 0, 0):
        raise ValueError('Application Info.plist excludes macOS 13')
    executable = app / 'Contents/MacOS' / info['CFBundleExecutable']
    if not executable.is_file():
        raise ValueError('Missing application executable')
    resources = app / 'Contents/Resources/media-tools'
    subprocess.run([str(ROOT / 'Scripts/verify_ffmpeg.sh'), str(resources), arch], check=True,
                   stdout=subprocess.DEVNULL)
    native = []
    app_bytes = 0
    for path in sorted(app.rglob('*')):
        if path.is_symlink():
            if not path.resolve().is_relative_to(app):
                raise ValueError(f'Link escapes app: {path}')
            continue
        if not path.is_file():
            continue
        app_bytes += path.stat().st_size
        if 'Mach-O' not in run('file', '-b', str(path)):
            continue
        if run('lipo', '-archs', str(path)).split() != [cpu]:
            raise ValueError(f'Architecture mismatch: {path}')
        loads = run('otool', '-l', str(path))
        minimums = re.findall(r'\bminos ([\d.]+)|\bversion ([\d.]+)\n\s+sdk', loads)
        if not minimums or any(version(a or b) > (13, 0, 0) for a, b in minimums):
            raise ValueError(f'Native code excludes macOS 13: {path}: {minimums}')
        subprocess.run(['codesign', '--verify', '--strict', str(path)], check=True)
        native.append({'path': str(path.relative_to(app)),
                       'minimum_macos': [a or b for a, b in minimums],
                       'sha256': hashlib.sha256(path.read_bytes()).hexdigest()})
    if not any(entry['path'] == str(executable.relative_to(app)) for entry in native):
        raise ValueError('Application executable is not Mach-O')
    # Video tooling belongs in Application Support, never in an installer.
    if any(path.name in {'python3', 'python3.13', 'qjs', 'yt-dlp'} for path in app.rglob('*')):
        raise ValueError('Optional video tools found inside application')
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    report = {'app': app.name, 'version': info['CFBundleShortVersionString'],
              'architecture': cpu, 'app_file_bytes': app_bytes, 'native_files': native,
              'audit_host_macos': run('sw_vers', '-productVersion'),
              'minimum_os_evidence': 'Deployment metadata only; run separately on macOS 13.'}
    if dmg is not None:
        report['dmg_bytes'] = dmg.stat().st_size
        report['dmg_sha256'] = hashlib.sha256(dmg.read_bytes()).hexdigest()
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('app', type=Path)
    parser.add_argument('architecture', choices=['arm64', 'x64'])
    parser.add_argument('--dmg', type=Path, help='Measure the image from which this app was copied')
    args = parser.parse_args()
    print(json.dumps(audit(args.app, args.architecture, args.dmg), indent=2))
