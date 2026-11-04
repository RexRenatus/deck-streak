"""Pull requests into dev merge through the Mergify merge queue, six to a batch, each by a merge
commit, and the `fragment` check reads a queue batch of dependency updates alone as it reads such a
Dependabot pull request (SPEC-604 R1 to R8).

`.mergify.yml` is read by a small reader for the file's own block form (no YAML library), and its
conditions are judged by a small evaluator over planted pull request states. The `fragment` cases run
the job's own step from `changelog.yml` under `bash -e`, as the runner does, in a planted repository
whose base commit and head commit stand for a pull request, the step's env taken from the event
payload of the scenario plus the runner's own `GITHUB_HEAD_REF`. Nothing here runs GitHub or Mergify.
"""

import copy
import json
import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

from _support import REPO, examined

QUEUE_FILE = REPO / ".mergify.yml"
# The six places Mergify reads its configuration from; only one file may exist among them.
LOCATIONS = (
    ".mergify.yml",
    ".mergify.yaml",
    ".mergify/config.yml",
    ".mergify/config.yaml",
    ".github/mergify.yml",
    ".github/mergify.yaml",
)
RULESET = REPO / ".github" / "rulesets" / "dev.json"
CHANGELOG = REPO / ".github" / "workflows" / "changelog.yml"
# Each required check and the workflow whose job of that name produces it.
WORKFLOWS = {
    "ci": REPO / ".github" / "workflows" / "ci.yml",
    "fragment": CHANGELOG,
    "dependency-review": REPO / ".github" / "workflows" / "dependency-review.yml",
}
CHECKS = tuple(WORKFLOWS)
BASE_SHA = "github.event.pull_request.base.sha"
AUTHOR = "github.event.pull_request.user.login"
# The login a queue batch is opened by, and its head branch: the documented prefix plus ten hex
# characters.
QUEUE_LOGIN = "mergify[bot]"
QUEUE_PREFIX = "mergify/merge-queue/"
QUEUE_REF = QUEUE_PREFIX + "5f" * 5
# The fixed bound on how long the queue waits for the checks of a batch.
CHECKS_TIMEOUT = "360 minutes"
REFUSED = "Add a fragment under changelog.d/ (see changelog.d/README.md).\n"
PASSED = "A Dependabot pull request that changes only dependency files needs no fragment.\n"

# Property names the published Mergify configuration schema lists for each object this file uses,
# copied from mergify-configuration-schema.json, sha256
# 7a41ae7c44c579dc69a807d704db8ff1a30808b65d0c7d405ec59cb6136aa148.
SCHEMA_TOP = {
    "commands_restrictions",
    "conflict_resolvers",
    "defaults",
    "extends",
    "merge_protections",
    "merge_protections_settings",
    "merge_queue",
    "priority_rules",
    "pull_request_rules",
    "queue_rules",
    "scopes",
    "shared",
}
SCHEMA_QUEUE_RULE = {
    "allow_inplace_checks",
    "allow_queue_branch_edit",
    "batch_max_failure_resolution_attempts",
    "batch_max_wait_time",
    "batch_size",
    "branch_protection_injection_mode",
    "checks_timeout",
    "commit_message_format",
    "commit_message_template",
    "draft_bot_account",
    "max_checks_retries",
    "merge_bot_account",
    "merge_conditions",
    "merge_method",
    "name",
    "queue_branch_merge_method",
    "queue_branch_prefix",
    "queue_conditions",
    "update_bot_account",
    "update_method",
}
SCHEMA_PULL_REQUEST_RULE = {"actions", "conditions", "description", "disabled", "name"}
SCHEMA_ACTIONS = {
    "assign",
    "backport",
    "close",
    "comment",
    "copy",
    "delete_head_branch",
    "dismiss_reviews",
    "edit",
    "github_actions",
    "label",
    "merge",
    "post_check",
    "queue",
    "rebase",
    "request_reviews",
    "review",
    "squash",
    "update",
}
SCHEMA_QUEUE_ACTION = {"name"}
SCHEMA_MERGE_QUEUE = {
    "batch_pull_request",
    "checking_label",
    "dequeued_label",
    "max_parallel_checks",
    "mode",
    "queue_controls_comment",
    "queued_label",
    "reset_on_external_merge",
    "resolve_conflicts",
    "skip_intermediate_results",
    "status_comments",
}

