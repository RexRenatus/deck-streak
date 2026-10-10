"""The mutation tools' configurations, the pull request's mutation jobs, the weekly battery and the
builder brief (SPEC-039 A20 to A25).

The workflows are read as text, job by job, the way `test_ci_workflows.py` reads them: a job is the
block under its two-space name, and a step is the block under its `- ` at six spaces.
"""

import ast
import copy
import json
import os
import re
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest
from pathlib import Path

from _mutants_finder import CARGO, Refused, mutants_in, run_texts
from _support import REPO, examined
from test_ci_workflows import workflow_file_text, workflow_files

WORKFLOWS = REPO / ".github" / "workflows"
WEEKLY = WORKFLOWS / "mutation-weekly.yml"
CI = WORKFLOWS / "ci.yml"
VERDICT = REPO / "scripts" / "mutation-verdict.py"
BRIEF = REPO / "docs" / "BUILDER-BRIEF.md"
ZERO_SCOPE = {
    "in_force": True,
    "state": "done",
    "reason": None,
    "oom": 0,
    "oom_kill": 0,
    "max": 0,
    "peak_percent": 0,
}
EXAMINED = re.compile(r"^examined (\d+)", re.MULTILINE)
INSTALL = re.compile(r"(?m)^\s*tool: cargo-mutants@27\.1\.0$")
#: Each mutation job's job-level conditions (SPEC-290): the plan and the web job always start, the
#: verdict runs whatever its needs returned, and a leg is not started only when the plan's listing
#: gives it nothing to examine.
JOB_CONDITIONS = {
    "mutation-plan": [],
    "mutation-rust": ["${{ needs.mutation-plan.outputs.listed != '0' }}"],
    "mutation-rows": [
        "${{ needs.mutation-plan.outputs.rows == 'true' || "
        "needs.mutation-plan.outputs.scope == 'diff' }}"
    ],
    "mutation-verdict": ["${{ always() }}"],
    "mutation-web": [],
}


def workflow(path):
    """The workflow's text; its absence is the criterion failing, never a crash."""
    if not path.is_file():
        raise AssertionError(f"{path.relative_to(REPO)} does not exist")
    return workflow_file_text(path)


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


class NoExclusionHidesAMutant(unittest.TestCase):
    """SPEC-057 A8 (R11): SPEC-039's A20, which held an exclusion to its reason, is retired by it."""

    def test_no_exclusion_hides_a_mutant_from_the_listing(self):
        # Planted: every form, even one with its reason and an issue, even an empty key.
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / ".cargo").mkdir()
            (root / ".cargo" / "mutants.toml").write_text(
                "# EQUIVALENT: the capacity hint changes no output (#1)\n"
                'exclude_re = ["crates/a/src/lib\\\\.rs:1:1: replace with_capacity"]\n'
                "exclude_globs = []\n"
                'examine_re = ["double"]\n'
                'examine_globs = ["crates/a/src/*.rs"]\n'
                'skip_calls = ["with_capacity"]\n'
                'test_tool = "nextest"\n',
                encoding="utf-8",
            )
            src = root / "crates" / "a" / "src"
            src.mkdir(parents=True)
            (src / "lib.rs").write_text(
                "#[mutants::skip]\n"
                "pub fn skipped() -> bool {\n    true\n}\n\n"
                '#[mutants::exclude_re("replace")]\n'
                "pub fn narrowed() -> bool {\n    false\n}\n\n"
                "// A comment that names mutants::skip is prose, not an attribute.\n",
                encoding="utf-8",
            )
            web = root / "web" / "app"
            (web / "src" / "lib").mkdir(parents=True)
            (web / "src" / "lib" / "a.ts").write_text(
                "// Stryker disable next-line EqualityOperator: EQUIVALENT: a is never b (#2)\n"
                "export const one = (a: number, b: number) => a <= b;\n",
                encoding="utf-8",
            )
            (web / "stryker.config.json").write_text(
                json.dumps({"testRunner": "vitest", "mutator": {"excludedMutations": ["Regex"]}}),
                encoding="utf-8",
            )
            planted = run(str(VERDICT), "exclusions", "--root", str(root))
        self.assertEqual(planted.returncode, 1, planted.stdout + planted.stderr)
        findings = [line for line in planted.stdout.splitlines() if line.startswith("exclusions: ")]
        refused = [
            "exclusions: .cargo/mutants.toml: the exclude_re key",
            "exclusions: .cargo/mutants.toml: the exclude_globs key",
            "exclusions: .cargo/mutants.toml: the examine_re key",
            "exclusions: .cargo/mutants.toml: the examine_globs key",
            "exclusions: .cargo/mutants.toml: the skip_calls key",
            "exclusions: crates/a/src/lib.rs:1: mutants::skip",
            "exclusions: crates/a/src/lib.rs:6: mutants::exclude_re",
            "exclusions: web/app/src/lib/a.ts:1: a Stryker disable comment",
            "exclusions: web/app/stryker.config.json: excludes the mutator Regex",
        ]
        for finding in examined("planted exclusions", refused):
            self.assertTrue(
                any(line.startswith(finding) for line in findings), f"{finding}: {planted.stdout}"
            )
        self.assertEqual(len(findings), len(refused), planted.stdout)
        self.assertRegex(planted.stdout, r"(?m)^examined 4 file\(s\)$")
        # The tree holds none.
        done = run(str(VERDICT), "exclusions", "--root", str(REPO))
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertNotIn("exclusions: ", done.stdout)
        self.assertRegex(done.stdout, r"(?m)^examined [1-9]\d* file\(s\)$")


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


class TheConfigurationCheckSeesWhatStrykerReads(unittest.TestCase):
    def test_the_configuration_check_refuses_what_stryker_would_read_otherwise(self):
        config = json.loads((REPO / "web/app/stryker.config.json").read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / ".cargo").mkdir()
            (root / ".cargo" / "mutants.toml").write_text('test_tool = "nextest"\n', "utf-8")
            app = root / "web" / "app"
            app.mkdir(parents=True)
            (app / "stryker.config.json").write_text(json.dumps(config), encoding="utf-8")
            clean = run(str(VERDICT), "configs", "--root", str(root))
            # Planted: two configurations StrykerJS would find beside this one, ignoreStatic under
            # a coverage analysis that cannot find static mutants, and a mutate list that drops
            # R2's spec exclusion.
            (app / "stryker.conf.mjs").write_text("export default {};\n", encoding="utf-8")
            (app / ".stryker.config.json").write_text("{}", encoding="utf-8")
            planted = dict(
                config,
                ignoreStatic=True,
                coverageAnalysis="all",
                mutate=[glob for glob in config["mutate"] if glob != "!src/**/*.spec.*"],
            )
            (app / "stryker.config.json").write_text(json.dumps(planted), encoding="utf-8")
            refused = run(str(VERDICT), "configs", "--root", str(root))
        self.assertEqual(clean.returncode, 0, clean.stdout + clean.stderr)
        self.assertEqual(refused.returncode, 1, refused.stdout + refused.stderr)
        for finding in examined(
            "refusals",
            [
                "configs: web/app/stryker.conf.mjs: a second Stryker configuration",
                "configs: web/app/.stryker.config.json: a second Stryker configuration",
                "configs: web/app/stryker.config.json: ignoreStatic needs coverageAnalysis perTest",
                "configs: web/app/stryker.config.json: mutate is not R2's web production code",
            ],
        ):
            self.assertIn(finding, refused.stdout)


