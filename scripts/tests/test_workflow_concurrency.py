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
    kind,
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


def workflow_texts():
    """Every workflow file of the directory as {file name: its text}."""
    return {path.name: path.read_text(encoding="utf-8") for path in workflow_files(WORKFLOWS)}


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


def declared(content):
    """A workflow's events as {event: its filters}, `on:` read as a name, a list or a mapping."""
    events = content.get("on")
    if isinstance(events, str):
        return {events: None}
    if isinstance(events, list):
        return dict.fromkeys(events)
    if not isinstance(events, dict):
        raise AssertionError(f"an `on:` the reader does not model: {events!r}")
    return events


def tag_events(content):
    """The events that start a run of a workflow for a tag, as GitHub's docs read them: a push whose
    filters admit a tag (a `tags` or `tags-ignore` filter, or neither a branch nor a tag filter), a
    create, which runs for every branch or tag created and takes no filter, and a release."""
    events = declared(content)
    found = []
    if "push" in events:
        filters = set(events["push"] or {})
        if filters & {"tags", "tags-ignore"} or not filters & {"branches", "branches-ignore"}:
            found.append("push")
    found += [event for event in ("create", "release") if event in events]
    return found


def workflow_name(name, content):
    """`github.workflow` for a workflow: its `name`, or its file's path when it has none."""
    return content.get("name") or f".github/workflows/{name}"


def releases_a_tag(content):
    """Whether a workflow runs for a tag: the workflows whose group can hold two runs of one
    release."""
    return bool(tag_events(content))


def tag_run(event, run_id, workflow="ci"):
    """A run of `event` for the tag v1.0.0 with the given run id, of the workflow named so."""
    run = push("refs/tags/v1.0.0", run_id=run_id)
    return dict(run, **{"github.event_name": event, "github.workflow": workflow})


def canonical(group):
    """A group with each `github['name']` inside an expression read as `github.name`: the contexts
    page's index syntax, which the expression reader does not model."""
    return re.sub(
        r"\$\{\{.*?\}\}",
        lambda found: re.sub(r"github\['([\w-]+)'\]", r"github.\1", found.group(0)),
        str(group),
    )


def rendered_groups(name, content):
    """Every group another workflow's block renders, read without case, as GitHub reads a group's
    name across the repository: with the workflow's own name as `github.workflow`, in the scenarios
    the tests model and in a run for a tag of each event it declares whose ref can be a tag."""
    block = content.get("concurrency")
    group = canonical(block.get("group", "") if isinstance(block, dict) else block or "")
    own = {"github.workflow": workflow_name(name, content)}
    scenarios = [*other_events("201").values(), pull_request("dev")]
    events = {"push", "release"} | (set(declared(content)) & set(TAG_REF_EVENTS))
    scenarios += [tag_run(event, "201") for event in sorted(events)]
    try:
        return {rendered(group, dict(context, **own)).casefold() for context in scenarios}
    except AssertionError as why:
        raise AssertionError(f"{name}'s group cannot be rendered: {why}") from why


def release_problems(content, text, others=(), name="release.yml"):
    """What a release workflow's concurrency gets wrong (SPEC-190 R10, ADR-292): one workflow-level
    block and no job's own, only the keys GitHub's parser defines, a group that is not empty, is the same for two runs of one tag under
    every event that runs it for a tag and is no other workflow's (`others`, as (name, content)),
    no run cancelled, and a group that queues every requested run, not one kept and replaced."""
    found = []
    blocks = len(re.findall(r"(?m)^\s*concurrency:", text))
    if blocks != 1:
        found.append(f"{blocks} concurrency blocks, so a job's own or none")
    for job_id, job in (content.get("jobs") or {}).items():
        if isinstance(job, dict) and any(str(key).casefold() == "concurrency" for key in job):
            found.append(f"job {job_id} sets a concurrency block of its own")
    block = content.get("concurrency")
    if block is None:
        return found + ["carries no concurrency block"]
    if not isinstance(block, dict):
        return found + [f"its block is the group {block!r} alone, so queue is the default"]
    unknown = sorted(str(key) for key in block if key not in BLOCK_KEYS)
    if unknown:
        found.append(f"its block holds {unknown}, which GitHub's parser does not define")
    group, cancel = canonical(block.get("group", "")), block.get("cancel-in-progress", "false")
    if kind(cancel) == "boolean":
        cancel = str(cancel).casefold()
    workflow = workflow_name(name, content)
    if not str(group).strip():
        found.append("has no group, which GitHub's workflow parser requires")
    for event in tag_events(content):
        first, second = tag_run(event, "201", workflow), tag_run(event, "202", workflow)
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


# The keys GitHub's workflow parser defines at a workflow's root, in a job that runs steps, in a job
# that calls a workflow, and in a push or a release filter. Each of these mappings is strict in the
# parser's schema (actions/languageservices, workflow-parser/src/workflow-v1.0.json), so the parser
# refuses a workflow that holds any other key there, read with case (SPEC-190 R11).
ROOT_KEYS = (
    "name",
    "run-name",
    "description",
    "on",
    "permissions",
    "env",
    "defaults",
    "concurrency",
    "jobs",
)
JOB_KEYS = (
    "name",
    "needs",
    "permissions",
    "if",
    "runs-on",
    "snapshot",
    "environment",
    "concurrency",
    "outputs",
    "env",
    "defaults",
    "steps",
    "timeout-minutes",
    "cancel-timeout-minutes",
    "strategy",
    "continue-on-error",
    "container",
    "services",
)
CALL_KEYS = (
    "name",
    "uses",
    "with",
    "secrets",
    "needs",
    "if",
    "permissions",
    "concurrency",
    "strategy",
)
FILTER_KEYS = {
    "push": ("branches", "branches-ignore", "paths", "paths-ignore", "tags", "tags-ignore"),
    "release": ("types",),
}
# The events GitHub's workflow parser defines under `on:`, a strict mapping in the same schema.
ON_EVENTS = (
    "branch_protection_rule",
    "check_run",
    "check_suite",
    "create",
    "delete",
    "deployment",
    "deployment_status",
    "discussion",
    "discussion_comment",
    "fork",
    "gollum",
    "image_version",
    "issue_comment",
    "issues",
    "label",
    "merge_group",
    "milestone",
    "page_build",
    "project",
    "project_card",
    "project_column",
    "public",
    "pull_request",
    "pull_request_comment",
    "pull_request_review",
    "pull_request_review_comment",
    "pull_request_target",
    "push",
    "registry_package",
    "release",
    "repository_dispatch",
    "schedule",
    "status",
    "watch",
    "workflow_call",
    "workflow_dispatch",
    "workflow_run",
)
# Every property of the github context, as GitHub's contexts page lists them: the population a
# release's group or cancel-in-progress could read besides github.ref (SPEC-190 R11).
GITHUB_PROPERTIES = (
    "action",
    "action_path",
    "action_ref",
    "action_repository",
    "action_status",
    "actor",
    "actor_id",
    "api_url",
    "base_ref",
    "env",
    "event",
    "event_name",
    "event_path",
    "graphql_url",
    "head_ref",
    "job",
    "path",
    "ref",
    "ref_name",
    "ref_protected",
    "ref_type",
    "repository",
    "repository_id",
    "repository_owner",
    "repository_owner_id",
    "retention_days",
    "run_attempt",
    "run_id",
    "run_number",
    "secret_source",
    "server_url",
    "sha",
    "token",
    "triggering_actor",
    "workflow",
    "workflow_ref",
    "workflow_sha",
    "workspace",
)
WORKFLOW_FIRST = re.compile(r"\$\{\{\s*github(?:\.workflow|\['workflow'\])\s*\}\}")


