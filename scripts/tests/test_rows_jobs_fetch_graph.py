"""Every CI job that proves mutation rows fetches the locked dependency graph first (SPEC-332).

A job restores the Rust cache by key, and with no exact hit it restores an older one by key prefix,
whose registry can lack a package the current `Cargo.lock` adds. The rows runner's first cargo call
is offline, so such a job read every row VOID (#606). The workflows are read as text, job by job,
the way `test_mutation_workflows.py` reads them: a job is the block under its two-space name, and a
step is the block under its `- ` at six spaces.
"""

import re
import unittest

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
PROVE = "mutation_rows.py prove"
FETCH = "cargo fetch --locked"
RESTORE = re.compile(r"uses:\s*actions/cache(/restore)?@")


def workflow_texts():
    """Every `.yml` and `.yaml` workflow as {file name: its text}; none is VOID."""
    paths = examined(
        "workflow files",
        sorted(p for p in WORKFLOWS.iterdir() if p.suffix in (".yml", ".yaml")),
    )
    return {p.name: p.read_bytes().decode("utf-8") for p in paths}


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


def code_lines(step):
    """A step's lines without comments and blank lines."""
    return [
        line.strip()
        for line in step.splitlines()
        if line.strip() and not line.strip().startswith("#")
    ]


def runs(step, needle):
    return any(needle in line for line in code_lines(step))


def proving_jobs():
    """[(file, job, its steps)] for every job that runs the rows runner's `prove`."""
    found = []
    for name, text in workflow_texts().items():
        for job, block in jobs(text).items():
            job_steps = steps(block)
            if any(runs(step, PROVE) for step in job_steps):
                found.append((name, job, job_steps))
    return examined("jobs that run mutation_rows.py prove", found)


def refusals(file, job, job_steps):
    """Why a proving job does not fetch the locked graph before its first prove; [] when it does."""
    where = f"{file}:{job}"
    prove = next(i for i, step in enumerate(job_steps) if runs(step, PROVE))
    restores = [i for i, step in enumerate(job_steps) if RESTORE.search(step)]
    fetches = [i for i, step in enumerate(job_steps) if runs(step, FETCH)]
    if not fetches:
        return [f"{where}: no step runs `{FETCH}`"]
    out = []
    if not restores:
        out.append(f"{where}: no cache-restore step to order the fetch after")
    elif fetches[0] <= restores[-1]:
        out.append(f"{where}: the fetch (step {fetches[0]}) is not after the last cache restore")
    if fetches[0] >= prove:
        out.append(f"{where}: the fetch (step {fetches[0]}) is not before the first prove")
    step = job_steps[fetches[0]]
    if "continue-on-error" in step:
        out.append(f"{where}: the fetch step carries continue-on-error")
    swallow = [line for line in code_lines(step) if "||" in line or "set +e" in line]
    if swallow:
        out.append(f"{where}: the fetch step swallows its exit code: {swallow[0]}")
    return out


class EveryRowsJobFetchesTheLockedGraph(unittest.TestCase):
    """SPEC-332 A1 and A2 (#606)."""

    def test_each_proving_job_fetches_after_its_cache_restore_and_before_prove(self):
        found = []
        for file, job, job_steps in proving_jobs():
            found.extend(
                line
                for line in refusals(file, job, job_steps)
                if "swallows" not in line and "continue-on-error" not in line
            )
        self.assertEqual([], found)

    def test_the_fetch_step_fails_its_job_when_the_fetch_fails(self):
        found = []
        for file, job, job_steps in proving_jobs():
            if any(runs(step, FETCH) for step in job_steps):
                found.extend(
                    line
                    for line in refusals(file, job, job_steps)
                    if "swallows" in line or "continue-on-error" in line
                )
            else:
                found.append(f"{file}:{job}: no fetch step to judge")
        self.assertEqual([], found)


JOB = (
    "jobs:\n  j:\n    steps:\n      - uses: actions/cache/restore@abc\n{middle}"
    "      - run: python3 scripts/mutation_rows.py prove --all\n"
)


def planted(middle):
    job = jobs("\n" + JOB.format(middle=middle))["j"]
    return refusals("planted.yml", "j", steps(job))


class TheJudgeRefusesEachPlantedShape(unittest.TestCase):
    """The positive controls: a judge that went blind to a shape fails here."""

    def test_a_good_job_is_accepted(self):
        self.assertEqual([], planted("      - run: cargo fetch --locked\n"))

    def test_a_job_with_no_fetch_is_refused(self):
        self.assertEqual(["planted.yml:j: no step runs `cargo fetch --locked`"], planted(""))

    def test_a_fetch_after_prove_is_refused(self):
        job = jobs(
            "\n"
            "jobs:\n  j:\n    steps:\n      - uses: actions/cache/restore@abc\n"
            "      - run: python3 scripts/mutation_rows.py prove --all\n"
            "      - run: cargo fetch --locked\n"
        )["j"]
        got = refusals("planted.yml", "j", steps(job))
        self.assertEqual(["planted.yml:j: the fetch (step 2) is not before the first prove"], got)

    def test_a_fetch_before_the_restore_is_refused(self):
        job = jobs(
            "\n"
            "jobs:\n  j:\n    steps:\n      - run: cargo fetch --locked\n"
            "      - uses: actions/cache/restore@abc\n"
            "      - run: python3 scripts/mutation_rows.py prove --all\n"
        )["j"]
        got = refusals("planted.yml", "j", steps(job))
        self.assertEqual(
            ["planted.yml:j: the fetch (step 0) is not after the last cache restore"], got
        )

    def test_a_continue_on_error_fetch_is_refused(self):
        got = planted("      - run: cargo fetch --locked\n        continue-on-error: true\n")
        self.assertEqual(["planted.yml:j: the fetch step carries continue-on-error"], got)

    def test_a_swallowed_fetch_is_refused(self):
        got = planted("      - run: cargo fetch --locked || true\n")
        self.assertEqual(
            [
                "planted.yml:j: the fetch step swallows its exit code: "
                "- run: cargo fetch --locked || true"
            ],
            got,
        )

    def test_a_commented_fetch_is_no_fetch(self):
        got = planted("      - run: |\n          # cargo fetch --locked\n          true\n")
        self.assertEqual(["planted.yml:j: no step runs `cargo fetch --locked`"], got)


if __name__ == "__main__":
    unittest.main()
