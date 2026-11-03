"""Sign and publish tested runtimes. Tools releases always refuse Latest."""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tarfile
import tempfile
import urllib.request

from build import TOOLS, fetch
from verify import verify


def release_flags(tag):
    if not re.fullmatch(r'python-runtime(?:-[1-9][0-9]*)?', tag):
        raise ValueError(f'Refusing non-runtime release: {tag}')
    return ['--prerelease', '--latest=false']


def sign_file(path, key):
    subprocess.run(['openssl', 'pkeyutl', '-sign', '-rawin', '-inkey', str(key),
                    '-in', str(path), '-out', str(path) + '.sig'], check=True)


def verify_file(path, key):
    subprocess.run(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin', '-inkey', str(key),
                    '-in', str(path), '-sigfile', str(path) + '.sig'], check=True, capture_output=True)


def manifest_for(output, repository, lock):
    tag = f"python-runtime-{lock['runtime']}"
    release_flags(tag)
    architectures = {}
    for arch in ['arm64', 'x64']:
        archive = output / f'{tag}-{arch}.tar.gz'
        url = f'https://github.com/{repository}/releases/download/{tag}/{archive.name}'
        architectures[arch] = {'url': url, 'sha256':hashlib.sha256(archive.read_bytes()).hexdigest(),
                               'size':archive.stat().st_size, 'signature_url':url + '.sig'}
    return {'schema_version':1, 'runtime':lock['runtime'],
            'minimum_opul_version':lock['minimum_opul_version'], 'architectures':architectures}


def pack(stage, destination):
    # Normalize archive metadata and gzip header. Signatures on Developer ID builds
    # carry Apple timestamps; byte-for-byte reproduction applies to ad-hoc builds
    # with identical pinned inputs and the same SDK/compiler.
    def normalize(info):
        info.uid = info.gid = 0
        info.uname = info.gname = ''
        info.mtime = 0
        info.pax_headers = {}
        info.mode = 0o755 if info.isdir() or info.mode & 0o111 else 0o644
        return info
    temporary = destination.with_name(destination.name + '.part')
    try:
        with temporary.open('wb') as raw:
            with gzip.GzipFile(filename='', mode='wb', fileobj=raw, mtime=0, compresslevel=9) as compressed:
                with tarfile.open(fileobj=compressed, mode='w', format=tarfile.PAX_FORMAT) as tar:
                    tar.add(stage, arcname='.', filter=normalize)
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def prepare(output, key):
    lock = json.loads((TOOLS / 'runtime.lock.json').read_text())
    tag = f"python-runtime-{lock['runtime']}"
    for arch in ['arm64', 'x64']:
        archive = output / f'{tag}-{arch}.tar.gz'
        if not archive.exists():
            pack(output / arch, archive)
        # Test exactly the bytes that will be published, in a relocated directory.
        with tempfile.TemporaryDirectory(prefix=f'opul-runtime-{arch}-') as folder:
            with tarfile.open(archive) as tar:
                tar.extractall(folder, filter='data')
            stage = Path(folder)
            if json.loads((stage / 'runtime.lock.json').read_text()) != lock:
                raise ValueError('Archive was built with different runtime pins')
            verify(stage, arch)
    pointer = output / 'python-runtime.json'
    pointer.write_text(json.dumps(manifest_for(output, 'slovakian/opul', lock), indent=2) + '\n')
    sums = output / 'SHA256SUMS'
    sums.write_text(''.join(f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
                           for p in sorted(output.glob(f'{tag}-*.tar.gz'))))
    artifacts = [*sorted(output.glob(f'{tag}-*.tar.gz')), pointer, sums]
    for artifact in artifacts:
        sign_file(artifact, key)
        # A mismatched or missing Opul key can never publish, even with Apple ID.
        verify_file(artifact, TOOLS / 'signing-public.pem')
    return tag, artifacts


