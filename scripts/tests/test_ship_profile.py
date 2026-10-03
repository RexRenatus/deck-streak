"""SPEC-330: the tests prove their rules in the build the product ships (#473).

A1 reads the gate: the `test-release` stage runs the workspace's tests in the release profile, and
CI runs it in a job the aggregate `ci` job needs. A2 reads `clippy.toml`: `cfg!` and `option_env!`
are disallowed, each with its reason."""

import re
import tomllib
import unittest
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[2]
STAGE = "test-release"
MACROS = ["std::cfg", "std::option_env"]


def examined(what, items):
    """The items, refusing an empty population: a check that examined nothing proved nothing."""
    items = list(items)
    assert items, f"examined 0 {what}"
    return items


def stage_body(check):
    found = re.search(r"(?ms)^stage_test_release\(\) \{\n(.*?)^\}", check)
    assert found, "check.sh defines no stage_test_release"
    return found.group(1)


class TheReleaseStageTestsTheShippedProfile(unittest.TestCase):
    def test_the_release_stage_is_in_the_gate_roster_and_runs_the_release_profile(self):
        check = (ROOT / "scripts" / "check.sh").read_text(encoding="utf-8")
        roster = re.search(r"(?m)^STAGES_ALL=\(([^)]*)\)", check).group(1).split()
        self.assertIn(STAGE, examined("gate stages", roster))
        body = stage_body(check)
        for needle in ("cargo nextest run", "--workspace", "--locked", "--release", "ENGINE_TESTS"):
            self.assertIn(needle, body)

    def test_the_release_stage_runs_in_one_ci_job_the_aggregate_needs(self):
        workflow = yaml.safe_load((ROOT / ".github" / "workflows" / "ci.yml").read_text())
        jobs = workflow["jobs"]
        calls = [
            name
            for name, job in examined("ci jobs", jobs.items())
            for step in job.get("steps", [])
            if re.search(rf"(?m)^\s*bash scripts/check\.sh {STAGE}\s*$", str(step.get("run", "")))
        ]
        self.assertEqual(calls, ["release"])
        self.assertIn("release", jobs["ci"]["needs"])


class ClippyRefusesBuildSelectedMacros(unittest.TestCase):
    def test_clippy_refuses_cfg_and_option_env_each_with_a_reason(self):
        config = tomllib.loads((ROOT / "clippy.toml").read_text(encoding="utf-8"))
        entries = {
            e["path"]: e for e in examined("disallowed macros", config.get("disallowed-macros", []))
        }
        for path in MACROS:
            self.assertIn(path, entries)
            self.assertTrue(entries[path].get("reason", "").strip(), f"{path} has no reason")


if __name__ == "__main__":
    unittest.main()
