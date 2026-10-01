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
        # (SPEC-129 R4); a scheduled run and a dispatch with no package size to 32.
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


def after_separator(words, program):
    """Where the command a wrapper runs begins: the word after the first standalone `--` that
    follows a program which is not cargo. It names no wrapper and reads no file, so this module
    stays runnable where the wrapper's own tests are not (ADR-306)."""
    if words[program].value.rsplit("/", 1)[-1] in CARGO:
        return None
    for k in range(program + 1, len(words)):
        if not words[k].dynamic and words[k].value == "--":
            return k + 1 if k + 1 < len(words) else None
    return None


def mutants_commands(directory):
    """(workflow name, command line) for every `cargo mutants` command of the directory's
    workflows, found by the one finder the dispatch-shard guard uses; a workflow the finder
    refuses is answered with the refusal, which carries no bounds."""
    found = []
    for path in workflow_files(directory):
        try:
            texts = run_texts(workflow_file_text(path))
            found += [
                (path.name, command)
                for text in texts
                for command in mutants_in(text, wrapped=after_separator)
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
        "python3 scripts/x.py --cap 1 -- cargo mutants --in-place",
        "cargo mutants --in-place",
    ),
)


def planted(command):
    """A workflow whose one job runs `command`."""
    return (
        "name: planted\njobs:\n  shard:\n    runs-on: ubuntu-24.04\n    steps:\n"
        f"      - run: {command}\n"
    )


def definitions_of_the_finder(directory):
    """The files of `directory` that define the command finder, read as text and never imported."""
    return [
        path.name
        for path in sorted(Path(directory).glob("*.py"))
        if re.search(r"(?m)^def mutants_(of\(words|in\(text)", path.read_text(encoding="utf-8"))
    ]


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
        self.assertEqual(defined, ["_mutants_finder.py"])
        with tempfile.TemporaryDirectory() as scratch:
            for name in ("one.py", "two.py"):
                (Path(scratch) / name).write_text("def mutants_in(text, handed=False):\n    pass\n")
            copies = definitions_of_the_finder(scratch)
        self.assertEqual(copies, ["one.py", "two.py"])


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
