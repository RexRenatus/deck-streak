"""Every CI job that proves mutation rows fetches the locked dependency graph first (SPEC-332).

SPEC-416 holds every `ci.yml` job that runs a cargo-mutants baseline to the same rule, judged by the
same `refusals()` with the anchor step it orders the fetch before given as a parameter. SPEC-458
reads that population from every workflow, and holds every job that saves the Rust cache to the
rule too, the save its anchor.

A job restores the Rust cache by key, and with no exact hit it restores an older one by key prefix,
whose registry can lack a package the current `Cargo.lock` adds. The rows runner's first cargo call
is offline, so such a job read every row VOID (#606). The workflows are read as text, job by job,
the way `test_mutation_workflows.py` reads them: a job is the block under its two-space name, and a
step is the block under its `- ` at six spaces.
"""

import re
import unittest
from collections import namedtuple

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
PROVE = "mutation_rows.py prove"
FETCH = "cargo fetch --locked"
RESTORE = re.compile(r"uses:\s*actions/cache(/restore)?@")
MUTANTS = "cargo mutants"
BASELINE_SCRIPT = "scripts/mutation-baseline.sh"
IF = re.compile(r"(?m)^        if:\s*(.+?)\s*$")


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


def runs_a_baseline(step):
    """True when a step's code runs `cargo mutants` and not only to list (`--list`), or runs the
    leg's own unmutated baseline script (ADR-509), which comes before cargo-mutants skips its own."""
    return any(
        (MUTANTS in line and "--list" not in line) or BASELINE_SCRIPT in line
        for line in code_lines(step)
    )


def step_if(step):
    """The step's own `if:` expression, or None."""
    match = IF.search(step)
    return match.group(1) if match else None


Anchor = namedtuple("Anchor", "label found same_if")
PROVE_ANCHOR = Anchor("prove", lambda step: runs(step, PROVE), False)
BASELINE_ANCHOR = Anchor("cargo-mutants baseline", runs_a_baseline, True)


def proving_jobs():
    """[(file, job, its steps)] for every job that runs the rows runner's `prove`."""
    found = []
    for name, text in workflow_texts().items():
        for job, block in jobs(text).items():
            job_steps = steps(block)
            if any(runs(step, PROVE) for step in job_steps):
                found.append((name, job, job_steps))
    return examined("jobs that run mutation_rows.py prove", found)


def refusals(file, job, job_steps, anchor=PROVE_ANCHOR):
    """Why a job does not fetch the locked graph before its first anchor step; [] when it does.

    The anchor defaults to the rows runner's `prove` (SPEC-332); SPEC-416 passes the cargo-mutants
    baseline, whose `if:` the fetch step must also carry.
    """
    where = f"{file}:{job}"
    prove = next(i for i, step in enumerate(job_steps) if anchor.found(step))
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
        out.append(f"{where}: the fetch (step {fetches[0]}) is not before the first {anchor.label}")
    step = job_steps[fetches[0]]
    if "continue-on-error" in step:
        out.append(f"{where}: the fetch step carries continue-on-error")
    swallow = [line for line in code_lines(step) if "||" in line or "set +e" in line]
    if swallow:
        out.append(f"{where}: the fetch step swallows its exit code: {swallow[0]}")
    if anchor.same_if and step_if(step) != step_if(job_steps[prove]):
        out.append(f"{where}: the fetch step's `if:` is not the {anchor.label} step's `if:`")
    return out


class EveryRowsJobFetchesTheLockedGraph(unittest.TestCase):
    """SPEC-332 A1 and A2 (#606)."""

    def test_each_proving_job_fetches_after_its_cache_restore_and_before_prove(self):
        found = []
        proving = proving_jobs()
        for file, job, job_steps in proving:
            found.extend(
                line
                for line in refusals(file, job, job_steps)
                if "swallows" not in line and "continue-on-error" not in line
            )
        self.assertIn(("ci.yml", "mutation-rows"), [(file, job) for file, job, _ in proving])
        self.assertEqual([], found)

    def test_the_fetch_step_fails_its_job_when_the_fetch_fails(self):
        found = []
        proving = proving_jobs()
        self.assertIn(("mutation-weekly.yml", "rows"), [(file, job) for file, job, _ in proving])
        for file, job, job_steps in proving:
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
        middle = "      - run: cargo fetch --locked\n"
        self.assertEqual(1, middle.count(FETCH))
        self.assertEqual([], planted(middle))

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