def blocks(name, content):
    """Every concurrency block a workflow holds, its own and each job's, as (where, block), the key
    read without case: GitHub reads a job's group in the one repository-wide namespace a workflow's
    group is in, so a job's block can replace a release's waiting run as a workflow's can."""
    found = [
        (name, value) for key, value in content.items() if str(key).casefold() == "concurrency"
    ]
    for job_id, job in (content.get("jobs") or {}).items():
        if isinstance(job, dict):
            found += [
                (f"{name} job {job_id}", value)
                for key, value in job.items()
                if str(key).casefold() == "concurrency"
            ]
    return found


def group_of(block):
    """A block's group: a mapping's `group`, or the block itself when it is a name alone."""
    return str(block.get("group", "") if isinstance(block, dict) else block or "")


def literal_prefix(name, content, group):
    """The text every rendering of a group starts with, read without case, or None when a run can
    choose it: a group that starts with an expression, or with `github.workflow` in a workflow another
    workflow can call (a called workflow reads its caller's name there). A leading `github.workflow`
    is otherwise the workflow's own name, or its file's path when it has none."""
    lead, rest = "", group
    first = WORKFLOW_FIRST.match(group)
    if first:
        if "workflow_call" in declared(content):
            return None
        lead, rest = workflow_name(name, content), group[first.end() :]
    return (lead + rest.split("${{", 1)[0]).casefold().lstrip() or None


def undefined_keys(content):
    """The keys a workflow holds at its root, in a job and in a push or release filter that GitHub's
    parser does not define there, read with case as the parser reads them."""
    found = [str(key) for key in content if key not in ROOT_KEYS]
    for job_id, job in (content.get("jobs") or {}).items():
        allowed = CALL_KEYS if isinstance(job, dict) and "uses" in job else JOB_KEYS
        found += [
            f"{job_id}.{key}"
            for key in (job if isinstance(job, dict) else {})
            if key not in allowed
        ]
    events = declared(content)
    found += [f"on.{event}" for event in events if event not in ON_EVENTS]
    for event, keys in FILTER_KEYS.items():
        filters = events.get(event)
        if isinstance(filters, dict):
            found += [f"{event}.{key}" for key in filters if key not in keys]
    return sorted(found)


def closed_by_construction(name, content, others):
    """The release class closed by construction, never by rendering a sample (SPEC-190 R11): every
    key it holds is one GitHub's parser defines; its group reads `github.ref` and nothing else (a
    leading `github.workflow` too, where no workflow can call it), so every run of one tag takes one
    group whatever its event or its tag; its cancel-in-progress is `false` as written; and no other
    block in any workflow file, a workflow's or a job's, starts with text its group's start can be,
    so no other block renders as its group for any run of any tag."""
    found = []
    undefined = undefined_keys(content)
    if undefined:
        found.append(f"it holds {undefined}, which GitHub's parser does not define there")
    block = content.get("concurrency")
    if not isinstance(block, dict):
        return found
    group = group_of(block)
    reads = [part.strip() for _start, _end, part in expressions(group)]
    allowed = set(REF_READS) | (
        set(WORKFLOW_READS) if "workflow_call" not in declared(content) else set()
    )
    if any(part not in allowed for part in reads):
        found.append(f"its group reads {reads}, not github.ref alone, so one tag's runs can split")
    cancel = block.get("cancel-in-progress", False)
    if "cancel-in-progress" in block and (
        kind(cancel) != "boolean" or str(cancel).casefold() != "false"
    ):
        found.append(f"cancel-in-progress is {cancel!r}, not false as written, so a run can cancel")
    mine = literal_prefix(name, content, group)
    if mine is None:
        found.append("its group starts with no text of its own, so another group can render as it")
        return found
    for other, other_content in others:
        for where, other_block in blocks(other, other_content):
            theirs = literal_prefix(other, other_content, group_of(other_block))
            if theirs is None or theirs.startswith(mine) or mine.startswith(theirs):
                found.append(f"{where}'s group can render as its group, so it can replace its runs")
    return found


# ------------------------------------------------ the release class, read as GitHub parses it (R12)

# GitHub's workflow parser (actions/languageservices, workflow-parser/src/workflow-v1.0.json, read
# 2026-09-30T09:29Z) types every value below a workflow's root. These constants are that schema's
# facts, and the types below are built from them; population.py proves each built type equals the
# schema's definition at its position. A value the parser refuses makes GitHub refuse the whole
# workflow, so a release workflow that holds one never runs for its tag.
NULL_EVENTS = (
    "create",
    "delete",
    "deployment",
    "deployment_status",
    "fork",
    "gollum",
    "page_build",
    "public",
    "status",
)
CHANGED = ("created", "edited", "deleted")
PULL_REQUEST_TYPES = (
    "assigned",
    "unassigned",
    "labeled",
    "unlabeled",
    "opened",
    "edited",
    "closed",
    "reopened",
    "synchronize",
    "converted_to_draft",
    "locked",
    "unlocked",
    "enqueued",
    "dequeued",
    "milestoned",
    "demilestoned",
    "ready_for_review",
    "review_requested",
    "review_request_removed",
    "auto_merge_enabled",
    "auto_merge_disabled",
    "stacked",
)
# The activity types each event's `types` admits, as a name or a list of names.
EVENT_TYPES = {
    "branch_protection_rule": CHANGED,
    "check_run": ("completed", "created", "rerequested", "requested_action"),
    "check_suite": ("completed",),
    "discussion": (
        *CHANGED,
        "transferred",
        "pinned",
        "unpinned",
        "labeled",
        "unlabeled",
        "locked",
        "unlocked",
        "category_changed",
        "answered",
        "unanswered",
    ),
    "discussion_comment": CHANGED,
    "image_version": ("created", "ready", "deleted"),
    "issue_comment": CHANGED,
    "issues": (
        "opened",
        "edited",
        "deleted",
        "transferred",
        "pinned",
        "unpinned",
        "closed",
        "reopened",
        "assigned",
        "unassigned",
        "labeled",
        "unlabeled",
        "locked",
        "unlocked",
        "milestoned",
        "demilestoned",
        "field_added",
        "field_removed",
        "typed",
        "untyped",
    ),
    "label": CHANGED,
    "merge_group": ("checks_requested",),
    "milestone": ("created", "closed", "opened", "edited", "deleted"),
    "project": ("created", "closed", "reopened", "edited", "deleted"),
    "project_card": ("created", "moved", "converted", "edited", "deleted"),
    "project_column": ("created", "updated", "moved", "deleted"),
    "pull_request": PULL_REQUEST_TYPES,
    "pull_request_comment": CHANGED,
    "pull_request_review": ("submitted", "edited", "dismissed"),
    "pull_request_review_comment": CHANGED,
    "pull_request_target": PULL_REQUEST_TYPES,
    "registry_package": ("published", "updated"),
    "release": (
        "published",
        "unpublished",
        "created",
        "edited",
        "deleted",
        "prereleased",
        "released",
    ),
    "watch": ("started",),
    "workflow_run": ("requested", "completed", "in_progress"),
}
# The keys of an event's mapping besides `types`: each takes a name or a list of names.
EVENT_FILTERS = {
    "push": FILTER_KEYS["push"],
    "pull_request": ("branches", "branches-ignore", "paths", "paths-ignore"),
    "pull_request_target": ("branches", "branches-ignore", "paths", "paths-ignore"),
    "merge_group": ("branches", "branches-ignore"),
    "image_version": ("names", "versions"),
    "workflow_run": ("workflows", "branches", "branches-ignore"),
}
ANY_LEVEL, WRITE_OR_NONE, READ_OR_NONE = (
    ("read", "write", "none"),
    ("write", "none"),
    ("read", "none"),
)
PERMISSION_SCOPES = {
    "actions": ANY_LEVEL,
    "artifact-metadata": ANY_LEVEL,
    "attestations": ANY_LEVEL,
    "checks": ANY_LEVEL,
    "code-quality": ANY_LEVEL,
    "contents": ANY_LEVEL,
    "copilot-requests": WRITE_OR_NONE,
    "deployments": ANY_LEVEL,
    "discussions": ANY_LEVEL,
    "drives": ANY_LEVEL,
    "id-token": WRITE_OR_NONE,
    "issues": ANY_LEVEL,
    "models": READ_OR_NONE,
    "packages": ANY_LEVEL,
    "pages": ANY_LEVEL,
    "pull-requests": ANY_LEVEL,
    "repository-projects": ANY_LEVEL,
    "security-events": ANY_LEVEL,
    "statuses": ANY_LEVEL,
    "vulnerability-alerts": READ_OR_NONE,
}
DISPATCH_INPUT_TYPES = ("string", "boolean", "number", "environment", "choice")
CALL_INPUT_TYPES = ("string", "boolean", "number")
# The contexts an expression may read, by where it is.
WORKFLOW_CONTEXTS = ("github", "inputs", "vars")
JOB_CONTEXTS = (*WORKFLOW_CONTEXTS, "needs", "strategy", "matrix")

