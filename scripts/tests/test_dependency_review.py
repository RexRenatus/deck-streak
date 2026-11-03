"""Every pull request into dev is reviewed for the dependencies it adds or changes (SPEC-038 R17
to R20, ADR-055): one workflow step runs the dependency review action on every pull request into
dev, pinned by a full commit SHA, and fails on a known advisory of any severity in every scope and
on a licence outside the allow list of deny.toml, with no condition, no allowed advisory, no exempt
package and no input outside the reviewed set. The census reads every workflow file, `.yml` and
`.yaml` alike, through the reader `test_ci_workflows.py` holds, and a planted workflow for each
weakening shows that it refuses one by name. Nothing here runs GitHub."""

import re
import tempfile
import tomllib
import unittest
from pathlib import Path

from _support import REPO, examined
from test_ci_workflows import WORKFLOWS, read_workflow, workflow_file_text, workflow_files

ACTION = "actions/dependency-review-action"
FULL_SHA = re.compile(r"[0-9a-f]{40}")
JOB = "dependency-review"
BRANCHES = ["dev"]
TYPES = ["opened", "synchronize", "reopened"]
FILTERS = ("paths", "paths-ignore", "branches-ignore")
# Every input the step sets but the licence list, each with its one value (SPEC-038 R18).
INPUTS = {
    "fail-on-severity": "low",
    "fail-on-scopes": "runtime, development, unknown",
    "license-check": "true",
    "vulnerability-check": "true",
    "show-patched-versions": "true",
}
LICENCES = "allow-licenses"
# The licence ids a planted workflow allows, so that no plant depends on what deny.toml holds.
PLANTED_LICENCES = ("MIT", "Apache-2.0 WITH LLVM-exception")
PLANTED_SHA = "0" * 40


def deny_toml_licences():
    """The licence ids deny.toml allows, in its order: the one licence list (SPEC-038 R19)."""
    manifest = tomllib.loads((REPO / "deny.toml").read_text(encoding="utf-8"))
    return examined("deny.toml licence ids", manifest["licenses"]["allow"])


def review_steps(directory):
    """Every step of every workflow in `directory` that runs the dependency review action, as
    (file name, workflow, job id, job, step), in the order the files and jobs are read."""
    found = []
    for path in workflow_files(directory):
        workflow = read_workflow(workflow_file_text(path))
        jobs = workflow.get("jobs")
        for job_id, job in (jobs if isinstance(jobs, dict) else {}).items():
            steps = job.get("steps") if isinstance(job, dict) else None
            for step in steps if isinstance(steps, list) else []:
                uses = step.get("uses") if isinstance(step, dict) else None
                if isinstance(uses, str) and uses.partition("@")[0] == ACTION:
                    found.append((path.name, workflow, job_id, job, step))
    return found


def review_problems(directory, licences):
    """What the workflows of `directory` get wrong about the dependency review, each by name; none
    when exactly one step runs it and that step meets SPEC-038 R17 to R19, `licences` being the
    licence list it must allow."""
    steps = review_steps(directory)
    if len(steps) != 1:
        return [f"{len(steps)} steps run {ACTION}, not 1"]
    name, workflow, job_id, job, step = steps[0]
    problems = []
    on = workflow.get("on")
    events = list(on) if isinstance(on, dict) else on
    if events != ["pull_request"]:
        problems.append(f"{name} runs on {events}, not pull_request alone")
    pull = on.get("pull_request") if isinstance(on, dict) else None
    pull = pull if isinstance(pull, dict) else {}
    branches, types = pull.get("branches"), pull.get("types")
    if branches != BRANCHES:
        problems.append(f"{name} runs on pull requests into {branches}, not {BRANCHES}")
    if types != TYPES:
        problems.append(f"{name} runs on the pull request types {types}, not {TYPES}")
    for key in FILTERS:
        if key in pull:
            problems.append(f"{name} filters pull requests by {key}")
    reported = job.get("name", job_id)
    if reported != JOB:
        problems.append(f"{name}: the review job reports as {reported}, not {JOB}")
    for where, holder in (("job", job), ("step", step)):
        for key in ("if", "continue-on-error"):
            if key in holder:
                problems.append(f"{name}: the review {where} sets {key}")
    ref = step["uses"].partition("@")[2]
    if not FULL_SHA.fullmatch(ref):
        problems.append(f"{name} pins the action by {ref!r}, not a full commit SHA")
    given = step.get("with")
    given = given if isinstance(given, dict) else {}
    extra = sorted(set(given) - set(INPUTS) - {LICENCES})
    if extra:
        problems.append(f"{name} sets inputs outside the reviewed set: {extra}")
    for key, value in INPUTS.items():
        if given.get(key) != value:
            problems.append(f"{name}: {key} is {given.get(key)!r}, not {value!r}")
    allowed = [each.strip() for each in str(given.get(LICENCES, "")).split(",") if each.strip()]
    if allowed != list(licences):
        problems.append(f"{name} allows {allowed}, not {list(licences)}")
    return problems


def reviewed(licences):
    """A planted workflow that meets every rule, in the shape of the repository's own."""
    listed = ", ".join(licences)
    return (
        "name: dependency-review\n"
        "on:\n"
        "  pull_request:\n"
        "    branches: [dev]\n"
        "    types: [opened, synchronize, reopened]\n"
        "permissions:\n"
        "  contents: read\n"
        "jobs:\n"
        "  dependency-review:\n"
        "    runs-on: ubuntu-24.04\n"
        "    steps:\n"
        f"      - uses: {ACTION}@{PLANTED_SHA}\n"
        "        with:\n"
        "          fail-on-severity: low\n"
        "          fail-on-scopes: runtime, development, unknown\n"
        f"          allow-licenses: {listed}\n"
        "          license-check: true\n"
        "          vulnerability-check: true\n"
        "          show-patched-versions: true\n"
    )