def baseline_jobs(files=("ci.yml",)):
    """[(file, job, its steps)] for every job of the workflow `files` that runs a cargo-mutants
    baseline; `ci.yml` alone by default (SPEC-416), every workflow when SPEC-458 names them."""
    texts = workflow_texts()
    found = []
    for file in files:
        for job, block in jobs(texts[file]).items():
            job_steps = steps(block)
            if any(runs_a_baseline(step) for step in job_steps):
                found.append((file, job, job_steps))
    return found


class EveryMutantsJobFetchesTheLockedGraph(unittest.TestCase):
    """SPEC-416 A1: the mutant legs' baseline reads the census offline (#827)."""

    def test_each_baseline_job_fetches_after_its_cache_restore_and_before_cargo_mutants(self):
        baseline = baseline_jobs()
        self.assertIn("mutation-rust", [job for _, job, _ in baseline])
        found = []
        for file, job, job_steps in examined("jobs that run a cargo-mutants baseline", baseline):
            found.extend(refusals(file, job, job_steps, BASELINE_ANCHOR))
        self.assertEqual([], found)


BASELINE_RESTORE = "      - uses: actions/cache/restore@abc\n"
BASELINE_IF = "        if: ${{ needs.plan.outputs.rust == 'true' }}\n"
BASELINE_RUN = (
    "      - name: baseline\n" + BASELINE_IF + "        run: cargo mutants --in-place --in-diff d\n"
)
LEG_BASELINE_RUN = (
    "      - name: baseline\n"
    "        id: baseline\n"
    + BASELINE_IF
    + "        run: bash scripts/mutation-baseline.sh listed.json 0 1 baseline.log\n"
)
SKIP_IF = "        if: ${{ needs.plan.outputs.rust == 'true' && steps.baseline.outputs.green == 'true' }}\n"
SKIP_RUN = (
    "      - name: mutants\n"
    + SKIP_IF
    + "        run: cargo mutants --in-place --in-diff d --baseline=skip\n"
)


def fetch_step(run="cargo fetch --locked --manifest-path Cargo.toml", extra="", cond=BASELINE_IF):
    return "      - name: fetch\n" + cond + f"        run: {run}\n" + extra


def planted_baseline(*step_blocks):
    job = jobs("\njobs:\n  j:\n    steps:\n" + "".join(step_blocks))["j"]
    return refusals("planted.yml", "j", steps(job), BASELINE_ANCHOR)


