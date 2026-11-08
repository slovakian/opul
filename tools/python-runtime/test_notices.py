"""The published runtime must retain every native curl dependency notice."""
import hashlib
from pathlib import Path
import tarfile
import tempfile
import unittest
from io import BytesIO

from notices import install_notices, notice_hashes, verify_notices


class NoticesTests(unittest.TestCase):
    def test_historical_runtime_is_not_reported_notice_complete(self):
        with self.assertRaisesRegex(ValueError, 'matching recipe revision'):
            notice_hashes({'runtime': 1})

    def test_complete_bundle_is_copied_and_verified(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            data = b'Upstream native dependency copyright and license text.\n'
            hashes = {'LICENSE_CURL': hashlib.sha256(data).hexdigest()}
            archive = root / 'notices.tar.gz'
            with tarfile.open(archive, 'w:gz') as tar:
                member = tarfile.TarInfo('LICENSE_CURL')
                member.size = len(data)
                tar.addfile(member, BytesIO(data))
            stage = root / 'runtime'
            install_notices(stage, archive, hashes)
            verify_notices(stage, hashes)
            notice = stage / 'licenses/curl-impersonate/LICENSE_CURL'
            self.assertEqual(notice.read_bytes(), data)
            notice.write_bytes(b'modified notice')
            with self.assertRaisesRegex(ValueError, 'notice checksum'):
                verify_notices(stage, hashes)

    def test_missing_dependency_notice_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            archive = root / 'notices.tar.gz'
            with tarfile.open(archive, 'w:gz'):
                pass
            with self.assertRaisesRegex(ValueError, 'Missing native dependency notice'):
                install_notices(root / 'runtime', archive, {'LICENSE_BORINGSSL': '0' * 64})

    def test_link_cannot_substitute_for_a_notice(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            archive = root / 'notices.tar.gz'
            with tarfile.open(archive, 'w:gz') as tar:
                member = tarfile.TarInfo('LICENSE_CURL')
                member.type = tarfile.SYMTYPE
                member.linkname = '/etc/passwd'
                tar.addfile(member)
            with self.assertRaisesRegex(ValueError, 'regular file'):
                install_notices(root / 'runtime', archive, {'LICENSE_CURL': '0' * 64})


if __name__ == '__main__':
    unittest.main()
