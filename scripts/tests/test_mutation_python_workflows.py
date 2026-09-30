"""The Python runner's jobs, the weekly battery's Python shards and the documents that teach them
(SPEC-087 A20 to A22).

The workflows are read as text, job by job, with the helpers `test_mutation_workflows.py` reads
them with; the documents are read as text, section by section.
"""

import re
import unittest

from _support import REPO, examined
from test_mutation_workflows import CI, WEEKLY, jobs, steps, uploads_always, workflow

DOCS = REPO / "docs"
BRIEF = DOCS / "BUILDER-BRIEF.md"
TESTING = DOCS / "TESTING.md"
SPEC_039 = DOCS / "specs" / "SPEC-039-every-change-proves-its-tests-kill-its-mutants.md"
ADR_057 = (
    DOCS / "decisions" / "ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md"
)
ADR_073 = (
    DOCS
    / "decisions"
    / "ADR-073-the-repositorys-python-is-mutated-by-a-runner-of-its-own-restored-by-digest.md"
)


def text(path):
    """A document's text; its absence is the criterion failing, never a crash."""
    if not path.is_file():
        raise AssertionError(f"{path.relative_to(REPO)} does not exist")
    return path.read_text(encoding="utf-8")


def section(body, heading):
    """The block under the `## ` heading that starts with `heading`, to the next `## `."""
    match = re.search(rf"(?ms)^## {re.escape(heading)}.*?(?=^## |\Z)", body)
    return match.group(0) if match else ""


class ThePythonJobsRunInCi(unittest.TestCase):
    def test_the_python_job_runs_the_plans_shards_and_the_verdict_reads_them(self):
        found = jobs(workflow(CI))
        plan, python, verdict, gate = (
            found.get(name, "")
            for name in ("mutation-plan", "mutation-python", "mutation-verdict", "ci")
        )
        # The plan lists the diff's Python and the whole population, and sizes the matrix from the first.
        self.assertRegex(plan, r"mutation_python\.py list --plan [^\n]*--out ")
        self.assertRegex(plan, r"mutation_python\.py list --all --out ")
        self.assertRegex(plan, r"mutation-verdict\.py shards [^\n]*--python-listed ")
        for output in examined("plan outputs", ["python_shards", "python_matrix"]):
            self.assertRegex(
                plan, rf"(?m)^      {output}: \$\{{\{{ steps\.shards\.outputs\.{output} \}}\}}$"
            )
        # One job per shard of that matrix, on every event, named, bounded and never cached.
        self.assertRegex(python, r"(?m)^    needs: \[mutation-plan\]$")
        self.assertRegex(python, r"(?m)^      fail-fast: false$")
        self.assertRegex(
            python,
            r"(?m)^        shard: \$\{\{ fromJSON\(needs\.mutation-plan\.outputs\.python_matrix\) \}\}$",
        )
        self.assertNotRegex(python, r"(?m)^    if:", "the job is skipped on some event")
        self.assertRegex(python, r"(?m)^    timeout-minutes: \d+$")
        self.assertRegex(python, r"(?m)^\s+SHARD: \$\{\{ matrix\.shard \}\}$")
        self.assertRegex(
            python, r"(?m)^\s+SHARDS: \$\{\{ needs\.mutation-plan\.outputs\.python_shards \}\}$"
        )
        self.assertRegex(python, r"echo \"mutation-python: shard \$SHARD of \$SHARDS: \$CASE\"")
        command = re.search(r"mutation_python\.py run [^\n]*", python)
        self.assertIsNotNone(command, "mutation-python runs no runner")
        for flag in examined(
            "runner flags", ["--plan ", '--shard "$SHARD/$SHARDS"', "--failfast", "--report "]
        ):
            self.assertIn(flag, command.group(0))
        self.assertTrue(uploads_always(python), "the report is not kept whatever the run returned")
        self.assertRegex(python, r"(?m)^\s+name: mutation-python-shard-\$\{\{ matrix\.shard \}\}$")
        self.assertNotIn("actions/cache", python)
        self.assertNotIn("cache:", python)
        # The verdict waits on it and judges both Python classes over the downloaded reports.
        self.assertRegex(
            verdict,
            r"(?m)^    needs: \[mutation-plan, mutation-rust, mutation-python, mutation-rows\]$",
        )
        for klass in examined("Python classes", ["scripts", "oracle"]):
            self.assertRegex(
                verdict,
                rf"judge [^\n]*--class {klass} [^\n]*--python \"\$reports\"[^\n]*--rows ",
            )
        self.assertIn("mutation-python", re.search(r"(?m)^    needs: .*$", gate).group(0))
        self.assertGreater(len(steps(python)), 3)


