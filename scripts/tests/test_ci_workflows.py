"""CI runs the whole gate on hosted runners with read-only tokens and pinned actions (SPEC-002 A9)."""

import re
import unittest

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
USES = re.compile(r"^\s*-?\s*uses:\s*([^\s#]+)", re.M)
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")
STAGES = re.compile(r"^STAGES_ALL=\(([^)]*)\)", re.M)


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


if __name__ == "__main__":
    unittest.main()
