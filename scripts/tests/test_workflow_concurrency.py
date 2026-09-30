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
    Unread,
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

    def test_no_two_workflows_share_a_pull_requests_group(self):
        judged = examined("workflows with a pull_request trigger", with_pull_request(read_all()))
        owners = {}
        for name, content, _text in judged:
            group = (content.get("concurrency") or {}).get("group", "")
            if group:
                # GitHub reads a group's name without case, across every workflow of the repository.
                key = rendered(group, pull_request("dev")).casefold()
                owners.setdefault(key, []).append(name)
        self.assertIn("ci.yml", [name for names in owners.values() for name in names])
        self.assertEqual([names for names in owners.values() if len(names) > 1], [])

    def test_no_workflow_cancels_a_run_that_is_not_a_pull_requests(self):
        judged = examined("workflows", read_all())
        found = []
        for name, content, text in judged:
            if re.search(r"(?m)^[ \t]+concurrency:", text):
                found.append(f"{name}: a job sets a concurrency block of its own")
            cancel = (content.get("concurrency") or {}).get("cancel-in-progress", "false")
            for scenario, context in other_events("201").items():
                if condition(cancel, context):
                    found.append(f"{name}: {scenario} is cancelled")
        self.assertIn("release.yml", [item[0] for item in judged])
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


def releases_a_tag(content):
    """Whether a workflow runs on a pushed tag or on a release event: the workflows whose group can
    hold two runs of one release."""
    events = content.get("on", {})
    pushed = events.get("push")
    return bool((isinstance(pushed, dict) and "tags" in pushed) or "release" in events)


def release_problems(content, text):
    """What a release workflow's concurrency gets wrong (SPEC-190 R10, ADR-292): its group must hold
    two runs of one tag so they never run at once, no run may be cancelled, and the group must
    queue every requested run, not keep one pending run and replace it (`queue: max`)."""
    found = []
    blocks = len(re.findall(r"(?m)^\s*concurrency:", text))
    if blocks != 1:
        found.append(f"{blocks} concurrency blocks, so a job's own or none")
    block = content.get("concurrency")
    if block is None:
        return found + ["carries no concurrency block"]
    group, cancel = block.get("group", ""), block.get("cancel-in-progress", "false")
    first, second = push("refs/tags/v1.0.0", run_id="201"), push("refs/tags/v1.0.0", run_id="202")
    if rendered(group, first) != rendered(group, second):
        found.append("two runs of one tag have two groups, so their release steps run at once")
    for scenario, context in (("a first run", first), ("a second run", second)):
        if condition(cancel, context):
            found.append(f"{scenario} of a tag cancels the run before it")
    if block.get("queue") != "max":
        found.append(
            f"queue is {block.get('queue')!r}, so a third run of a tag replaces the waiting second"
        )
    return found


def planted(old, new):
    """release.yml's text with one shape planted in place of the ADR-292 shape."""
    text = (WORKFLOWS / "release.yml").read_text(encoding="utf-8")
    assert old in text, old
    return text.replace(old, new, 1)


class EveryReleaseWorkflowQueuesEveryRun(unittest.TestCase):
    def test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them(self):
        judged = examined(
            "release workflows (push of tags or release event)",
            [item for item in read_all() if releases_a_tag(item[1])],
        )
        self.assertIn("release.yml", [item[0] for item in judged])
        found = []
        for name, content, text in judged:
            found += [f"{name}: {problem}" for problem in release_problems(content, text)]
        self.assertEqual(found, [])

    def test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused(self):
        group = "  group: release-${{ github.ref }}\n"
        shapes = {
            "the tag group with no queue (today)": (
                "  cancel-in-progress: false\n",
                "  cancel-in-progress: false\n",
                "queue is None",
            ),
            "queue: single": (
                "  cancel-in-progress: false\n",
                "  cancel-in-progress: false\n  queue: single\n",
                "queue is 'single'",
            ),
            "a group keyed by the run id": (
                group,
                "  group: release-${{ github.run_id }}\n  queue: max\n",
                "two runs of one tag have two groups",
            ),
            "cancel-in-progress true": (
                "  cancel-in-progress: false\n",
                "  cancel-in-progress: true\n  queue: max\n",
                "cancels the run before it",
            ),
        }
        for label, (old, new, expected) in shapes.items():
            text = planted(old, new)
            problems = release_problems(read_workflow(text), text)
            self.assertTrue(any(expected in problem for problem in problems), (label, problems))
        text = planted(
            "  cancel-in-progress: false\n", "  cancel-in-progress: false\n  queue: max\n"
        )
        self.assertEqual(release_problems(read_workflow(text), text), [])

    def test_a_job_level_block_and_a_missing_block_are_refused(self):
        text = planted(
            "jobs:\n  release:\n", "jobs:\n  release:\n    concurrency:\n      group: x\n"
        )
        self.assertIn("2 concurrency blocks", " ".join(release_problems(read_workflow(text), text)))
        bare = planted("concurrency:\n", "x-unused:\n")
        self.assertIn("no concurrency block", " ".join(release_problems(read_workflow(bare), bare)))

    def test_a_release_workflow_the_reader_cannot_read_is_refused(self):
        text = planted("jobs:\n", "jobs:\n  anchor: &a b\n")
        with self.assertRaises(Unread):
            read_workflow(text)

    def test_only_the_tag_or_release_workflows_are_in_the_class(self):
        names = [
            n for n, content, _t in examined("workflows", read_all()) if releases_a_tag(content)
        ]
        self.assertEqual(names, ["release.yml"])


if __name__ == "__main__":
    unittest.main()
