"""SPEC-290: a mutation leg with nothing to examine is not started, decided from the plan's listing
and never from a leg's absence; the verdict and `ci` keep every refusal (ADR-290, #435)."""

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

from _support import REPO, examined
from test_ci_workflows import condition
from test_mutation_verdict import LIB, LIB_TEXT, Fixture, listing, run_shards, verdict_module
from test_mutation_workflows import CI, jobs, steps, workflow

VERDICT = REPO / "scripts" / "mutation-verdict.py"
#: Two pull request runs' plans and reports, as the verdict's job downloads them, reduced to what
#: the verdict reads: `listed/` lists mutants and holds its shard's whole report, `empty/` is a Rust
#: diff whose listing is empty and whose leg left nothing.
RECORDED = REPO / "scripts" / "tests" / "fixtures" / "not-started-legs"
PLAN = "needs.mutation-plan.outputs"
#: The two legs the plan's listing can leave with nothing to examine, and the only skips ci admits.
LEGS = ("mutation-rust", "mutation-rows")
MERGED = "Merge pull request #7 from RexRenatus/feat/probe"


def verdict(*args):
    """`mutation-verdict.py` with `args`, outside GitHub Actions."""
    env = dict(os.environ, PYTHONDONTWRITEBYTECODE="1")
    env.pop("GITHUB_OUTPUT", None)
    return subprocess.run(
        [sys.executable, str(VERDICT), *map(str, args)],
        capture_output=True,
        text=True,
        env=env,
        timeout=300,
        check=False,
    )


def downloaded(test, recorded, without=()):
    """A scratch copy of a recorded run's downloads, less the artifacts named in `without`."""
    scratch = tempfile.TemporaryDirectory()
    test.addCleanup(scratch.cleanup)
    reports = Path(scratch.name) / "reports"
    shutil.copytree(RECORDED / recorded, reports)
    for name in without:
        shutil.rmtree(reports / name)
    return reports


def judge(reports, rows=True):
    """The Rust class's judge line of the verdict's job over `reports`; the fixtures hold no
    equivalence record, so no whole-tree listing is read."""
    args = ["judge", "--plan", reports / "mutation-plan" / "plan.json", "--class", "rust"]
    args += ["--shard-reports", reports, "--root", RECORDED]
    if rows:
        args += ["--rows", reports / "mutation-rows" / "rows.json"]
    return verdict(*args)


def legs(plan, rust, rows):
    return verdict("legs", "--plan", plan, "--rust-leg", rust, "--rows-leg", rows)


def planned(reports):
    """(each shard's listed mutants, in shard order) of a downloaded plan."""
    plan = json.loads((reports / "mutation-plan" / "plan.json").read_text(encoding="utf-8"))
    return [shard["mutants"] for shard in plan["shards"]["shards"]]


def outcomes_of(reports, shard=0):
    return reports / f"mutation-rust-shard-{shard}" / "mutants.out" / "outcomes.json"


def rewrite(path, change):
    document = json.loads(path.read_text(encoding="utf-8"))
    change(document)
    path.write_text(json.dumps(document), encoding="utf-8")
    return document


def job_condition(job):
    """A job's job-level `if:` expressions, in order."""
    return re.findall(r"(?m)^    if: (.*)$", job)


def step_env(step):
    """{name: expression} of a step's `env:` entries that are one `${{ }}` each."""
    return dict(re.findall(r"(?m)^          ([A-Z_]+): \$\{\{ (.*?) \}\}$", step))


def step_script(step):
    """A step's `run:` script, a block or one line."""
    block = re.search(r"(?ms)^        run: \|\n(.*?)(?=^      \S|^        \S|\Z)", step)
    if block:
        return textwrap.dedent(block.group(1))
    line = re.search(r"(?m)^        run: (.+)$", step)
    if line is None:
        raise AssertionError("the step runs no script")
    return line.group(1)


def needs_of(job):
    found = re.search(r"(?m)^    needs: \[([^\]]*)\]$", job)
    if found is None:
        raise AssertionError("the job needs nothing")
    return [name.strip() for name in found.group(1).split(",")]