IDENTITY = (
    "-c",
    "user.name=fixture",
    "-c",
    "user.email=fixture" + "@" + "example.invalid",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "maintenance.auto=false",
)
# A planted tree: each kind of file an ecosystem updates, a source file, the changelog and the
# README of the fragments.
TREE = {
    "README.md": "planted\n",
    "CHANGELOG.md": "# Changelog\n",
    "changelog.d/README.md": "fragments\n",
    "Cargo.toml": "[workspace]\n",
    "Cargo.lock": "version = 4\n",
    "crates/kernel/Cargo.toml": "[package]\n",
    "crates/kernel/src/lib.rs": "pub fn kernel() {}\n",
    "package.json": "{}\n",
    "pnpm-lock.yaml": "lockfileVersion: 9\n",
    "web/app/package.json": "{}\n",
    "vendor/deepmerge/package.json": "{}\n",
    ".github/workflows/ci.yml": "name: ci\n",
}


# The reader of the queue file's block form.

KEY = re.compile(r"([A-Za-z_][A-Za-z0-9_-]*):(?: (.*))?")


def _scalar(text, number):
    if text.startswith('"'):
        body = text[1:-1]
        if len(text) < 2 or not text.endswith('"') or '"' in body or "\\" in body:
            raise AssertionError(f".mergify.yml line {number}: a quoted form the reader refuses")
        return body
    if text[0] in "'{[&*!|>%@`" or " #" in text:
        raise AssertionError(f".mergify.yml line {number}: a form the reader refuses")
    if re.fullmatch(r"-?[0-9]+", text):
        return int(text)
    return {"true": True, "false": False, "null": None}.get(text, text)


def _block(lines, i, indent):
    if lines[i][1] == "-" or lines[i][1].startswith("- "):
        return _sequence(lines, i, indent)
    return _mapping(lines, i, indent)


def _mapping(lines, i, indent):
    node = {}
    while i < len(lines) and lines[i][0] == indent and not lines[i][1].startswith("- "):
        found = KEY.fullmatch(lines[i][1])
        if found is None:
            raise AssertionError(f".mergify.yml line {lines[i][2]}: not a key the reader knows")
        key, value = found.group(1), found.group(2)
        if key in node:
            raise AssertionError(f".mergify.yml line {lines[i][2]}: {key} twice")
        i += 1
        if value:
            node[key] = _scalar(value, lines[i - 1][2])
        elif i < len(lines) and lines[i][0] > indent:
            node[key], i = _block(lines, i, lines[i][0])
        elif i < len(lines) and lines[i][0] == indent and lines[i][1].startswith("- "):
            node[key], i = _sequence(lines, i, indent)
        else:
            node[key] = None
    if i < len(lines) and lines[i][0] > indent:
        raise AssertionError(f".mergify.yml line {lines[i][2]}: an indent the reader refuses")
    return node, i


def _sequence(lines, i, indent):
    items = []
    while i < len(lines) and lines[i][0] == indent and lines[i][1].startswith("- "):
        rest = lines[i][1][2:].strip()
        if KEY.fullmatch(rest):
            lines[i] = (indent + 2, rest, lines[i][2])
            item, i = _mapping(lines, i, indent + 2)
        else:
            item = _scalar(rest, lines[i][2])
            i += 1
        items.append(item)
    return items, i


def read_queue_file(text):
    """The queue file's block form as dicts, lists and scalars; a form outside it is refused."""
    lines = []
    for number, raw in enumerate(text.split("\n"), 1):
        if not raw.strip() or raw.lstrip().startswith("#"):
            continue
        if "\t" in raw:
            raise AssertionError(f".mergify.yml line {number}: a tab")
        lines.append((len(raw) - len(raw.lstrip(" ")), raw.strip(), number))
    if not lines:
        return {}
    node, end = _block(lines, 0, 0)
    if end != len(lines) or not isinstance(node, dict):
        raise AssertionError(".mergify.yml: lines the reader could not place")
    return node


def queue_file():
    """The queue file as read, or an empty one where none exists, so a test judges its values."""
    if not QUEUE_FILE.is_file():
        return {}
    return read_queue_file(QUEUE_FILE.read_bytes().decode("utf-8"))


def queue_rules(config):
    return [rule for rule in config.get("queue_rules") or [] if isinstance(rule, dict)]


