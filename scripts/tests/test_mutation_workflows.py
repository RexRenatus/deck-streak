"""The mutation tools' configurations, the pull request's mutation jobs, the weekly battery and the
builder brief (SPEC-039 A20 to A25).

The workflows are read as text, job by job, the way `test_ci_workflows.py` reads them: a job is the
block under its two-space name, and a step is the block under its `- ` at six spaces.
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
WEEKLY = WORKFLOWS / "mutation-weekly.yml"
CI = WORKFLOWS / "ci.yml"
VERDICT = REPO / "scripts" / "mutation-verdict.py"
BRIEF = REPO / "docs" / "BUILDER-BRIEF.md"
EXAMINED = re.compile(r"^examined (\d+)", re.MULTILINE)
INSTALL = re.compile(r"(?m)^\s*tool: cargo-mutants@27\.1\.0$")


def workflow(path):
    """The workflow's text; its absence is the criterion failing, never a crash."""
    if not path.is_file():
        raise AssertionError(f"{path.relative_to(REPO)} does not exist")
    return path.read_text(encoding="utf-8")


def jobs(text):
    """{job: its block} for the workflow's top-level jobs."""
    body = text.split("\njobs:\n", 1)[1] if "\njobs:\n" in text else ""
    found = {}
    for match in re.finditer(r"(?ms)^  ([a-z0-9-]+):\n(.*?)(?=^  [a-z0-9-]+:\n|\Z)", body):
        found[match.group(1)] = match.group(2)
    return found


def steps(job):
    """Each step's block, in order."""
    return [m.group(0) for m in re.finditer(r"(?ms)^      - .*?(?=^      - |\Z)", job)]


def uploads_always(job):
    return any(
        "actions/upload-artifact@" in step and re.search(r"if: \$\{\{ always\(\) \}\}", step)
        for step in steps(job)
    )


def run(*args, env=None):
    return subprocess.run(
        [sys.executable, *args],
        capture_output=True,
        text=True,
        env=dict(os.environ, PYTHONDONTWRITEBYTECODE="1", **(env or {})),
        timeout=300,
        check=False,
    )


class ExclusionsAreNeverSilent(unittest.TestCase):
    def test_every_exclusion_names_its_reason_and_issue(self):
        done = run(str(VERDICT), "exclusions", "--root", str(REPO))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertRegex(done.stdout, r"(?m)^examined \d+ exclusion")
        # Planted: an exclusion with no reason, a Stryker comment with no issue, and a skip
        # attribute, each refused by name; a justified exclusion passes beside them.
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / ".cargo").mkdir()
            (root / ".cargo" / "mutants.toml").write_text(
                "exclude_re = [\n"
                "    # EQUIVALENT: the capacity hint changes no output (#1)\n"
                '    "crates/a/src/lib\\\\.rs:1:1: replace with_capacity",\n'
                '    "crates/a/src/lib\\\\.rs:9:9: replace && with \\\\|\\\\|",\n'
                "]\n",
                encoding="utf-8",
            )
            web = root / "web" / "app" / "src" / "lib"
            web.mkdir(parents=True)
            (web / "a.ts").write_text(
                "// Stryker disable next-line EqualityOperator: EQUIVALENT: a is never b (#2)\n"
                "export const one = (a: number, b: number) => a <= b;\n"
                "// Stryker disable next-line all\n"
                "export const two = 2;\n",
                encoding="utf-8",
            )
            src = root / "crates" / "a" / "src"
            src.mkdir(parents=True)
            (src / "lib.rs").write_text(
                "#[mutants::skip]\npub fn skipped() -> bool {\n    true\n}\n", encoding="utf-8"
            )
            planted = run(str(VERDICT), "exclusions", "--root", str(root))
        self.assertEqual(planted.returncode, 1, planted.stdout + planted.stderr)
        findings = [line for line in planted.stdout.splitlines() if line.startswith("exclusions: ")]
        self.assertEqual(len(findings), 3, planted.stdout)
        self.assertIn("replace && with", planted.stdout)
        self.assertIn("web/app/src/lib/a.ts:3", planted.stdout)
        self.assertIn("crates/a/src/lib.rs:1: mutants::skip", planted.stdout)
        self.assertRegex(planted.stdout, r"(?m)^examined 5 exclusion")


