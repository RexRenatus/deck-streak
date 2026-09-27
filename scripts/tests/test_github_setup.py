"""The GitHub setup keeps Dependabot's security updates off and its alerts on (SPEC-035 A1-A3)."""

import os
import stat
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SETUP = REPO / "scripts" / "github-setup.sh"
# A stand-in for gh: it records its arguments, one call per line, and answers success.
FAKE_GH = """#!/usr/bin/env bash
printf '%s\\n' "$*" >> "$FAKE_GH_LOG"
cat > /dev/null
exit 0
"""


def security_calls():
    with tempfile.TemporaryDirectory() as tmp:
        fake = Path(tmp) / "gh"
        fake.write_text(FAKE_GH, encoding="utf-8")
        fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
        log = Path(tmp) / "calls.log"
        path = f"{tmp}:{os.environ['PATH']}"
        env = dict(os.environ, PATH=path, FAKE_GH_LOG=str(log), REPO="owner/name")
        done = subprocess.run(
            ["bash", str(SETUP), "security"],
            env=env,
            stdin=subprocess.DEVNULL,
            capture_output=True,
            text=True,
            check=False,
            timeout=60,
        )
        if done.returncode != 0:
            raise AssertionError(f"github-setup.sh security failed: {done.stdout}{done.stderr}")
        return log.read_text(encoding="utf-8").splitlines() if log.exists() else []


class TheSecuritySetupKeepsTheWorkflowWhole(unittest.TestCase):
    def test_the_security_setup_turns_automated_security_fixes_off_and_never_on(self):
        calls = examined("gh call(s)", security_calls())
        fixes = [call for call in calls if "automated-security-fixes" in call]
        self.assertEqual(fixes, ["api -X DELETE repos/owner/name/automated-security-fixes"])

    def test_the_security_setup_keeps_alerts_scanning_and_private_reporting_on(self):
        calls = examined("gh call(s)", security_calls())
        self.assertIn("api -X PUT repos/owner/name/vulnerability-alerts", calls)
        self.assertIn("api -X PUT repos/owner/name/private-vulnerability-reporting", calls)
        self.assertIn("api -X PATCH repos/owner/name --input -", calls)

    def test_the_documents_say_an_alert_becomes_a_pull_request_into_dev(self):
        for name in examined("document(s)", ["SECURITY.md", "docs/OWNER-SETUP.md"]):
            text = " ".join((REPO / name).read_text(encoding="utf-8").split())
            self.assertIn("automated security fixes stay off", text, name)
            self.assertIn("a pull request into `dev`", text, name)


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]])