def entry_rules(config):
    """Each pull request rule whose actions hold the queue action."""
    rules = [rule for rule in config.get("pull_request_rules") or [] if isinstance(rule, dict)]
    return [rule for rule in rules if "queue" in (rule.get("actions") or {})]


# The evaluator of the conditions this file uses, over a planted pull request state.

CONDITION = re.compile(r"(-)?(#)?([a-z][a-z-]*)(?: (=|!=|>=|<=) (\S+))?")


def holds(condition, state):
    found = CONDITION.fullmatch(str(condition))
    if found is None:
        raise AssertionError(f"a condition the evaluator refuses: {condition}")
    negated, count, attribute, operator, operand = found.groups()
    if attribute not in state:
        raise AssertionError(f"{condition} reads {attribute}, which no scenario models")
    value = state[attribute]
    if count:
        value = len(value)
        operand = None if operand is None else int(operand)
    if operator is None:
        result = bool(value)
    elif isinstance(value, list):
        result = any(item == operand for item in value)
        result = result if operator == "=" else not result
    else:
        result = {
            "=": value == operand,
            "!=": value != operand,
            ">=": value >= operand,
            "<=": value <= operand,
        }[operator]
    return not result if negated else result


def all_hold(conditions, state):
    return bool(conditions) and all(holds(condition, state) for condition in conditions)


def enters(config, state):
    """Whether a pull request in `state` enters the queue: a rule with the queue action matches it
    and the queue rule it names accepts it."""
    rules = {rule.get("name"): rule for rule in queue_rules(config)}
    for entry in entry_rules(config):
        target = rules.get((entry["actions"]["queue"] or {}).get("name"))
        if target and all_hold(entry.get("conditions") or [], state):
            if all_hold(target.get("queue_conditions") or [], state):
                return True
    return False


def merges(config, batch, members):
    """Whether a batch pull request in `batch`, holding pull requests in `members`, meets the merge
    conditions of the one queue rule. A condition on a check reads the batch pull request and every
    other condition reads each member, as Mergify's configuration file format page states for a
    draft pull request."""
    rules = queue_rules(config)
    conditions = (rules[0].get("merge_conditions") or []) if len(rules) == 1 else []
    on_batch = [c for c in conditions if attribute(c).startswith("check-")]
    on_members = [c for c in conditions if c not in on_batch]
    return (
        bool(conditions)
        and bool(members)
        and all(holds(c, batch) for c in on_batch)
        and all(holds(c, member) for member in members for c in on_members)
    )


def attribute(condition):
    found = CONDITION.fullmatch(str(condition))
    if found is None:
        raise AssertionError(f"a condition the evaluator refuses: {condition}")
    return found.group(3)


def direct_merge_blockers(config):
    """Each documented reason the queue would test a lone queued pull request that already holds
    the tip of dev on a batch pull request rather than merge it directly (Mergify's Direct Merge
    page): a two-step queue, whose merge conditions differ from its queue conditions; the
    `merge-batch` method; and in-place checks, where no batch pull request exists to skip."""
    rules = queue_rules(config)
    rule = rules[0] if len(rules) == 1 else {}
    merged = rule.get("merge_conditions") or []
    blockers = []
    if not rule:
        blockers.append("no single queue rule")
    if merged and merged != rule.get("queue_conditions"):
        blockers.append("two-step: merge_conditions differ from queue_conditions")
    if rule.get("merge_method") == "merge-batch":
        blockers.append("merge_method merge-batch")
    parallel = (config.get("merge_queue") or {}).get("max_parallel_checks", 5)
    if rule.get("batch_size", 1) == 1 and parallel == 1:
        blockers.append("in-place checks: no batch pull request")
    return blockers


READY = {
    "base": "dev",
    "label": ["queue"],
    "draft": False,
    "review-threads-unresolved": [],
    "check-success": list(CHECKS),
}
BATCH = {
    "base": "dev",
    "label": [],
    "draft": True,
    "review-threads-unresolved": [],
    "check-success": list(CHECKS),
}