class TheJudgeRefusesEachPlantedBaselineShape(unittest.TestCase):
    """SPEC-416 A2: positive controls on a cargo-mutants job."""

    def test_the_judge_refuses_each_planted_shape_on_a_baseline_job(self):
        where = "planted.yml:j"
        planted_shapes = [
            ("the fixed shape", (BASELINE_RESTORE, fetch_step(), BASELINE_RUN), []),
            (
                "no fetch",
                (BASELINE_RESTORE, BASELINE_RUN),
                [f"{where}: no step runs `{FETCH}`"],
            ),
            (
                "a fetch before the restore",
                (fetch_step(), BASELINE_RESTORE, BASELINE_RUN),
                [f"{where}: the fetch (step 0) is not after the last cache restore"],
            ),
            (
                "a fetch after cargo-mutants",
                (BASELINE_RESTORE, BASELINE_RUN, fetch_step()),
                [f"{where}: the fetch (step 2) is not before the first cargo-mutants baseline"],
            ),
            (
                "a swallowed fetch (|| true)",
                (BASELINE_RESTORE, fetch_step(run="cargo fetch --locked || true"), BASELINE_RUN),
                [
                    f"{where}: the fetch step swallows its exit code: "
                    "run: cargo fetch --locked || true"
                ],
            ),
            (
                "a fetch behind set +e",
                (
                    BASELINE_RESTORE,
                    "      - name: fetch\n"
                    + BASELINE_IF
                    + "        run: |\n          set +e\n          cargo fetch --locked\n",
                    BASELINE_RUN,
                ),
                [f"{where}: the fetch step swallows its exit code: set +e"],
            ),
            (
                "continue-on-error",
                (
                    BASELINE_RESTORE,
                    fetch_step(extra="        continue-on-error: true\n"),
                    BASELINE_RUN,
                ),
                [f"{where}: the fetch step carries continue-on-error"],
            ),
            (
                "an if that differs",
                (
                    BASELINE_RESTORE,
                    fetch_step(cond="        if: ${{ needs.plan.outputs.rows == 'true' }}\n"),
                    BASELINE_RUN,
                ),
                [f"{where}: the fetch step's `if:` is not the cargo-mutants baseline step's `if:`"],
            ),
            (
                "the ADR-509 shape: the leg's own baseline, then cargo-mutants skipping its own",
                (BASELINE_RESTORE, fetch_step(), LEG_BASELINE_RUN, SKIP_RUN),
                [],
            ),
            (
                "a fetch after the leg's own baseline",
                (BASELINE_RESTORE, LEG_BASELINE_RUN, fetch_step(), SKIP_RUN),
                [f"{where}: the fetch (step 2) is not before the first cargo-mutants baseline"],
            ),
        ]
        for name, blocks, expected in examined("planted baseline shapes", planted_shapes):
            with self.subTest(shape=name):
                self.assertEqual(expected, planted_baseline(*blocks))

    def test_a_listing_step_is_not_a_baseline(self):
        listing = "      - name: list\n        run: cargo mutants --no-shuffle --list --json\n"
        job = jobs("\njobs:\n  j:\n    steps:\n" + listing)["j"]
        self.assertEqual([], [s for s in steps(job) if runs_a_baseline(s)])

    def test_the_legs_own_baseline_script_is_a_baseline(self):
        job = jobs("\njobs:\n  j:\n    steps:\n" + LEG_BASELINE_RUN)["j"]
        self.assertEqual(1, len([s for s in steps(job) if runs_a_baseline(s)]))


class AllWorkflowsBaselineJobsFetchTheLockedGraph(unittest.TestCase):
    """SPEC-458 A1 (R1, R2): SPEC-416's rule over the mutant legs of every workflow (#827)."""

    def test_each_baseline_job_of_every_workflow_fetches_after_its_cache_restore_and_before_cargo_mutants(
        self,
    ):
        baseline = examined(
            "jobs of every workflow that run a cargo-mutants baseline",
            baseline_jobs(files=tuple(workflow_texts())),
        )
        held = [(file, job) for file, job, _ in baseline]
        for job in (
            ("ci.yml", "mutation-rust"),
            ("mutation-weekly.yml", "rust"),
            ("mutation-weekly.yml", "rehearsal"),
        ):
            self.assertIn(job, held)
        found = []
        for file, job, job_steps in baseline:
            found.extend(refusals(file, job, job_steps, BASELINE_ANCHOR))
        self.assertEqual([], found)


def saves_the_rust_cache(step):
    """True when a step saves the Rust cache's paths (SPEC-458 R3)."""
    return "actions/cache/save@" in step and "~/.cargo/registry/cache/" in step


SAVE_ANCHOR = Anchor("Rust cache save", saves_the_rust_cache, False)


def saving_jobs():
    """[(file, job, its steps)] for every job, in every workflow, holding a step that saves the
    Rust cache's paths."""
    found = []
    for name, text in workflow_texts().items():
        for job, block in jobs(text).items():
            job_steps = steps(block)
            if any(saves_the_rust_cache(step) for step in job_steps):
                found.append((name, job, job_steps))
    return found