class TheWeeklyBattery(unittest.TestCase):
    def test_the_weekly_shards_cover_their_denominator_and_keep_reports(self):
        text = workflow(WEEKLY)
        self.assertRegex(text, r"(?m)^  schedule:\n    - cron: ")
        self.assertRegex(text, r"(?m)^  workflow_dispatch:")
        self.assertRegex(text, r"(?ms)^  pull_request:\n.*?paths:\n.*?mutation-weekly\.yml")
        found = jobs(text)
        rust = found.get("rust", "")
        # The legs are the sized matrix and the denominator is the size step's own count
        # (SPEC-129 R4); a scheduled run and a dispatch with no package are sized from the whole
        # workspace's listing at the battery run's ceiling (SPEC-362 R11).
        self.assertRegex(rust, r"(?m)^\s+SHARDS: \$\{\{ needs\.size\.outputs\.shards \}\}$")
        self.assertIn('--shard "$SHARD/$SHARDS"', rust)
        self.assertIn("shard: ${{ fromJSON(needs.size.outputs.matrix) }}", rust)
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
        sized = r'"\$SHARDS"'
        for name, promised in examined(
            "jobs that judge a battery", [("survivors", sized), ("rehearsal", "1")]
        ):
            counted = [
                step for step in steps(found.get(name, "")) if "mutation-verdict.py battery" in step
            ]
            self.assertEqual(len(counted), 1, f"{name} never counts the battery's reports")
            self.assertRegex(counted[0], rf"--shards {promised}(?!\d)", name)
            # It runs whatever the jobs before it returned, and it is the job's last word.
            self.assertRegex(counted[0], r"if: \$\{\{ always\(\) \}\}", name)
            self.assertEqual(steps(found[name])[-1], counted[0], f"{name}: a step follows it")


class TheMutationJobsGateEveryPullRequest(unittest.TestCase):
    def test_the_mutation_jobs_are_needs_of_ci_with_pinned_tools_and_no_saved_cache(self):
        text = workflow(CI)
        found = jobs(text)
        aggregate = found.get("ci", "")
        names = ["mutation-plan", "mutation-rust", "mutation-rows", "mutation-verdict"]
        for name in examined("mutation jobs", [*names, "mutation-web"]):
            self.assertIn(name, found, f"ci.yml has no {name} job")
            job = found[name]
            self.assertRegex(aggregate, rf"needs: \[[^\]]*\b{name}\b", f"ci does not need {name}")
            self.assertNotRegex(job, r"actions/cache@|actions/cache/save@", name)
            # A job-level if may only make a job run, but on the plan's listing: ci reads a skipped
            # need as failed, and admits a skip from the two legs a listing can leave with nothing
            # to examine (SPEC-290 R2, R3, R8). Each condition is admitted by job name, and only it.
            conditions = re.findall(r"(?m)^    if: (.*)$", job)
            self.assertEqual(
                conditions, JOB_CONDITIONS[name], f"{name} may be skipped, which ci reads as failed"
            )
            if name != "mutation-verdict":
                self.assertTrue(uploads_always(job), f"{name} keeps no report when it fails")
        for name in ("mutation-verdict", "mutation-web"):
            self.assertIn("mutation-verdict.py judge", found[name], name)
        for name in ("mutation-plan", "mutation-rust"):
            self.assertTrue(
                INSTALL.search(found[name]), f"{name} does not pin cargo-mutants 27.1.0"
            )
            self.assertRegex(found[name], r"(?m)^\s+fallback: none$", name)
        command = re.search(r"cargo mutants[^\n]*", found["mutation-rust"]).group(0)
        for flag in ("--in-diff", "--in-place", "--timeout "):
            self.assertIn(flag, command)
        self.assertIn("mutation_rows.py prove", found["mutation-rows"])
        self.assertIn("mutation_rows.py retired", found["mutation-rows"])
        # The runner writes its report into a directory: one the job never made holds no report,
        # and the verdict reads the rows VOID (the first run of this layout, SPEC-039 section 8).
        prove = next(s for s in steps(found["mutation-rows"]) if "mutation_rows.py prove" in s)
        report = re.search(r'--report "([^"]+)/[^"/]+"', prove)
        self.assertIsNotNone(report, "the rows job writes no report")
        made = prove.find(f'mkdir -p "{report.group(1)}"')
        self.assertTrue(
            0 <= made < prove.find("mutation_rows.py prove"),
            "the rows job never makes the directory its report goes to",
        )
        self.assertIn("stryker run", found["mutation-web"])


#: A planted `python3` for SPEC-397 A10: it prints the arguments it was called with, then exits with
#: the code the environment plants for the check its first argument names.
FAKE_PYTHON = """#!/bin/sh
echo "called $*"
case "$1" in
  scripts/mutation_rows.py) exit "$ROWS_EXIT" ;;
  scripts/swift_mutants.py) exit "$SWIFT_EXIT" ;;
esac
exit 97
"""


class TheRowsJobHoldsTheSwiftRows(unittest.TestCase):
    def test_the_rows_job_runs_the_swift_retired_check_on_a_diff(self):
        """SPEC-397 A10 (R10): the `mutation-rows` job's retired step, under the diff scope, runs
        both retired checks on its one `run:` line, calls both whatever either returns, and fails
        when either fails."""
        job = jobs(workflow(CI))["mutation-rows"]
        (step,) = [s for s in steps(job) if "mutation_rows.py retired" in s]
        self.assertIn("if: ${{ needs.mutation-plan.outputs.scope == 'diff' }}\n", step)
        lines = re.findall(r"(?m)^        run: (.+)$", step)
        self.assertEqual(len(lines), 1, step)
        called = [
            "called scripts/mutation_rows.py retired --base HEAD^1",
            "called scripts/swift_mutants.py retired --base HEAD^1",
        ]
        with tempfile.TemporaryDirectory() as scratch:
            fake = Path(scratch) / "python3"
            fake.write_text(FAKE_PYTHON, encoding="utf-8")
            fake.chmod(0o755)
            path = f"{scratch}{os.pathsep}{os.environ['PATH']}"
            for rows, swift in examined("planted exits", [(0, 0), (1, 0), (0, 1), (1, 1)]):
                with self.subTest(rows=rows, swift=swift):
                    done = subprocess.run(
                        ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", lines[0]],
                        capture_output=True,
                        text=True,
                        env=dict(os.environ, PATH=path, ROWS_EXIT=str(rows), SWIFT_EXIT=str(swift)),
                        timeout=60,
                        check=False,
                    )
                    self.assertEqual(done.stdout.splitlines(), called, done.stderr)
                    self.assertEqual(done.returncode != 0, bool(rows or swift), done.stdout)