def ruleset_gaps(ruleset, config):
    """Each condition the queue lacks for a rule the ruleset file requires of a pull request."""
    rules = queue_rules(config)
    rule = rules[0] if len(rules) == 1 else {}
    queued = rule.get("queue_conditions") or []
    merged = rule.get("merge_conditions") or []
    gaps = []
    for item in ruleset["rules"]:
        parameters = item.get("parameters") or {}
        if item["type"] == "required_status_checks":
            for check in parameters["required_status_checks"]:
                wanted = f"check-success = {check['context']}"
                gaps += [
                    f"{where}: {wanted}"
                    for where, held in (("queue", queued), ("merge", merged))
                    if wanted not in held
                ]
        if item["type"] == "pull_request":
            if parameters.get("required_review_thread_resolution"):
                if "#review-threads-unresolved = 0" not in queued:
                    gaps.append("queue: #review-threads-unresolved = 0")
            approvals = parameters.get("required_approving_review_count", 0)
            if approvals and f"#approved-reviews-by >= {approvals}" not in queued:
                gaps.append(f"queue: #approved-reviews-by >= {approvals}")
            if rule.get("merge_method") not in parameters.get("allowed_merge_methods", []):
                gaps.append(f"merge_method {rule.get('merge_method')} not allowed")
    return gaps


# The reader of a workflow's trigger and job conditions, by line.

SKIPS = re.compile(
    r"draft|head_ref|head\.ref|\.user\.|actor|sender|mergify|merge.queue|\[bot\]", re.I
)


def top_block(lines, key):
    found, block = False, []
    for line in lines:
        if re.match(r"[^\s#]", line):
            found = line == f"{key}:"
            continue
        if found:
            block.append(line)
    return block


def flow_list(block, key):
    for index, line in enumerate(block):
        found = re.fullmatch(rf"    {key}:(?: \[(.*)\])?", line)
        if found and found.group(1) is not None:
            return [item.strip() for item in found.group(1).split(",")]
        if found:
            return [item.strip()[2:] for item in block[index + 1 :] if re.match(r"      - ", item)]
    return None


def batch_blockers(text, job):
    """Each reason a workflow would not run its job `job` on a queue batch pull request into dev."""
    lines = text.split("\n")
    blockers = []
    trigger, inside = [], False
    for line in top_block(lines, "on"):
        if re.match(r"  \S", line):
            inside = line == "  pull_request:"
        elif inside:
            trigger.append(line)
    if not trigger:
        blockers.append("no pull_request trigger")
    if "dev" not in (flow_list(trigger, "branches") or []):
        blockers.append("its branches do not name dev")
    types = flow_list(trigger, "types") or ["opened", "synchronize", "reopened"]
    if not {"opened", "synchronize", "reopened"} <= set(types):
        blockers.append(f"its types are {types}")
    blockers += [
        f"a filter: {line.strip()}"
        for line in trigger
        if re.match(r"    (paths|paths-ignore|branches-ignore):", line)
    ]
    jobs = top_block(lines, "jobs")
    if f"  {job}:" not in jobs:
        blockers.append(f"no job {job}")
    for index, line in enumerate(jobs):
        if re.match(r"    if:", line):
            condition = [line]
            for more in jobs[index + 1 :]:
                if not re.match(r"     ", more):
                    break
                condition.append(more)
            if SKIPS.search(" ".join(condition)):
                blockers.append(f"a job condition: {line.strip()}")
    return blockers


# The `fragment` step, run over a planted pull request.


def fragment_step():
    """The `fragment` job's step in changelog.yml: its env as (name, expression) pairs and its
    script, read up to the next step."""
    text = CHANGELOG.read_bytes().decode("utf-8")
    job = re.search(r"(?ms)^  fragment:\n(.*?)(?=^  [a-z-]+:\n|\Z)", text).group(1)
    names = re.findall(r"(?m)^          ([A-Z_]+): \$\{\{ ([a-z_.]+) \}\}$", job)
    block = re.search(r"(?ms)^        run: \|\n(.*?)(?=^      - |\Z)", job)
    return names, textwrap.dedent(block.group(1))


def environment(home):
    """PATH from the environment of the test, HOME at `home` and no system git configuration; no
    other variable of the run reaches the step."""
    return {"PATH": os.environ["PATH"], "HOME": str(home), "GIT_CONFIG_NOSYSTEM": "1"}


def git(root, *args):
    done = subprocess.run(
        ["git", *IDENTITY, "-C", str(root), *args],
        env=environment(root.parent),
        check=True,
        capture_output=True,
        text=True,
    )
    return done.stdout.strip()