class TheToolsConfigurationsAreValid(unittest.TestCase):
    def test_the_tool_configurations_load_under_their_own_rules(self):
        done = run(str(VERDICT), "configs", "--root", str(REPO))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertRegex(done.stdout, r"(?m)^examined 2 configuration")
        stryker = json.loads((REPO / "web/app/stryker.config.json").read_text(encoding="utf-8"))
        self.assertEqual(stryker["testRunner"], "vitest")
        self.assertIn("json", stryker["reporters"])
        self.assertEqual(stryker["thresholds"]["break"], 100)
        package = json.loads((REPO / "web/app/package.json").read_text(encoding="utf-8"))
        for name in ("@stryker-mutator/core", "@stryker-mutator/vitest-runner"):
            self.assertEqual(package["devDependencies"].get(name), "10.0.0", name)
        # Planted: what each tool itself refuses. cargo-mutants 27.1.0 denies an unknown key and a
        # value of the wrong type; StrykerJS refuses a high threshold below the low one (80 and 60
        # by default), a threshold out of 0 to 100, and a JSON file with a comment.
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / ".cargo").mkdir()
            (root / ".cargo" / "mutants.toml").write_text(
                'exclude_glob = ["x"]\ntest_tool = "pytest"\ntimeout_multiplier = "2"\n'
                'sharding = "round-robin"\n',
                encoding="utf-8",
            )
            app = root / "web" / "app"
            app.mkdir(parents=True)
            (app / "stryker.config.json").write_text(
                '{"testRunner": "vitest", "reporters": ["json"], '
                '"thresholds": {"high": 50, "break": 101}}',
                encoding="utf-8",
            )
            planted = run(str(VERDICT), "configs", "--root", str(root))
            (app / "stryker.config.json").write_text(
                '{\n  // a note\n  "testRunner": "vitest"\n}\n', encoding="utf-8"
            )
            commented = run(str(VERDICT), "configs", "--root", str(root))
        self.assertEqual(planted.returncode, 1, planted.stdout + planted.stderr)
        for finding in examined(
            "refusals",
            [
                "configs: .cargo/mutants.toml: exclude_glob is not a cargo-mutants 27.1.0 key",
                "configs: .cargo/mutants.toml: test_tool is 'pytest', not one of cargo, nextest",
                "configs: .cargo/mutants.toml: timeout_multiplier is not a number",
                "configs: web/app/stryker.config.json: thresholds.high 50 is below "
                "thresholds.low 60",
                "configs: web/app/stryker.config.json: thresholds.break 101 is outside 0 to 100",
            ],
        ):
            self.assertIn(finding, planted.stdout)
        self.assertNotIn("sharding", planted.stdout)
        self.assertRegex(planted.stdout, r"(?m)^examined 2 configuration")
        self.assertEqual(commented.returncode, 1, commented.stdout + commented.stderr)
        self.assertIn("configs: web/app/stryker.config.json: is not JSON", commented.stdout)


class TheWeeklyBattery(unittest.TestCase):
    def test_the_weekly_shards_cover_their_denominator_and_keep_reports(self):
        text = workflow(WEEKLY)
        self.assertRegex(text, r"(?m)^  schedule:\n    - cron: ")
        self.assertRegex(text, r"(?m)^  workflow_dispatch:")
        self.assertRegex(text, r"(?ms)^  pull_request:\n.*?paths:\n.*?mutation-weekly\.yml")
        found = jobs(text)
        rust = found.get("rust", "")
        denominator = re.search(r"--shard \$\{\{ matrix\.shard \}\}/(\d+)", rust)
        self.assertIsNotNone(denominator, "the rust job does not shard")
        matrix = re.search(r"(?m)^\s+shard: \[([0-9, ]+)\]$", rust)
        self.assertIsNotNone(matrix, "the rust job names no shard matrix")
        shards = sorted(int(value) for value in matrix.group(1).split(","))
        self.assertEqual(shards, list(range(int(denominator.group(1)))))
        mutating = [
            name
            for name, job in found.items()
            if re.search(r"cargo mutants|stryker run|mutation_rows\.py prove", job)
        ]
        for name in examined("jobs that run mutants", mutating):
            job = found[name]
            self.assertTrue(uploads_always(job), f"{name} keeps no report when it fails")
            self.assertRegex(job, r"(?m)^    timeout-minutes: \d+$", name)
            for command in re.findall(r"cargo mutants[^\n]*", job):
                self.assertIn("--timeout ", command, f"{name}: {command}")
                self.assertIn("--in-place", command, f"{name}: {command}")
        self.assertRegex(found.get("rehearsal", ""), r"github\.event_name == 'pull_request'")
        for name in ("rust", "web", "rows"):
            self.assertRegex(found.get(name, ""), r"github\.event_name != 'pull_request'", name)
        self.assertTrue(INSTALL.search(text), "the battery does not pin cargo-mutants 27.1.0")

    def test_only_the_survivors_job_writes_issues_and_never_on_a_pull_request(self):
        text = workflow(WEEKLY)
        self.assertRegex(text, r"(?m)^permissions:\n  contents: read$")
        found = jobs(text)
        writers = [name for name, job in found.items() if "issues: write" in job]
        self.assertEqual(writers, ["survivors"])
        survivors = found["survivors"]
        self.assertRegex(survivors, r"github\.event_name != 'pull_request'")
        self.assertRegex(survivors, r"always\(\)")
        body = "".join(steps(survivors))
        drafted = body.find("mutation-verdict.py survivors")
        scrubbed = body.find("public-scrub.py")
        filed = body.find("gh issue create")
        self.assertTrue(0 <= drafted < scrubbed < filed, "survivors: draft, scrub, then file")
        self.assertIn("--open-titles", body, "the drafts are not deduplicated by title")
        self.assertRegex(body, r"gh issue list[^\n]*--state open[^\n]*--json title")


    def test_the_battery_counts_every_report_its_jobs_promise(self):
        found = jobs(workflow(WEEKLY))
        matrix = re.search(r"(?m)^\s+shard: \[([0-9, ]+)\]$", found.get("rust", ""))
        self.assertIsNotNone(matrix, "the rust job names no shard matrix")
        shards = len(matrix.group(1).split(","))
        for name, promised in examined(
            "jobs that judge a battery", [("survivors", shards), ("rehearsal", 1)]
        ):
            counted = [
                step
                for step in steps(found.get(name, ""))
                if "mutation-verdict.py battery" in step
            ]
            self.assertEqual(len(counted), 1, f"{name} never counts the battery's reports")
            self.assertRegex(counted[0], rf"--shards {promised}\b", name)
            # It runs whatever the jobs before it returned, and it is the job's last word.
            self.assertRegex(counted[0], r"if: \$\{\{ always\(\) \}\}", name)
            self.assertEqual(steps(found[name])[-1], counted[0], f"{name}: a step follows it")


