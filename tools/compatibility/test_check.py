"""Offline contracts for the scheduled compatibility workflow."""
import datetime as dt
import json
import os
import subprocess
import sys
from pathlib import Path
import tempfile
import unittest

from check import channels_due, result_name, already_tested, report_failure, record_result, check_health


class CompatibilityTests(unittest.TestCase):
    def test_stable_runs_every_three_days_across_month_boundaries(self):
        start = dt.date(2026, 12, 20)
        days = [start + dt.timedelta(days=n) for n in range(45)
                if 'stable' in channels_due('17 4 * * *', start + dt.timedelta(days=n))]
        self.assertGreater(len(days), 10)
        self.assertTrue(all((b - a).days == 3 for a, b in zip(days, days[1:])))
        self.assertEqual(channels_due('41 4 * * 0', start), ['nightly'])
        self.assertEqual(channels_due('', start), ['stable', 'nightly'])

    def test_not_due_command_exits_before_network_or_creating_artifacts(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder) / 'output'
            result = subprocess.run([sys.executable, 'tools/compatibility/check.py',
                                     '--channel', 'stable', '--arch', 'arm64', '--output', str(output)],
                                    env=dict(os.environ, COMPATIBILITY_SCHEDULE='41 4 * * 0'),
                                    capture_output=True, text=True, check=True)
            self.assertIn('not due', result.stdout)
            self.assertFalse(output.exists())

    def test_skip_identity_covers_channel_runtime_script_and_architecture(self):
        plan = dict(channel='stable', runtime=1, yt_dlp='2026.10.26', arch='arm64', pointer_sha256='abc')
        assets = [{'name': result_name(plan)}]
        self.assertTrue(already_tested(plan, assets))
        for field, value in [('channel', 'nightly'), ('runtime', 2), ('yt_dlp', '2026.11.01'),
                             ('arch', 'x64'), ('pointer_sha256', 'def')]:
            self.assertFalse(already_tested(dict(plan, **{field: value}), assets))

    def test_failure_issue_has_actionable_evidence_and_deduplicates_closed_issues(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder)
            plan = dict(channel='nightly', runtime=2, yt_dlp='2026.11.04.232659', arch='x64', pointer_sha256='abc')
            calls = []
            issues = []
            def gh(*args):
                calls.append(args)
                if args[0] == 'api':
                    return json.dumps([issues])
                body = Path(args[args.index('--body-file') + 1]).read_text()
                issues.append({'state': 'closed', 'body': body})
                return 'https://example.test/issues/1'
            report_failure(plan, output, 'QuickJS unavailable', 'https://example.test/run/1', gh)
            report_failure(plan, output, 'QuickJS unavailable', 'https://example.test/run/2', gh)
            self.assertEqual(sum(c[0] == 'issue' for c in calls), 1)
            for evidence in ['runtime 2', '2026.11.04.232659', 'x64', 'QuickJS unavailable', 'https://example.test/run/1']:
                self.assertIn(evidence, issues[0]['body'])

    def test_results_can_never_promote_the_tools_release_to_latest(self):
        calls = []
        def gh(*args):
            calls.append(args)
            return '{}'
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'result.json'
            path.write_text('{}')
            record_result(path, None, gh)
        create = next(c for c in calls if c[:2] == ('release', 'create'))
        self.assertIn('--prerelease', create)
        self.assertIn('--latest=false', create)
        self.assertTrue(any(c[:2] == ('release', 'upload') for c in calls))

    def test_health_requires_supported_handlers_engine_version_and_python(self):
        healthy = ('[debug] yt-dlp version nightly@2026.11.04.232659 from fixture\n'
                   '[debug] Python 3.13.18 (CPython)\n'
                   '[debug] Request Handlers: urllib, curl_cffi\n'
                   '[debug] JS runtimes: quickjs-2026-06-04\n')
        check_health(healthy, '2026.11.04.232659')
        for text in [healthy.replace('curl_cffi', 'curl_cffi (unsupported)'),
                     healthy.replace('quickjs-', 'deno-'), healthy.replace('2026.11.04.232659', 'wrong'),
                     healthy + '\nDeprecated Feature: Support for Python 3.13',
                     healthy.replace('[debug] Python', '[debug] Unknown')]:
            with self.assertRaises(ValueError):
                check_health(text, '2026.11.04.232659')

    def test_workflow_runs_only_dedicated_network_checks_and_both_architectures(self):
        workflow = Path('.github/workflows/tools-compatibility.yml').read_text()
        self.assertIn('"17 4 * * *"', workflow)
        self.assertIn('"41 4 * * 0"', workflow)
        self.assertIn('arch: [arm64, x64]', workflow)
        self.assertNotIn('pull_request:', workflow)
        self.assertNotIn('push:', workflow)
        self.assertIn('if: always()', workflow)
        self.assertIn('issues: write', workflow)


if __name__ == '__main__':
    unittest.main()
