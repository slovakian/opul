"""Validate the relocated, signed runtime, then make a permitted-content download."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile

from build import ROOT, TOOLS, fetch
from notices import notice_hashes, verify_notices


def verify(stage, arch, smoke=True):
    stage = stage.resolve()
    lock = json.loads((stage / 'runtime.lock.json').read_text())
    verify_notices(stage, notice_hashes(lock))
    cpu = 'arm64' if arch == 'arm64' else 'x86_64'
    native = []
    for path in sorted(stage.rglob('*')):
        if path.is_symlink():
            if not path.resolve().is_relative_to(stage):
                raise ValueError(f'Symlink escapes runtime: {path}')
            continue
        if not path.is_file():
            continue
        if 'Mach-O' not in subprocess.check_output(['file', '-b', str(path)], text=True):
            continue
        native.append(path)
        slices = subprocess.check_output(['lipo', '-archs', str(path)], text=True).split()
        if slices != [cpu]:
            raise ValueError(f'Wrong architecture: {path}: {slices}')
        loads = subprocess.check_output(['otool', '-l', str(path)], text=True)
        minimums = re.findall(r'\bminos ([\d.]+)|\bversion ([\d.]+)\n\s+sdk', loads)
        if not minimums or any(tuple(int(n) for n in (a or b).split('.')) > (13, 0, 0)
                              for a, b in minimums):
            raise ValueError(f'Unsupported deployment target: {path}: {minimums}')
        dependencies = subprocess.check_output(['otool', '-L', str(path)], text=True).splitlines()[1:]
        for dependency in dependencies:
            name = dependency.strip().split(' (')[0]
            if not name.startswith(('/usr/lib/', '/System/Library/', '@loader_path/', '@rpath/', '@executable_path/')):
                raise ValueError(f'External dependency: {path}: {name}')
        subprocess.run(['codesign', '--verify', '--strict', str(path)], check=True)
    if not native:
        raise ValueError('No native runtime files')
    python = stage / 'bin/python3'
    result = subprocess.run([str(python), '-I', '-c', '''
import importlib.util, ssl, os, sys, zipfile
import curl_cffi, cffi, certifi, brotli, websockets, Cryptodome, pycparser
from Cryptodome.Cipher import AES
assert AES.new(bytes(16), AES.MODE_ECB).encrypt(bytes(16))
assert ssl.get_default_verify_paths().cafile == '/etc/ssl/cert.pem'
assert os.path.isfile(certifi.where())
curl = curl_cffi.Curl()
assert curl._cacert == certifi.where(), 'curl_cffi must use certifi'
curl.close()
assert curl_cffi.aio.DEFAULT_CACERT == certifi.where(), 'AsyncCurl must use certifi'
assert ssl.create_default_context().cert_store_stats()['x509_ca'] > 0
for name in ['pip', 'tkinter', 'idlelib', 'test', 'mutagen', 'requests', 'urllib3']:
    assert importlib.util.find_spec(name) is None, name
assert any(p.endswith('python313.zip') for p in sys.path)
print(sys.version)
print('Python uses macOS CAs; curl_cffi uses', certifi.where())
'''], text=True, capture_output=True)
    if result.returncode:
        raise ValueError(f'Runtime import validation failed: {result.stderr}')
    print(result.stdout, end='')
    if result.stderr:
        raise ValueError(f'Unexpected Python startup diagnostics: {result.stderr}')
    if not (stage / 'lib/python3.13/lib-dynload').is_dir():
        raise ValueError('Missing Python extension directory')
    env = dict(os.environ, QJS_DRIVER=str(stage / 'bin/qjs'))
    subprocess.run([os.environ.get('BUILD_PYTHON', 'python3'), str(TOOLS / 'test_sandbox.py')], env=env, check=True)
    if smoke:
        lock = json.loads((TOOLS / 'runtime.lock.json').read_text())
        cache = ROOT / '.build/python-runtime/smoke'
        cache.mkdir(parents=True, exist_ok=True)
        yt_dlp = fetch(lock['yt_dlp'], cache)
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'trailer.mp4'
            command = [str(python), '-I', str(yt_dlp), '--ignore-config', '--no-plugin-dirs',
                       '--no-cache-dir', '--no-js-runtimes', '--js-runtimes', f'quickjs:{stage / "bin/qjs"}',
                       '--impersonate', 'chrome', '-v', '--no-playlist', '-o', str(output),
                       'https://media.w3.org/2010/05/bunny/trailer.mp4']
            result = subprocess.run(command, text=True, capture_output=True, timeout=180)
            print(result.stdout, end='')
            print(result.stderr, end='')
            if result.returncode or not output.is_file() or output.stat().st_size < 100_000:
                raise ValueError('Permitted-content smoke download failed')
            for required in [f"stable@{lock['yt_dlp']['version']}", 'curl_cffi', 'quickjs-']:
                if required not in result.stderr:
                    raise ValueError(f'Missing health evidence: {required}')
            for forbidden in ['curl_cffi (unsupported)', 'Deprecated Feature: Support for Python',
                              'Could not find platform dependent libraries']:
                if forbidden in result.stderr:
                    raise ValueError(f'Unhealthy pairing: {forbidden}')
            print(f'CC BY 3.0 Big Buck Bunny trailer download: {output.stat().st_size} bytes')
    print(f'Validated {len(native)} signed Mach-O files for {cpu}, macOS <= 13.0')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('runtime', type=Path)
    parser.add_argument('arch', choices=['arm64', 'x64'])
    parser.add_argument('--no-download', action='store_true', help='Local structural check only; publication always downloads')
    args = parser.parse_args()
    verify(args.runtime, args.arch, smoke=not args.no_download)
