"""The scheduled job that saves the Rust cache in the default branch's scope (SPEC-191 A1 to A9,
issue #370).

A run restores a cache only from its own ref, its pull request's base branch and the default branch,
and a scheduled run executes the default branch's copy of a workflow, so what it saves is
restorable by a run on any ref. These tests read `rust-cache.yml` the way `test_ci_workflows.py`
reads the other workflows and compare it, character for character, with the cache steps of
`ci.yml` and `mutation-weekly.yml`. Nothing here runs GitHub: the live proof is two dispatched runs
quoted in the pull request.
"""

import re
import tempfile
import unittest
from pathlib import Path

from _support import examined
from test_ci_workflows import (
    RUST_CACHE,
    WORKFLOWS,
    action,
    cache_problems,
    lines_of,
    paths,
    read_workflow,
)

CACHE_WORKFLOW = "rust-cache.yml"
HIT = "steps.lookup.outputs.cache-hit"
STOP_ON_A_HIT = "${{ " + HIT + " != 'true' }}"
KEY_TAIL = "${{ hashFiles('Cargo.lock') }}"
COMPILES = [
    "cargo clippy --workspace --all-targets --locked",
    "cargo nextest run --workspace --locked --no-run",
]
TOOLS = "cargo-nextest,cargo-deny"


def workflow_text(name):
    """A workflow's text; its absence is the criterion failing, never a crash."""
    path = WORKFLOWS / name
    if not path.is_file():
        raise AssertionError(f".github/workflows/{name} does not exist")
    return path.read_text(encoding="utf-8")


def load(name):
    return read_workflow(workflow_text(name))


def steps_of(workflow):
    (job,) = examined("jobs", list((workflow.get("jobs") or {}).values()))
    return examined("steps", job.get("steps") or [])


def rust_restores(name):
    """Every step of a workflow that restores the Rust cache's paths."""
    found = []
    for job in (load(name).get("jobs") or {}).values():
        found += [
            step
            for step in job.get("steps") or []
            if action(step) == "actions/cache/restore" and paths(step) == RUST_CACHE
        ]
    return found


def lookup_step(steps):
    found = [s for s in steps if action(s) == "actions/cache/restore" and s.get("id") == "lookup"]
    if len(found) != 1:
        raise AssertionError(f"{len(found)} lookup steps: the job must have exactly one")
    return found[0]


def cron_minutes(name):
    """The minute field of each cron a workflow schedules, or [] for a workflow with none."""
    on = load(name).get("on") or {}
    return [str(item["cron"]).split()[0] for item in (on.get("schedule") or [])]


def other_cron_minutes(directory):
    """(workflow name, minute) for each cron minute a workflow of the directory schedules, the
    cache workflow itself left out."""
    found = []
    for path in sorted(directory.glob("*.yml")):
        if path.name != CACHE_WORKFLOW:
            on = read_workflow(path.read_text(encoding="utf-8")).get("on") or {}
            found += [
                (path.name, str(item["cron"]).split()[0]) for item in on.get("schedule") or []
            ]
    return found


SCHEDULED_SAVE = """\
on:
  schedule:
    - cron: "43 */2 * * *"
  workflow_dispatch:
jobs:
  warm:
    runs-on: ubuntu-24.04
    steps:
      - id: lookup
        uses: actions/cache/restore@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
          lookup-only: true
      - id: warm
        if: ${{ steps.lookup.outputs.cache-hit != 'true' }}
        uses: actions/cache/restore@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
      - name: save
        if: ${{ steps.lookup.outputs.cache-hit != 'true' }}
        uses: actions/cache/save@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: ${{ steps.warm.outputs.cache-primary-key }}
"""


