"""CI runs the whole gate on hosted runners with read-only tokens and pinned actions (SPEC-002 A9),
and only this repository's dev reaches main (SPEC-034 A5 to A7)."""

import os
import re
import subprocess
import textwrap
import unittest

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
USES = re.compile(r"^\s*-?\s*uses:\s*([^\s#]+)", re.M)
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")
STAGES = re.compile(r"^STAGES_ALL=\(([^)]*)\)", re.M)
THIS_REPOSITORY = "RexRenatus/deck-streak"
# A pull request into main, as the github context presents it; each test changes what it needs.
INTO_MAIN = {
    "github.event_name": "pull_request",
    "github.base_ref": "main",
    "github.head_ref": "dev",
    "github.event.pull_request.head.repo.full_name": THIS_REPOSITORY,
    "github.repository": THIS_REPOSITORY,
}


class WorkflowsAreHardened(unittest.TestCase):
    def setUp(self):
        self.files = examined("workflow files", sorted(WORKFLOWS.glob("*.yml")))

    def test_every_workflow_defaults_to_a_read_only_token(self):
        for path in self.files:
            self.assertRegex(path.read_text(), r"(?m)^permissions:\n  contents: read$", path.name)

    def test_every_action_is_pinned_by_a_full_commit_sha(self):
        uses = [(path.name, ref) for path in self.files for ref in USES.findall(path.read_text())]
        for name, ref in examined("action references", uses):
            self.assertRegex(ref, PINNED, f"{name} uses {ref}")

    def test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger(self):
        runners = []
        for path in self.files:
            code = re.sub(r"(?m)#.*$", "", path.read_text())
            self.assertNotIn("pull_request_target", code, path.name)
            runners += [(path.name, runner) for runner in re.findall(r"runs-on:\s*(.+)", code)]
        for name, runner in examined("runs-on values", runners):
            self.assertRegex(runner.strip(), r"^ubuntu-\d\d\.\d\d$", f"{name} runs on {runner}")

    def test_ci_runs_every_stage_of_the_local_gate(self):
        stages = STAGES.search((REPO / "scripts" / "check.sh").read_text()).group(1).split()
        ci = (WORKFLOWS / "ci.yml").read_text()
        for stage in examined("gate stages", stages):
            self.assertRegex(ci, rf"bash scripts/check\.sh [a-z ]*\b{stage}\b", stage)

    def test_the_aggregate_check_needs_every_job_and_always_runs(self):
        ci = (WORKFLOWS / "ci.yml").read_text()
        jobs = re.findall(r"(?m)^  ([a-z-]+):\n", ci.split("\njobs:\n", 1)[1])
        aggregate = re.search(r"(?ms)^  ci:\n(.*?)(?=^  [a-z-]+:\n|\Z)", ci).group(1)
        self.assertIn("if: ${{ always() }}", aggregate)
        for job in examined("jobs", [j for j in jobs if j != "ci"]):
            self.assertIn(job, aggregate, f"the aggregate ci job does not need {job}")



def run_base_is_dev(context):
    """Run base-is-dev's own step from ci.yml under `bash -e`, its env taken from `context`."""
    ci = (WORKFLOWS / "ci.yml").read_text(encoding="utf-8")
    job = re.search(r"(?ms)^  base-is-dev:\n(.*?)(?=^  [a-z-]+:\n|\Z)", ci).group(1)
    names = re.findall(r"(?m)^          ([A-Z_]+): \$\{\{ ([a-z_.]+) \}\}$", job)
    script = textwrap.dedent(re.search(r"(?ms)^        run: \|\n(.*?)(?=^\S|\Z)", job).group(1))
    env = {"PATH": os.environ["PATH"]}
    for name, expression in names:
        if expression not in context:
            raise AssertionError(f"base-is-dev reads {expression}, which no scenario models")
        env[name] = context[expression]
    return subprocess.run(
        ["bash", "-e", "-c", script], env=env, capture_output=True, text=True, check=False
    )


class OnlyThisRepositorysDevReachesMain(unittest.TestCase):
    def test_base_is_dev_refuses_a_fork_whose_branch_is_named_dev(self):
        fork = dict(INTO_MAIN, **{"github.event.pull_request.head.repo.full_name": "someone/fork"})
        done = run_base_is_dev(fork)
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("base-is-dev: a pull request into main must come from", done.stdout)

    def test_base_is_dev_admits_this_repositorys_dev_into_main(self):
        done = run_base_is_dev(INTO_MAIN)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("base-is-dev: ok (pull_request into 'main')", done.stdout)

    def test_base_is_dev_refuses_this_repositorys_other_branches_into_main(self):
        done = run_base_is_dev(dict(INTO_MAIN, **{"github.head_ref": "feature/probe"}))
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertIn("feature/probe", done.stdout)

if __name__ == "__main__":
    unittest.main()