class TheMutationJobsGateEveryPullRequest(unittest.TestCase):
    def test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache(self):
        text = workflow(CI)
        found = jobs(text)
        aggregate = found.get("ci", "")
        for name in examined("mutation jobs", ["mutation-rust", "mutation-web"]):
            self.assertIn(name, found, f"ci.yml has no {name} job")
            job = found[name]
            self.assertRegex(aggregate, rf"needs: \[[^\]]*\b{name}\b", f"ci does not need {name}")
            self.assertTrue(uploads_always(job), f"{name} keeps no report when it fails")
            self.assertNotRegex(job, r"actions/cache@|actions/cache/save@", name)
            self.assertNotRegex(
                job, r"(?m)^    if:", f"{name} is skipped, which ci reads as failed"
            )
            self.assertIn("mutation-verdict.py judge", job, name)
        rust = found["mutation-rust"]
        self.assertTrue(INSTALL.search(rust), "mutation-rust does not pin cargo-mutants 27.1.0")
        self.assertRegex(rust, r"(?m)^\s+fallback: none$")
        command = re.search(r"cargo mutants[^\n]*", rust).group(0)
        for flag in ("--in-diff", "--in-place", "--timeout "):
            self.assertIn(flag, command)
        self.assertIn("mutation_rows.py prove", rust)
        self.assertIn("mutation_rows.py retired", rust)
        self.assertIn("stryker run", found["mutation-web"])


class EveryRunIsBounded(unittest.TestCase):
    def test_every_cargo_mutants_command_bounds_its_builds_and_its_tests(self):
        # --timeout bounds each test run; under --in-place no build is bounded unless
        # --build-timeout says so (the tool's own timeouts chapter).
        commands = [
            (path.name, command)
            for path in sorted(WORKFLOWS.glob("*.yml"))
            for command in re.findall(r"cargo mutants [^\n]*", path.read_text(encoding="utf-8"))
        ]
        for name, command in examined("cargo-mutants commands", commands):
            self.assertRegex(command, r"--timeout \d+", f"{name}: {command}")
            self.assertRegex(command, r"--build-timeout \d+", f"{name}: {command}")


class TheBuilderBriefTeachesTheRules(unittest.TestCase):
    def test_the_builder_brief_teaches_the_mutation_rules(self):
        text = BRIEF.read_text(encoding="utf-8")
        section = re.search(r"(?ms)^## Mutation testing\n(.*?)(?=^## |\Z)", text)
        self.assertIsNotNone(section, "docs/BUILDER-BRIEF.md has no Mutation testing section")
        for needle in examined(
            "rules the brief teaches",
            [
                "mutation-rust",
                "mutation-web",
                "VOID",
                "scripts/mutation-rows.d/",
                "python3 scripts/mutation_rows.py prove",
                "EQUIVALENT",
                "--in-place",
                "-j 1",
                "cargo clean",
                "scripts/mutation-rows.retired.json",
            ],
        ):
            self.assertIn(needle, section.group(1))


class CargoMutantsRunsTheGatesTestTool(unittest.TestCase):
    def test_every_job_that_runs_cargo_mutants_installs_the_test_tool_it_names(self):
        # The gate runs nextest, a process per test; cargo-mutants runs cargo test, one process per
        # binary, unless its configuration says otherwise (SPEC-039 R1, measured in section 8).
        config = (REPO / ".cargo" / "mutants.toml").read_text(encoding="utf-8")
        self.assertRegex(config, r'(?m)^test_tool = "nextest"$')
        running = [
            (path.name, name, job)
            for path in sorted(WORKFLOWS.glob("*.yml"))
            for name, job in jobs(path.read_text(encoding="utf-8")).items()
            if "cargo mutants" in job
        ]
        for workflow_name, name, job in examined("jobs that run cargo-mutants", running):
            self.assertRegex(
                job,
                r"(?m)^\s*tool: cargo-nextest@0\.9\.146$",
                f"{workflow_name}:{name} runs cargo-mutants without the nextest it names",
            )


if __name__ == "__main__":
    unittest.main()