class TheRustCacheWorkflow(unittest.TestCase):
    def test_it_runs_on_a_two_hourly_schedule_and_on_dispatch_only(self):
        on = load(CACHE_WORKFLOW).get("on")
        self.assertEqual(sorted(on), ["schedule", "workflow_dispatch"], "the events it runs on")
        crons = examined("crons", [item["cron"] for item in on["schedule"]])
        self.assertEqual(len(crons), 1)
        minute, hour, rest = crons[0].split(None, 2)
        self.assertRegex(minute, r"^\d+$", "a fixed minute")
        self.assertEqual(hour, "*/2", "every two hours")
        self.assertEqual(rest, "* * *")

    def test_it_checks_out_dev_without_persisting_credentials(self):
        first = steps_of(load(CACHE_WORKFLOW))[0]
        self.assertEqual(action(first), "actions/checkout")
        self.assertEqual(first["with"].get("ref"), "dev")
        self.assertEqual(first["with"].get("persist-credentials"), "false")
        checkouts = [s for s in steps_of(load(CACHE_WORKFLOW)) if action(s) == "actions/checkout"]
        self.assertEqual(len(checkouts), 1, "one checkout: the key and the tree agree")

    def test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore(self):
        mine = lookup_step(steps_of(load(CACHE_WORKFLOW)))
        self.assertEqual(paths(mine), RUST_CACHE)
        self.assertTrue(mine["with"]["key"].endswith(KEY_TAIL), "the key ends in the lockfile hash")
        compared = []
        for name in ("ci.yml", "mutation-weekly.yml"):
            for step in rust_restores(name):
                compared.append((name, step["with"]["key"]))
        for name, key in examined("restore keys of ci.yml and mutation-weekly.yml", compared):
            self.assertEqual(mine["with"]["key"], key, f"{name} restores another key")
        self.assertEqual({name for name, _ in compared}, {"ci.yml", "mutation-weekly.yml"})

    def test_it_looks_the_exact_key_up_and_every_later_step_stops_on_a_hit(self):
        steps = steps_of(load(CACHE_WORKFLOW))
        at = steps.index(lookup_step(steps))
        lookup = steps[at]
        self.assertEqual(lookup["with"].get("lookup-only"), "true")
        self.assertNotIn("restore-keys", lookup["with"], "an exact lookup has no prefix fallback")
        self.assertLess(0, at, "the checkout comes first")
        later = examined("steps after the lookup", steps[at + 1 :])
        for step in later:
            name = step.get("name") or step.get("id") or step.get("uses")
            self.assertEqual(step.get("if"), STOP_ON_A_HIT, f"{name} does not stop on a hit")

    def test_it_installs_restores_by_prefix_and_compiles_without_running_a_test(self):
        ci = load("ci.yml")["jobs"]["rust"]["steps"]
        theirs = {
            "rustup": next(s for s in ci if str(s.get("run", "")).strip() == "rustup show"),
            "tools": next(s for s in ci if action(s) == "taiki-e/install-action"),
            "protoc": next(s for s in ci if "protoc" in str(s.get("name", ""))),
        }
        steps = steps_of(load(CACHE_WORKFLOW))
        mine_rustup = [s for s in steps if str(s.get("run", "")).strip() == "rustup show"]
        mine_tools = [s for s in steps if action(s) == "taiki-e/install-action"]
        mine_protoc = [s for s in steps if "protoc" in str(s.get("name", ""))]
        for found in (mine_rustup, mine_tools, mine_protoc):
            self.assertEqual(len(found), 1)
        self.assertEqual(mine_tools[0]["uses"], theirs["tools"]["uses"])
        self.assertEqual(mine_tools[0]["with"]["tool"], TOOLS)
        self.assertEqual(mine_tools[0]["with"], theirs["tools"]["with"])
        self.assertEqual(mine_protoc[0]["env"], theirs["protoc"]["env"])
        self.assertEqual(mine_protoc[0]["run"], theirs["protoc"]["run"])
        restore = next(s for s in steps if s.get("id") == "warm")
        self.assertEqual(action(restore), "actions/cache/restore")
        self.assertEqual(paths(restore), RUST_CACHE)
        key = restore["with"]["key"]
        self.assertEqual(key, lookup_step(steps)["with"]["key"])
        self.assertEqual(lines_of(restore["with"]["restore-keys"]), [key.split(KEY_TAIL)[0]])
        runs = [str(s.get("run", "")).strip() for s in steps]
        first_build = min(at for at, run in enumerate(runs) if run in COMPILES)
        for command in examined("compile commands", COMPILES):
            self.assertIn(command, runs, f"the job never runs {command}")
        for at in (
            runs.index("rustup show"),
            steps.index(mine_tools[0]),
            steps.index(mine_protoc[0]),
        ):
            self.assertLess(at, first_build, "an install comes after a build")
        self.assertLess(steps.index(restore), first_build, "the restore comes after a build")
        self.assertFalse(
            [
                r
                for r in runs
                if re.search(r"cargo (nextest run|test)\b", r) and "--no-run" not in r
            ],
            "a step runs a test",
        )
        self.assertFalse([r for r in runs if "check.sh" in r], "the job runs the gate")
        self.assertEqual(load(CACHE_WORKFLOW)["jobs"]["warm"]["env"]["CARGO_INCREMENTAL"], "0")

    def test_it_cleans_the_workspace_before_it_saves_under_the_exact_key(self):
        steps = steps_of(load(CACHE_WORKFLOW))
        saves = [s for s in steps if action(s) == "actions/cache/save"]
        self.assertEqual(len(saves), 1)
        save = saves[0]
        at = steps.index(save)
        self.assertEqual(paths(save), RUST_CACHE)
        clean = steps[at - 1]
        self.assertEqual(str(clean.get("run", "")).strip(), "cargo clean --workspace")
        self.assertEqual(clean.get("if"), save.get("if"))
        runs = [str(s.get("run", "")).strip() for s in steps]
        for command in examined("builds", COMPILES):
            self.assertLess(runs.index(command), at - 1, f"{command} runs after the clean")
        restore = next(s for s in steps if s.get("id") == "warm")
        self.assertEqual(save["with"]["key"], "${{ steps.warm.outputs.cache-primary-key }}")
        self.assertEqual(restore["with"]["key"], lookup_step(steps)["with"]["key"])

    def test_the_save_rule_admits_a_scheduled_save_only_when_a_lookup_missed(self):
        admitted, found = cache_problems(CACHE_WORKFLOW, read_workflow(SCHEDULED_SAVE))
        self.assertEqual((admitted, len(found)), ([], 1), "the planted good shape is refused")
        unconditional = SCHEDULED_SAVE.replace(
            "      - name: save\n        if: ${{ steps.lookup.outputs.cache-hit != 'true' }}\n",
            "      - name: save\n",
        )
        refused, _ = cache_problems(CACHE_WORKFLOW, read_workflow(unconditional))
        self.assertTrue(refused, "a scheduled save with no condition is admitted")
        pushed = SCHEDULED_SAVE.replace("  workflow_dispatch:\n", "  workflow_dispatch:\n  push:\n")
        refused, _ = cache_problems(CACHE_WORKFLOW, read_workflow(pushed))
        self.assertTrue(refused, "a scheduled save that also runs on push is admitted")
        pulled = SCHEDULED_SAVE.replace(
            "  workflow_dispatch:\n", "  workflow_dispatch:\n  pull_request:\n"
        )
        refused, _ = cache_problems(CACHE_WORKFLOW, read_workflow(pulled))
        self.assertTrue(refused, "a scheduled save that also runs on pull_request is admitted")
        wrong_key = SCHEDULED_SAVE.replace("steps.warm.outputs.cache-primary-key", "runner.os")
        refused, _ = cache_problems(CACHE_WORKFLOW, read_workflow(wrong_key))
        self.assertTrue(refused, "a scheduled save under another key is admitted")
        refused, _ = cache_problems("planted.yml", read_workflow(SCHEDULED_SAVE))
        self.assertTrue(refused, "another workflow's scheduled save is admitted")
        widened = SCHEDULED_SAVE.replace(
            "      - name: save\n        if: ${{ steps.lookup.outputs.cache-hit != 'true' }}\n",
            "      - name: save\n        if: ${{ steps.lookup.outputs.cache-hit != 'true'"
            " || github.event_name == 'workflow_dispatch' }}\n",
        )
        self.assertIn("workflow_dispatch' }}", widened)
        refused, _ = cache_problems(CACHE_WORKFLOW, read_workflow(widened))
        self.assertTrue(refused, "a scheduled save that also saves on a hit is admitted")
        problems, saves = cache_problems(CACHE_WORKFLOW, load(CACHE_WORKFLOW))
        self.assertEqual(problems, [])
        self.assertEqual(len(saves), 1)

    def test_its_cron_minute_collides_with_no_other_workflows(self):
        mine = cron_minutes(CACHE_WORKFLOW)
        self.assertEqual(len(mine), 1)
        others = examined("other workflows' cron minutes", other_cron_minutes(WORKFLOWS))
        for name, minute in others:
            self.assertNotEqual(mine[0], minute, f"{name} schedules minute {minute} too")

    def test_the_cron_scan_reads_a_workflow_saved_with_the_yaml_suffix(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yaml").write_text(SCHEDULED_SAVE, encoding="utf-8")
            others = other_cron_minutes(Path(scratch))
        self.assertEqual(others, [("planted.yaml", "43")])

    def test_it_reads_only_and_queues_instead_of_cancelling(self):
        workflow = load(CACHE_WORKFLOW)
        self.assertEqual(workflow.get("permissions"), {"contents": "read"})
        self.assertEqual(
            workflow.get("concurrency"),
            {"group": "rust-cache", "cancel-in-progress": "false"},
        )
        self.assertNotIn("permissions", workflow["jobs"]["warm"], "a job widens the token")


if __name__ == "__main__":
    unittest.main()
