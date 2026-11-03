"""Dedicated maintainer build. App CI never calls this recipe."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tomllib
import time
import urllib.parse
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[2]
TOOLS = ROOT / 'tools/python-runtime'


def fetch(pin, cache):
    name = urllib.parse.unquote(pin['url'].rsplit('/', 1)[1])
    path = cache / name
    if not path.exists() or hashlib.sha256(path.read_bytes()).hexdigest() != pin['sha256']:
        for attempt in range(5):
            try:
                with urllib.request.urlopen(pin['url'], timeout=120) as response:
                    data = response.read()
                break
            except OSError:
                if attempt == 4:
                    raise
                time.sleep(2 ** attempt)
        if hashlib.sha256(data).hexdigest() != pin['sha256']:
            raise ValueError(f'Checksum mismatch: {name}')
        path.write_bytes(data)
    return path


def build(arch, output):
    lock = json.loads((TOOLS / 'runtime.lock.json').read_text())
    pin = lock['architectures'][arch]
    cpu = 'arm64' if arch == 'arm64' else 'x86_64'
    work = ROOT / '.build/python-runtime' / arch
    cache = work / 'downloads'
    cache.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    stage = output / arch
    if stage.exists():
        shutil.rmtree(stage)
    # These exact PBS archives are upstream cross builds for macOS 11+, including
    # Intel. We do not rebuild CPython. Only the QuickJS driver is compiled here.
    with tarfile.open(fetch(pin['python'], cache)) as tar:
        tar.extractall(work, filter='data')
    shutil.move(work / 'python', stage)
    site = stage / 'lib/python3.13/site-packages'
    shutil.rmtree(site)
    site.mkdir()
    licenses = stage / 'licenses'
    licenses.mkdir(exist_ok=True)
    shutil.copyfile(stage / 'lib/python3.13/LICENSE.txt', licenses / 'Python-LICENSE.txt')
    shutil.copyfile(fetch(lock['python_licenses'], cache), licenses / 'Python-dependencies.rst')
    with tarfile.open(fetch(lock['quickjs'], cache)) as tar:
        tar.extractall(work, filter='data')
    source = work / f"quickjs-{lock['quickjs']['version']}"
    sdk = subprocess.check_output(['xcrun', '--sdk', 'macosx', '--show-sdk-path'], text=True).strip()
    command = ['xcrun', 'clang', '-arch', cpu, '-isysroot', sdk,
               '-mmacosx-version-min=13.0', '-Os', '-D_GNU_SOURCE',
               f'-ffile-prefix-map={ROOT}=/opul/source',
               f'-DCONFIG_VERSION="{lock["quickjs"]["version"]}"', '-I', str(source),
               str(TOOLS / 'qjs-driver.c')]
    command.extend(str(source / f'{name}.c') for name in ['quickjs', 'dtoa', 'libregexp', 'libunicode', 'cutils'])
    subprocess.run([*command, '-lm', '-o', str(stage / 'bin/qjs')], check=True)
    shutil.copyfile(source / 'LICENSE', licenses / 'QuickJS-LICENSE.txt')
    pins = fetch({'url':lock['yt_dlp']['pins_url'], 'sha256':lock['yt_dlp']['pins_sha256']}, cache)
    upstream = tomllib.loads(pins.read_text())['project']['optional-dependencies']
    requirements = upstream['pin'] + upstream['pin-curl-cffi']
    for wheel in pin['wheels']:
        normalized = wheel['package'].replace('_', '-')
        if not any(r.split(';')[0].strip() == f"{normalized}=={wheel['version']}" for r in requirements):
            raise ValueError(f"Wheel differs from yt-dlp pins: {wheel['package']}")
        archive = fetch(wheel, cache)
        if archive.suffix != '.whl':
            raise ValueError('Only prebuilt wheels can be installed')
        # Wheel extraction never runs pip, setup.py, entry points or install scripts.
        with zipfile.ZipFile(archive) as z:
            for name in z.namelist():
                if Path(name).is_absolute() or '..' in Path(name).parts or '.data/' in name or name.endswith('.pth'):
                    raise ValueError(f'Unsupported wheel member: {name}')
            z.extractall(site)
    python = stage / 'bin/python3'
    subprocess.run([str(python), '-I', str(TOOLS / 'prepare.py'), str(stage)], check=True)
    # Thin every universal wheel before signing. Preserve only this Mac's slice.
    for path in sorted(stage.rglob('*')):
        if path.is_symlink() or not path.is_file():
            continue
        kind = subprocess.check_output(['file', '-b', str(path)], text=True)
        if 'Mach-O' not in kind:
            continue
        slices = subprocess.check_output(['lipo', '-archs', str(path)], text=True).split()
        if cpu not in slices:
            raise ValueError(f'Wrong architecture: {path}')
        if len(slices) > 1:
            subprocess.run(['lipo', str(path), '-thin', cpu, '-output', str(path) + '.thin'], check=True)
            path.with_name(path.name + '.thin').replace(path)
        subprocess.run(['strip', '-x', str(path)], check=True)
    shutil.copyfile(TOOLS / 'runtime.lock.json', stage / 'runtime.lock.json')
    (stage / 'BUILD-INFO.json').write_text(json.dumps({
        'recipe_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        'compiler': subprocess.check_output(['xcrun', 'clang', '--version'], text=True).replace(str(ROOT), '/opul/source'),
        'sdk_version': subprocess.check_output(['xcrun', '--sdk', 'macosx', '--show-sdk-version'], text=True).strip(),
        'deployment_target': '13.0',
        'architecture': cpu,
    }, indent=2) + '\n')
    (stage / 'SOURCE.txt').write_text(
        f"Opul Python runtime {lock['runtime']}; macOS 13+.\n"
        'Python: https://github.com/astral-sh/python-build-standalone\n'
        'QuickJS: https://bellard.org/quickjs/ (unmodified engine, custom Opul driver).\n'
        'Wheels: PyPI, exact versions and hashes in runtime.lock.json.\n'
        'certifi is MPL-2.0, https://github.com/certifi/python-certifi (unmodified).\n'
        f"Build recipe: https://github.com/slovakian/opul/tree/python-runtime-{lock['runtime']}/tools/python-runtime\n"
        'PSF and bundled library licenses: licenses/; wheel licenses: lib/python3.13/site-packages/*.dist-info/.\n')
    yt_dlp = fetch(lock['yt_dlp'], cache)
    print(f'Prepared {stage}; pinned smoke-test script: {yt_dlp}', flush=True)


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('arch', choices=['arm64', 'x64'])
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    build(args.arch, args.output.resolve())