def rendered_env(step, needs, results):
    """The step's env as GitHub renders it from each need's result."""
    env = {}
    for name, expression in step_env(step).items():
        if expression == "join(needs.*.result, ' ')":
            env[name] = " ".join(results[need] for need in needs)
            continue
        one = re.fullmatch(r"needs\.([a-z-]+)\.result", expression)
        if one is None or one.group(1) not in results:
            raise AssertionError(f"the step reads {expression}, which no scenario models")
        env[name] = results[one.group(1)]
    return env


def bash(script, env):
    """A script as a GitHub-hosted runner's default shell runs it."""
    return subprocess.run(
        ["bash", "--noprofile", "--norc", "-eo", "pipefail", "-c", script],
        env=env,
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )


RECORDING_PYTHON = """#!/bin/bash
printf '%s\\n' "$*" >> "$SHIM_LOG"
case "$*" in
  *" judge "*"--class rust"*) exit "$RUST_RC" ;;
  *" judge "*"--class oracle"*) exit "$ORACLE_RC" ;;
  *" legs "*) exit "$LEGS_RC" ;;
esac
exit 99
"""


class ALegWithNothingToExamineIsNotStarted(unittest.TestCase):
    def test_the_plan_writes_how_many_rust_mutants_its_listing_holds(self):
        module = verdict_module()
        other = Fixture(self)
        other.head({"README.md": "a fixture, changed\n"})
        other.plan()
        constant = Fixture(self)
        constant.head(
            {
                LIB: LIB_TEXT.replace(
                    "pub const LAST_HOUR: u8 = 23;",
                    "pub const LAST_HOUR: u8 = 23; // the day's last hour",
                )
            }
        )
        self.assertTrue(constant.plan()["classes"]["rust"]["applies"])
        code = Fixture(self)
        code.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        code.plan()
        cases = [
            # No Rust change, and a listing beside it: the class does not apply, so none is read.
            ("no Rust change, a listing beside it", other, listing(["fix"]), []),
            # A changed constant: the class applies and cargo-mutants lists nothing.
            ("a Rust diff whose listing is empty", constant, [], []),
            ("a Rust diff that lists three mutants", code, listing(["fix"] * 3), ["fix"] * 3),
        ]
        for where, fixture, listed, packages in examined("plans", cases):
            done, plan, outputs = run_shards(fixture, listed)
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertEqual(outputs.get("listed"), str(len(packages)), where)
            count = module.fewest_shards(module.mutant_costs(packages))
            self.assertEqual(plan["shards"]["count"], count, where)
            self.assertEqual(outputs["shards"], str(count), where)
            self.assertEqual(outputs["matrix"], json.dumps(list(range(count))), where)
            held = sum(len(shard["mutants"]) for shard in plan["shards"]["shards"])
            self.assertEqual(held, len(packages), where)
        plan_job = jobs(workflow(CI)).get("mutation-plan", "")
        self.assertRegex(
            plan_job, r"(?m)^      listed: \$\{\{ steps\.shards\.outputs\.listed \}\}$"
        )

    def test_each_legs_job_condition_reads_the_plans_listing_alone(self):
        found = jobs(workflow(CI))
        conditions = {}
        for name in LEGS:
            written = job_condition(found.get(name, ""))
            self.assertEqual(len(written), 1, f"{name} has no job-level condition")
            conditions[name] = written[0]
            # The plan's outputs alone: no leg's result, no status function.
            read = set(re.findall(r"\bneeds\.[a-z-]+\.[a-z]+", written[0]))
            self.assertEqual(read, {PLAN}, name)
            self.assertNotRegex(written[0], r"always\(\)|failure\(\)|cancelled\(\)|\.result\b")
        self.assertNotIn(f"{PLAN}.rust", conditions["mutation-rust"])
        scenarios = [
            (listed, rust, rows, scope)
            for listed in ("0", "1", "12")
            for rust in ("true", "false")
            for rows in ("true", "false")
            for scope in ("diff", "not-applicable")
        ]
        for listed, rust, rows, scope in examined("plan outputs", scenarios):
            context = {
                f"{PLAN}.listed": listed,
                f"{PLAN}.rust": rust,
                f"{PLAN}.rows": rows,
                f"{PLAN}.scope": scope,
            }
            where = f"listed {listed}, rust {rust}, rows {rows}, scope {scope}"
            self.assertEqual(condition(conditions["mutation-rust"], context), listed != "0", where)
            self.assertEqual(
                condition(conditions["mutation-rows"], context),
                rows == "true" or scope == "diff",
                where,
            )
        # The matrix is the plan's, as before, and no other mutation job gains a condition.
        self.assertRegex(
            found["mutation-rust"],
            r"(?m)^        shard: \$\{\{ fromJSON\(needs\.mutation-plan\.outputs\.matrix\) \}\}$",
        )
        self.assertEqual(job_condition(found["mutation-verdict"]), ["${{ always() }}"])
        for name in ("mutation-plan", "mutation-web"):
            self.assertEqual(job_condition(found[name]), [], name)

    def test_a_leg_the_listing_gives_nothing_reads_not_started(self):
        reports = downloaded(self, "empty")
        self.assertEqual(examined("shards planned", planned(reports)), [[]])
        done = judge(reports)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn(
            "mutation: rust: mutation-rust-shard-0: not started: the plan lists no mutant for it",
            done.stdout,
        )
        by_rows = r"(?m)^mutation: rust: examined 0 by cargo-mutants and (\d+) by rows$"
        carried = re.search(by_rows, done.stdout)
        self.assertIsNotNone(carried, done.stdout)
        self.assertGreater(int(carried.group(1)), 0)
        self.assertRegex(done.stdout, rf"(?m)^examined {carried.group(1)}$")
        # With no row to carry it, the changed code examined nothing: still VOID.
        bare = judge(reports, rows=False)
        self.assertEqual(bare.returncode, 3, bare.stdout + bare.stderr)
        self.assertIn("VOID production code changed and nothing was examined", bare.stdout)
        # legs: the rust leg the listing gave nothing, not started, reads correct.
        plan = reports / "mutation-plan" / "plan.json"
        skipped = legs(plan, "skipped", "success")
        self.assertEqual(skipped.returncode, 0, skipped.stdout + skipped.stderr)
        self.assertIn(
            "mutation: legs: mutation-rust: not started: the plan lists no Rust mutant",
            skipped.stdout,
        )
        self.assertRegex(skipped.stdout, r"(?m)^examined 2$")
        # A push that merges a pull request: no row selected and no retirement check due.
        merged = Fixture(self)
        merged.head({LIB: LIB_TEXT.replace("x * 2", "x + x")})
        document = merged.plan("--event", "push", "--subject", MERGED)
        self.assertEqual(document["scope"]["decision"], "not-applicable")
        run_shards(merged, None)
        both = legs(merged.out / "plan.json", "skipped", "skipped")
        self.assertEqual(both.returncode, 0, both.stdout + both.stderr)
        for line in examined(
            "not-started legs",
            [
                "mutation: legs: mutation-rust: not started: the plan lists no Rust mutant",
                "mutation: legs: mutation-rows: not started: the plan selects no row and no "
                "retirement check is due",
            ],
        ):
            self.assertIn(line, both.stdout)

    def test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name(self):
        # The first plant: the recorded plan lists mutants, and its shard's leg left nothing.
        reports = downloaded(self, "listed", without=["mutation-rust-shard-0"])
        mutants = sum(map(len, examined("shards planned", planned(reports))))
        self.assertGreater(mutants, 0)
        missing = judge(reports)
        self.assertEqual(missing.returncode, 3, missing.stdout + missing.stderr)
        self.assertIn("VOID mutation-rust-shard-0: no report", missing.stdout)
        self.assertNotIn("not started", missing.stdout)
        plan = reports / "mutation-plan" / "plan.json"
        started = legs(plan, "success", "success")
        self.assertEqual(started.returncode, 0, started.stdout + started.stderr)
        self.assertRegex(started.stdout, r"(?m)^examined 2$")
        rust = legs(plan, "skipped", "success")
        self.assertEqual(rust.returncode, 3, rust.stdout + rust.stderr)
        self.assertIn(
            f"VOID mutation-rust: not started while the plan lists {mutants} mutant(s) across 1 "
            "shard(s)",
            rust.stdout,
        )
        selected = len(json.loads(plan.read_text(encoding="utf-8"))["rows"])
        self.assertGreater(selected, 0)
        rows = legs(plan, "success", "skipped")
        self.assertEqual(rows.returncode, 3, rows.stdout + rows.stderr)
        self.assertIn(
            f"VOID mutation-rows: not started while the plan selects {selected} row(s)",
            rows.stdout,
        )
        # A diff that selects no row still owes the retirement check.
        diff = Fixture(self)
        diff.head({"README.md": "a fixture, changed\n"})
        document = diff.plan()
        self.assertEqual((document["scope"]["decision"], document["rows"]), ("diff", []))
        # Before `shards` writes its listing into the plan, no listing says the leg had nothing.
        unsharded = legs(diff.out / "plan.json", "skipped", "success")
        self.assertEqual(unsharded.returncode, 3, unsharded.stdout + unsharded.stderr)
        self.assertIn(
            "VOID mutation-rust: not started while the plan names no shards", unsharded.stdout
        )
        run_shards(diff, None)
        retired = legs(diff.out / "plan.json", "skipped", "skipped")
        self.assertEqual(retired.returncode, 3, retired.stdout + retired.stderr)
        self.assertIn(
            "VOID mutation-rows: not started while the scope is diff, whose retirement check is "
            "due",
            retired.stdout,
        )
        self.assertIn("mutation-rust: not started: the plan lists no Rust mutant", retired.stdout)
        # A result that is not a job's, an unset one included, is VOID.
        for result in examined("results that are not a job's", ["", "neutral"]):
            unknown = legs(plan, result, "success")
            self.assertEqual(unknown.returncode, 3, unknown.stdout + unknown.stderr)
            self.assertIn(f"VOID mutation-rust: {result!r} is not a job's result", unknown.stdout)

    def test_an_examined_sum_that_differs_from_the_listing_is_refused(self):
        # The control: the recorded report reads whole and ok.
        whole = downloaded(self, "listed")
        mutants = sum(map(len, examined("shards planned", planned(whole))))
        recorded = json.loads(outcomes_of(whole).read_text(encoding="utf-8"))
        control = judge(whole)
        self.assertEqual(control.returncode, 0, control.stdout + control.stderr)
        tool = recorded["caught"] + recorded["missed"] + recorded["timeout"]
        self.assertRegex(control.stdout, rf"(?m)^mutation: rust: examined {tool} by cargo-mutants")
        self.assertIn("mutation: rust: verdict: ok", control.stdout)
        # The second plant: one caught mutant more, and one more in the total, so the report still
        # reads whole and every mutant it names is listed once; so too one fewer of each.
        for step in examined("counts moved off the listing", [1, -1]):
            planted = downloaded(self, "listed")

            def moved(document, step=step):
                document["caught"] += step
                document["total_mutants"] += step

            rewrite(outcomes_of(planted), moved)
            done = judge(planted)
            self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
            self.assertIn(
                f"VOID the shards' reports count {mutants + step} mutant(s), and the plan's "
                f"listing holds {mutants}",
                done.stdout,
            )
        # A report in a shard the listing gives no mutant is counted against the listing, and one
        # with no recorded exit is partial, never a leg that was not started.
        empty = downloaded(self, "empty")
        shard = empty / "mutation-rust-shard-0"
        (shard / "mutants.out").mkdir(parents=True)
        report = {
            "outcomes": [{"scenario": "Baseline", "summary": "Success"}],
            "total_mutants": 1,
            "caught": 1,
            "missed": 0,
            "timeout": 0,
            "unviable": 0,
            "success": 0,
        }
        outcomes_of(empty).write_text(json.dumps(report), encoding="utf-8")
        exitless = judge(empty)
        self.assertEqual(exitless.returncode, 3, exitless.stdout + exitless.stderr)
        self.assertIn("VOID mutation-rust-shard-0: no cargo-mutants exit recorded", exitless.stdout)
        self.assertNotIn("not started", exitless.stdout)
        (shard / "cargo-mutants.exit").write_text("0\n", encoding="utf-8")
        counted = judge(empty)
        self.assertEqual(counted.returncode, 3, counted.stdout + counted.stderr)
        self.assertIn(
            "VOID the shards' reports count 1 mutant(s), and the plan's listing holds 0",
            counted.stdout,
        )

    def test_ci_admits_a_not_started_leg_and_no_other_skip(self):
        aggregate = jobs(workflow(CI)).get("ci", "")
        self.assertEqual(job_condition(aggregate), ["${{ always() }}"])
        needs = needs_of(aggregate)
        self.assertIn("mutation-verdict", needs)
        for leg in LEGS:
            self.assertIn(leg, needs)
        (step,) = steps(aggregate)
        script = step_script(step)
        scenarios = [
            ("every need succeeded", {}, 0),
            ("mutation-rust not started", {"mutation-rust": "skipped"}, 0),
            ("mutation-rows not started", {"mutation-rows": "skipped"}, 0),
            ("both legs not started", {"mutation-rust": "skipped", "mutation-rows": "skipped"}, 0),
            ("the verdict skipped", {"mutation-rust": "skipped", "mutation-verdict": "skipped"}, 1),
            ("another job skipped", {"mutation-rust": "skipped", "mutation-web": "skipped"}, 1),
            ("a gate job skipped", {"mutation-rows": "skipped", "hygiene": "skipped"}, 1),
            ("mutation-web skipped alone", {"mutation-web": "skipped"}, 1),
            ("a cancelled leg", {"mutation-rust": "cancelled"}, 1),
            ("a failed leg", {"mutation-rows": "failure"}, 1),
            ("a failed verdict", {"mutation-rust": "skipped", "mutation-verdict": "failure"}, 1),
        ]
        for where, changed, expected in examined("need results", scenarios):
            results = dict(dict.fromkeys(needs, "success"), **changed)
            env = dict(rendered_env(step, needs, results), PATH=os.environ["PATH"])
            done = bash(script, env)
            self.assertEqual(done.returncode, expected, f"{where}: {done.stdout}{done.stderr}")
            for leg in LEGS:
                if expected == 0 and results[leg] == "skipped":
                    self.assertIn(f"ci: {leg} was not started", done.stdout, where)

    def test_the_verdict_step_fails_on_the_legs_check(self):
        job = jobs(workflow(CI)).get("mutation-verdict", "")
        (step,) = [s for s in steps(job) if "mutation-verdict.py judge" in s]
        script = step_script(step)
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        root = Path(scratch.name)
        (root / "bin").mkdir()
        shim = root / "bin" / "python3"
        shim.write_text(RECORDING_PYTHON, encoding="utf-8")
        shim.chmod(0o700)
        results = {
            "mutation-plan": "success",
            "mutation-rust": "skipped",
            "mutation-rows": "success",
        }
        scenarios = [
            ("legs refuses, the judges passed", (0, 0, 3), 3),
            ("every check passed", (0, 0, 0), 0),
            ("the Rust judge failed first", (1, 0, 3), 1),
            ("the oracle's judge failed first", (0, 3, 1), 3),
        ]
        for where, (rust, oracle, legs_rc), expected in examined("verdict statuses", scenarios):
            log = root / f"calls-{rust}{oracle}{legs_rc}.log"
            env = dict(
                rendered_env(step, needs_of(job), results),
                PATH=f"{root / 'bin'}{os.pathsep}{os.environ['PATH']}",
                RUNNER_TEMP=str(root),
                SHIM_LOG=str(log),
                RUST_RC=str(rust),
                ORACLE_RC=str(oracle),
                LEGS_RC=str(legs_rc),
            )
            done = bash(script, env)
            self.assertEqual(done.returncode, expected, f"{where}: {done.stdout}{done.stderr}")
            calls = log.read_text(encoding="utf-8").splitlines()
            checked = [call for call in calls if " legs " in f" {call} "]
            self.assertEqual(len(checked), 1, f"{where}: the step never runs legs: {calls}")
            self.assertIn(f"--plan {root}/reports/mutation-plan/plan.json", checked[0])
            self.assertIn("--rust-leg skipped --rows-leg success", checked[0])


if __name__ == "__main__":
    unittest.main()