def edited(text, old, new):
    """`text` with its one `old` replaced by `new`; a plant whose edit does not find exactly one
    is refused by name, never written."""
    if text.count(old) != 1:
        raise AssertionError(f"a plant finds {text.count(old)} of {old!r}, not 1")
    return text.replace(old, new)


class EveryPullRequestIntoDevIsReviewed(unittest.TestCase):
    def test_the_workflows_hold_one_review_that_meets_every_rule(self):
        self.assertEqual(review_problems(WORKFLOWS, deny_toml_licences()), [])
        files = [found[0] for found in review_steps(WORKFLOWS)]
        self.assertEqual(files, ["dependency-review.yml"])

    def test_each_planted_weakening_is_refused_by_name(self):
        good = reviewed(PLANTED_LICENCES)
        pin = f"@{PLANTED_SHA}"
        into = "    branches: [dev]\n"
        types = "    types: [opened, synchronize, reopened]\n"
        steps = "    steps:\n"
        given = "        with:\n"
        severity = "          fail-on-severity: low\n"
        scopes = "          fail-on-scopes: runtime, development, unknown\n"
        allow = "          allow-licenses: MIT, Apache-2.0 WITH LLVM-exception\n"
        licence = "          license-check: true\n"
        checks = "          vulnerability-check: true\n"
        ghsas = "          allow-ghsas: GHSA-0000-0000-0000\n"
        exempt = "          allow-dependencies-licenses: left-pad\n"
        warn = "          warn-only: true\n"
        outside = "review.yml sets inputs outside the reviewed set: "
        plants = {
            "the reviewed shape": ({"review.yml": good}, []),
            "the reviewed shape saved as .yaml": ({"review.yaml": good}, []),
            "no review": (
                {"review.yml": edited(good, ACTION, "actions/other-action")},
                ["0 steps run actions/dependency-review-action, not 1"],
            ),
            "two reviews": (
                {"review.yml": good, "again.yaml": good},
                ["2 steps run actions/dependency-review-action, not 1"],
            ),
            "a tag in place of a SHA": (
                {"review.yml": edited(good, pin, "@v5")},
                ["review.yml pins the action by 'v5', not a full commit SHA"],
            ),
            "pull requests into main": (
                {"review.yml": edited(good, into, "    branches: [main]\n")},
                ["review.yml runs on pull requests into ['main'], not ['dev']"],
            ),
            "a path filter": (
                {"review.yml": edited(good, types, types + "    paths: [package.json]\n")},
                ["review.yml filters pull requests by paths"],
            ),
            "a push beside the pull request": (
                {"review.yml": edited(good, types, types + "  push:\n    branches: [dev]\n")},
                ["review.yml runs on ['pull_request', 'push'], not pull_request alone"],
            ),
            "a job of another name": (
                {"review.yml": edited(good, "  dependency-review:\n", "  review:\n")},
                ["review.yml: the review job reports as review, not dependency-review"],
            ),
            "a condition on the job": (
                {"review.yml": edited(good, steps, "    if: true\n" + steps)},
                ["review.yml: the review job sets if"],
            ),
            "a step that continues on error": (
                {"review.yml": edited(good, given, "        continue-on-error: true\n" + given)},
                ["review.yml: the review step sets continue-on-error"],
            ),
            "the runtime scope alone": (
                {"review.yml": edited(good, scopes, "          fail-on-scopes: runtime\n")},
                ["review.yml: fail-on-scopes is 'runtime', not 'runtime, development, unknown'"],
            ),
            "a severity floor": (
                {"review.yml": edited(good, severity, "          fail-on-severity: critical\n")},
                ["review.yml: fail-on-severity is 'critical', not 'low'"],
            ),
            "the licence check off": (
                {"review.yml": edited(good, licence, licence.replace("true", "false"))},
                ["review.yml: license-check is 'false', not 'true'"],
            ),
            "the advisory check off": (
                {"review.yml": edited(good, checks, checks.replace("true", "false"))},
                ["review.yml: vulnerability-check is 'false', not 'true'"],
            ),
            "an allowed advisory": (
                {"review.yml": edited(good, checks, checks + ghsas)},
                [outside + "['allow-ghsas']"],
            ),
            "a package exempt from the licence list": (
                {"review.yml": edited(good, checks, checks + exempt)},
                [outside + "['allow-dependencies-licenses']"],
            ),
            "a warning in place of a failure": (
                {"review.yml": edited(good, checks, checks + warn)},
                [outside + "['warn-only']"],
            ),
            "a licence left out": (
                {"review.yml": edited(good, allow, "          allow-licenses: MIT\n")},
                ["review.yml allows ['MIT'], not ['MIT', 'Apache-2.0 WITH LLVM-exception']"],
            ),
        }
        for label, (files, refusal) in examined("planted reviews", list(plants.items())):
            with self.subTest(label), tempfile.TemporaryDirectory() as scratch:
                for file_name, text in files.items():
                    (Path(scratch) / file_name).write_text(text, encoding="utf-8")
                self.assertEqual(review_problems(Path(scratch), PLANTED_LICENCES), refusal)
