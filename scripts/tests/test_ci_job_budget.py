"""Each workflow run's job count is held to a recorded budget, at warn (SPEC-662, ADR-684).

One push to dev and one pull request into dev each start some of the workflows in
`.github/workflows/`, and each run starts jobs. The census counts, for every workflow file and each
counted event, the jobs one run can start, by one rule (SPEC-662 R2): each job is one; a job with a
static matrix is its combinations, `exclude` taken first and `include` after, as GitHub expands
them; a job that calls a workflow of the same directory is that workflow's own count, once per
combination of the caller's matrix; a job its condition or a path filter can skip is still
counted, since the bound is what the run can start; and a matrix computed at run time, or a call
the census cannot read, is counted once and named as undecided, with what it reads. Every count is
a lower bound.

Each run is held to `BUDGET` and each undecided job to `UNDECIDED`. At `Level.WARN` the verdict
prints a WARN line for a run past its budget, a run with no budget, an undecided job not listed
and an entry with no reason, and passes them; it prints a STALE line for an entry nothing
matches, and that line fails at both levels. At `Level.FAIL` every WARN and STALE line fails. A
directory with no workflow file, or a file the reader refuses, fails at both levels: it is VOID,
never a pass. The tests read every workflow file with the reader `test_ci_workflows.py` uses, and
the planted trees under `fixtures/job-budget/`. Nothing here runs GitHub.
"""

import enum
import re
import unittest

from _support import REPO, examined
from test_ci_workflows import (
    WORKFLOWS,
    read_workflow,
    workflow_file_text,
    workflow_files,
)

PLANTED = REPO / "scripts" / "tests" / "fixtures" / "job-budget"
#: The events a run is counted for, and the branch each is read against (SPEC-662 R1).
EVENTS = ("push", "pull_request", "pull_request_target")
BRANCH = "dev"
#: A branch filter entry holding one of these is a pattern, never evaluated: the run is counted.
PATTERN = re.compile(r"[*?+\[\]!]")
#: The keys of a matrix that are not axes.
NOT_AXES = ("include", "exclude")


class Level(enum.Enum):
    WARN = "warn"
    FAIL = "fail"


#: Warn until the census's own delivery moves it, under ADR-450 D6's rule (SPEC-662 R9).
LEVEL = Level.WARN

#: Each run's budget: (workflow file, event) to (the most jobs the run may start, why). A change
#: that raises a run past its figure records its reason here (SPEC-662 R6, R7).
BUDGET = {
    ("apple-on-change.yml", "pull_request"): (6, "the Apple job body's six jobs"),
    ("card-speculative.yml", "pull_request"): (1, "one measuring job"),
    ("changelog.yml", "pull_request"): (1, "the fragment check"),
    ("ci.yml", "pull_request"): (25, "the gate's jobs, each plan-dealt matrix once"),
    ("ci.yml", "push"): (25, "the gate's jobs, each plan-dealt matrix once"),
    ("coverage.yml", "pull_request"): (4, "the coverage plan and its three classes"),
    ("coverage.yml", "push"): (4, "the coverage plan and its three classes"),
    ("dependency-review.yml", "pull_request"): (1, "the dependency review"),
    ("engine-measure.yml", "pull_request"): (1, "one measuring job"),
    ("fsrs7-measure.yml", "pull_request"): (1, "one measuring job"),
    ("matrix-limit.yml", "pull_request"): (5, "the rehearsal, each computed matrix once"),
    ("mutation-prices.yml", "pull_request"): (2, "the population, its computed prices once"),
    ("mutation-weekly.yml", "pull_request"): (23, "the battery's jobs, its Rust shards once"),
    ("testflight-internal.yml", "push"): (8, "the plan, the app and the job body's six jobs"),
}

#: Each job the census cannot count: (workflow file, job) to what bounds it (SPEC-662 R7).
UNDECIDED = {
    ("ci.yml", "mutation-python"): "the plan's Python shards, within the share (SPEC-420 R1)",
    ("ci.yml", "mutation-python-release"): "the release legs past one matrix (SPEC-422 R8)",
    ("ci.yml", "mutation-rows"): "the rows legs, within the rows' most legs (SPEC-417 R11)",
    ("ci.yml", "mutation-rust"): "the plan's Rust shards, within the run's ceiling (SPEC-362 R3)",
    ("matrix-limit.yml", "empty"): "an empty axis whose job's condition is false (SPEC-422 R14)",
    ("matrix-limit.yml", "first"): "the dispatch's legs, two on a pull request (SPEC-422 R14)",
    ("matrix-limit.yml", "second"): "the dispatch's legs, two on a pull request (SPEC-422 R14)",
    ("mutation-prices.yml", "price"): "the population job's files (SPEC-421 R11)",
    ("mutation-weekly.yml", "rust"): "the battery's Rust shards, within its ceiling (SPEC-362 R3)",
}


