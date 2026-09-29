"""The mutation verdict reads each report by name (SPEC-126 A1 to A4, issue #351).

`actions/download-artifact` at the pinned v8.0.1 lays a download out by how many artifacts matched
(`src/download-artifact.ts`): it extracts into `path` itself when the download is by `name`, when
`merge-multiple` is true, or when exactly one artifact matched, and into `path/<artifact name>` only
for two or more matches without `merge-multiple`. These tests apply that rule, in pure Python, to
the verdict's download steps and the shard job's output directory as `ci.yml` writes them, over
every combination of shard count, rows report and `mutation-web` upload, and assert the paths the
judge reads. Nothing here runs GitHub: the pull request's own verdict run is the live check.

The workflow is read as text, job by job, the way `test_mutation_workflows.py` reads it.
"""

import fnmatch
import posixpath
import re
import shlex
import unittest

from _support import examined
from test_mutation_workflows import CI, jobs, steps, workflow

TEMP = "T"
SHARD_COUNTS = (0, 1, 3)
DOWNLOAD = "actions/download-artifact@"
UPLOAD = "actions/upload-artifact@"
ROWS_PLANNED = "needs.mutation-plan.outputs.rows == 'true'"


REPORTS = "$reports"
PLAN = f"{REPORTS}/mutation-plan/plan.json"
ROWS = f"{REPORTS}/mutation-rows/rows.json"
# The flags each named class's judge line must carry, and the path each reads. A class the table
# does not name (a later class another change adds) is neither asserted nor an error.
JUDGE_FLAGS = {
    "rust": {
        "--plan": PLAN,
        "--shard-reports": REPORTS,
        "--rows": ROWS,
        "--whole": f"{REPORTS}/mutation-plan/whole.json",
    },
    "oracle": {"--plan": PLAN, "--rows": ROWS},
}
JUDGE = re.compile(r"(?m)^\s*python3 scripts/mutation-verdict\.py judge (.*?)(?: \|\| \w+=\$\?)?$")


def judge_lines(verdict):
    """{class: {flag: value}} for each `mutation-verdict.py judge` command line of the verdict
    job, read one line at a time; a line with no `--class` is refused, and zero lines is too."""
    found = {}
    for command in examined("judge command lines", JUDGE.findall(verdict)):
        words = shlex.split(command)
        flags = {w: words[i + 1] for i, w in enumerate(words[:-1]) if w.startswith("--")}
        if "--class" not in flags:
            raise AssertionError(f"a judge line names no --class: {command}")
        found[flags["--class"]] = flags
    return found


class StepFailed(Exception):
    """A download step that the action would fail."""


def inputs(step):
    """The `with:` inputs of a step, as {key: value}."""
    found = {}
    block = re.search(r"(?ms)^        with:\n(.*?)(?=^      - |^        \w|\Z)", step)
    for line in (block.group(1) if block else "").splitlines():
        match = re.match(r"^          ([a-z-]+): (.*)$", line)
        if match:
            found[match.group(1)] = match.group(2).strip()
    return found


def local(path):
    """A workflow path under the runner's temp directory, as a path relative to it."""
    return posixpath.normpath(path.replace("${{ runner.temp }}", TEMP)).strip("/")


def shard_artifact(shard):
    """(name, {file inside the artifact}) for one shard, from the shard job's own text."""
    rust = jobs(workflow(CI))["mutation-rust"]
    made = re.search(r'out="\$RUNNER_TEMP/([^"]+)"', rust)
    upload = next(s for s in steps(rust) if UPLOAD in s and "mutation-rust-shard-" in s)
    given = inputs(upload)
    if made is None or "path" not in given:
        raise AssertionError("mutation-rust does not name its output directory and its upload")
    out = local(f"{TEMP}/" + made.group(1).replace("$SHARD", str(shard)))
    root = local(given["path"])
    if out != root and not out.startswith(root + "/"):
        raise AssertionError(f"the shard writes to {out}, outside what it uploads ({root})")
    name = given["name"].replace("${{ matrix.shard }}", str(shard))
    inside = posixpath.relpath(out, root)
    prefix = "" if inside == "." else inside + "/"
    files = {f"{prefix}mutants.out/outcomes.json", f"{prefix}cargo-mutants.exit"}
    return name, files


def artifacts(shards, rows, web):
    """{artifact name: {file inside it}} the run uploaded, as the jobs' own text produces them."""
    found = {"mutation-plan": {"plan.json", "whole.json", "git.diff"}}
    for shard in range(shards):
        name, files = shard_artifact(shard)
        found[name] = files
    if rows:
        found["mutation-rows"] = {"rows.json"}
    if web:
        found["mutation-web"] = {"plan.json", "stryker/mutation.json"}
    return found


def runs(step, rows):
    """Whether the step's `if:` holds; a condition this test does not model is refused."""
    match = re.search(r"(?m)^        if: (.*)$", step)
    if match is None:
        return True
    condition = match.group(1).replace("${{", "").replace("}}", "").strip()
    if condition == ROWS_PLANNED:
        return rows
    raise AssertionError(f"a download condition this test does not model: {condition}")