class EveryRustCacheSaveFetchesTheLockedGraph(unittest.TestCase):
    """SPEC-458 A2 (R3, R4): every entry saved under the Rust key holds every target's packages."""

    def test_each_job_that_saves_the_rust_cache_fetches_after_its_restore_and_before_its_save(
        self,
    ):
        saving = examined("jobs that save the Rust cache", saving_jobs())
        held = [(file, job) for file, job, _ in saving]
        for job in (("ci.yml", "rust"), ("rust-cache.yml", "warm")):
            self.assertIn(job, held)
        found = []
        for file, job, job_steps in saving:
            found.extend(refusals(file, job, job_steps, SAVE_ANCHOR))
        self.assertEqual([], found)


SAVE_IF = "        if: ${{ steps.lookup.outputs.cache-hit != 'true' }}\n"
SAVE_RESTORE = (
    "      - name: restore by prefix\n" + SAVE_IF + "        uses: actions/cache/restore@abc\n"
)
SAVE_BUILD = (
    "      - name: build\n"
    + SAVE_IF
    + "        run: cargo clippy --workspace --all-targets --locked\n"
)
SAVE_STEP = (
    "      - name: save\n"
    + SAVE_IF
    + "        uses: actions/cache/save@abc\n"
    + "        with:\n          path: |\n            ~/.cargo/registry/cache/\n            target/\n"
)
SAVE_FETCH_RUN = "cargo fetch --locked --manifest-path Cargo.toml"


def planted_save(*step_blocks):
    job = jobs("\njobs:\n  j:\n    steps:\n" + "".join(step_blocks))["j"]
    return refusals("planted.yml", "j", steps(job), SAVE_ANCHOR)


class TheJudgeRefusesEachPlantedSaveShape(unittest.TestCase):
    """SPEC-458 A3 (R3, R5): positive controls on a job that saves the Rust cache."""

    def test_the_judge_refuses_each_planted_shape_on_a_saving_job(self):
        where = "planted.yml:j"
        fetch = fetch_step(run=SAVE_FETCH_RUN, cond=SAVE_IF)
        planted_shapes = [
            ("the fixed shape", (SAVE_RESTORE, fetch, SAVE_BUILD, SAVE_STEP), []),
            (
                "no fetch",
                (SAVE_RESTORE, SAVE_BUILD, SAVE_STEP),
                [f"{where}: no step runs `{FETCH}`"],
            ),
            (
                "a fetch before the restore",
                (fetch, SAVE_RESTORE, SAVE_BUILD, SAVE_STEP),
                [f"{where}: the fetch (step 0) is not after the last cache restore"],
            ),
            (
                "a fetch after the save",
                (SAVE_RESTORE, SAVE_BUILD, SAVE_STEP, fetch),
                [f"{where}: the fetch (step 3) is not before the first Rust cache save"],
            ),
            (
                "a swallowed fetch (|| true)",
                (
                    SAVE_RESTORE,
                    fetch_step(run=f"{SAVE_FETCH_RUN} || true", cond=SAVE_IF),
                    SAVE_BUILD,
                    SAVE_STEP,
                ),
                [f"{where}: the fetch step swallows its exit code: run: {SAVE_FETCH_RUN} || true"],
            ),
            (
                "continue-on-error",
                (
                    SAVE_RESTORE,
                    fetch_step(
                        run=SAVE_FETCH_RUN, extra="        continue-on-error: true\n", cond=SAVE_IF
                    ),
                    SAVE_BUILD,
                    SAVE_STEP,
                ),
                [f"{where}: the fetch step carries continue-on-error"],
            ),
        ]
        for name, blocks, expected in examined("planted save shapes", planted_shapes):
            with self.subTest(shape=name):
                self.assertEqual(expected, planted_save(*blocks))


if __name__ == "__main__":
    unittest.main()