def as_list(value):
    """A filter's entries: a sequence as itself, a lone scalar as one entry."""
    return value if isinstance(value, list) else [value]


def starts(filters):
    """Whether one event's filters start a run on `BRANCH`: no branch filter starts it unless the
    event is filtered by tags alone; `branches` starts it when it lists the branch or a pattern;
    `branches-ignore` starts it unless it lists the branch (SPEC-662 R1)."""
    branches = filters.get("branches")
    ignored = filters.get("branches-ignore")
    if branches is None and ignored is None:
        return "tags" not in filters and "tags-ignore" not in filters
    if branches is not None:
        return any(entry == BRANCH or PATTERN.search(entry) for entry in as_list(branches))
    return BRANCH not in as_list(ignored)


def started(on):
    """The counted events `on` starts a run for, in `EVENTS` order. `on` is a scalar, a list or a
    mapping keyed by event."""
    if isinstance(on, str):
        on = [on]
    if isinstance(on, list):
        on = {event: None for event in on}
    on = on if isinstance(on, dict) else {}
    found = []
    for event in EVENTS:
        if event in on:
            found.append(event)
    return found


def combinations(strategy):
    """(count, source) for one job's matrix: its combinations and None for a static matrix, every
    `exclude` entry dropping each combination it matches and every `include` entry that extends no
    combination adding one; (1, the expression) for a matrix computed at run time; (1, None) for a
    job with no matrix (SPEC-662 R2)."""
    return 1, None


def run_count(name, workflows):
    """(jobs, undecided) for one run of the workflow file `name`: its jobs by the rule, and each
    job it cannot count as (file, job, what it reads). A job-level call of a workflow in
    `workflows` counts that workflow's own jobs; any other call is undecided."""
    jobs, undecided = 0, []
    for job_id, job in (workflows[name].get("jobs") or {}).items():
        count, source = combinations(job.get("strategy"))
        if source is not None:
            undecided.append((name, job_id, source))
        jobs += count
    return jobs, undecided


def runs(directory):
    """({(file, event): (jobs, undecided)}, files examined) for every run a push to `BRANCH` and a
    pull request into it start among `directory`'s workflow files. No file is VOID, and a file the
    reader refuses fails by its name and line."""
    files = workflow_files(directory)
    workflows = {path.name: read_workflow(workflow_file_text(path)) for path in files}
    found = {}
    for name, workflow in workflows.items():
        for event in started(workflow.get("on")):
            found[name, event] = run_count(name, workflows)
    return found, len(files)


def verdict(found, budget, undecided, level):
    """(lines, failures): a RUN line for each run, then a WARN line for each run past or outside
    its budget, each undecided job `undecided` does not list and each entry with no reason, and a
    STALE line for each entry nothing matches. At `Level.WARN` each STALE line fails and no WARN
    line does; at `Level.FAIL` every WARN and STALE line is a failure (SPEC-662 R8, R9)."""
    lines = []
    for (name, event), (jobs, sites) in sorted(found.items()):
        lines.append(f"RUN {name} {event}: {jobs} job(s), {len(sites)} undecided")
    return lines, []