# A type: ("null",), ("boolean",), ("number",), ("string",) for any scalar GitHub converts to a
# string, ("text",) for a non-empty one, ("is", value) for one constant, ("one-of", *types),
# ("sequence", item or None), ("mapping", {key: type} or None, required keys) for a strict mapping,
# ("loose", value type) for a mapping of any keys, and ("expr", contexts, type), where an expression
# reading those contexts may stand for the value. A None item or key map is a subtree read by its
# kind alone: a job's steps, strategy, container, services, and the mappings of runs-on, environment
# and snapshot (#464).
NULL, BOOLEAN, NUMBER, STRING, TEXT = (
    ("null",),
    ("boolean",),
    ("number",),
    ("string",),
    ("text",),
)
ANY_MAPPING, ANY_SEQUENCE = ("mapping", None, ()), ("sequence", None)


def one_of(*types):
    return ("one-of", *types)


def names(*values):
    return one_of(*(("is", value) for value in values))


def mapping(keys, required=()):
    return ("mapping", keys, tuple(required))


def expr(contexts, of):
    return ("expr", tuple(contexts), of)


TEXTS = one_of(TEXT, ("sequence", TEXT))
SCALAR = one_of(STRING, BOOLEAN, NUMBER)


def event_type(event):
    """The type of `on.<event>`'s value, built from the constants above."""
    if event in NULL_EVENTS:
        return NULL
    if event == "schedule":
        return ("sequence", mapping({"cron": TEXT, "timezone": TEXT}, ["cron"]))
    if event == "workflow_dispatch":
        field = {
            "description": STRING,
            "type": names(*DISPATCH_INPUT_TYPES),
            "required": BOOLEAN,
        }
        field |= {"default": SCALAR, "options": ("sequence", STRING)}
        return one_of(NULL, mapping({"inputs": ("loose", mapping(field))}))
    if event == "workflow_call":
        field = {
            "description": STRING,
            "type": names(*CALL_INPUT_TYPES),
            "required": BOOLEAN,
        }
        field["default"] = expr(WORKFLOW_CONTEXTS, SCALAR)
        secret = one_of(NULL, mapping({"description": STRING, "required": BOOLEAN}))
        output = mapping(
            {
                "description": STRING,
                "value": expr((*WORKFLOW_CONTEXTS, "jobs"), STRING),
            },
            ["value"],
        )
        keys = {
            "inputs": ("loose", mapping(field, ["type"])),
            "secrets": ("loose", secret),
        }
        return one_of(NULL, mapping(keys | {"outputs": ("loose", output)}))
    keys = dict.fromkeys(EVENT_FILTERS.get(event, ()), TEXTS)
    if event == "repository_dispatch":
        keys["types"] = ("sequence", TEXT)
    elif event in EVENT_TYPES:
        keys["types"] = one_of(names(*EVENT_TYPES[event]), ("sequence", names(*EVENT_TYPES[event])))
    return one_of(NULL, mapping(keys))


PERMISSIONS = one_of(
    mapping({scope: names(*levels) for scope, levels in PERMISSION_SCOPES.items()}),
    ("is", "read-all"),
    ("is", "write-all"),
)
CONCURRENCY = mapping(
    {"group": TEXT, "cancel-in-progress": BOOLEAN, "queue": names("single", "max")},
    ["group"],
)
RUN_DEFAULTS = mapping({"shell": TEXT, "working-directory": TEXT})
NEEDS = one_of(("sequence", TEXT), TEXT)
JOB_IF = expr(("github", "inputs", "vars", "needs"), STRING)
STEPS_JOB = mapping(
    {
        "needs": NEEDS,
        "if": JOB_IF,
        "strategy": expr(("github", "inputs", "vars", "needs"), ANY_MAPPING),
        "name": expr(JOB_CONTEXTS, STRING),
        "runs-on": expr(JOB_CONTEXTS, one_of(TEXT, ("sequence", TEXT), ANY_MAPPING)),
        "timeout-minutes": expr(JOB_CONTEXTS, NUMBER),
        "cancel-timeout-minutes": expr(JOB_CONTEXTS, NUMBER),
        "continue-on-error": expr(JOB_CONTEXTS, BOOLEAN),
        "container": expr(JOB_CONTEXTS, one_of(STRING, ANY_MAPPING)),
        "services": expr(JOB_CONTEXTS, ANY_MAPPING),
        "env": expr((*JOB_CONTEXTS, "secrets"), ("loose", STRING)),
        "environment": expr(JOB_CONTEXTS, one_of(STRING, ANY_MAPPING)),
        "permissions": PERMISSIONS,
        "concurrency": expr(JOB_CONTEXTS, one_of(TEXT, CONCURRENCY)),
        "outputs": (
            "loose",
            expr((*JOB_CONTEXTS, "secrets", "steps", "job", "runner", "env"), STRING),
        ),
        "defaults": mapping({"run": expr((*JOB_CONTEXTS, "env"), RUN_DEFAULTS)}),
        "steps": ANY_SEQUENCE,
        "snapshot": one_of(TEXT, ANY_MAPPING),
    },
    ["runs-on"],
)
CALL_JOB = mapping(
    {
        "name": expr(JOB_CONTEXTS, STRING),
        "uses": TEXT,
        "with": ("loose", expr(JOB_CONTEXTS, SCALAR)),
        "secrets": one_of(("loose", expr((*JOB_CONTEXTS, "secrets"), SCALAR)), ("is", "inherit")),
        "needs": NEEDS,
        "if": JOB_IF,
        "permissions": PERMISSIONS,
        "concurrency": expr(JOB_CONTEXTS, one_of(TEXT, CONCURRENCY)),
        "strategy": expr(("github", "inputs", "vars", "needs"), ANY_MAPPING),
    },
    ["uses"],
)
WORKFLOW = mapping(
    {
        "on": one_of(
            names(*ON_EVENTS),
            ("sequence", names(*ON_EVENTS)),
            mapping({event: event_type(event) for event in ON_EVENTS}),
        ),
        "name": STRING,
        "description": STRING,
        "run-name": expr(WORKFLOW_CONTEXTS, STRING),
        "defaults": mapping({"run": RUN_DEFAULTS}),
        "env": expr((*WORKFLOW_CONTEXTS, "secrets"), ("loose", STRING)),
        "permissions": PERMISSIONS,
        "concurrency": expr(WORKFLOW_CONTEXTS, one_of(STRING, CONCURRENCY)),
        "jobs": ("loose", "job"),
    },
    ["on", "jobs"],
)


