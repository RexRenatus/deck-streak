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


# The events whose run can hold a tag's ref, as GitHub's events page gives GITHUB_REF for each.
TAG_REF_EVENTS = (
    "push",
    "create",
    "release",
    "workflow_dispatch",
    "registry_package",
    "deployment",
    "deployment_status",
)
# The keys GitHub's workflow parser defines for a concurrency block. It reads a key with case, and
# refuses the whole workflow for a key it does not define.
BLOCK_KEYS = ("group", "cancel-in-progress", "queue")


def tag_events(content):
    """The events that start a run of a workflow for a tag, as GitHub's workflow syntax reads its
    filters: a push whose filters admit a tag (a `tags` or `tags-ignore` filter, or neither a
    branch nor a tag filter) and a release. `on:` is read as a name, a list or a mapping."""
    events = content.get("on")
    if isinstance(events, str):
        events = {events: None}
    elif isinstance(events, list):
        events = dict.fromkeys(events)
    elif not isinstance(events, dict):
        raise AssertionError(f"an `on:` the reader does not model: {events!r}")
    found = []
    if "push" in events:
        filters = set(events["push"] or {})
        if filters & {"tags", "tags-ignore"} or not filters & {"branches", "branches-ignore"}:
            found.append("push")
    if "release" in events:
        found.append("release")
    return found


def releases_a_tag(content):
    """Whether a workflow runs for a tag: the workflows whose group can hold two runs of one
    release."""
    return bool(tag_events(content))


def tag_run(event, run_id):
    """A run of `event` for the tag v1.0.0 with the given run id."""
    return dict(push("refs/tags/v1.0.0", run_id=run_id), **{"github.event_name": event})


def rendered_groups(name, content):
    """Every group another workflow's block renders in the scenarios the tests model, read without
    case, as GitHub reads a group's name across the repository."""
    block = content.get("concurrency")
    group = block.get("group", "") if isinstance(block, dict) else block or ""
    scenarios = [*other_events("201").values(), pull_request("dev")]
    scenarios += [tag_run(event, "201") for event in ("push", "release")]
    try:
        return {rendered(group, context).casefold() for context in scenarios}
    except AssertionError as why:
        raise AssertionError(f"{name}'s group cannot be rendered: {why}") from why


def release_problems(content, text, others=()):
    """What a release workflow's concurrency gets wrong (SPEC-190 R10, ADR-292): one workflow-level
    block and no job's own, a group that is not empty, is the same for two runs of one tag under
    every event that runs it for a tag and is no other workflow's (`others`, as (name, content)),
    no run cancelled, and a group that queues every requested run, not one kept and replaced."""
    found = []
    blocks = len(re.findall(r"(?m)^\s*concurrency:", text))
    if blocks != 1:
        found.append(f"{blocks} concurrency blocks, so a job's own or none")
    for job_id, job in (content.get("jobs") or {}).items():
        if isinstance(job, dict) and "concurrency" in job:
            found.append(f"job {job_id} sets a concurrency block of its own")
    block = content.get("concurrency")
    if block is None:
        return found + ["carries no concurrency block"]
    if not isinstance(block, dict):
        return found + [f"its block is the group {block!r} alone, so queue is the default"]
    group, cancel = block.get("group", ""), block.get("cancel-in-progress", "false")
    if not str(group).strip():
        found.append("has no group, which GitHub's workflow parser requires")
    for event in tag_events(content):
        first, second = tag_run(event, "201"), tag_run(event, "202")
        if rendered(group, first) != rendered(group, second):
            found.append(
                f"two runs of one tag have two groups ({event}), so their release steps run at once"
            )
        for scenario, context in (("a first run", first), ("a second run", second)):
            if condition(cancel, context):
                found.append(f"{scenario} of a tag cancels the run before it ({event})")
        mine = rendered(group, first).casefold()
        for other, other_content in others:
            if mine in rendered_groups(other, other_content):
                found.append(f"its group renders as {other}'s does, so {other} replaces its runs")
    if block.get("queue") != "max":
        found.append(
            f"queue is {block.get('queue')!r}, so a third run of a tag replaces the waiting second"
        )
    return found