class TheRunsAreCountedByOneRule(unittest.TestCase):
    """SPEC-662 A1 to A5: the planted runs, each counted by the rule (R1, R2)."""

    def planted(self):
        found, files = runs(PLANTED / "planted")
        self.assertEqual(files, 10)
        return found

    def test_a_static_matrix_counts_its_combinations_after_exclude_then_include(self):
        self.assertEqual(self.planted()["matrix.yml", "pull_request"], (6, []))

    def test_a_job_level_call_counts_the_called_jobs_per_combination(self):
        found = self.planted()
        self.assertEqual(found["caller.yml", "push"], (7, []))
        self.assertNotIn(("called.yml", "push"), found)

    def test_a_job_its_condition_can_skip_is_still_counted(self):
        self.assertEqual(self.planted()["skipped.yml", "push"], (4, []))

    def test_a_computed_matrix_or_a_foreign_call_is_counted_once_and_undecided(self):
        self.assertEqual(
            self.planted()["computed.yml", "pull_request"],
            (
                3,
                [
                    ("computed.yml", "legs", "leg: ${{ fromJSON(needs.plan.outputs.legs) }}"),
                    (
                        "computed.yml",
                        "remote",
                        "uses: example-org/example-repo/.github/workflows/build.yml@v1",
                    ),
                ],
            ),
        )

    def test_only_a_push_to_dev_and_a_pull_request_into_dev_start_a_run(self):
        self.assertEqual(
            sorted(self.planted()),
            [
                ("caller.yml", "push"),
                ("computed.yml", "pull_request"),
                ("ignored.yml", "pull_request"),
                ("matrix.yml", "pull_request"),
                ("past.yml", "pull_request"),
                ("pattern.yml", "pull_request_target"),
                ("skipped.yml", "push"),
                ("under.yml", "pull_request"),
                ("under.yml", "push"),
            ],
        )


class TheVerdictHoldsEachRunToItsBudget(unittest.TestCase):
    """SPEC-662 A6 to A8: the verdict over the planted runs, and the empty tree (R6 to R9)."""

    BUDGET = {
        ("past.yml", "pull_request"): (3, "planted under the run's count"),
        ("under.yml", "pull_request"): (3, "planted above the run's count"),
        ("under.yml", "push"): (2, "planted at the run's count"),
        ("gone.yml", "push"): (1, "planted for a workflow the tree does not hold"),
        ("matrix.yml", "pull_request"): (7, ""),
    }
    UNDECIDED = {
        ("computed.yml", "legs"): "planted: the plan's legs",
        ("gone.yml", "legs"): "planted for a job the tree does not hold",
    }

    def problems(self, level):
        found, _files = runs(PLANTED / "planted")
        lines, failures = verdict(found, self.BUDGET, self.UNDECIDED, level)
        return [line for line in lines if not line.startswith("RUN ")], failures

    def test_a_run_past_its_budget_warns_and_a_run_at_or_under_it_does_not(self):
        problems, failures = self.problems(Level.WARN)
        self.assertIn("WARN job-budget past.yml pull_request: 4 job(s), past 3", problems)
        self.assertEqual([line for line in problems if "under.yml" in line], [])
        self.assertNotIn("WARN job-budget past.yml pull_request: 4 job(s), past 3", failures)

    def test_each_unlisted_run_or_job_stale_entry_and_missing_reason_is_named(self):
        problems, failures = self.problems(Level.WARN)
        self.assertEqual(
            problems,
            [
                "WARN job-budget caller.yml push: 7 job(s), no budget",
                "WARN job-budget computed.yml pull_request: 3 job(s), no budget",
                "WARN job-budget-undecided computed.yml remote: "
                "uses: example-org/example-repo/.github/workflows/build.yml@v1",
                "WARN job-budget ignored.yml pull_request: 1 job(s), no budget",
                "WARN job-budget past.yml pull_request: 4 job(s), past 3",
                "WARN job-budget pattern.yml pull_request_target: 1 job(s), no budget",
                "WARN job-budget skipped.yml push: 4 job(s), no budget",
                "STALE job-budget gone.yml push",
                "WARN job-budget matrix.yml pull_request: no reason",
                "STALE job-budget-undecided gone.yml legs",
            ],
        )
        self.assertEqual(
            failures,
            ["STALE job-budget gone.yml push", "STALE job-budget-undecided gone.yml legs"],
        )
        _problems, failures = self.problems(Level.FAIL)
        self.assertEqual(failures, problems)

    def test_an_empty_population_is_void(self):
        with self.assertRaisesRegex(AssertionError, "examined 0 workflow files"):
            runs(PLANTED / "empty")


class TheRepositoryRunsHoldTheirBudget(unittest.TestCase):
    """SPEC-662 A9 (R3 to R9): every workflow file at warn, each run printed with its count."""

    def test_every_run_is_counted_and_held_to_its_budget_at_warn(self):
        self.assertIs(LEVEL, Level.WARN)
        found, files = runs(WORKFLOWS)
        examined("runs a push to dev or a pull request into dev starts", found)
        print(f"examined {files} workflow(s)")
        lines, failures = verdict(found, BUDGET, UNDECIDED, LEVEL)
        for line in lines:
            print(line)
        self.assertEqual(failures, [])


if __name__ == "__main__":
    unittest.main()
