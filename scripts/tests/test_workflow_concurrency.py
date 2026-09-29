"""Every workflow with a pull-request trigger cancels the run a newer push supersedes (SPEC-190 A1 to
A4, issue #369).

The rule is `ci.yml`'s workflow-level block with the workflow's own name as the group's prefix: a
pull request's runs share its ref and `cancel-in-progress` is true for them; every other event gets
its run id as the group, so a push, a tag, a schedule or a dispatch run is never cancelled and never
replaced while it waits (ADR-055, ADR-190). The tests read every workflow file in the directory,
present and future, with the reader and the expression evaluator `test_ci_workflows.py` uses, and
render the group and the condition in scenarios. Nothing here runs GitHub.
"""

import re
import unittest

from _support import examined
from test_ci_workflows import (
    WORKFLOWS,
    concurrency_problems,
    condition,
    pull_request,
    push,
    read_workflow,
    rendered,
    workflow_files,
)

GROUP = "-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}"
CANCEL = "${{ github.event_name == 'pull_request' }}"


def read_all():
    """Every workflow file of the directory as (file name, its read content, its text)."""
    found = []
    for path in workflow_files(WORKFLOWS):
        text = path.read_text(encoding="utf-8")
        found.append((path.name, read_workflow(text), text))
    return found


def with_pull_request(found):
    """The workflows that run on a pull request."""
    return [item for item in found if "pull_request" in item[1].get("on", {})]


def other_events(run_id):
    """Runs that are not a pull request's: a push to dev and to main, a tag, a schedule and a
    dispatch. Each takes the given run id."""
    scenarios = {
        "a push to dev": push("refs/heads/dev", run_id=run_id),
        "a push to main": push("refs/heads/main", run_id=run_id),
        "a pushed tag": push("refs/tags/v1.0.0", run_id=run_id),
    }
    schedule = dict(push("refs/heads/main", run_id=run_id), **{"github.event_name": "schedule"})
    dispatch = dict(
        push("refs/heads/feature", run_id=run_id), **{"github.event_name": "workflow_dispatch"}
    )
    scenarios["a schedule"] = schedule
    scenarios["a dispatch"] = dispatch
    return scenarios


class EveryWorkflowFollowsTheRule(unittest.TestCase):
    def test_every_workflow_with_a_pull_request_trigger_follows_the_rule(self):
        judged = examined("workflows with a pull_request trigger", with_pull_request(read_all()))
        found = []
        for name, content, text in judged:
            groups = len(re.findall(r"(?m)^\s*concurrency:", text))
            if groups != 1:
                found.append(f"{name}: {groups} concurrency blocks, so a job's own or none")
            block = content.get("concurrency")
            if block is None:
                found.append(f"{name}: carries no concurrency block")
                continue
            if block.get("group") != content["name"] + GROUP:
                found.append(f"{name}: group is {block.get('group')!r}")
            if block.get("cancel-in-progress") != CANCEL:
                found.append(
                    f"{name}: cancel-in-progress is {block.get('cancel-in-progress')!r}, "
                    "so a superseded pull-request run finishes"
                )
            found += [f"{name}: {problem}" for problem in concurrency_problems(block)]
        self.assertIn("ci.yml", [item[0] for item in judged])
        self.assertEqual(found, [])

    def test_a_run_that_is_not_a_pull_requests_is_unique_and_never_cancelled(self):
        judged = examined("workflows with a pull_request trigger", with_pull_request(read_all()))
        first, second = other_events("201"), other_events("202")
        found = []
        for name, content, _text in judged:
            block = content.get("concurrency") or {}
            group = block.get("group", "")
            cancel = block.get("cancel-in-progress", "false")
            for scenario in first:
                if rendered(group, first[scenario]) == rendered(group, second[scenario]):
                    found.append(f"{name}: two runs of {scenario} share a group")
                if condition(cancel, first[scenario]):
                    found.append(f"{name}: {scenario} is cancelled")
        self.assertIn("ci.yml", [item[0] for item in judged])
        self.assertEqual(found, [])

    def test_the_release_workflow_never_cancels(self):
        found = [item for item in examined("workflows", read_all()) if item[0] == "release.yml"]
        ((_name, content, _text),) = examined("release workflows", found)
        self.assertNotIn("pull_request", content["on"])
        cancel = content["concurrency"]["cancel-in-progress"]
        self.assertEqual(cancel, "false")
        for scenario, context in other_events("201").items():
            self.assertFalse(condition(cancel, context), scenario)
        self.assertFalse(condition(cancel, pull_request("dev")))

    def test_the_blocks_that_already_followed_the_rule_are_unchanged(self):
        found = {name: content for name, content, _text in examined("workflows", read_all())}
        for name in ("ci.yml", "mutation-weekly.yml"):
            prefix = found[name]["name"]
            self.assertEqual(
                found[name]["concurrency"],
                {"group": prefix + GROUP, "cancel-in-progress": CANCEL},
                name,
            )
        self.assertEqual(found["ci.yml"]["name"], "ci")
        self.assertEqual(found["mutation-weekly.yml"]["name"], "mutation-weekly")


if __name__ == "__main__":
    unittest.main()
