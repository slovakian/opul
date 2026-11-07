"""Check published yt-dlp/runtime pairs; keep durable results separate from app releases."""
import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
REPOSITORY = 'slovakian/opul'
LEDGER = 'tools-compatibility'
POINTER = f'https://github.com/{REPOSITORY}/releases/download/python-runtime/python-runtime.json'
REPOSITORIES = {'stable': 'yt-dlp/yt-dlp', 'nightly': 'yt-dlp/yt-dlp-nightly-builds'}


def channels_due(schedule, today):
    if not schedule:
        return ['stable', 'nightly']
    if schedule == '41 4 * * 0':
        return ['nightly']
    if schedule == '17 4 * * *' and (today - dt.date(2026, 1, 1)).days % 3 == 0:
        return ['stable']
    return []


def result_name(plan):
    identity = json.dumps(plan, sort_keys=True, separators=(',', ':')).encode()
    return f"compatibility-{plan['channel']}-{plan['arch']}-{hashlib.sha256(identity).hexdigest()}.json"


def already_tested(plan, assets):
    return any(asset['name'] == result_name(plan) for asset in assets)


def gh(*args):
    return subprocess.check_output(['gh', *args, '--repo', REPOSITORY] if args[0] != 'api'
                                   else ['gh', *args], text=True)


def ledger_release():
    result = subprocess.run(['gh', 'api', f'repos/{REPOSITORY}/releases/tags/{LEDGER}'],
                            text=True, capture_output=True)
    if result.returncode:
        if '(HTTP 404)' in result.stderr:
            return None
        raise RuntimeError(result.stderr)
    release = json.loads(result.stdout)
    if not release['prerelease'] or release['draft']:
        raise ValueError('Compatibility results must be a published pre-release')
    pages = json.loads(gh('api', '--paginate', '--slurp',
                         f"repos/{REPOSITORY}/releases/{release['id']}/assets?per_page=100"))
    release['assets'] = [asset for page in pages for asset in page]
    return release


def record_result(path, release, github=gh):
    if release is None:
        github('release', 'create', LEDGER, '--prerelease', '--latest=false', '--title',
               'Video tools compatibility results', '--notes',
               'Tested runtime/yt-dlp pairs. Diagnostics are in the linked workflow artifacts.',
               '--target', os.environ.get('GITHUB_SHA', 'integration/58-opul-managed-tools'))
    github('release', 'upload', LEDGER, str(path))


def report_failure(plan, output, details, run_url, github=gh):
    marker = f'<!-- {result_name(plan)} -->'
    # List all states directly: search indexing can lag after a newly created failure issue.
    pages = json.loads(github('api', '--paginate', '--slurp',
                             f'repos/{REPOSITORY}/issues?state=all&per_page=100'))
    if any(marker in (issue.get('body') or '') for page in pages for issue in page):
        return
    title = f"Video tools: {plan['channel']} yt-dlp {plan['yt_dlp']} fails on runtime {plan['runtime']} ({plan['arch']})"
    body = output / 'failure-issue.md'
    body.write_text(f"{marker}\n## Tested pair\n\nPython runtime {plan['runtime']}\n\n"
                    f"yt-dlp {plan['channel']} {plan['yt_dlp']}\n\nArchitecture: {plan['arch']}\n\n"
                    f"Signed runtime pointer SHA-256: `{plan['pointer_sha256']}`\n\n"
                    f"Workflow and diagnostic artifacts: {run_url}\n\n"
                    f"## Failure\n\n```text\n{details[-12000:]}\n```\n\n"
                    "Download the compatibility artifact for health.log, smoke.log and the signed pointer. "
                    "Reproduce with the runtime and yt-dlp versions above using "
                    "`python3 tools/compatibility/check.py --channel <channel> --arch <arch> --local "
                    "--plan <artifact>/plan.json --output <empty-folder>`. "
                    "Check whether yt-dlp needs newer Python, curl_cffi or QuickJS; publish a new numbered "
                    "runtime if needed. Opul keeps the previous working pair when an update fails.\n")
    github('issue', 'create', '--title', title, '--body-file', str(body))


def fetch(url, path):
    if not url.startswith('https://'):
        raise ValueError('Compatibility downloads require HTTPS')
    with urllib.request.urlopen(url, timeout=120) as response:
        if not response.geturl().startswith('https://'):
            raise ValueError('Insecure redirect')
        path.write_bytes(response.read(100 * 1024 * 1024 + 1))
    if path.stat().st_size > 100 * 1024 * 1024:
        raise ValueError('Oversized tools download')
    return path