def write(root, files):
    for name, text in files.items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")


def bumped(*paths):
    return {path: TREE[path] + "bumped\n" for path in paths}


def verdict(author, head_ref, head):
    """The run of the step over a planted pull request by `author` from the branch `head_ref`: a
    base commit that holds the planted tree, then a head commit that writes `head`."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "repo"
        root.mkdir()
        git(root, "init", "-q", "-b", "dev")
        write(root, TREE)
        git(root, "add", "-A")
        git(root, "commit", "-q", "-m", "base")
        context = {BASE_SHA: git(root, "rev-parse", "HEAD"), AUTHOR: author}
        write(root, head)
        git(root, "add", "-A")
        git(root, "commit", "-q", "--allow-empty", "-m", "head")
        names, script = fragment_step()
        step = Path(scratch) / "step.sh"
        step.write_text(script, encoding="utf-8")
        env = environment(scratch)
        for name, expression in names:
            if expression not in context:
                raise AssertionError(f"fragment reads {expression}, which no scenario models")
            env[name] = context[expression]
        env["GITHUB_HEAD_REF"] = head_ref
        return subprocess.run(
            ["bash", "-e", str(step)],
            cwd=root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )


class TheQueueFile(unittest.TestCase):
    def test_the_queue_tests_six_together_and_merges_each_by_a_merge_commit(self):
        config = queue_file()
        rules = queue_rules(config)
        self.assertEqual(len(rules), 1, ".mergify.yml holds exactly one queue rule")
        rule = rules[0]
        self.assertEqual(rule.get("batch_size"), 6, "a batch holds up to six pull requests")
        self.assertEqual(rule.get("merge_method"), "merge", "each member merges by a merge commit")
        self.assertNotIn("draft_bot_account", rule, "a batch is opened by the queue login")
        self.assertEqual(rule.get("queue_branch_prefix", QUEUE_PREFIX), QUEUE_PREFIX)
        self.assertEqual(
            (config.get("merge_queue") or {}).get("max_parallel_checks"),
            1,
            "the queue checks one batch at a time",
        )
        self.assertEqual(
            rule.get("checks_timeout"),
            CHECKS_TIMEOUT,
            "the checks of a batch have a fixed timeout, not the computed auto",
        )
        self.assertNotIn(
            "priority_rules",
            config,
            "no pull request is marked release-blocking, so no rule ranks one above another",
        )
        entries = config.get("pull_request_rules") or []
        for entry in examined("pull request rules", entries):
            self.assertEqual(
                set(entry.get("actions") or {}),
                {"queue"},
                "a rule only queues: it never comments, labels, closes, assigns or deletes",
            )

    def test_the_queue_file_is_the_only_mergify_configuration_and_names_one_queue(self):
        present = [
            name for name in examined("configuration places", LOCATIONS) if (REPO / name).is_file()
        ]
        self.assertEqual(present, [".mergify.yml"], "one configuration file, at the root")
        config = queue_file()
        names = [rule.get("name") for rule in queue_rules(config)]
        self.assertEqual(len(names), 1, "the file names one queue")
        targets = [(entry["actions"]["queue"] or {}).get("name") for entry in entry_rules(config)]
        for target in examined("queue actions", targets):
            self.assertEqual(target, names[0], "every queue action names the one queue")

    def test_every_key_the_queue_file_uses_is_one_the_published_schema_lists(self):
        config = queue_file()
        used = [("top", key) for key in config]
        settings = config.get("merge_queue") or {}
        used += [("merge queue", key) for key in examined("merge queue settings", settings)]
        for rule in queue_rules(config):
            used += [("queue rule", key) for key in rule]
            self.assertIn("name", rule)
        for rule in config.get("pull_request_rules") or []:
            used += [("pull request rule", key) for key in rule]
            self.assertTrue({"name", "conditions", "actions"} <= set(rule))
            actions = rule.get("actions") or {}
            used += [("action", key) for key in actions]
            if isinstance(actions.get("queue"), dict):
                used += [("queue action", key) for key in actions["queue"]]
        listed = {
            "top": SCHEMA_TOP,
            "merge queue": SCHEMA_MERGE_QUEUE,
            "queue rule": SCHEMA_QUEUE_RULE,
            "pull request rule": SCHEMA_PULL_REQUEST_RULE,
            "action": SCHEMA_ACTIONS,
            "queue action": SCHEMA_QUEUE_ACTION,
        }
        unknown = [
            (where, key)
            for where, key in examined("keys of .mergify.yml", used)
            if key not in listed[where]
        ]
        self.assertEqual(unknown, [], "every key is one the published schema lists")


class TheQueueConditions(unittest.TestCase):
    def test_a_pull_request_enters_the_queue_only_into_dev_labelled_ready_resolved_and_green(self):
        config = queue_file()
        self.assertTrue(enters(config, READY), "a ready, labelled, green pull request enters")
        refused = {
            "into main": {**READY, "base": "main"},
            "with no label": {**READY, "label": []},
            "with another label": {**READY, "label": ["queued"]},
            "a draft": {**READY, "draft": True},
            "with an unresolved thread": {**READY, "review-threads-unresolved": ["thread"]},
        }
        for check in CHECKS:
            green = [name for name in CHECKS if name != check]
            refused[f"without {check}"] = {**READY, "check-success": green}
        for what, state in examined("pull requests the queue refuses", refused.items()):
            with self.subTest(what):
                self.assertFalse(enters(config, state), f"a pull request {what} stays out")

    def test_a_batch_merges_only_after_ci_fragment_and_dependency_review_pass_on_it(self):
        config = queue_file()
        rules = queue_rules(config)
        rule = rules[0] if len(rules) == 1 else {}
        self.assertEqual(
            rule.get("merge_conditions"),
            rule.get("queue_conditions"),
            "the merge conditions repeat the queue conditions, so the queue is single-step",
        )
        self.assertEqual(
            direct_merge_blockers(config),
            [],
            "a lone queued pull request that holds the tip of dev merges without a batch",
        )
        members = examined("batch members", [READY] * 6)
        self.assertTrue(
            merges(config, BATCH, members), "a draft batch with the three checks green merges"
        )
        for check in examined("required checks", CHECKS):
            with self.subTest(check):
                green = [name for name in CHECKS if name != check]
                state = {**BATCH, "check-success": green}
                self.assertFalse(merges(config, state, members), f"a batch without {check} waits")
        self.assertEqual(rule.get("merge_method"), "merge")
        planted = []
        for what, change in (
            ("two-step", {"merge_conditions": [f"check-success = {c}" for c in CHECKS]}),
            ("merge-batch", {"merge_method": "merge-batch"}),
        ):
            changed = copy.deepcopy(config)
            queue_rules(changed)[0].update(change)
            planted.append((what, changed))
        for what, changed in examined("planted queue copies", planted):
            with self.subTest(what):
                self.assertNotEqual(direct_merge_blockers(changed), [], f"a {what} queue refused")

    def test_the_queue_requires_every_check_and_rule_the_dev_ruleset_file_requires(self):
        ruleset = json.loads(RULESET.read_bytes().decode("utf-8"))
        config = queue_file()
        required = [
            check["context"]
            for item in ruleset["rules"]
            if item["type"] == "required_status_checks"
            for check in item["parameters"]["required_status_checks"]
        ]
        examined("checks the ruleset file requires", required)
        self.assertEqual(ruleset_gaps(ruleset, config), [], "the queue meets the ruleset's bar")
        planted = copy.deepcopy(ruleset)
        for item in planted["rules"]:
            if item["type"] == "required_status_checks":
                item["parameters"]["required_status_checks"].append({"context": "planted-check"})
        self.assertIn("queue: check-success = planted-check", ruleset_gaps(planted, config))


class TheRequiredWorkflows(unittest.TestCase):
    def test_the_three_required_workflows_run_on_a_queue_batch_pull_request_into_dev(self):
        for check, path in examined("required workflows", WORKFLOWS.items()):
            with self.subTest(check):
                text = path.read_bytes().decode("utf-8")
                self.assertEqual(batch_blockers(text, check), [], f"{path.name} runs on a batch")
        text = CHANGELOG.read_bytes().decode("utf-8")
        filtered = text.replace(
            "    types: [opened, synchronize, reopened]\n",
            '    types: [opened, synchronize, reopened]\n    paths: ["src/**"]\n',
            1,
        )
        skipped = text.replace(
            "  fragment:\n",
            "  fragment:\n    if: ${{ !github.event.pull_request.draft }}\n",
            1,
        )
        for what, planted in examined(
            "planted workflow copies", [("paths", filtered), ("draft", skipped)]
        ):
            with self.subTest(what):
                self.assertNotEqual(planted, text)
                self.assertNotEqual(batch_blockers(planted, "fragment"), [], f"{what} refused")


class TheBatchFragment(unittest.TestCase):
    def test_a_queue_batch_whose_members_add_fragments_passes_the_fragment_check(self):
        batches = {
            "with source changes": {
                "changelog.d/one-change.md": "### Added\n- one\n",
                "changelog.d/two-change.md": "### Fixed\n- two\n",
                "crates/kernel/src/lib.rs": "pub fn kernel() -> u8 { 1 }\n",
            },
            "with a dependency update": {
                "changelog.d/three-change.md": "### Changed\n- three\n",
                **bumped("Cargo.lock"),
            },
        }
        for what, head in examined("batches whose members add fragments", batches.items()):
            with self.subTest(what):
                run = verdict(QUEUE_LOGIN, QUEUE_REF, head)
                self.assertEqual(run.returncode, 0, run.stdout + run.stderr)

    def test_a_queue_batch_of_dependency_updates_alone_needs_no_fragment(self):
        batches = [
            ("Cargo.lock",),
            ("Cargo.toml", "Cargo.lock", "crates/kernel/Cargo.toml"),
            ("package.json", "pnpm-lock.yaml"),
            ("web/app/package.json",),
            (".github/workflows/ci.yml",),
            ("Cargo.lock", "pnpm-lock.yaml", ".github/workflows/ci.yml"),
        ]
        for paths in examined("dependency-only batches", batches):
            with self.subTest(paths):
                run = verdict(QUEUE_LOGIN, QUEUE_REF, bumped(*paths))
                self.assertEqual(run.returncode, 0, run.stdout + run.stderr)
                self.assertEqual(run.stdout, PASSED)

    def test_a_queue_batch_that_changes_another_path_needs_a_fragment(self):
        batches = {
            "a source file alone": bumped("crates/kernel/src/lib.rs"),
            "a lockfile and a source file": bumped("Cargo.lock", "crates/kernel/src/lib.rs"),
            "a manifest no ecosystem updates": bumped(
                "package.json", "vendor/deepmerge/package.json"
            ),
            "the README": bumped("README.md", "Cargo.lock"),
            "nothing": {},
        }
        for what, head in examined("batches that need a fragment", batches.items()):
            with self.subTest(what):
                run = verdict(QUEUE_LOGIN, QUEUE_REF, head)
                self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
                self.assertEqual(run.stdout, REFUSED)

    def test_only_the_queue_login_on_a_queue_branch_takes_the_batch_arm(self):
        names, _ = fragment_step()
        self.assertEqual([name for name, _ in names], ["BASE", "AUTHOR"], "the step's env")
        cases = [
            ("someone", QUEUE_REF),
            ("mergify", QUEUE_REF),
            ("mergifyt", QUEUE_REF),
            ("Mergify[bot]", QUEUE_REF),
            ("mergify[bot]x", QUEUE_REF),
            ("xmergify[bot]", QUEUE_REF),
            (" mergify[bot]", QUEUE_REF),
            ("mergify-bot", QUEUE_REF),
            (QUEUE_LOGIN, "feature/one"),
            (QUEUE_LOGIN, ""),
            (QUEUE_LOGIN, "mergify/merge-queue"),
            (QUEUE_LOGIN, "mergify/merge-queue-" + "5f" * 5),
            (QUEUE_LOGIN, "mergify/other/" + "5f" * 5),
            (QUEUE_LOGIN, "Mergify/merge-queue/" + "5f" * 5),
            (QUEUE_LOGIN, "refs/heads/" + QUEUE_REF),
            (QUEUE_LOGIN, "x" + QUEUE_REF),
        ]
        for author, head_ref in examined("logins and branches outside the arm", cases):
            with self.subTest(author=author, head_ref=head_ref):
                run = verdict(author, head_ref, bumped("Cargo.lock"))
                self.assertEqual(run.returncode, 1, run.stdout + run.stderr)
                self.assertEqual(run.stdout, REFUSED)


if __name__ == "__main__":
    unittest.main()