class TheShardsAreThePlans(unittest.TestCase):
    def test_the_sharded_job_runs_the_plans_matrix_and_the_verdict_counts_every_shard(self):
        found = jobs(workflow(CI))
        plan, rust, verdict = (
            found.get(name, "") for name in ("mutation-plan", "mutation-rust", "mutation-verdict")
        )
        # The plan lists the diff's mutants with cargo-mutants itself and sizes the shards from them.
        self.assertRegex(plan, r"cargo mutants [^\n]*--list --json --in-diff ")
        self.assertRegex(plan, r"mutation-verdict\.py shards --plan ")
        for output in examined("plan outputs", ["shards", "matrix"]):
            self.assertRegex(
                plan, rf"(?m)^      {output}: \$\{{\{{ steps\.shards\.outputs\.{output} \}}\}}$"
            )
        # Each shard is one entry of the plan's matrix, and runs as one of the plan's count.
        self.assertRegex(rust, r"(?m)^    needs: \[mutation-plan\]$")
        self.assertRegex(rust, r"(?m)^      fail-fast: false$")
        self.assertRegex(
            rust,
            r"(?m)^        shard: \$\{\{ fromJSON\(needs\.mutation-plan\.outputs\.matrix\) \}\}$",
        )
        self.assertRegex(rust, r"(?m)^\s+SHARD: \$\{\{ matrix\.shard \}\}$")
        self.assertRegex(
            rust, r"(?m)^\s+SHARDS: \$\{\{ needs\.mutation-plan\.outputs\.shards \}\}$"
        )
        command = re.search(r"cargo mutants [^\n]*", rust)
        self.assertIsNotNone(command, "mutation-rust runs no cargo-mutants")
        self.assertIn("--sharding round-robin", command.group(0))
        self.assertIn('--shard "$SHARD/$SHARDS"', command.group(0))
        self.assertRegex(rust, r"(?m)^\s+name: mutation-rust-shard-\$\{\{ matrix\.shard \}\}$")
        # The verdict runs whatever the shards returned, reads every artifact, and counts the
        # shards the plan promised rather than the ones that reported.
        self.assertRegex(
            verdict,
            r"(?m)^    needs: \[mutation-plan, mutation-rust, mutation-python, mutation-rows\]$",
        )
        self.assertRegex(verdict, r"(?m)^    if: \$\{\{ always\(\) \}\}$")
        self.assertRegex(verdict, r"(?m)^\s+pattern: mutation-rust-shard-\*$")
        self.assertRegex(verdict, r"judge [^\n]*--class rust [^\n]*--shard-reports ")


#: The guard's module, read here as text and never imported: its tests load the wrapper, and this
#: module stays runnable where they do not (SPEC-129 section 12, ADR-312).
GUARD = Path(__file__).parent / "test_dispatch_shards.py"


def wrapper_assignments(text):
    """The value of each module-level assignment to `WRAPPER` in a module's text, read by
    `ast.literal_eval` from `ast.parse`, so the module is never imported (ADR-312)."""
    return [
        ast.literal_eval(node.value)
        for node in ast.parse(text).body
        if isinstance(node, (ast.Assign, ast.AnnAssign))
        and node.value is not None
        and any(
            isinstance(target, ast.Name) and target.id == "WRAPPER"
            for target in (node.targets if isinstance(node, ast.Assign) else [node.target])
        )
    ]


#: Every module-level assignment to `WRAPPER` in the guard's text; exactly one is the wrapper's form.
FORMS = wrapper_assignments(GUARD.read_text(encoding="utf-8"))
WRAPPER = FORMS[0] if len(FORMS) == 1 else ()
WRAPPED = " ".join(WRAPPER)


def after_separator(words, program):
    """Where the command the wrapper runs begins: the word after the first standalone `--` that
    follows the wrapper's form, `WRAPPER` as the guard's text spells it; after any other program
    the words are that program's, so None (#533). The guard also checks each option word against
    its wrapper's parser, which this module cannot load, so here an option the guard would refuse
    reads as accepted: the scan over-finds, and fails closed (SPEC-129 section 12, ADR-312)."""
    end = program + len(WRAPPER)
    if not WRAPPER or [w.value for w in words[program:end]] != list(WRAPPER):
        return None
    if any(w.dynamic for w in words[program:end]):
        return None
    for k in range(end, len(words)):
        if not words[k].dynamic and words[k].value == "--":
            return k + 1 if k + 1 < len(words) else None
    return None


def every_program_separator(words, program):
    """The reading section 10 of SPEC-129 gave this scan, kept only as the control of the separated
    population (#533): the word after the first standalone `--` that follows any program which is
    not cargo."""
    if words[program].value.rsplit("/", 1)[-1] in CARGO:
        return None
    for k in range(program + 1, len(words)):
        if not words[k].dynamic and words[k].value == "--":
            return k + 1 if k + 1 < len(words) else None
    return None


def mutants_commands(directory, wrapped=None):
    """(workflow name, command line) for every `cargo mutants` command of the directory's
    workflows, found by the one finder the dispatch-shard guard uses; a workflow the finder
    refuses is answered with the refusal, which carries no bounds. `wrapped` stands in for the
    scan's own recognizer, `after_separator`, in a control."""
    reading = after_separator if wrapped is None else wrapped
    found = []
    for path in workflow_files(directory):
        try:
            texts = run_texts(workflow_file_text(path))
            found += [
                (path.name, command)
                for text in texts
                for command in mutants_in(text, wrapped=reading)
            ]
        except Refused as refusal:
            found.append((path.name, f"refused: {refusal}"))
    return found


def mutants_jobs(directory):
    """(workflow name, job name, job block) for every job that runs `cargo mutants`, found by the
    same finder; a job it refuses to read counts as running one."""
    found = []
    for path in workflow_files(directory):
        for name, job in jobs(workflow_file_text(path)).items():
            try:
                texts = run_texts(f"jobs:\n  {name}:\n{job}")
                runs = any(mutants_in(text, wrapped=after_separator) for text in texts)
            except Refused:
                runs = True
            if runs:
                found.append((path.name, name, job))
    return found


PLANTED_MUTANTS = (
    "name: planted\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n"
    "      - run: cargo mutants --in-place\n"
)