def verify_opul(path):
    subprocess.run(['openssl', 'pkeyutl', '-verify', '-rawin', '-pubin', '-inkey',
                    str(ROOT / 'tools/python-runtime/signing-public.pem'), '-in', str(path),
                    '-sigfile', str(path) + '.sig'], check=True, capture_output=True)


def discover(channel, arch, output):
    pointer = fetch(POINTER, output / 'python-runtime.json')
    fetch(POINTER + '.sig', output / 'python-runtime.json.sig')
    verify_opul(pointer)
    manifest = json.loads(pointer.read_bytes())
    if manifest['schema_version'] != 1 or manifest['runtime'] < 1:
        raise ValueError('Unsupported runtime pointer')
    url = f'https://github.com/{REPOSITORIES[channel]}/releases/latest'
    with urllib.request.urlopen(url, timeout=60) as response:
        prefix = f'https://github.com/{REPOSITORIES[channel]}/releases/tag/'
        resolved = response.geturl()
    if not resolved.startswith(prefix):
        raise ValueError('yt-dlp latest did not resolve to its own release tag')
    version = resolved.removeprefix(prefix)
    if not re.fullmatch(r'[A-Za-z0-9][A-Za-z0-9._-]{0,99}', version):
        raise ValueError('Unsafe yt-dlp release tag')
    return dict(channel=channel, arch=arch, runtime=manifest['runtime'], yt_dlp=version,
                pointer_sha256=hashlib.sha256(pointer.read_bytes()).hexdigest()), manifest


def check_health(text, version):
    lines = text.splitlines()
    expected = any(line.startswith('[debug] yt-dlp version ') and
                   any(word.split('@')[-1] == version for word in line.split()) for line in lines)
    curl = any(line.startswith('[debug] Request Handlers: ') and
               'curl_cffi' in [part.strip() for part in line.split(': ', 1)[1].split(',')] for line in lines)
    quickjs = any(line.startswith('[debug] JS runtimes: ') and
                  any(part.strip().startswith('quickjs-') and '(unavailable)' not in part
                      for part in line.split(': ', 1)[1].split(',')) for line in lines)
    python = any(re.match(r'\[debug\] Python \d+\.\d+\.\d+', line) for line in lines)
    if not (expected and curl and quickjs and python) or ('deprecat' in text.lower() and 'python' in text.lower()):
        raise ValueError(f'Unhealthy yt-dlp {version}: expected version, Python, supported curl_cffi and QuickJS\n{text}')


def run_logged(command, log):
    result = subprocess.run(command, text=True, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=240)
    with log.open('a') as file:
        file.write(f'$ {" ".join(map(str, command))}\n{result.stdout}\n')
    if result.returncode:
        raise RuntimeError(f'Exit {result.returncode}: {log.name}\n{result.stdout}')
    return result.stdout