def expressions(value):
    """Each `${{ }}` in a string as (start, end, its text), found as GitHub's parser finds them: from
    each `${{`, a `'` opens or closes a string and `}}` outside one closes the expression
    (template-reader.ts, parseScalar). An expression with no closing `}}` ends the list with end
    None: GitHub refuses the workflow ("The expression is not closed")."""
    found, start = [], value.find("${{")
    while start >= 0:
        at, quoted, end = start + 3, False, None
        while at < len(value):
            if value[at] == "'":
                quoted = not quoted
            elif not quoted and value[at] == "}" and value[at - 1] == "}":
                end = at + 1
                break
            at += 1
        found.append((start, end, value[start + 3 : end - 2] if end else value[start + 3 :]))
        if end is None:
            return found
        start = value.find("${{", end)
    return found


def contexts_read(text):
    """The contexts an expression names: each name that is not a property, a function or a literal."""
    bare = re.sub(r"'(?:[^']|'')*'", "''", text)
    found = re.findall(r"(?<![\w.\-])([A-Za-z_][\w-]*)(?![\w-])(?!\s*\()", bare)
    return set(found) - {"true", "false", "null", "NaN", "Infinity"}


def typed_problems(value, of, where, contexts=()):
    """Where `value` is not of type `of`, as GitHub's parser types it at `where`."""
    head = of[0]
    if head == "expr":
        return typed_problems(value, of[2], where, (*contexts, *of[1]))
    if isinstance(value, str) and "${{" in value:
        found = expressions(value)
        if found[-1][1] is None:
            return [f"{where} holds a `${{{{` with no closing `}}}}`"]
        if not contexts:
            return [f"{where} holds an expression where GitHub's parser admits none"]
        read = set().union(*(contexts_read(text) for _s, _e, text in found)) - set(contexts)
        return [f"{where} reads {sorted(read)}, which it cannot read there"] if read else []
    if head == "one-of":
        tried = [typed_problems(value, option, where, contexts) for option in of[1:]]
        if any(not problems for problems in tried):
            return []
        fits = [
            problems for option, problems in zip(of[1:], tried) if shape(option) == shape(value)
        ]
        return fits[0] if len(fits) == 1 else [f"{where} is a {kind(value)} it cannot be"]
    if head in ("mapping", "loose"):
        if kind(value) != "mapping":
            return [f"{where} is a {kind(value)}, not a mapping"]
        if head == "loose":
            return [
                problem
                for key, item in value.items()
                for problem in typed_problems(item, _named(of[1], item), f"{where}.{key}", contexts)
            ]
        keys, required = of[1], of[2]
        if keys is None:
            return []
        found = [
            f"{where}.{key} is a key GitHub's parser does not define"
            for key in value
            if key not in keys
        ]
        found += [f"{where} holds no {key}" for key in required if key not in value]
        for key, item in value.items():
            if key in keys:
                found += typed_problems(item, keys[key], f"{where}.{key}", contexts)
        return found
    if head == "sequence":
        if kind(value) != "sequence":
            return [f"{where} is a {kind(value)}, not a sequence"]
        if of[1] is None:
            return []
        return [
            problem
            for at, item in enumerate(value)
            for problem in typed_problems(item, of[1], f"{where}[{at}]", contexts)
        ]
    scalar = kind(value)
    if scalar in ("mapping", "sequence"):
        return [f"{where} is a {scalar}, not a scalar"]
    text = "" if value is None else str(value)
    accepted = {
        "null": scalar == "null",
        "boolean": scalar == "boolean",
        "number": scalar == "number",
        "string": True,
        "text": bool(text),
        "is": scalar != "null" and text == of[-1],
    }[head]
    return [] if accepted else [f"{where} is the {scalar} {text!r}, which it cannot be"]


def shape(value_or_type):
    """A value's or a type's shape: a mapping, a sequence or a scalar, so a one-of names the one
    option a value was written for."""
    of = value_or_type
    if isinstance(of, tuple):
        while of[0] == "expr":
            of = of[2]
        return {"mapping": "mapping", "loose": "mapping", "sequence": "sequence"}.get(
            of[0], "scalar"
        )
    return {"mapping": "mapping", "sequence": "sequence"}.get(kind(of), "scalar")


def _named(of, value):
    """A job's type is chosen by its keys, as the parser's one-of reads it: `uses` makes it a call."""
    if of != "job":
        return of
    return CALL_JOB if isinstance(value, dict) and "uses" in value else STEPS_JOB


def unclosed(value, where="it"):
    """Every string a workflow holds, key or value, at any depth, that holds a `${{` with no closing
    `}}`: GitHub's parser refuses the file for one anywhere, steps included."""
    if isinstance(value, dict):
        found = [f"{where}.{key} is a key" for key in value if unclosed(str(key))]
        for key, item in value.items():
            found += unclosed(item, f"{where}.{key}")
        return found
    if isinstance(value, list):
        return [p for at, item in enumerate(value) for p in unclosed(item, f"{where}[{at}]")]
    if isinstance(value, str) and "${{" in value and expressions(value)[-1][1] is None:
        return [f"{where} holds a `${{{{` with no closing `}}}}`"]
    return []


def schema_problems(content):
    """Every value in a workflow GitHub's parser refuses, from its root down to a job's keys, and
    every unclosed expression at any depth (SPEC-190 R12)."""
    found = typed_problems(content, WORKFLOW, "it")
    return found + [p for p in unclosed(content) if p not in found]


# A reusable workflow in this repository, called as GitHub reads it from the caller's own commit.
LOCAL_CALL = re.compile(r"\./\.github/workflows/([^/@\s]+)")
REF_READS = ("github.ref", "github['ref']")
WORKFLOW_READS = ("github.workflow", "github['workflow']")


def calls(content):
    """A workflow's call jobs as (job id, `uses`)."""
    jobs = content.get("jobs") if isinstance(content, dict) else None
    return [
        (job_id, str(job["uses"]))
        for job_id, job in (jobs if isinstance(jobs, dict) else {}).items()
        if isinstance(job, dict) and "uses" in job
    ]


def group_prefix(block, runner):
    """A block's group as ((text every rendering starts with, read without case and leading
    space), its expressions) in a run of `runner`: a leading github.workflow renders the runner's
    name, since a called workflow reads its caller's github context (the reuse page)."""
    group = str(block.get("group", "") if isinstance(block, dict) else block or "")
    found = expressions(group)
    lead = ""
    if found and found[0][0] == 0 and found[0][2].strip() in WORKFLOW_READS:
        lead, group, found = runner, group[found[0][1] :], found[1:]
        found = expressions(group)
    text = group[: found[0][0]] if found else group
    return (lead + text).casefold().lstrip(), [part.strip() for _s, _e, part in found]


def membership(files):
    """Each workflow file's place in the release class (SPEC-190 R12): "release" for a workflow a
    tag can start (a push that admits a tag, a create or a release) and for every workflow such a
    workflow calls, at any depth, and "reacher" for every other one. Every run of a release
    workflow is a release run, whatever its event; a reacher's runs are not, and its blocks are
    judged only by whether they can reach a release run's group."""
    read = {name: _read_quietly(text) for name, text in files.items()}
    found = {name: "reacher" for name in files}
    todo = [name for name, content in read.items() if _events(content) and releases_a_tag(content)]
    while todo:
        name = todo.pop()
        if found.get(name) == "release":
            continue
        found[name] = "release"
        for _job, uses in calls(read.get(name) or {}):
            local = LOCAL_CALL.fullmatch(uses)
            if local and local.group(1) in read:
                todo.append(local.group(1))
    return found


def _events(content):
    """A workflow's events, or none when its `on:` is not one the reader models."""
    try:
        return declared(content) if isinstance(content, dict) else {}
    except AssertionError:
        return {}


def _read_quietly(text):
    try:
        return read_workflow(text)
    except Unread as why:
        return why.workflow
    except AssertionError:
        return None


