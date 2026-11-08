"""Exercise the driver through the same command line yt-dlp uses."""
import os
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


class SandboxTests(unittest.TestCase):
    def run_script(self, source):
        driver = Path(os.environ['QJS_DRIVER']).resolve()
        with tempfile.TemporaryDirectory() as folder:
            script = Path(folder) / 'challenge.js'
            script.write_text(source.replace("$MODULE", json.dumps(str(Path(folder) / "secret.js"))))
            (Path(folder) / 'secret.js').write_text('export default 42;')
            return subprocess.run([str(driver), '--script', str(script)],
                                  capture_output=True, text=True, timeout=40)

    def test_challenge_helpers_and_jobs(self):
        result = self.run_script("""
            if (typeof performance.now() !== 'number') throw Error('clock');
            if (!Array.isArray(scriptArgs)) throw Error('args');
            Promise.resolve().then(() => console.log('solved', 6 * 7));
        """)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'solved 42\n')
        self.assertEqual(result.stderr, '')

    def test_no_host_capabilities_even_via_function_constructor(self):
        result = self.run_script("""
            for (const name of ['std', 'os', '__loadScript', 'load', 'read',
                                'require', 'process', 'fetch', 'Worker']) {
                if (Function('return typeof ' + name)() !== 'undefined')
                    throw Error(name + ' is exposed');
            }
            print('isolated');
        """)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'isolated\n')

    def test_no_dynamic_imports(self):
        for module in ['std', 'os', './secret.js', 'file:///etc/passwd', 'https://example.org/a.js']:
            with self.subTest(module=module):
                result = self.run_script(f"import({module!r}).then(() => print('ESCAPED'), () => print('blocked'))")
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(result.stdout, 'blocked\n')

    def test_cannot_import_an_existing_file(self):
        result = self.run_script("import($MODULE).then(() => print('ESCAPED'), () => print('blocked'))")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, 'blocked\n')

    def test_no_static_modules(self):
        result = self.run_script("import * as os from 'os'; os.exec(['touch', '/tmp/escaped']);")
        self.assertNotEqual(result.returncode, 0)

    def test_script_errors_fail(self):
        result = self.run_script("throw Error('challenge failed')")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('challenge failed', result.stderr)

    def test_unhandled_rejections_fail(self):
        result = self.run_script("Promise.reject(Error('rejected'))")
        self.assertNotEqual(result.returncode, 0)


if __name__ == '__main__':
    unittest.main()