def publish(output, tag, artifacts):
    repository = 'slovakian/opul'
    base = ['gh', 'release']
    repo = ['--repo', repository]
    existing = subprocess.run([*base, 'view', tag, *repo], capture_output=True)
    if existing.returncode == 0:
        raise ValueError(f'Immutable release already exists: {tag}; increment runtime number')
    # Refuse to overwrite even a draft. Auth/network errors also fail gh create.
    files = [str(path) for artifact in artifacts
             for path in [artifact, artifact.with_name(artifact.name + '.sig')]]
    files += [str(TOOLS / 'signing-public.pem')]
    files += [str(p) for p in sorted(output.glob('verification-*.log'))]
    target = os.environ.get('GITHUB_SHA') or subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    notes = output / 'release-notes.md'
    notes.write_text(
        f'Python runtime {tag.removeprefix("python-runtime-")} for macOS 13+.\n\n'
        'Two single-architecture archives, independently signed by Opul with Ed25519. '
        'Raw 64-byte detached signatures cover the exact archive/JSON/checksum-file bytes.\n\n'
        'Licenses and pinned build inputs are inside each archive. Verification and permitted-content download logs are attached. '
        'Build and verification instructions: https://github.com/slovakian/opul/blob/integration/58-opul-managed-tools/docs/python-runtime.md\n')
    subprocess.run([*base, 'create', tag, *files, *repo, '--target', target, '--title', f'Python runtime {tag}',
                    '--notes-file', str(notes), '--draft', *release_flags(tag)], check=True)
    subprocess.run([*base, 'edit', tag, *repo, '--draft=false', *release_flags(tag)], check=True)
    # API-free retrieval proves signatures, exact sizes and hashes on published bytes
    # before advertising this runtime through the mutable pointer.
    manifest = json.loads((output / 'python-runtime.json').read_text())
    with tempfile.TemporaryDirectory() as folder:
        cache = Path(folder)
        for entry in manifest['architectures'].values():
            archive = fetch(entry, cache)
            if archive.stat().st_size != entry['size']:
                raise ValueError('Published size mismatch')
            with urllib.request.urlopen(entry['signature_url'], timeout=120) as response:
                archive.with_name(archive.name + '.sig').write_bytes(response.read())
            verify_file(archive, TOOLS / 'signing-public.pem')
    pointer_tag = 'python-runtime'
    if subprocess.run([*base, 'view', pointer_tag, *repo], capture_output=True).returncode:
        subprocess.run([*base, 'create', pointer_tag, *repo, '--target', target,
                        '--title', 'Current Python runtime', '--notes', 'Signed runtime discovery pointer. See docs/python-runtime.md.',
                        *release_flags(pointer_tag)], check=True)
    else:
        subprocess.run([*base, 'edit', pointer_tag, *repo, *release_flags(pointer_tag)], check=True)
    subprocess.run([*base, 'upload', pointer_tag, str(output / 'python-runtime.json'),
                    str(output / 'python-runtime.json.sig'), str(TOOLS / 'signing-public.pem'), *repo, '--clobber'], check=True)
    with tempfile.TemporaryDirectory() as folder:
        url = f'https://github.com/{repository}/releases/download/python-runtime/python-runtime.json'
        pointer = fetch({'url':url, 'sha256':hashlib.sha256((output / 'python-runtime.json').read_bytes()).hexdigest()}, Path(folder))
        with urllib.request.urlopen(url + '.sig', timeout=120) as response:
            pointer.with_name(pointer.name + '.sig').write_bytes(response.read())
        verify_file(pointer, TOOLS / 'signing-public.pem')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output', type=Path)
    parser.add_argument('--key', required=True, type=Path, help='Private Ed25519 PEM key; never printed')
    parser.add_argument('--publish', action='store_true', help='Publish immutable release and replace signed pointer')
    args = parser.parse_args()
    tag, artifacts = prepare(args.output.resolve(), args.key.resolve())
    if args.publish:
        publish(args.output.resolve(), tag, artifacts)