class TheWeeklyBatterySweepsThePython(unittest.TestCase):
    def test_the_weekly_battery_sweeps_the_whole_python_population(self):
        found = jobs(workflow(WEEKLY))
        python, listing, survivors, rust, web = (
            found.get(name, "") for name in ("python", "listing", "survivors", "rust", "web")
        )
        self.assertRegex(
            python, r"(?m)^        shard: \[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15\]$"
        )
        command = re.search(r"mutation_python\.py run [^\n]*", python)
        self.assertIsNotNone(command, "the python job runs no runner")
        self.assertIn("--all", command.group(0))
        self.assertIn("--shard ${{ matrix.shard }}/16", command.group(0))
        self.assertNotIn("--failfast", python, "a weekly shard stops at a first survivor")
        self.assertTrue(uploads_always(python), "the report is not kept whatever the run returned")
        self.assertRegex(python, r"(?m)^\s+name: mutation-python-shard-\$\{\{ matrix\.shard \}\}$")
        self.assertRegex(listing, r"mutation_python\.py list --all --out ")
        self.assertRegex(survivors, r"(?m)^    needs: \[size, rust, python, web, rows, listing\]$")
        # A scope names what runs: a crate's or the Mini App's runs no Python shard, and `python`
        # runs no Rust shard and no Stryker sweep.
        self.assertRegex(
            python, r"(?m)^    if: .*!inputs\.package \|\| inputs\.package == 'python'"
        )
        self.assertIn("inputs.package != 'python'", re.search(r"(?m)^    if: .*$", rust).group(0))
        self.assertNotIn("python", re.search(r"(?m)^    if: .*$", web).group(0))
        self.assertIn("python", workflow(WEEKLY).split("package:", 1)[1].split("\n", 2)[1])


class TheRehearsalRunsOnePythonShard(unittest.TestCase):
    def test_the_rehearsal_runs_one_python_file_and_the_battery_counts_that_shard(self):
        rehearsal = jobs(workflow(WEEKLY)).get("rehearsal", "")
        command = re.search(r"mutation_python\.py run [^\n]*", rehearsal)
        self.assertIsNotNone(command, "the rehearsal runs no python runner")
        for flag in examined(
            "rehearsal runner flags",
            ["--file scripts/audit-web-verdict.py", "--shard 0/1", "mutation-python-shard-0"],
        ):
            self.assertIn(flag, rehearsal, flag)
        self.assertRegex(rehearsal, r"mutation-verdict\.py battery [^\n]*--python-shards 1\b")


class TheDocumentsTeachThePythonRun(unittest.TestCase):
    def test_the_builder_brief_and_the_amendments_teach_the_python_run(self):
        brief, testing = text(BRIEF), text(TESTING)
        for name, body in examined("documents", [("BUILDER-BRIEF", brief), ("TESTING", testing)]):
            self.assertIn("mutation_python.py", body, name)
            self.assertIn("mutation-python", body, name)
        self.assertIn("scripts/mutation-equivalent.d/python.json", brief)
        self.assertRegex(brief, r"(?i)a surviving python mutant[^\n]*\n?[^\n]*two ways")
        self.assertRegex(brief, r"(?i)kill it with a test|test that kills")
        for skip in ("nomutate", "no mutate", "pragma", "mutmut"):
            self.assertNotIn(skip, brief.lower(), f"the brief names a skip: {skip}")
        amendment = re.search(
            r"(?ms)^## \d+\. Amendment, 2026-09-29: the Python[^\n]*\n.*?(?=^## |\Z)",
            text(SPEC_039),
        )
        self.assertIsNotNone(amendment, "SPEC-039 carries no dated amendment for the Python")
        self.assertIn("ADR-073", amendment.group(0))
        self.assertIn("SPEC-087", amendment.group(0))
        note = section(text(ADR_057), "Note, 2026-09-29")
        self.assertIn("ADR-073", note)
        self.assertRegex(text(ADR_073), r"(?m)^status: accepted$")


if __name__ == "__main__":
    unittest.main()