class EveryRunIsBounded(unittest.TestCase):
    def test_the_mutants_command_scan_reads_a_workflow_saved_with_the_yaml_suffix(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yaml").write_text(PLANTED_MUTANTS, encoding="utf-8")
            commands = mutants_commands(Path(scratch))
        self.assertEqual(commands, [("planted.yaml", "cargo mutants --in-place")])

    def test_the_mutants_job_scan_reads_a_workflow_saved_with_the_yaml_suffix(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yaml").write_text(PLANTED_MUTANTS, encoding="utf-8")
            running = mutants_jobs(Path(scratch))
        self.assertEqual([(name, job) for name, job, _ in running], [("planted.yaml", "shard")])

    def test_every_cargo_mutants_command_bounds_its_builds_and_its_tests(self):
        # --timeout bounds each test run; under --in-place no build is bounded unless
        # --build-timeout says so (the tool's own timeouts chapter).
        for name, command in examined("cargo-mutants commands", mutants_commands(WORKFLOWS)):
            self.assertRegex(command, r"--timeout \d+", f"{name}: {command}")
            self.assertRegex(command, r"--build-timeout \d+", f"{name}: {command}")


SPELLINGS = (
    (
        "a toolchain selector",
        "cargo +nightly mutants --in-place",
        "cargo +nightly mutants --in-place",
    ),
    (
        "the hyphenated binary",
        "cargo-mutants mutants --in-place",
        "cargo-mutants mutants --in-place",
    ),
    ("two blanks", "cargo  mutants --in-place", "cargo  mutants --in-place"),
    (
        "an option before the subcommand",
        "cargo --config net.retry=2 mutants --in-place",
        "cargo --config net.retry=2 mutants --in-place",
    ),
    ("the end of a line", "cargo mutants", "cargo mutants"),
    (
        "a wrapper's words after its separator",
        f"{WRAPPED} --report out -- cargo mutants --in-place",
        "cargo mutants --in-place",
    ),
)


def planted(command):
    """A workflow whose one job runs `command`."""
    return (
        "name: planted\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n"
        f"      - run: {command}\n"
    )


#: The finder's one home; the copy check reads its text and never imports it again.
FINDER = Path(__file__).parent / "_mutants_finder.py"
#: A definition spelled with the finder's own name and first argument (section 10's census).
FINDER_NAME = re.compile(r"(?m)^def mutants_(of\(words|in\(text)")
FUNCTIONS = (ast.FunctionDef, ast.AsyncFunctionDef)
#: The one helper equal to a finder function by accident (SPEC-129 section 12): a line's count of
#: leading blanks, the arithmetic any reader of indented text writes, and no copy of the finder.
FLOOR = (("test_ci_workflows.py", "_indent", "indent_of"),)


def functions_of(text):
    """Every function a module's text defines, at any depth, in the order `ast.walk` meets them."""
    return [node for node in ast.walk(ast.parse(text)) if isinstance(node, FUNCTIONS)]


def walk_in_order(node):
    """Every node under `node`, itself first, in the order of its fields: source order."""
    yield node
    for child in ast.iter_child_nodes(node):
        yield from walk_in_order(child)


def normal_body(function, own=None):
    """The body of `function` as `ast.dump` text, its docstring dropped and every name it binds
    renamed in order of first appearance: its own name, its arguments, the names it stores, its
    exception names and the functions and classes it nests. Two bodies equal under this reading
    are one body, whatever their names and their wrapping (#532).

    `own` is a finder function's name that `function` calls where the finder calls itself. In a
    copy that call is a free name, so it is renamed as the copy's own name is, and a copy of a
    finder function that calls itself by its name reads equal to it too (#532)."""
    body = function.body
    if (
        body
        and isinstance(body[0], ast.Expr)
        and isinstance(body[0].value, ast.Constant)
        and isinstance(body[0].value.value, str)
    ):
        body = body[1:]
    module = ast.Module(body=copy.deepcopy(body), type_ignores=[])
    a = function.args
    local = {function.name}
    local |= {x.arg for x in (*a.posonlyargs, *a.args, *a.kwonlyargs, a.vararg, a.kwarg) if x}
    for node in ast.walk(module):
        if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
            local.add(node.id)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            local.add(node.name)
        elif isinstance(node, ast.arg):
            local.add(node.arg)
        elif isinstance(node, (*FUNCTIONS, ast.ClassDef)):
            local.add(node.name)
    order = {function.name: "_self"}

    def new(name):
        if name == own:
            return "_self"
        return order.setdefault(name, f"_{len(order)}") if name in local else name

    for node in walk_in_order(module):
        if isinstance(node, ast.Name):
            node.id = new(node.id)
        elif isinstance(node, ast.arg):
            node.arg = new(node.arg)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            node.name = new(node.name)
        elif isinstance(node, (*FUNCTIONS, ast.ClassDef)):
            node.name = new(node.name)
    return ast.dump(module)


def calls_itself_as(function, selves):
    """The finder function that `function` copies by calling it where the finder calls itself, or
    None. `selves` maps each finder function's name to its normalised bodies; only a name the
    function loads is tried, and only an equal body under that same name is a copy (#532)."""
    called = {
        node.id
        for node in ast.walk(function)
        if isinstance(node, ast.Name) and isinstance(node.ctx, ast.Load)
    }
    for own in sorted(called & selves.keys()):
        if normal_body(function, own) in selves[own]:
            return own
    return None


def definitions_of_the_finder(directory):
    """[(file, line, def name, the finder function it copies)] for every copy of a finder function
    under `directory`, at any depth, outside the finder's own file: a `def` whose body equals a
    finder function's under `normal_body`, whatever its name and also where it calls the finder
    function by name as the finder calls itself, or a `def` spelled with the finder's own name and
    first argument, whatever its body (#532; SPEC-129 section 12). Each file is read as text,
    never imported."""
    bodies, selves = {}, {}
    for function in functions_of(FINDER.read_text(encoding="utf-8")):
        bodies.setdefault(normal_body(function), function.name)
        selves.setdefault(function.name, set()).add(normal_body(function))
    root = Path(directory)
    found = []
    for path in sorted(p for p in root.rglob("*.py") if "__pycache__" not in p.parts):
        if path.relative_to(root) == Path(FINDER.name):
            continue
        text = path.read_text(encoding="utf-8")
        hits = {}
        for function in functions_of(text):
            finder = bodies.get(normal_body(function))
            if finder is None:
                finder = calls_itself_as(function, selves)
            if finder is not None:
                hits.setdefault(function.lineno, (function.name, finder))
        for match in FINDER_NAME.finditer(text):
            name = f"mutants_{match.group(1)[:2]}"
            hits.setdefault(text.count("\n", 0, match.start()) + 1, (name, name))
        found += [
            (path.relative_to(root).as_posix(), line, name, finder)
            for line, (name, finder) in sorted(hits.items())
        ]
    return found


def renamed(function, n):
    """A copy of `function` named `copy_<n>`, its own name, arguments and locals renamed and its
    docstring replaced; `ast.unparse` then writes it in its own wrapping, not the finder's."""
    planted = copy.deepcopy(function)
    a = planted.args
    names = {function.name: f"copy_{n}"}
    for arg in (*a.posonlyargs, *a.args, *a.kwonlyargs, a.vararg, a.kwarg):
        if arg is not None:
            names[arg.arg] = f"renamed_{arg.arg}"
    for node in ast.walk(planted):
        if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
            names.setdefault(node.id, f"renamed_{node.id}")
        elif isinstance(node, ast.ExceptHandler) and node.name:
            names.setdefault(node.name, f"renamed_{node.name}")
    for node in ast.walk(planted):
        if isinstance(node, ast.Name):
            node.id = names.get(node.id, node.id)
        elif isinstance(node, ast.arg):
            node.arg = names.get(node.arg, node.arg)
        elif isinstance(node, ast.ExceptHandler) and node.name:
            node.name = names[node.name]
    planted.name = names[function.name]
    body = planted.body
    if body and isinstance(body[0], ast.Expr) and isinstance(body[0].value, ast.Constant):
        body = body[1:]
    planted.body = [ast.Expr(ast.Constant("A planted copy, under another name.")), *body]
    return planted


class EveryMutantsSpellingIsFound(unittest.TestCase):
    def test_each_spelling_of_the_command_is_found_by_the_command_scan(self):
        for what, command, found in SPELLINGS:
            with self.subTest(what), tempfile.TemporaryDirectory() as scratch:
                (Path(scratch) / "planted.yml").write_text(planted(command), encoding="utf-8")
                commands = mutants_commands(Path(scratch))
                self.assertEqual([name for name, _ in commands], ["planted.yml"], what)
                self.assertEqual([c.split() for _, c in commands], [found.split()], what)

    def test_each_spelling_of_the_command_is_found_by_the_job_scan(self):
        for what, command, _ in SPELLINGS:
            with self.subTest(what), tempfile.TemporaryDirectory() as scratch:
                (Path(scratch) / "planted.yml").write_text(planted(command), encoding="utf-8")
                running = mutants_jobs(Path(scratch))
                self.assertEqual(
                    [(name, job) for name, job, _ in running], [("planted.yml", "shard")], what
                )

    def test_a_job_without_the_command_is_found_by_neither_scan(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yml").write_text(
                planted("cargo nextest run"), encoding="utf-8"
            )
            (Path(scratch) / "control.yml").write_text(
                planted("cargo mutants --in-place"), encoding="utf-8"
            )
            commands = mutants_commands(Path(scratch))
            running = mutants_jobs(Path(scratch))
        self.assertEqual([name for name, _ in commands], ["control.yml"])
        self.assertEqual([name for name, _, _ in running], ["control.yml"])

    def test_the_finder_is_defined_once_and_a_planted_copy_is_caught(self):
        defined = definitions_of_the_finder(Path(__file__).parent)
        self.assertEqual(tuple((path, name, finder) for path, _, name, finder in defined), FLOOR)
        with tempfile.TemporaryDirectory() as scratch:
            for name in ("one.py", "two.py"):
                (Path(scratch) / name).write_text("def mutants_in(text, handed=False):\n    pass\n")
            copies = definitions_of_the_finder(scratch)
        self.assertEqual(
            [(path, line, name) for path, line, name, _ in copies],
            [("one.py", 1, "mutants_in"), ("two.py", 1, "mutants_in")],
        )

    def test_a_renamed_and_rewrapped_copy_of_each_finder_function_is_caught(self):
        functions = examined("finder functions", functions_of(FINDER.read_text(encoding="utf-8")))
        with tempfile.TemporaryDirectory() as scratch:
            deeper = Path(scratch) / "deeper"
            deeper.mkdir()
            planted = "\n\n".join(ast.unparse(renamed(f, n)) for n, f in enumerate(functions))
            (deeper / "copies.py").write_text(planted + "\n", encoding="utf-8")
            copies = definitions_of_the_finder(scratch)
        self.assertEqual(
            [(path, name, finder) for path, _, name, finder in copies],
            [("deeper/copies.py", f"copy_{n}", f.name) for n, f in enumerate(functions)],
        )

    def test_a_literal_copy_of_each_finder_function_under_another_name_is_caught(self):
        text = FINDER.read_text(encoding="utf-8")
        functions = examined("finder functions", functions_of(text))
        lines = text.splitlines(keepends=True)
        forms = {
            "literal.py": ("def literal_{}(", ""),
            "method.py": ("def method_{}(", "    "),
            "waited.py": ("async def waited_{}(", ""),
        }
        with tempfile.TemporaryDirectory() as scratch:
            for name, (spelling, pad) in forms.items():
                planted = ["class Holder:\n"] if pad else []
                for n, f in enumerate(functions):
                    source = textwrap.dedent("".join(lines[f.lineno - 1 : f.end_lineno]))
                    copied = source.replace(f"def {f.name}(", spelling.format(n), 1)
                    self.assertNotEqual(copied, source, f.name)
                    planted.append("\n" + textwrap.indent(copied, pad))
                (Path(scratch) / name).write_text("".join(planted), encoding="utf-8")
            copies = definitions_of_the_finder(scratch)
        for name, (spelling, _) in forms.items():
            with self.subTest(name):
                self.assertEqual(
                    [(def_name, finder) for path, _, def_name, finder in copies if path == name],
                    [
                        (spelling.format(n)[:-1].split()[-1], f.name)
                        for n, f in enumerate(functions)
                    ],
                )

    def test_a_copy_with_changed_logic_is_caught_only_under_the_finders_name(self):
        suspect = next(
            f for f in functions_of(FINDER.read_text(encoding="utf-8")) if f.name == "suspect"
        )
        changed = ast.unparse(suspect).replace("'mutants'", "'mutant'", 1)
        self.assertNotEqual(changed, ast.unparse(suspect))
        own = changed.replace("def suspect(text):", "def mutants_in(text, handed=False):", 1)
        fresh = changed.replace("def suspect(text):", "def suspicious(text):", 1)
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "own.py").write_text(own + "\n", encoding="utf-8")
            (Path(scratch) / "fresh.py").write_text(fresh + "\n", encoding="utf-8")
            copies = definitions_of_the_finder(scratch)
        self.assertEqual(
            [(path, name, finder) for path, _, name, finder in copies],
            [("own.py", "mutants_in", "mutants_in")],
        )


#: The programs of the separated population (#533), each bare and with options before its `--`.
#: Only the guard's wrapper runs the words after its separator. Its options are the one its parser
#: declares, `--report`, given a literal value, a double-quoted expansion, and joined by `=`.
SEPARATED = {
    "echo": ("echo", "echo -n -e"),
    "env": ("env", "env -i HOME=/tmp"),
    "timeout": ("timeout", "timeout -k 5 300"),
    "git": ("git", "git -C . --no-pager"),
    "python3 <script>": ("python3 scripts/x.py", "python3 -I scripts/x.py --report out"),
    "the wrapper": (
        WRAPPED,
        f"{WRAPPED} --report out",
        f'{WRAPPED} --report "$OUT"',
        f"{WRAPPED} --report=out",
    ),
}
COMMAND = "cargo mutants --in-place"
#: What the guard reads of each program's members, written from the text of its `wrapped` and of
#: the finder, never computed here: the wrapper's form is followed to the command after its `--`,
#: and after any other program `cargo mutants` is refused, since that program decides its words.
GUARD_READS = {
    "echo": "refused",
    "env": "refused",
    "timeout": "refused",
    "git": "refused",
    "python3 <script>": "refused",
    "the wrapper": COMMAND,
}
SEPARATORS = ("--", '"--"')
FORMS_OF_RUN = (
    ("one line", "      - run: {}\n"),
    ("block", "      - run: |\n          {}\n          echo done\n"),
)
#: Where the scan departs from the guard (SPEC-129 section 12): each member's words before the
#: command, what the guard reads (from its rule) and what the scan reads.
NAMED_LIMITS = {
    "over-find": (
        (f"{WRAPPED} --cap 1 --", "refused", COMMAND),
        (f"{WRAPPED} --report $OUT --", "refused", COMMAND),
        (f"{WRAPPED} $SEP --", "refused", COMMAND),
        (f"{WRAPPED} --report --", "refused", COMMAND),
    ),
    "value-dash-dash": ((f"{WRAPPED} --report -- --", COMMAND, "refused"),),
}
#: Each over-find member's twin after a script that is not the wrapper, which both refuse.
TWINS = (
    "python3 scripts/x.py --cap 1 --",
    "python3 scripts/x.py --report $OUT --",
    "python3 scripts/x.py $SEP --",
    "python3 scripts/x.py --report --",
)


def separated_population():
    """[(program, member, run step)]: each program's words, a separator and the command, in a
    one-line `run:` and in a `run: |` block whose next line runs another command."""
    return [
        (
            program,
            f"{prefix} {separator} {COMMAND} ({form})",
            step.format(f"{prefix} {separator} {COMMAND}"),
        )
        for program, prefixes in SEPARATED.items()
        for prefix in prefixes
        for separator in SEPARATORS
        for form, step in FORMS_OF_RUN
    ]


def scan_reads(steps, wrapped=None):
    """What the command scan reads of each run step, saved as its own workflow: its commands joined
    by ` | `, `refused`, or `nothing`."""
    with tempfile.TemporaryDirectory() as scratch:
        for n, step in enumerate(steps):
            (Path(scratch) / f"m{n:03d}.yml").write_text(
                "name: planted\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n" + step,
                encoding="utf-8",
            )
        found = mutants_commands(Path(scratch), wrapped)
    reads = {}
    for name, command in found:
        read = "refused" if command.startswith("refused: ") else command
        reads.setdefault(int(name[1:4]), []).append(read)
    return [" | ".join(reads.get(n, ["nothing"])) for n in range(len(steps))]


class TheScanReadsTheWrappersForm(unittest.TestCase):
    def test_the_wrapper_form_is_read_once_from_the_guards_text(self):
        self.assertEqual(len(FORMS), 1, f"module-level assignments to WRAPPER: {FORMS}")
        self.assertIsInstance(WRAPPER, tuple)
        self.assertEqual([type(word) for word in WRAPPER], [str, str])
        self.assertEqual(wrapper_assignments("def f():\n    WRAPPER = ('a', 'b')\n"), [])
        twice = "WRAPPER = ('a', 'b')\nOTHER = ('x',)\nWRAPPER: tuple = ('c', 'd')\n"
        self.assertEqual(wrapper_assignments(twice), [("a", "b"), ("c", "d")])

    def test_the_scan_and_the_guard_agree_on_every_separated_member(self):
        members = examined("separated members", separated_population())
        reads = scan_reads([step for _, _, step in members])
        mismatches = [
            f"{member}: the scan reads {read}, the guard {GUARD_READS[program]}"
            for (program, member, _), read in zip(members, reads, strict=True)
            if read != GUARD_READS[program]
        ]
        print(
            f"examined {len(members)}, mismatches {len(mismatches)}; named limits "
            f"{len(NAMED_LIMITS)}: {', '.join(NAMED_LIMITS)}"
        )
        self.assertEqual(mismatches, [])
        self.assertEqual({program for program, _, _ in members}, set(GUARD_READS))
        limited = {prefix for cases in NAMED_LIMITS.values() for prefix, _, _ in cases}
        self.assertEqual(
            [m for _, m, _ in members if any(m.startswith(f"{p} ") for p in limited)], []
        )

    def test_the_old_reading_disagrees_on_every_program_but_the_wrapper(self):
        members = examined("separated members", separated_population())
        reads = scan_reads([step for _, _, step in members], wrapped=every_program_separator)
        mismatched = {program: 0 for program in SEPARATED}
        for (program, member, _), read in zip(members, reads, strict=True):
            if read != GUARD_READS[program]:
                mismatched[program] += 1
                print(f"old reading: {member}: reads {read}, the guard {GUARD_READS[program]}")
        sizes = {program: 4 * len(prefixes) for program, prefixes in SEPARATED.items()}
        self.assertEqual(
            mismatched,
            {program: 0 if program == "the wrapper" else sizes[program] for program in SEPARATED},
        )

    def test_the_scan_departs_from_the_guard_only_at_its_two_named_limits(self):
        cases = [
            (name, prefix, guard, scan)
            for name, members in NAMED_LIMITS.items()
            for prefix, guard, scan in members
        ]
        cases += [("twin", prefix, "refused", "refused") for prefix in TWINS]
        reads = scan_reads([f"      - run: {prefix} {COMMAND}\n" for _, prefix, _, _ in cases])
        self.assertEqual(
            [(name, prefix, read) for (name, prefix, _, _), read in zip(cases, reads, strict=True)],
            [(name, prefix, scan) for name, prefix, _, scan in cases],
        )
        for name, prefix, guard, scan in cases:
            if name == "twin":
                self.assertEqual(guard, scan, prefix)
            else:
                self.assertNotEqual(guard, scan, prefix)


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
        running = mutants_jobs(WORKFLOWS)
        for workflow_name, name, job in examined("jobs that run cargo-mutants", running):
            self.assertRegex(
                job,
                r"(?m)^\s*tool: cargo-nextest@0\.9\.146$",
                f"{workflow_name}:{name} runs cargo-mutants without the nextest it names",
            )


class EveryMutationCommandRunsTheMutantsProfile(unittest.TestCase):
    def test_every_mutation_command_runs_the_mutants_profile(self):
        # A caught mutant stops at its first failing test, and one no test kills still runs every
        # test (SPEC-362 R9): every cargo-mutants command reads .cargo/mutants.toml, which runs
        # nextest under the profile .config/nextest.toml declares, and no command turns it off.
        nextest = REPO / ".config" / "nextest.toml"
        self.assertTrue(nextest.is_file(), ".config/nextest.toml does not exist")
        profiles = tomllib.loads(nextest.read_text(encoding="utf-8")).get("profile", {})
        self.assertEqual(
            profiles.get("mutants", {}).get("fail-fast"),
            {"max-fail": 1, "terminate": "immediate"},
        )
        config = tomllib.loads((REPO / ".cargo" / "mutants.toml").read_text(encoding="utf-8"))
        self.assertEqual(config.get("test_tool"), "nextest")
        self.assertEqual(config.get("additional_cargo_test_args"), ["--profile", "mutants"])
        commands = [
            (name, command)
            for name, command in mutants_commands(WORKFLOWS)
            if name in (CI.name, WEEKLY.name)
        ]
        for name, command in examined("cargo-mutants commands of ci and the battery", commands):
            for flag in ("--no-config", "--test-tool", "--cargo-test-arg"):
                self.assertNotIn(flag, command, f"{name}: {command}")


# --------------------------------------------------------------------------- SPEC-057

SCOPED = '${PACKAGE:+--package "$PACKAGE"}'
LIB = "crates/fix/src/lib.rs"
LIB_TEXT = "pub fn double(x: i64) -> i64 {\n    x * 2 * 1\n}\n"


def listed(name, package="deck-streak-fix", column=7, description="replace * with + in double"):
    """One mutant of LIB_TEXT's second line, as cargo-mutants lists and reports it."""
    return {
        "file": LIB,
        "package": package,
        "name": f"{LIB}:2:{column}: {description}{name}",
        "span": {"start": {"line": 2, "column": column}, "end": {"line": 2, "column": column + 1}},
    }


def shard(reports, number, entries, code="2"):
    """Shard `number`'s artifact: its exit and, unless `entries` is None, its outcomes.json."""
    directory = reports / f"mutants-shard-{number}"
    (directory / "mutants.out").mkdir(parents=True)
    (directory / "memory-scope.json").write_text(json.dumps(ZERO_SCOPE), encoding="utf-8")
    (directory / "cargo-mutants.exit").write_text(f"{code}\n", encoding="utf-8")
    if entries is None:
        return
    outcomes = [{"scenario": "Baseline", "summary": "Success"}]
    outcomes += [
        {"scenario": {"Mutant": mutant}, "summary": summary} for mutant, summary in entries
    ]
    counted = {"caught": 0, "missed": 0, "timeout": 0, "unviable": 0}
    for _, summary in entries:
        counted[{"CaughtMutant": "caught", "MissedMutant": "missed"}[summary]] += 1
    report = {"outcomes": outcomes, "total_mutants": len(entries), **counted}
    (directory / "mutants.out" / "outcomes.json").write_text(json.dumps(report), encoding="utf-8")


def recorded_tree(root, anchor="x * 2", description="replace * with + in double"):
    """A tree holding LIB and one record of its mutant at column 7."""
    (root / "crates" / "fix" / "src").mkdir(parents=True)
    (root / LIB).write_text(LIB_TEXT, encoding="utf-8")
    fragments = root / "scripts" / "mutation-equivalent.d"
    fragments.mkdir(parents=True)
    record = {
        "file": LIB,
        "mutant": description,
        "anchor": anchor,
        "reason": "the fixture's one caller cannot tell the products apart",
        "evidence": "double is called as double(1) alone",
        "reached_by": "double::one_doubles_to_two",
        "issue": "#1",
    }
    (fragments / "deck-streak-fix.json").write_text(json.dumps({"records": [record]}), "utf-8")


class TheBatteryTakesAScope(unittest.TestCase):
    def test_a_dispatch_scoped_to_one_package_sweeps_only_its_mutants(self):
        text = workflow(WEEKLY)
        dispatch = re.search(r"(?ms)^  workflow_dispatch:\n(.*?)(?=^  [a-z_]+:\n)", text)
        self.assertIsNotNone(dispatch, "the battery takes no dispatch")
        package = re.search(
            r"(?ms)^      package:\n(.*?)(?=^      [a-z_]+:\n|\Z)", dispatch.group(1)
        )
        self.assertIsNotNone(package, "the dispatch takes no package input")
        self.assertRegex(package.group(1), r"(?m)^        type: string$")
        self.assertRegex(package.group(1), r'(?m)^        default: ""$')
        found = jobs(text)
        # The rust shards sweep the package the input names, and none for the Mini App; the input
        # reaches the shell through the environment, never interpolated into it.
        rust = found.get("rust", "")
        self.assertRegex(rust, r"(?m)^    if: \$\{\{ [^\n]*inputs\.package != 'miniapp'")
        self.assertRegex(rust, r"(?m)^\s+PACKAGE: \$\{\{ inputs\.package \}\}$")
        commands = re.findall(r"cargo mutants [^\n]*", rust)
        self.assertEqual(len(commands), 2, "the shard runs one branch per package state")
        self.assertIn('--package="$PACKAGE"', commands[0])
        self.assertNotIn("--package", commands[1])
        for command in commands:
            self.assertIn('--shard "$SHARD/$SHARDS"', command)
        self.assertRegex(rust, r"(?m)^\s+SHARD: \$\{\{ matrix\.shard \}\}$")
        self.assertRegex(rust, r"(?m)^\s+SHARDS: \$\{\{ needs\.size\.outputs\.shards \}\}$")
        for name, job in examined("battery jobs", list(found.items())):
            for block in re.findall(r"(?ms)^        run: [|]?\n?(.*?)(?=^      - |\Z)", job):
                self.assertNotIn("inputs.package", block, f"{name} interpolates the input")
        # The Mini App's sweep runs for no scope or for miniapp; the rows only for no scope.
        self.assertRegex(
            found.get("web", ""),
            r"(?m)^    if: \$\{\{ [^\n]*\(!inputs\.package \|\| inputs\.package == 'miniapp'\)",
        )
        self.assertRegex(
            found.get("rows", ""), r"(?m)^    if: \$\{\{ [^\n]*&& !inputs\.package \}\}"
        )
        # The survivors job ends by counting what the scope promised, then the scope's table.
        last = steps(found.get("survivors", ""))[-1]
        counted = re.search(r"mutation-verdict\.py battery [^\n]*", last)
        tabled = re.search(r"mutation-verdict\.py table [^\n]*", last)
        self.assertIsNotNone(counted, "the survivors job ends without the battery's count")
        self.assertIsNotNone(tabled, "the survivors job ends without the table")
        self.assertLess(counted.start(), tabled.start())
        for line in (counted.group(0), tabled.group(0)):
            self.assertIn(SCOPED, line)
            self.assertIn('--listed "$reports/listing/whole.json"', line)
        # Behaviour: three mutants of the scope over five shards owe reports from shards 0 to 2,
        # and the table ends with the package's own line.
        with tempfile.TemporaryDirectory() as scratch:
            reports = Path(scratch) / "reports"
            scope = [listed(f" ({n})") for n in range(3)]
            other = [listed(f" ({n})", package="deck-streak-other") for n in range(4)]
            (reports / "listing").mkdir(parents=True)
            (reports / "listing" / "whole.json").write_text(json.dumps(other + scope), "utf-8")
            for number, mutant in enumerate(scope):
                shard(reports, number, [(mutant, "CaughtMutant")], code="0")
            # Shard 3's run found no mutant of the scope: cargo-mutants exited 0 and wrote none.
            shard(reports, 3, None, code="0")
            listing = str(reports / "listing" / "whole.json")
            args = ["--reports", str(reports), "--package", "deck-streak-fix", "--listed", listing]
            battery = run(str(VERDICT), "battery", "--shards", "5", *args)
            table = run(str(VERDICT), "table", "--root", scratch, *args)
            unscoped = run(str(VERDICT), "battery", "--shards", "5", "--reports", str(reports))
        self.assertEqual(battery.returncode, 0, battery.stdout + battery.stderr)
        self.assertIn("battery: counted 4 of 4 reports whole", battery.stdout)
        for number in examined("shards the scope gave no mutant", [3, 4]):
            self.assertIn(
                f"battery: mutants-shard-{number}: the scope lists no mutant", battery.stdout
            )
        self.assertEqual(table.returncode, 0, table.stdout + table.stderr)
        self.assertEqual(
            table.stdout.strip().splitlines()[-1],
            "table: deck-streak-fix: listed 3, killed 3, equivalent 0, unexplained 0, unviable 0",
        )
        self.assertNotIn("deck-streak-other", table.stdout)
        # Unscoped, the battery owes every shard, the rows and the Stryker sweep.
        self.assertEqual(unscoped.returncode, 1, unscoped.stdout)
        self.assertIn("battery: MISSING mutants-shard-4: no outcomes.json", unscoped.stdout)


class TheVerdictBindsEveryRecord(unittest.TestCase):
    def test_the_verdict_binds_every_record_against_the_whole_listing(self):
        found = jobs(workflow(CI))
        plan = steps(found.get("mutation-plan", ""))
        diff = [step for step in plan if re.search(r"cargo mutants [^\n]*--in-diff", step)]
        whole = [
            step
            for step in plan
            if re.search(r"cargo mutants [^\n]*--list --json", step) and "--in-diff" not in step
        ]
        self.assertEqual(len(diff), 1, "the plan lists the diff's mutants once")
        self.assertEqual(len(whole), 1, "the plan never lists the whole tree's mutants")
        # Whenever the plan lists the diff's mutants it lists the whole tree's: the same condition.
        condition = [re.search(r"(?m)^\s+if: (.*)$", step).group(1) for step in (*diff, *whole)]
        self.assertEqual(condition[0], condition[1])
        self.assertIn('> "$RUNNER_TEMP/mutation/whole.json"', whole[0])
        self.assertRegex(
            found.get("mutation-verdict", ""),
            r'judge [^\n]*--class rust [^\n]*--whole "\$reports/mutation-plan/whole\.json"',
        )
        # The weekly battery lists the whole tree too, and its survivors job reads that listing.
        weekly = jobs(workflow(WEEKLY))
        listing = weekly.get("listing", "")
        self.assertRegex(listing, r"cargo mutants [^\n]*--list --json [^\n]*> \"\$RUNNER_TEMP")
        self.assertNotRegex(listing, r"cargo mutants [^\n]*--package")
        self.assertRegex(listing, r"(?m)^\s+name: listing$")
        needs = re.search(r"(?m)^    needs: \[([^\]]*)\]$", weekly.get("survivors", ""))
        self.assertIsNotNone(needs, "the survivors job needs nothing")
        self.assertIn("listing", needs.group(1).replace(" ", "").split(","))
        # Behaviour: the battery drafts no issue for an equivalent mutant.
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch) / "tree"
            recorded_tree(root)
            reports = Path(scratch) / "reports"
            equivalent = listed("")
            unexplained = listed("", column=11, description="replace * with / in double")
            shard(reports, 0, [(equivalent, "MissedMutant"), (unexplained, "MissedMutant")])
            drafts = Path(scratch) / "drafts"
            both = run(
                str(VERDICT), "survivors", "--reports", str(reports), "--out", str(drafts),
                "--root", str(root),
            )  # fmt: skip
            manifest = json.loads((drafts / "drafts.json").read_text(encoding="utf-8"))
            body = (drafts / manifest[0]["body"]).read_text(encoding="utf-8") if manifest else ""
            # With the unexplained mutant killed, the file's only survivor is equivalent.
            only = Path(scratch) / "only"
            shard(only, 0, [(equivalent, "MissedMutant"), (unexplained, "CaughtMutant")])
            quiet = Path(scratch) / "quiet"
            none = run(
                str(VERDICT), "survivors", "--reports", str(only), "--out", str(quiet),
                "--root", str(root),
            )  # fmt: skip
            silent = json.loads((quiet / "drafts.json").read_text(encoding="utf-8"))
        self.assertEqual(both.returncode, 0, both.stdout + both.stderr)
        self.assertEqual([entry["title"] for entry in manifest], [f"Mutation survivors: {LIB}"])
        self.assertIn(unexplained["name"], body)
        self.assertNotIn(equivalent["name"], body)
        self.assertEqual(none.returncode, 0, none.stdout + none.stderr)
        self.assertEqual(silent, [], none.stdout)


