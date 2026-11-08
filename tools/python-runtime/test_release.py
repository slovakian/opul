"""Manifest and signature contract consumed by the managed video-tools installer."""
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from release import manifest_for, pack, release_flags, sign_file, verify_file


class ReleaseTests(unittest.TestCase):
    def test_archives_are_reproducible_across_file_timestamps(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            stage = output / 'runtime'
            stage.mkdir()
            program = stage / 'python'
            program.write_bytes(b'pinned runtime bytes')
            program.chmod(0o755)
            (stage / 'python3').symlink_to('python')
            first, second = output / 'first.tar.gz', output / 'second.tar.gz'
            pack(stage, first)
            os.utime(program, (1234567890, 1234567890))
            os.utime(stage, (1234567890, 1234567890))
            pack(stage, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())

    def test_exact_sizes_hashes_and_immutable_urls(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            for arch in ['arm64', 'x64']:
                (output / f'python-runtime-7-{arch}.tar.gz').write_bytes(arch.encode() * 137)
            manifest = manifest_for(output, 'slovakian/opul', {'runtime':7, 'minimum_opul_version':'0.2.2'})
            self.assertEqual(manifest['runtime'], 7)
            self.assertEqual(manifest['minimum_opul_version'], '0.2.2')
            for arch, entry in manifest['architectures'].items():
                data = (output / f'python-runtime-7-{arch}.tar.gz').read_bytes()
                self.assertEqual(entry['size'], len(data))
                self.assertEqual(entry['sha256'], hashlib.sha256(data).hexdigest())
                self.assertEqual(entry['url'], f'https://github.com/slovakian/opul/releases/download/python-runtime-7/python-runtime-7-{arch}.tar.gz')
                self.assertEqual(entry['signature_url'], entry['url'] + '.sig')

    def test_tools_releases_cannot_be_latest_or_app_releases(self):
        for tag in ['python-runtime', 'python-runtime-1', 'python-runtime-17']:
            self.assertIn('--latest=false', release_flags(tag))
            self.assertIn('--prerelease', release_flags(tag))
        for tag in ['v0.2.2', 'latest', 'python-runtime-0', 'python-runtime-1/evil']:
            with self.assertRaises(ValueError):
                release_flags(tag)

    def test_opul_signatures_reject_tampering_and_wrong_keys(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            private = output / 'private.pem'
            public = output / 'public.pem'
            subprocess.run(['openssl', 'genpkey', '-algorithm', 'ED25519', '-out', str(private)], check=True)
            subprocess.run(['openssl', 'pkey', '-in', str(private), '-pubout', '-out', str(public)], check=True)
            artifact = output / 'python-runtime.json'
            artifact.write_text(json.dumps({'runtime':1}) + '\n')
            sign_file(artifact, private)
            self.assertEqual(artifact.with_name(artifact.name + '.sig').stat().st_size, 64)
            verify_file(artifact, public)
            artifact.write_bytes(artifact.read_bytes() + b' ')
            with self.assertRaises(subprocess.CalledProcessError):
                verify_file(artifact, public)
            artifact.write_text(json.dumps({'runtime':1}) + '\n')
            subprocess.run(['openssl', 'genpkey', '-algorithm', 'ED25519', '-out', str(private)], check=True)
            sign_file(artifact, private)
            with self.assertRaises(subprocess.CalledProcessError):
                verify_file(artifact, public)


if __name__ == '__main__':
    unittest.main()