def release_class_problems(files):
    """The release class closed by construction over every workflow file (`files`, {name: text}),
    never by a sample (SPEC-190 R12):
    - a release workflow is read as GitHub's parser reads it: typed, tab-free, and every value it
      holds down to a job's keys is one the parser defines there;
    - every call, in any workflow, is `./.github/workflows/<file>` with the file here and taking
      `workflow_call`, walked to its end with no cycle, so no block is unread;
    - every block a release run holds, its own or a callee's, renders in the runner's context with
      text of its own, then github.ref and nothing else; never cancels (cancel-in-progress absent
      or the boolean false); and queues (`queue: max`), a callee's as its caller's, since the docs
      say nothing of how many calls can wait in a callee's group; a release workflow holds no
      job-level block;
    - no other block, a reacher's or another workflow's, starts with text a release block's start
      can be, so none renders as its group for any run of any tag."""
    found, read = [], {}
    for name, text in files.items():
        try:
            read[name] = read_workflow(text)
        except AssertionError as why:
            found.append(f"{name}: the reader refuses it ({why})")
            read[name] = _read_quietly(text)
    classes = membership(files)
    names = {
        name: workflow_name(name, content) for name, content in read.items() if content is not None
    }
    instances = []
    for runner, content in read.items():
        if not isinstance(content, dict) or set(_events(content)) <= {"workflow_call"}:
            continue
        todo = [(runner, (runner,))]
        while todo:
            name, path = todo.pop()
            for where, block in blocks(name, read[name]):
                instances.append((runner, path, name, where, block))
            for job_id, uses in calls(read[name]):
                local = LOCAL_CALL.fullmatch(uses)
                callee = local.group(1) if local else None
                problem = None
                if callee is None or callee not in read or read[callee] is None:
                    problem = f"calls {uses!r}, which is not a workflow file here, so it is unread"
                elif callee in path:
                    problem = f"calls {callee}, which is already on the path {list(path)}: a cycle"
                elif "workflow_call" not in _events(read[callee]):
                    problem = f"calls {callee}, which does not take workflow_call"
                if problem:
                    message = f"{name}: job {job_id} {problem}"
                    if message not in found:
                        found.append(message)
                    continue
                todo.append((callee, (*path, callee)))
    for name, content in read.items():
        if classes[name] != "release" or content is None:
            continue
        found += [f"{name}: {problem}" for problem in schema_problems(content)]
        for where, _block in blocks(name, content):
            if where != name:
                found.append(f"{where} sets a concurrency block of its own")
    release = [item for item in instances if classes[item[0]] == "release"]
    for runner, _path, name, where, block in release:
        if where != name:
            continue
        label = f"{where} in a run of {runner}"
        prefix, reads = group_prefix(block, names[runner])
        if not prefix:
            found.append(f"{label}: its group starts with no text of its own")
        if any(part not in REF_READS for part in reads):
            found.append(f"{label}: its group reads {reads}, not github.ref alone")
        cancel = block.get("cancel-in-progress") if isinstance(block, dict) else None
        if isinstance(block, dict) and "cancel-in-progress" in block:
            if kind(cancel) != "boolean" or str(cancel).casefold() != "false":
                found.append(f"{label}: cancel-in-progress is {cancel!r}, not the boolean false")
        queue = block.get("queue") if isinstance(block, dict) else None
        if queue != "max":
            found.append(f"{label}: queue is {queue!r}, so a waiting run can be replaced")
        for other in instances:
            if other[3] == where:
                continue
            theirs, _reads = group_prefix(other[4], names[other[0]])
            if theirs.startswith(prefix) or prefix.startswith(theirs):
                message = f"{other[3]} in a run of {other[0]} can render as {label}'s group"
                if message not in found:
                    found.append(message)
    return found