def test_pair(plan, manifest, output):
    asset = manifest['architectures'][plan['arch']]
    prefix = f"https://github.com/{REPOSITORY}/releases/download/python-runtime-{plan['runtime']}/"
    if not asset['url'].startswith(prefix) or asset['signature_url'] != asset['url'] + '.sig':
        raise ValueError('Runtime must belong to its numbered immutable release')
    archive = fetch(asset['url'], output / 'runtime.tar.gz')
    fetch(asset['signature_url'], output / 'runtime.tar.gz.sig')
    verify_opul(archive)
    if archive.stat().st_size != asset['size'] or hashlib.sha256(archive.read_bytes()).hexdigest() != asset['sha256']:
        raise ValueError('Runtime checksum or size mismatch')
    runtime = output / 'runtime'
    with tarfile.open(archive) as tar:
        if sum(member.size for member in tar.getmembers()) > 256 * 1024 * 1024:
            raise ValueError('Oversized expanded runtime')
        tar.extractall(runtime, filter='data')
    base = f"https://github.com/{REPOSITORIES[plan['channel']]}/releases/download/{plan['yt_dlp']}"
    sums = fetch(base + '/SHA2-256SUMS', output / 'SHA2-256SUMS')
    signature = fetch(base + '/SHA2-256SUMS.sig', output / 'SHA2-256SUMS.sig')
    keyring = output / 'yt-dlp.gpg'
    run_logged(['gpg', '--no-options', '--batch', '--yes', '--dearmor', '--output', str(keyring),
                str(ROOT / 'src/video_tools/keys/yt-dlp.asc')], output / 'verification.log')
    run_logged(['gpgv', '--keyring', str(keyring), str(signature), str(sums)], output / 'verification.log')
    hashes = [match[1] for line in sums.read_text().splitlines()
              if (match := re.fullmatch(r'([0-9a-fA-F]{64})\s+\*?yt-dlp', line))]
    if len(hashes) != 1:
        raise ValueError('Missing or ambiguous bare-script checksum')
    script = fetch(base + '/yt-dlp', output / 'yt-dlp')
    if hashlib.sha256(script.read_bytes()).hexdigest() != hashes[0].lower():
        raise ValueError('yt-dlp checksum mismatch')
    cpu = 'arm64' if plan['arch'] == 'arm64' else 'x86_64'
    run_logged([str(runtime / 'bin/python3'), '-I', '-c',
                f'import platform, sys; print(sys.version); print(platform.machine()); assert platform.machine() == {cpu!r}'],
               output / 'health.log')
    command = [str(runtime / 'bin/python3'), '-I', str(script), '--ignore-config', '--no-plugin-dirs',
               '--no-cache-dir', '--no-update', '--impersonate', 'chrome', '--no-js-runtimes',
               '--js-runtimes', f'quickjs:{runtime / "bin/qjs"}', '--ffmpeg-location',
               str(ROOT / 'src-tauri/resources/media-tools')]
    version = run_logged([*command, '--version'], output / 'health.log').strip()
    if version != plan['yt_dlp']:
        raise ValueError(f'Unexpected yt-dlp --version: {version}')
    text = run_logged([*command, '--verbose', '--list-impersonate-targets'], output / 'health.log')
    check_health(text, plan['yt_dlp'])
    # Blender Big Buck Bunny trailer, CC BY 3.0, via W3C's public media sample.
    download = output / 'trailer.mp4'
    run_logged([*command, '--verbose', '--no-playlist', '-o', str(download),
                'https://media.w3.org/2010/05/bunny/trailer.mp4'], output / 'smoke.log')
    if not download.is_file() or download.stat().st_size < 100_000:
        raise ValueError('Permitted-content smoke output is missing or too small')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--channel', choices=REPOSITORIES, required=True)
    parser.add_argument('--arch', choices=['arm64', 'x64'], required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--local', action='store_true', help='Run without reading or writing GitHub results/issues')
    parser.add_argument('--plan', type=Path, help='Reproduce the exact pair from a previous diagnostic artifact')
    args = parser.parse_args()
    schedule = os.environ.get('COMPATIBILITY_SCHEDULE', '')
    if not args.local and args.channel not in channels_due(schedule, dt.datetime.now(dt.timezone.utc).date()):
        print('This channel is not due.')
        return
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if args.plan:
        plan = json.loads(args.plan.read_text())
        if plan['channel'] != args.channel or plan['arch'] != args.arch:
            raise ValueError('Plan channel and architecture must match the command')
        for name in ['python-runtime.json', 'python-runtime.json.sig']:
            shutil.copyfile(args.plan.parent / name, output / name)
        pointer = output / 'python-runtime.json'
        verify_opul(pointer)
        if hashlib.sha256(pointer.read_bytes()).hexdigest() != plan['pointer_sha256']:
            raise ValueError('Plan does not match the signed runtime pointer')
        manifest = json.loads(pointer.read_bytes())
        if manifest['runtime'] != plan['runtime']:
            raise ValueError('Plan does not match the published runtime')
    else:
        plan, manifest = discover(args.channel, args.arch, output)
    (output / 'plan.json').write_text(json.dumps(plan, indent=2) + '\n')
    release = None if args.local else ledger_release()
    if release and already_tested(plan, release.get('assets', [])):
        print(f'Already tested: {result_name(plan)}')
        return
    run_url = f"https://github.com/{REPOSITORY}/actions/runs/{os.environ.get('GITHUB_RUN_ID', 'local')}"
    result = dict(plan, run_url=run_url, tested_at=dt.datetime.now(dt.timezone.utc).isoformat())
    try:
        test_pair(plan, manifest, output)
        result['ok'] = True
    except (urllib.error.URLError, TimeoutError) as error:
        # Infrastructure trouble leaves this pair eligible for the next scheduled run.
        (output / 'infrastructure-error.log').write_text(str(error))
        raise
    except (ValueError, RuntimeError, subprocess.SubprocessError) as error:
        result.update(ok=False, error=str(error))
        if not args.local:
            report_failure(plan, output, str(error), run_url)
    result_path = output / result_name(plan)
    result_path.write_text(json.dumps(result, indent=2) + '\n')
    if not args.local:
        record_result(result_path, release)
    print(result_path.read_text())
    if not result['ok']:
        raise SystemExit(1)


if __name__ == '__main__':
    main()
