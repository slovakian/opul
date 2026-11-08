"""Materialize an authenticated, immutable runtime-1 pair for update acceptance.

This does not change the production pointer. New app installations still fetch runtime 2.
Run with OpenSSL 3 and gpg/gpgv on PATH. Archive bytes are verified before extraction.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[1]


def fetch(url, path):
    with urllib.request.urlopen(url, timeout=120) as response:
        path.write_bytes(response.read())
    return path


def verify(path):
    subprocess.run(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin',
        '-inkey', str(ROOT / 'tools/python-runtime/signing-public.pem'),
        '-in', str(path), '-sigfile', str(path) + '.sig'], check=True)


def prepare(arch, output):
    output.mkdir(parents=True, exist_ok=False)
    release = 'https://github.com/slovakian/opul/releases/download/python-runtime-1/'
    pointer = fetch(release + 'python-runtime.json', output / 'python-runtime.json')
    fetch(release + 'python-runtime.json.sig', output / 'python-runtime.json.sig')
    verify(pointer)
    manifest = json.loads(pointer.read_text())
    if manifest['runtime'] != 1 or manifest['schema_version'] != 1:
        raise ValueError('Not the immutable runtime 1 manifest')
    entry = manifest['architectures'][arch]
    archive = fetch(entry['url'], output / 'runtime.tar.gz')
    fetch(entry['signature_url'], output / 'runtime.tar.gz.sig')
    verify(archive)
    if archive.stat().st_size != entry['size'] or hashlib.sha256(archive.read_bytes()).hexdigest() != entry['sha256']:
        raise ValueError('Runtime archive size/checksum mismatch')
    tools = output / 'Tools'
    runtime = tools / 'python-runtime/1'
    runtime.mkdir(parents=True)
    with tarfile.open(archive) as tar:
        tar.extractall(runtime, filter='data')
    lock = json.loads((runtime / 'runtime.lock.json').read_text())
    tag = lock['yt_dlp']['version']
    upstream = f'https://github.com/yt-dlp/yt-dlp/releases/download/{tag}/'
    sums = fetch(upstream + 'SHA2-256SUMS', output / 'SHA2-256SUMS')
    signature = fetch(upstream + 'SHA2-256SUMS.sig', output / 'SHA2-256SUMS.sig')
    keyring = output / 'yt-dlp.gpg'
    subprocess.run(['gpg', '--batch', '--yes', '--dearmor', '--output', str(keyring),
                    str(ROOT / 'src/video_tools/keys/yt-dlp.asc')], check=True)
    subprocess.run(['gpgv', '--keyring', str(keyring.resolve()), str(signature), str(sums)], check=True)
    script = tools / 'yt-dlp' / tag / 'yt-dlp'
    script.parent.mkdir(parents=True)
    fetch(upstream + 'yt-dlp', script)
    expected = next(line.split()[0] for line in sums.read_text().splitlines()
                    if line.split()[-1] == 'yt-dlp')
    if hashlib.sha256(script.read_bytes()).hexdigest() != expected:
        raise ValueError('Bare script checksum mismatch')
    (tools / 'state.json').write_text(json.dumps({'installed': {
        'runtime': 1, 'python_version': lock['python_version'], 'yt_dlp': tag,
        'yt_dlp_channel': 'stable'}, 'yt_dlp_channel': 'stable',
        'keep_up_to_date': False, 'turned_off': False}) + '\n')
    subprocess.run([str(runtime / 'bin/python3'), '-I', str(script), '--version'], check=True)
    subprocess.run(['tmutil', 'addexclusion', str(tools)], check=True)
    print(f'Authenticated baseline: {tools}', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('architecture', choices=['arm64', 'x64'])
    parser.add_argument('output', type=lambda p: Path(p).resolve())
    args = parser.parse_args()
    prepare(args.architecture, args.output)