def other_block(on, key, value, where, name="publish"):
    """A planted workflow with one block under `key`, at its root or in its job: `value` is a
    mapping's text or a group's name."""
    block = f"{key}:{value}"
    root = block + "\n" if where == "the workflow" else ""
    job = "".join(f"    {line}\n" for line in block.splitlines()) if where == "a job" else ""
    return (
        f"name: {name}\n\n{on}\npermissions:\n  contents: read\n\n{root}\njobs:\n  publish:\n"
        f"    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n{job}    steps:\n      - run: echo publish\n"
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
            problems = release_problems(content, text, others, name)
            problems += closed_by_construction(name, content, others)
            found += [f"{name}: {problem}" for problem in problems]
        found += release_class_problems(workflow_texts())
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
        """The class is derived from R12's membership rule, never pinned by name: every workflow
        file is a release workflow (a tag can start it, or a release workflow calls it) or a
        reacher, and release.yml is a release workflow. A new tag workflow joins the class and is
        judged by the class rule, never refused here by its name."""
        files = workflow_texts()
        classes = membership(files)
        judged = examined("workflows", sorted(classes.items()))
        self.assertEqual(sorted(classes), sorted(files))
        self.assertIn(("release.yml", "release"), judged)
        for name, text in files.items():
            if releases_a_tag(read_workflow(text)):
                self.assertEqual(classes[name], "release", name)
            for _job, uses in calls(read_workflow(text)):
                called = LOCAL_CALL.fullmatch(uses)
                if classes[name] == "release" and called:
                    self.assertEqual(classes.get(called.group(1)), "release", uses)
        self.assertEqual({place for _name, place in judged} - {"release", "reacher"}, set())
        self.assertTrue(classes, "the census holds no workflow")
        self.assertEqual(classes.get("release.yml"), "release")
        # A planted directory: the census finds a second workflow a tag starts and a workflow the
        # release calls, and leaves a reacher out, from the rule's own text helpers.
        tag = "on:\n  push:\n    tags: [v1]\n"
        plain = "jobs:\n  publish:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n"
        plain += "    steps:\n      - run: echo publish\n"
        caller = "jobs:\n  build:\n    uses: ./.github/workflows/called.yml\n"
        planted_files = {
            "release.yml": second_workflow(tag, name="release").replace(plain, caller),
            "second.yml": second_workflow(tag, name="second"),
            "called.yml": second_workflow("on:\n  workflow_call:\n", name="called"),
            "reacher.yml": second_workflow("on:\n  pull_request:\n", name="reacher"),
        }
        self.assertEqual(
            membership(planted_files),
            {
                "release.yml": "release",
                "second.yml": "release",
                "called.yml": "release",
                "reacher.yml": "reacher",
            },
        )

    def test_the_release_class_is_closed_by_construction(self):
        """SPEC-190 R11's population, generated, never listed: a release's group read through every
        github property and every other context a block can read, in each operator form; each
        cancellation; every other block, a workflow's or a job's, under each key spelling and form,
        for each event a run takes, whose group can render as the release's; and every key GitHub's
        parser defines, misspelt, at the root, in a job and in a filter. A group another workflow
        spells with text of its own is admitted, so the rule refuses by construction, not by name."""
        release = second_workflow(
            "on:\n  push:\n    tags: [v1]\n",
            "release-${{ github.ref }}",
            "release",
            "  queue: max\n",
        )
        good = read_workflow(release)
        self.assertEqual(closed_by_construction("release.yml", good, []), [])
        missed = []
        contexts = [f"github.{p}" for p in GITHUB_PROPERTIES]
        contexts += ["inputs.tag", "vars.GROUP", "env.GROUP", "needs.build.result", "matrix.tag"]
        forms = (
            "{0}",
            "{0} || github.ref",
            "github.ref == 'refs/tags/v2.0.0' && {0} || github.ref",
            "format('{{0}}', {0})",
        )
        reads = [form.format(c) for c in contexts for form in forms]
        reads = [r for r in reads if r not in ("github.ref", "github.workflow")]
        for read in examined("groups a release reads besides github.ref", reads):
            text = release.replace("release-${{ github.ref }}", f"release-${{{{ {read} }}}}", 1)
            if not closed_by_construction("release.yml", read_workflow(text), []):
                missed.append(f"a group reading {read}")
        cancels = [
            "true",
            *(f"${{{{ {r} }}}}" for r in reads + ["github.ref == 'refs/tags/v2.0.0'"]),
        ]
        for cancel in examined("cancellations a release reads", cancels):
            text = release.replace("cancel-in-progress: false", f"cancel-in-progress: {cancel}", 1)
            if not closed_by_construction("release.yml", read_workflow(text), []):
                missed.append(f"cancel-in-progress: {cancel}")
        called = release.replace("tags: [v1]\n", "tags: [v1]\n  workflow_call:\n", 1).replace(
            "release-${{ github.ref }}", "${{ github.workflow }}-${{ github.ref }}", 1
        )
        if not closed_by_construction("release.yml", read_workflow(called), []):
            missed.append("a release another workflow can call reads github.workflow")
        wf = "${{ github.workflow }}-${{ github.ref }}"
        shares = [
            ("publish", "release-${{ github.ref }}"),
            ("Release", wf),
            ("RELEASE", wf),
            ("publish", "release-refs/tags/v2.0.0"),
            ("publish", "RELEASE-refs/tags/${{ github.ref_name }}"),
            ("publish", "${{ inputs.group }}"),
            ("publish", "${{ vars.GROUP }}"),
        ]
        events = [*TAG_REF_EVENTS, "workflow_call", "schedule"]
        keys = ("concurrency", "'concurrency'", '"concurrency"')
        places = ("the workflow", "a job")
        shapes = [
            (f"on:\n  {event}:\n", key, place, name, value)
            for name, group in shares
            for event in events
            for key in keys
            for place in places
            for value in (f"\n  group: {group}\n  cancel-in-progress: false", f" {group}")
        ]
        shapes += [
            ("on:\n  workflow_call:\n", key, place, "publish", value)
            for key in keys
            for place in places
            for value in (f"\n  group: {wf}\n  cancel-in-progress: false", f" {wf}")
        ]
        for on, key, place, name, value in examined(
            "blocks that can render as a release's", shapes
        ):
            other = read_workflow(other_block(on, key, value, place, name))
            if not closed_by_construction("release.yml", good, [("publish.yml", other)]):
                missed.append(f"{place}'s {key}{value!r} under {on!r}, named {name}")
        admitted = [
            (f"on:\n  {event}:\n", key, place, value)
            for event in events
            for key in keys
            for place in places
            for group in ("publish-${{ github.ref }}", *((wf,) if event != "workflow_call" else ()))
            for value in (f"\n  group: {group}\n  cancel-in-progress: false", f" {group}")
        ]
        for on, key, place, value in examined("blocks of text of their own", admitted):
            other = read_workflow(other_block(on, key, value, place))
            if closed_by_construction("release.yml", good, [("publish.yml", other)]):
                missed.append(f"{place}'s {key}{value!r} under {on!r} is refused")
        spelt = [
            (where, key, variant)
            for where, population in (
                ("root", ROOT_KEYS),
                ("on", ON_EVENTS),
                ("job", sorted(set(JOB_KEYS) | set(CALL_KEYS))),
                ("push", FILTER_KEYS["push"]),
                ("release", FILTER_KEYS["release"]),
            )
            for key in population
            for variant in (key.upper(), key.title(), key[:-1])
            if variant not in population
        ]
        for where, key, variant in examined("keys GitHub's parser does not define there", spelt):
            text = {
                "root": lambda: release.replace("\njobs:\n", f"\n{variant}: x\njobs:\n", 1),
                "on": lambda: release.replace("  push:\n", f"  {variant}:\n  push:\n", 1),
                "job": lambda: release.replace(
                    "  publish:\n", f"  publish:\n    {variant}: x\n", 1
                ),
                "push": lambda: release.replace("  push:\n", f"  push:\n    {variant}: [v1]\n", 1),
                "release": lambda: release.replace(
                    "tags: [v1]\n", f"tags: [v1]\n  release:\n    {variant}: [published]\n", 1
                ),
            }[where]()
            try:
                problems = closed_by_construction("release.yml", read_workflow(text), [])
            except Unread:
                continue
            if not any("does not define there" in p for p in problems):
                missed.append(f"{variant} for {where}.{key}")
        cancelling = release.replace("cancel-in-progress: false", "cancel-in-progress: true", 1)
        refusal = closed_by_construction("release.yml", read_workflow(cancelling), [])
        self.assertIn("not false as written", " ".join(refusal))
        self.assertEqual(missed, [])

    def test_the_release_class_is_read_as_github_parses_it(self):
        """SPEC-190 R12's population, generated by construction, never listed. Each member is a set
        of workflow files beside a release workflow: every cancel-in-progress and queue a YAML 1.2
        reader types (quoted, block, the YAML 1.1 words); a tab in each line's indentation and a tab
        inside a value; each unclosed or misplaced expression; every event of the parser's schema
        with each wrong kind of value, and each root and job key with one; callees one, two and
        three calls deep, local, remote, missing and in a cycle; and every block that can render
        as a release's group, in each context and event. A member GitHub refuses, or whose tag run
        can be cancelled or replaced, is refused by the class rule; a member GitHub runs safely,
        with its tag's runs kept, is not."""
        tag = "on:\n  push:\n    tags: [v1]\n"
        release = second_workflow(tag, "release-${{ github.ref }}", "release", "  queue: max\n")
        self.assertEqual(release_class_problems({"release.yml": release}), [])

        def swap(old, new, text=release):
            assert old in text, old
            return text.replace(old, new, 1)

        members = []
        cancel = "  cancel-in-progress: false\n"
        refused = ["'false'", '"false"', "'False'", "|\n    false", "|-\n    false", "true"]
        refused += ["no", "No", "NO", "off", "Off", "OFF", "n", "N", "0", "~", "''", "yes", "on"]
        members += [
            ("types", f"cancel {v}", swap(cancel, f"  cancel-in-progress: {v}\n"), True)
            for v in refused
        ]
        members += [
            ("types", f"cancel {v}", swap(cancel, f"  cancel-in-progress: {v}\n"), False)
            for v in ("false", "False", "FALSE", "false # kept")
        ]
        members += [
            ("types", f"queue {v}", swap("  queue: max\n", f"  queue: {v}\n"), True)
            for v in ("MAX", "Max", "~", "single", "|\n    max")
        ]
        members += [
            ("types", f"queue {v}", swap("  queue: max\n", f"  queue: {v}\n"), False)
            for v in ("'max'", '"max"')
        ]
        lines = release.splitlines(keepends=True)
        for at, line in enumerate(lines):
            if line.strip() and not line.lstrip().startswith("#"):
                depth = len(line) - len(line.lstrip(" "))
                for spaced in {"\t" + line.lstrip(" "), line[:depth] + "\t" + line.lstrip(" ")}:
                    text = "".join(lines[:at] + [spaced] + lines[at + 1 :])
                    members.append(("tabs", f"a tab indents line {at + 1}", text, True))
        members += [
            (
                "tabs",
                "a tab inside a step's text",
                swap("run: echo publish", "run: echo\tpublish"),
                False,
            ),
            ("tabs", "a tab inside the group", swap("release-${{", "release-\t${{"), False),
            (
                "tabs",
                "a tab-indented comment line",
                swap("\njobs:\n", "\n\t# the jobs\njobs:\n"),
                False,
            ),
            (
                "tabs",
                "a tab in a block's text",
                swap("- run: echo publish\n", "- run: |\n          echo\tpublish\n"),
                False,
            ),
        ]
        block = "- run: |\n          echo one\n{}\n"
        members += [
            ("tabs", label, swap("- run: echo publish\n", block.format(second)), harmful)
            for label, second, harmful in (
                ("a tab inside a block's indentation", "         \techo two", True),
                ("a block's line indented less than its text", "         echo two", True),
                ("a tab after a block's indentation", "          \techo two", False),
                ("a block's line indented more than its text", "            echo two", False),
            )
        ]
        members.append(
            (
                "tabs",
                "a blank line above a block's text indented more than it",
                swap("- run: echo publish\n", "- run: |\n            \n          echo one\n"),
                True,
            )
        )
        group = "  group: release-${{ github.ref }}\n"
        for label, text in (
            ("an unclosed group", swap(group, "  group: release-${{ github.ref\n")),
            (
                "a group whose }} is in a string",
                swap(group, "  group: release-${{ github.ref == '}}'\n"),
            ),
            ("an unclosed step", swap("run: echo publish", "run: echo ${{ github.sha")),
            ("an unclosed name", swap("name: release\n", "name: release ${{\n")),
            ("an unclosed env", swap("\njobs:\n", "\nenv:\n  A: ${{ github.sha\njobs:\n")),
            (
                "an expression in a tag filter",
                swap("tags: [v1]\n", "tags:\n      - ${{ github.ref }}\n"),
            ),
            ("an expression in a name", swap("name: release\n", "name: ${{ github.ref }}\n")),
            (
                "a step's context in the root env",
                swap("\njobs:\n", "\nenv:\n  A: ${{ steps.a.outputs.b }}\njobs:\n"),
            ),
        ):
            members.append(("expressions", label, text, True))
        for label, text in (
            ("index syntax", swap(group, "  group: release-${{ github['ref'] }}\n")),
            ("no spaces", swap(group, "  group: release-${{github.ref}}\n")),
            ("text after the ref", swap(group, "  group: release-${{ github.ref }}-run\n")),
            ("a closing brace pair in a quoted step", swap("run: echo publish", "run: echo '}}'")),
            (
                "the root env reading github",
                swap("\njobs:\n", "\nenv:\n  A: ${{ github.sha }}\njobs:\n"),
            ),
        ):
            members.append(("expressions", label, text, False))
        wrong = {
            "a sequence": "\n    - x",
            "a scalar": " x",
            "an undefined key": "\n    Types: [x]",
        }
        for event in ON_EVENTS:
            anchor = "on:\n  create:\n" if event == "push" else tag
            for label, value in wrong.items():
                text = swap(tag, anchor + f"  {event}:{value}\n")
                members.append(("schema", f"on.{event} as {label}", text, True))
            safe = event != "schedule"
            text = swap(tag, anchor + f"  {event}:\n")
            members.append(("schema", f"on.{event} as null", text, not safe))
        for label, old, new in (
            ("root permissions Contents", "  contents: read\n", "  Contents: read\n"),
            ("root permissions admin", "  contents: read\n", "  contents: admin\n"),
            ("root permissions as a sequence", "  contents: read\n", "  - contents\n"),
            ("root defaults Run", "\njobs:\n", "\ndefaults:\n  Run:\n    shell: bash\njobs:\n"),
            ("root env as a sequence", "\njobs:\n", "\nenv:\n  - A\njobs:\n"),
            ("a name that is a mapping", "name: release\n", "name:\n  a: b\n"),
            ("a group that is a mapping", group, "  group:\n    a: b\n"),
            (
                "a dispatch input Inputs",
                tag,
                tag + "  workflow_dispatch:\n    Inputs:\n      a:\n        type: string\n",
            ),
            (
                "a dispatch input of type text",
                tag,
                tag + "  workflow_dispatch:\n    inputs:\n      a:\n        type: text\n",
            ),
            (
                "a call input with no type",
                tag,
                tag + "  workflow_call:\n    inputs:\n      a:\n        required: true\n",
            ),
            ("a job timeout that is text", "timeout-minutes: 5", "timeout-minutes: '5'"),
            (
                "a job's continue-on-error that is text",
                "timeout-minutes: 5",
                "timeout-minutes: 5\n    continue-on-error: 'false'",
            ),
            (
                "a job's permissions Contents",
                "timeout-minutes: 5",
                "timeout-minutes: 5\n    permissions:\n      Contents: read",
            ),
            ("a job with no runs-on", "    runs-on: ubuntu-24.04\n", ""),
            (
                "a job's needs as a mapping",
                "timeout-minutes: 5",
                "timeout-minutes: 5\n    needs:\n      a: b",
            ),
        ):
            members.append(("schema", label, swap(old, new), True))
        for label, old, new in (
            (
                "root permissions read-all",
                "permissions:\n  contents: read\n",
                "permissions: read-all\n",
            ),
            (
                "a job timeout read from an input",
                "timeout-minutes: 5",
                "timeout-minutes: ${{ inputs.minutes }}",
            ),
            (
                "a job's env reading a secret",
                "timeout-minutes: 5",
                "timeout-minutes: 5\n    env:\n      A: ${{ secrets.A }}",
            ),
        ):
            members.append(("schema", label, swap(old, new), False))
        members = [
            (axis, label, {"release.yml": text}, harmful) for axis, label, text, harmful in members
        ]

        def callee(name, body_jobs, group=None, queue=""):
            block = (
                f"concurrency:\n  group: {group}\n  cancel-in-progress: false\n{queue}"
                if group
                else ""
            )
            return f"name: {name}\n\non:\n  workflow_call:\n\npermissions:\n  contents: read\n\n{block}\njobs:\n{body_jobs}"

        steps = "  work:\n    runs-on: ubuntu-24.04\n    timeout-minutes: 5\n    steps:\n      - run: echo work\n"
        remote = "octo-org/tools/.github/workflows/publish.yml@" + "0" * 40

        def call(uses, extra=""):
            return f"  call:\n    uses: {uses}\n{extra}"

        def calling(uses, extra=""):
            return swap("\njobs:\n", "\njobs:\n" + call(uses, extra))

        for depth in (1, 2, 3):
            chain = [f"stage{n}.yml" for n in range(1, depth + 1)]
            for end, harmful in (
                ("steps", False),
                ("remote", True),
                ("missing", True),
                ("cycle", True),
            ):
                files = {"release.yml": calling(f"./.github/workflows/{chain[0]}")}
                for n, name in enumerate(chain):
                    if n + 1 < depth:
                        jobs = call(f"./.github/workflows/{chain[n + 1]}")
                    else:
                        jobs = {
                            "steps": steps,
                            "remote": call(remote),
                            "missing": call("./.github/workflows/gone.yml"),
                            "cycle": call(f"./.github/workflows/{chain[0]}"),
                        }[end]
                    group = f"{name[:-4]}-${{{{ github.ref }}}}"
                    files[name] = callee(name[:-4], jobs, group, "  queue: max\n")
                members.append(("callees", f"depth {depth} ending in {end}", files, harmful))
        stage = callee("stage", steps, "stage-${{ github.ref }}")
        for label, files, harmful in (
            ("a remote call", {"release.yml": calling(remote)}, True),
            (
                "this repository pinned by sha",
                {
                    "release.yml": calling(
                        "RexRenatus/deck-streak/.github/workflows/stage.yml@" + "0" * 40
                    ),
                    "stage.yml": stage,
                },
                True,
            ),
            (
                "a callee without workflow_call",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": stage.replace("workflow_call", "workflow_dispatch"),
                },
                True,
            ),
            (
                "a callee called three times with the default queue",
                {
                    "release.yml": calling("./.github/workflows/stage.yml").replace(
                        "  call:\n",
                        "  one:\n    uses: ./.github/workflows/stage.yml\n  two:\n    uses: ./.github/workflows/stage.yml\n  call:\n",
                    ),
                    "stage.yml": stage,
                },
                True,
            ),
            (
                "a callee a reacher calls too, with the default queue",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": stage,
                    "publish.yml": second_workflow(
                        "on:\n  workflow_dispatch:\n", "publish-${{ github.ref }}"
                    ).replace("\njobs:\n", "\njobs:\n" + call("./.github/workflows/stage.yml")),
                },
                True,
            ),
            (
                "a callee called twice with the default queue",
                {
                    "release.yml": calling("./.github/workflows/stage.yml").replace(
                        "  call:\n", "  again:\n    uses: ./.github/workflows/stage.yml\n  call:\n"
                    ),
                    "stage.yml": stage,
                },
                True,
            ),
            (
                "a callee called twice that queues",
                {
                    "release.yml": calling("./.github/workflows/stage.yml").replace(
                        "  call:\n", "  again:\n    uses: ./.github/workflows/stage.yml\n  call:\n"
                    ),
                    "stage.yml": callee(
                        "stage", steps, "stage-${{ github.ref }}", "  queue: max\n"
                    ),
                },
                False,
            ),
            (
                "a callee under a matrix with the default queue",
                {
                    "release.yml": calling(
                        "./.github/workflows/stage.yml",
                        "    strategy:\n      matrix:\n        a: [1, 2, 3]\n",
                    ),
                    "stage.yml": stage,
                },
                True,
            ),
            (
                "a callee whose group renders as its caller's",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": callee("stage", steps, "${{ github.workflow }}-${{ github.ref }}"),
                },
                True,
            ),
            (
                "a callee that cancels",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": stage.replace(
                        "cancel-in-progress: false", "cancel-in-progress: true"
                    ),
                },
                True,
            ),
            (
                "a callee called once with the default queue",
                {"release.yml": calling("./.github/workflows/stage.yml"), "stage.yml": stage},
                True,
            ),
            (
                "a callee with no block",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": callee("stage", steps),
                },
                False,
            ),
            (
                "a reacher calling a remote workflow",
                {
                    "release.yml": release,
                    "publish.yml": second_workflow(
                        "on:\n  workflow_dispatch:\n", "publish-${{ github.ref }}", "release"
                    ).replace("\njobs:\n", "\njobs:\n" + call(remote)),
                },
                True,
            ),
            (
                "a callee holding a key GitHub's parser does not define",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": callee(
                        "stage",
                        steps.replace("timeout-minutes", "Timeout-minutes"),
                        "stage-${{ github.ref }}",
                        "  queue: max\n",
                    ),
                },
                True,
            ),
            (
                "a callee two calls deep holding a key GitHub's parser does not define",
                {
                    "release.yml": calling("./.github/workflows/stage.yml"),
                    "stage.yml": callee("stage", call("./.github/workflows/deep.yml")),
                    "deep.yml": callee("deep", steps.replace("runs-on", "Runs-on")),
                },
                True,
            ),
        ):
            members.append(("callees", label, files, harmful))
        # A release block's own shape: a group that splits one tag's runs, and a job's own block.
        release_group = "  group: release-${{ github.ref }}\n"
        for read in (
            "github.run_id",
            "github['run_id']",
            "github.run_number",
            "github.run_attempt",
        ):
            text = swap(
                release_group, f"  group: release-${{{{ github.ref }}}}-${{{{ {read} }}}}\n"
            )
            members.append(("blocks", f"a group reading {read}", {"release.yml": text}, True))
        text = swap(release_group, "  group: release-${{ github.triggering_actor }}\n")
        members.append(("blocks", "a group reading the actor", {"release.yml": text}, True))
        for label, block in (
            ("a job's own block as a group", "    concurrency: job-${{ github.ref }}\n"),
            (
                "a job's own block as a mapping",
                "    concurrency:\n      group: job-${{ github.ref }}\n",
            ),
        ):
            text = swap("  publish:\n    runs-on", "  publish:\n" + block + "    runs-on")
            members.append(("blocks", label, {"release.yml": text}, True))
        wf = "${{ github.workflow }}-${{ github.ref }}"
        events = (
            "workflow_dispatch",
            "schedule",
            "pull_request",
            "workflow_run",
            "repository_dispatch",
        )
        events_on = {event: f"on:\n  {event}:\n" for event in events}
        events_on["schedule"] = "on:\n  schedule:\n    - cron: '0 0 * * *'\n"
        for event, on in events_on.items():
            for name, grp, harmful in (
                ("publish", "release-${{ github.ref }}", True),
                ("publish", "release-refs/tags/v1", True),
                ("publish", "RELEASE-refs/tags/v1", True),
                ("release", wf, True),
                ("Release", wf, True),
                ("publish", "${{ inputs.group }}", True),
                ("publish", "${{ vars.GROUP }}-x", True),
                ("publish", "publish-${{ github.ref }}", False),
                ("publish", wf, False),
            ):
                for place in ("the workflow", "a job"):
                    text = other_block(on, "concurrency", f"\n  group: {grp}", place, name)
                    files = {"release.yml": release, "publish.yml": text}
                    members.append(
                        ("collisions", f"{place} {name} {grp!r} on {event}", files, harmful)
                    )
        for label, queue, harmful in (
            ("queues", "  queue: max\n", False),
            ("keeps one waiting", "", True),
        ):
            other = second_workflow(tag, "publish-${{ github.ref }}", "publish", queue)
            members.append(
                (
                    "census",
                    f"a second tag workflow that {label}",
                    {"release.yml": release, "publish.yml": other},
                    harmful,
                )
            )
        # A value YAML refuses to start a plain scalar with (a c-indicator it reserves or gives to
        # flow collections, and `-`, `?` or `:` before a space on a mapping's value) and a flow list
        # with an empty entry, at every scalar position of the release workflow, a second tag
        # workflow and a called workflow: GitHub refuses the file, so the class rule refuses it
        # (SPEC-190 R12 part 1).
        homes = {
            "release.yml": ({}, release),
            "publish.yml": (
                {"release.yml": release},
                second_workflow(tag, "publish-${{ github.ref }}", "publish", "  queue: max\n"),
            ),
            "stage.yml": (
                {"release.yml": calling("./.github/workflows/stage.yml")},
                callee("stage", steps, "stage-${{ github.ref }}", "  queue: max\n"),
            ),
        }
        for home, (beside, text) in homes.items():
            members.append(("grammar", f"{home} as it is", dict(beside, **{home: text}), False))
            lines = text.split("\n")
            for n, line in enumerate(lines):
                entry = re.fullmatch(r"( *(?:- )?[\w.-]+: )(.+)", line)
                item = re.fullmatch(r"( *- )(.+)", line)
                starts = ["@", "`", "%", ",", "]", "}"]
                if entry:
                    (prefix, value), starts = entry.groups(), starts + ["- ", "? ", ": "]
                elif item:
                    prefix, value = item.groups()
                else:
                    continue
                for start in starts:
                    planted = "\n".join(lines[:n] + [prefix + start + value] + lines[n + 1 :])
                    label = f"{home} line {n + 1} starting {start!r}"
                    members.append(("grammar", label, dict(beside, **{home: planted}), True))
            for flow in ("[v1,,v2]", "[, v1]", "[v1, ,]", "[,]"):
                if "tags: [v1]" in text:
                    planted = text.replace("tags: [v1]", f"tags: {flow}", 1)
                    label = f"{home} tags {flow}"
                    members.append(("grammar", label, dict(beside, **{home: planted}), True))
        judged = examined("workflow sets GitHub parses", members)
        wrong = []
        for axis, label, files, harmful in judged:
            problems = release_class_problems(files)
            if harmful and not problems:
                wrong.append(f"{axis}: {label} is accepted")
            if not harmful and problems:
                wrong.append(f"{axis}: {label} is refused: {problems[:2]}")
        self.assertEqual(wrong, [])


if __name__ == "__main__":
    unittest.main()