def layout(uploaded, rows):
    """The files under the temp directory after the verdict's downloads, as the pinned action
    lays them out. A download by name of an artifact that does not exist fails the step."""
    verdict = jobs(workflow(CI))["mutation-verdict"]
    placed = set()
    for step in steps(verdict):
        if DOWNLOAD not in step or not runs(step, rows):
            continue
        given = inputs(step)
        destination = local(given.get("path", ""))
        if "name" in given:
            if given["name"] not in uploaded:
                raise StepFailed(f"no artifact named {given['name']}")
            matched = [given["name"]]
        else:
            matched = sorted(n for n in uploaded if fnmatch.fnmatchcase(n, given["pattern"]))
        flat = "name" in given or given.get("merge-multiple") == "true" or len(matched) == 1
        for name in matched:
            base = destination if flat else posixpath.join(destination, name)
            placed |= {posixpath.join(base, f) for f in uploaded[name]}
    return placed


def scenarios():
    return [
        (shards, rows, web)
        for shards in SHARD_COUNTS
        for rows in (False, True)
        for web in (False, True)
    ]


class TheVerdictReadsEachReportByName(unittest.TestCase):
    def test_the_layout_holds_for_every_count_of_artifacts(self):
        reports = f"{TEMP}/reports"
        for shards, rows, web in examined("artifact scenarios", scenarios()):
            where = f"{shards} shard(s), rows {rows}, web {web}"
            placed = layout(artifacts(shards, rows, web), rows)
            self.assertIn(f"{reports}/mutation-plan/plan.json", placed, where)
            self.assertIn(f"{reports}/mutation-plan/whole.json", placed, where)
            if rows:
                self.assertIn(f"{reports}/mutation-rows/rows.json", placed, where)
            for shard in range(shards):
                self.assertIn(
                    f"{reports}/mutation-rust-shard-{shard}/mutants.out/outcomes.json",
                    placed,
                    where,
                )
                self.assertIn(
                    f"{reports}/mutation-rust-shard-{shard}/cargo-mutants.exit", placed, where
                )

    def test_the_verdict_reads_no_artifact_of_a_job_it_does_not_need(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        needs = re.search(r"(?m)^    needs: \[(.*)\]$", verdict)
        self.assertIsNotNone(needs, "the verdict names no needs")
        needed = [name.strip() for name in needs.group(1).split(",")]
        self.assertNotIn("mutation-web", needed)
        downloads = [s for s in steps(verdict) if DOWNLOAD in s]
        for step in examined("verdict download steps", downloads):
            given = inputs(step)
            wanted = given.get("name") or given.get("pattern") or "*"
            self.assertFalse(
                fnmatch.fnmatchcase("mutation-web", wanted),
                f"a verdict download reaches mutation-web: {wanted}",
            )
        for shards, rows, _ in examined("artifact scenarios", scenarios()):
            with_web = layout(artifacts(shards, rows, True), rows)
            without = layout(artifacts(shards, rows, False), rows)
            self.assertEqual(with_web, without, f"{shards} shard(s), rows {rows}")

    def test_a_producer_that_uploaded_nothing_leaves_the_reading_unchanged(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        self.assertNotIn("continue-on-error", verdict)
        rows_steps = [
            s for s in steps(verdict) if DOWNLOAD in s and inputs(s).get("name") == "mutation-rows"
        ]
        self.assertEqual(len(rows_steps), 1, "the rows' report is not downloaded by name")
        self.assertIn(ROWS_PLANNED, rows_steps[0], "the rows' download does not track the rows job")
        for shards, _, web in examined("artifact scenarios", scenarios()):
            uploaded = artifacts(shards, False, web)
            placed = layout(uploaded, False)
            stray = [p for p in placed if "/mutation-rows/" in p]
            self.assertEqual(stray, [], f"{shards} shard(s), web {web}")
            with_rows = layout(artifacts(shards, True, web), True)
            self.assertIn(f"{TEMP}/reports/mutation-rows/rows.json", with_rows)

    def test_the_judge_reads_the_paths_the_downloads_lay_down(self):
        verdict = jobs(workflow(CI))["mutation-verdict"]
        self.assertIn('reports="$RUNNER_TEMP/reports"', verdict)
        lines = judge_lines(verdict)
        for cls, wanted in examined("judge classes asserted", list(JUDGE_FLAGS.items())):
            self.assertIn(cls, lines, f"the verdict has no judge line for class {cls}")
            for flag, path in wanted.items():
                self.assertEqual(
                    lines[cls].get(flag),
                    path,
                    f"the {cls} judge line does not read {flag} at {path}",
                )
        for step in [s for s in steps(verdict) if DOWNLOAD in s]:
            destination = local(inputs(step).get("path", ""))
            self.assertTrue(
                destination == f"{TEMP}/reports" or destination.startswith(f"{TEMP}/reports/"),
                f"a download lands outside the judge's reports directory: {destination}",
            )


if __name__ == "__main__":
    unittest.main()