def second_workflow(on, group="publish-${{ github.ref }}", name="publish", queue=""):
    """A planted workflow, with the default queue unless `queue` sets one: `on` is its triggers."""
    return (
        f"name: {name}\n\n{on}\npermissions:\n  contents: read\n\nconcurrency:\n"
        f"  group: {group}\n  cancel-in-progress: false\n{queue}\njobs:\n  publish:\n"
        "    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n    steps:\n      - run: echo publish\n"
    )


def planted(old, new):
    """release.yml's text with one shape planted in place of the ADR-292 shape."""
    text = (WORKFLOWS / "release.yml").read_text(encoding="utf-8")
    assert old in text, old
    return text.replace(old, new, 1)


class EveryReleaseWorkflowQueuesEveryRun(unittest.TestCase):
    def test_every_workflow_that_can_hold_two_runs_of_a_release_queues_them(self):
        judged = examined(
            "release workflows (a push that admits a tag, or a release)",
            [item for item in read_all() if releases_a_tag(item[1])],
        )
        self.assertIn("release.yml", [item[0] for item in judged])
        everything = read_all()
        found = []
        for name, content, text in judged:
            others = [(other, c) for other, c, _t in everything if other != name]
            found += [f"{name}: {problem}" for problem in release_problems(content, text, others)]
        self.assertEqual(found, [])

    def test_the_release_class_is_read_as_github_reads_it(self):
        """The class as GitHub reads it, by a population planted on a workflow of its own, never on
        the live file: every event the docs run for a tag, in each form `on:` takes; every event
        whose ref can be a tag, beside a group spelt through the workflow's name or its event;
        every block key in another case or misspelt; and every key held twice, without case."""
        tags = "on:\n  push:\n    tags: [v1]\n"
        release = second_workflow(tags, "release-${{ github.ref }}", "release", "  queue: max\n")
        self.assertEqual(release_problems(read_workflow(release), release), [])
        missed = []
        forms = {"a name": "on: {}\n", "a list": "on: [{}]\n", "a mapping": "on:\n  {}:\n"}
        runs_for_a_tag = [
            (event, form, on.format(event))
            for event in ("push", "create", "release")
            for form, on in forms.items()
        ]
        for event, form, on in examined("events that run for a tag, in each form", runs_for_a_tag):
            content = read_workflow(second_workflow(on))
            problems = release_problems(content, second_workflow(on))
            if tag_events(content) != [event] or not any("queue is None" in p for p in problems):
                missed.append(f"{event} as {form}: {problems}")
        for on in ("on:\n  workflow_dispatch:\n", "on:\n  push:\n    branches: [dev]\n"):
            if tag_events(read_workflow(second_workflow(on))):
                missed.append(f"{on!r} is not a tag's run, yet it is in the class")
        spellings = [
            (event, name, group)
            for event in TAG_REF_EVENTS
            for name, group in (
                ("Release", "${{ github.workflow }}-${{ github.ref }}"),
                (
                    "publish",
                    f"${{{{ github.event_name == '{event}' && 'release-' || 'x-' }}}}"
                    "${{ github.ref }}",
                ),
            )
        ]
        for event, name, group in examined("groups another workflow spells", spellings):
            other = read_workflow(second_workflow(f"on:\n  {event}:\n", group, name))
            problems = release_problems(read_workflow(release), release, [("publish.yml", other)])
            if not any("renders as publish.yml's" in problem for problem in problems):
                missed.append(f"{event}, {name}, {group}: {problems}")
            text = second_workflow(f"on:\n  {event}:\n", "${{ github.workflow }}-${{ github.ref }}")
            pair = [("publish.yml", read_workflow(text))]
            if release_problems(read_workflow(release), release, pair):
                missed.append(f"{event}: a group of another name is refused")
        keys = [
            (key, spelt) for key in BLOCK_KEYS for spelt in (key.upper(), key.title(), key[:-1])
        ]
        for key, spelt in examined("block keys GitHub's parser does not define", keys):
            text = release.replace(f"\n  {key}:", f"\n  {spelt}:", 1)
            problems = release_problems(read_workflow(text), text)
            if text == release or not any("does not define" in p for p in problems):
                missed.append(f"{spelt} for {key}: {problems}")
        twice = [(f"  {key}:", f"  {spelt}:") for key in BLOCK_KEYS for spelt in (key, key.upper())]
        twice += [("concurrency:", "'concurrency':"), ("concurrency:", "Concurrency:")]
        block = release[release.index("\nconcurrency:\n") + 1 :].split("\n\n", 1)[0]
        for old, spelt in examined("keys held twice", twice):
            held = block
            if old != "concurrency:":
                held = next(line for line in block.splitlines() if line.startswith(old))
            try:
                read_workflow(release.replace(held, held + "\n" + held.replace(old, spelt, 1), 1))
            except Unread:
                continue
            missed.append(f"{spelt.strip()} held twice is read")
        jobs = "\njobs:\n  publish:\n"
        text = release.replace(jobs, jobs + "    Concurrency:\n      group: x\n", 1)
        if not any("of its own" in p for p in release_problems(read_workflow(text), text)):
            missed.append("a job's Concurrency key is read")
        self.assertEqual(missed, [])
        every = read_workflow(second_workflow("on: [create, push, release, workflow_dispatch]\n"))
        self.assertEqual(tag_events(every), ["push", "create", "release"])

    def test_a_shape_that_replaces_drops_or_runs_two_at_once_is_refused(self):
        queue = "  queue: max\n"
        tag = "  push:\n    tags:\n      - 'v[0-9]+.[0-9]+.[0-9]+'\n"
        release = tag + "  release:\n    types: [published]\n"
        shapes = {
            "the tag group with no queue (today)": ([(queue, "")], "queue is None"),
            "queue: single": ([(queue, "  queue: single\n")], "queue is 'single'"),
            "a group keyed by the run id": (
                [("group: release-${{ github.ref }}", "group: release-${{ github.run_id }}")],
                "two runs of one tag have two groups (push)",
            ),
            "cancel-in-progress true": (
                [("cancel-in-progress: false", "cancel-in-progress: true")],
                "cancels the run before it (push)",
            ),
            "a group another workflow renders, in another case": (
                [("group: release-${{ github.ref }}", "group: RUST-CACHE")],
                "renders as rust-cache.yml's",
            ),
            "a block with no group": (
                [("  group: release-${{ github.ref }}\n", "")],
                "has no group",
            ),
            "a job-level block under a quoted key": (
                [
                    (
                        "jobs:\n  release:\n",
                        "jobs:\n  release:\n    'concurrency':\n      group: x\n",
                    )
                ],
                "job release sets a concurrency block",
            ),
            "a release whose group is its run id": (
                [
                    (tag, release),
                    (
                        "group: release-${{ github.ref }}",
                        "group: release-${{ github.event_name == 'push' && github.ref"
                        " || github.run_id }}",
                    ),
                ],
                "two runs of one tag have two groups (release)",
            ),
            "a release that cancels": (
                [
                    (tag, release),
                    (
                        "cancel-in-progress: false",
                        "cancel-in-progress: ${{ github.event_name == 'release' }}",
                    ),
                ],
                "cancels the run before it (release)",
            ),
        }
        others = [(name, c) for name, c, _t in read_all() if name != "release.yml"]
        for label, (edits, expected) in shapes.items():
            text = planted(*edits[0])
            for old, new in edits[1:]:
                assert old in text, old
                text = text.replace(old, new, 1)
            problems = release_problems(read_workflow(text), text, others)
            self.assertTrue(any(expected in problem for problem in problems), (label, problems))
        text = planted(queue, queue)
        self.assertEqual(release_problems(read_workflow(text), text, others), [])

    def test_every_push_or_release_that_runs_for_a_tag_is_in_the_class(self):
        tag = "on:\n  push:\n    tags:\n      - 'v[0-9]+.[0-9]+.[0-9]+'\n"
        triggers = {
            "a push with a tags-ignore filter": (
                "on:\n  push:\n    tags-ignore:\n      - 'v0.*'\n",
                ["push"],
            ),
            "a push with a paths filter only": (
                "on:\n  push:\n    paths:\n      - 'deploy/**'\n",
                ["push"],
            ),
            "a bare push": ("on:\n  push:\n", ["push"]),
            "a push named in a list": ("on: [push]\n", ["push"]),
            "a release named alone": ("on: release\n", ["release"]),
            "a push of branches only": ("on:\n  push:\n    branches: [main]\n", []),
        }
        judged = examined("planted triggers", list(triggers.items()))
        for label, (on, expected) in judged:
            text = planted(tag, on)
            self.assertEqual(tag_events(read_workflow(text)), expected, label)

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
