"""The xp census runs once per run, in a job of its own (SPEC-632 A1 and A2).

check.sh defines the census set once, beside the engine set; the test and release stages run
every test outside both sets; test-census runs the census set alone; and ci.yml's census job runs
that stage and is a need of the aggregate ci. Each binary the set names spawns its own cargo with a
cleared environment and a target directory of its own, so its verdict cannot depend on the outer
profile and a test of shipped behaviour cannot join the set.
"""

import re
import unittest

from _support import REPO, examined

CHECK = REPO / "scripts" / "check.sh"
CI = REPO / ".github" / "workflows" / "ci.yml"
CENSUS_DEFINITION = re.compile(r"^CENSUS_TESTS='([^'\n]*)'$", re.M)
BINARY = re.compile(r"binary_id\(=([a-z0-9-]+)::([a-z0-9_]+)\)")
PACKAGE_NAME = re.compile(r'(?m)^name = "([^"]+)"$')
REST = '-E "not ($ENGINE_TESTS | $CENSUS_TESTS)"'
CENSUS_ONLY = '-E "$CENSUS_TESTS"'
# A top-level job of ci.yml: its name at two spaces, then its lines up to the next such name.
JOB = re.compile(r"(?ms)^  ([a-z0-9-]+):\n(.*?)(?=^  [a-z0-9-]+:\n|\Z)")
NEEDS = re.compile(r"(?m)^    needs: \[([^\]\n]*)\]$")


def stage_body(text, name):
    """The body of check.sh's `stage_<name>()`, up to its closing brace at column 0."""
    found = re.search(rf"(?ms)^stage_{name}\(\) \{{\n(.*?)^\}}$", text)
    return found.group(1) if found else None


def census_set(text):
    definitions = CENSUS_DEFINITION.findall(text)
    if len(definitions) != 1:
        raise AssertionError(
            f"check.sh defines the census set {len(definitions)} time(s), not once"
        )
    return definitions[0]


def jobs(text):
    return {name: body for name, body in JOB.findall(text)}


class TheCensusRunsOnce(unittest.TestCase):
    def test_the_census_set_runs_once_in_its_own_job(self):
        check = CHECK.read_text(encoding="utf-8")
        census = census_set(check)
        self.assertEqual(census, "binary_id(=deck-streak-progression::xp_census)")
        # The two stages of the rest each exclude both sets, in one filterset.
        for stage in examined("stages that run the rest", ["test", "test_release"]):
            body = stage_body(check, stage)
            self.assertIsNotNone(body, f"check.sh has no stage_{stage}")
            self.assertIn(REST, body, f"stage_{stage} runs the census set")
            self.assertEqual(body.count(" -E "), 1, f"stage_{stage} runs more than one filterset")
        # The census stage runs the census set alone, with no slice.
        body = stage_body(check, "test_census")
        self.assertIsNotNone(body, "check.sh has no stage_test_census")
        self.assertIn(CENSUS_ONLY, body)
        self.assertEqual(body.count(" -E "), 1, "stage_test_census runs more than one filterset")
        self.assertNotIn("ENGINE_TESTS", body)
        self.assertNotIn("--partition", body)
        listed = re.search(r"^STAGES_ALL=\(([^)]*)\)", check, re.M).group(1).split()
        self.assertIn("test-census", listed)
        # The census job runs that stage, once, and the aggregate needs it.
        workflow = jobs(CI.read_text(encoding="utf-8"))
        self.assertIn("census", workflow, "ci.yml has no census job")
        calls = re.findall(r"(?m)^ +run: bash scripts/check\.sh (.+)$", workflow["census"])
        self.assertEqual(calls, ["test-census"])
        self.assertNotIn("matrix:", workflow["census"])
        self.assertNotIn("needs:", workflow["census"])
        callers = [name for name, job in workflow.items() if "check.sh test-census" in job]
        self.assertEqual(callers, ["census"], "another job runs the census stage")
        needs = NEEDS.search(workflow["ci"])
        self.assertIsNotNone(needs, "ci has no needs list")
        named = [need.strip() for need in needs.group(1).split(",")]
        self.assertEqual(named.count("census"), 1)
        self.assertEqual(named[named.index("engine") + 1], "census", "census is not after engine")

    def test_census_set_spawns_its_own_cargo(self):
        census = census_set(CHECK.read_text(encoding="utf-8"))
        packages = {}
        for manifest in sorted((REPO / "crates").glob("*/Cargo.toml")):
            named = PACKAGE_NAME.search(manifest.read_text(encoding="utf-8"))
            if named:
                packages[named.group(1)] = manifest.parent
        binaries = examined("binaries in the census set", BINARY.findall(census))
        for package, target in binaries:
            with self.subTest(binary=f"{package}::{target}"):
                self.assertIn(package, packages, f"no crate declares the package {package}")
                source = packages[package] / "tests" / f"{target}.rs"
                text = source.read_text(encoding="utf-8")
                # It spawns cargo, clears the child's environment, and builds in its own target
                # directory, with no --release, so the outer profile cannot reach its verdict.
                self.assertRegex(
                    text, r'var_os\("CARGO"\)\.unwrap_or_else\(\|\| "cargo"\.into\(\)\)'
                )
                self.assertRegex(text, r"Command::new\(program\);\n\s*command\.env_clear\(\);")
                self.assertIn("env_clear()", text)
                self.assertIn('"--target-dir"', text)
                self.assertNotIn('"--release"', text)


if __name__ == "__main__":
    unittest.main()