class TheBriefTeachesTheRecord(unittest.TestCase):
    def test_the_builder_brief_teaches_the_equivalence_record(self):
        text = BRIEF.read_text(encoding="utf-8")
        section = re.search(r"(?ms)^## Mutation testing\n(.*?)(?=^## |\Z)", text)
        self.assertIsNotNone(section, "docs/BUILDER-BRIEF.md has no Mutation testing section")
        taught = [
            "scripts/mutation-equivalent.d/<package>.json",
            "miniapp.json",
            "ADR-070",
            *(f"`{field}`" for field in ("file", "mutant", "anchor", "span", "reason")),
            *(f"`{field}`" for field in ("evidence", "reached_by", "issue")),
            "python3 scripts/mutation-verdict.py census",
            "mutation-verdict.py table",
            "STALE",
            "REFUTED",
            "UNCOVERED",
        ]
        for needle in examined("parts of the record the brief teaches", taught):
            self.assertIn(needle, section.group(1))
        # No exclusion is named as a way to record an equivalent, in any place that teaches it.
        forms = ["exclude_re", "exclude_globs", "Stryker disable", "mutants::skip", "(#N)`:"]
        config = (REPO / ".cargo" / "mutants.toml").read_text(encoding="utf-8")
        stryker = json.loads((REPO / "web" / "app" / "stryker.config.json").read_text("utf-8"))
        with tempfile.TemporaryDirectory() as scratch:
            reports = Path(scratch) / "reports"
            shard(reports, 0, [(listed(""), "MissedMutant")])
            run(str(VERDICT), "survivors", "--reports", str(reports), "--out", f"{scratch}/d")
            draft = (Path(scratch) / "d" / "draft-001.md").read_text(encoding="utf-8")
        places = [
            ("docs/BUILDER-BRIEF.md", section.group(1)),
            (".cargo/mutants.toml", config),
            ("web/app/stryker.config.json", stryker["_comment"]),
            ("the survivors' draft", draft),
        ]
        for name, words in examined("places that teach the record", places):
            self.assertIn("scripts/mutation-equivalent.d/", words, name)
            for form in forms:
                self.assertNotIn(form, words, f"{name} teaches {form}")


if __name__ == "__main__":
    unittest.main()
