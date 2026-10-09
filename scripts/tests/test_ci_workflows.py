"""CI runs the whole gate on hosted runners with read-only tokens and pinned actions (SPEC-002 A9),
on pull requests into dev and main and pushes to both (SPEC-030 A1), and only this repository's dev
reaches main (SPEC-034 A5 to A7). The gate runs in parallel jobs, each stage in exactly one, the
engine's slow tests in a job of their own, a cache is saved only by a push to dev or main, and every
job that compiles Rust installs the protoc Anki's engine needs (SPEC-038). No workflow reads a
secret but the default token, or checks out or fetches another repository (SPEC-034 A9 to A12), and
a `.yaml` workflow is held to the hardening rules as a `.yml` one is, the hardening tests reading
keys the way the checker does (A13). The web engine's job builds the module, holds it to its budget
and runs the browser tests over it, and the aggregate needs it (SPEC-338 A16)."""

import ast
import builtins
import collections
import contextlib
import functools
import json
import math
import os
import re
import subprocess
import sys
import tempfile
import textwrap
import tomllib
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined
from test_one_static_library import umbrella_closure

WORKFLOWS = REPO / ".github" / "workflows"
PINNED = re.compile(r"^[\w.-]+/[\w.-]+(?:/[\w.-]+)*@[0-9a-f]{40}$")
# A reusable workflow in this repository, called as GitHub reads it from the caller's own commit: the
# release class's call shape (SPEC-190 R12) and the pin census's one admitted local call, a call
# job's own `uses` (SPEC-344 R5).
LOCAL_CALL = re.compile(r"\$/\.github/workflows/([^/@\s]+)")
STAGES = re.compile(r"^STAGES_ALL=\(([^)]*)\)", re.M)
THIS_REPOSITORY = "RexRenatus/deck-streak"
# The one workflow admitted to a runner that is not a pinned Ubuntu image, and the one runner it is
# admitted to (SPEC-336, ADR-345 D4): the XCFramework job needs Apple's SDKs, which only a macOS
# runner holds. The admission is by the workflow's file name, so the same runner under any other
# name is refused, and this workflow on any other runner is refused.
ADMITTED_RUNNERS = {"xcframework.yml": "macos-26"}
# The jobs admitted to that runner by file and job (SPEC-352 R15, ADR-363): each TestFlight lane's
# `app` job archives, signs and uploads the app, which only a macOS runner can. The lane's plan
# job, a job of the same name in any other file, and the job on any other runner are refused.
ADMITTED_JOB_RUNNERS = {
    ("testflight-internal.yml", "app"): "macos-26",
    ("testflight-release.yml", "app"): "macos-26",
}
# A pull request into main, as the github context presents it; each test changes what it needs.
INTO_MAIN = {
    "github.event_name": "pull_request",
    "github.base_ref": "main",
    "github.head_ref": "dev",
    "github.event.pull_request.head.repo.full_name": THIS_REPOSITORY,
    "github.repository": THIS_REPOSITORY,
}
# The owner's layout of the gate (SPEC-038 R3): each job and the stages it runs, in order. The engine
# job, which runs the engine set beside rust, joined it by the amendment of section 8 (R14).
OWNER_LAYOUT = {
    "rust": ["fmt", "clippy", "test", "doctest", "audit-rust"],
    "engine": ["test-engine"],
    # The release job runs the workspace's tests in the shipped profile (SPEC-330).
    "release": ["test-release"],
    "web": ["web", "audit-web"],
    "hygiene": ["python", "scrub", "secrets"],
}
# What the Rust cache holds (SPEC-038 R1): the crates Cargo downloaded, and the build.
RUST_CACHE = ["~/.cargo/registry/index/", "~/.cargo/registry/cache/", "~/.cargo/git/db/", "target/"]
BROWSERS = ["~/.cache/ms-playwright"]
# The stages that compile Rust: python's among them, because a guard test builds the ingest crate
# twice (SPEC-055 A2), and test-engine, which builds the engine set's tests (R13).
COMPILES_RUST = {"clippy", "test", "doctest", "python", "test-engine", "test-release"}
# The one build tool Anki's engine needs: protoc 31.1, at the version and archive digest Anki's own
# build pins (ADR-022).
PROTOC_VERSION = "31.1"
PROTOC_SHA256 = "96553041f1a91ea0efee963cb16f462f5985b4d65365f3907414c360044d8065"
PROTOC_ARCHIVE = (
    f"https://github.com/protocolbuffers/protobuf/releases/download/v{PROTOC_VERSION}/"
    f"protoc-{PROTOC_VERSION}-linux-x86_64.zip"
)
# A `bash scripts/check.sh [stage...]` line of a step's script.
GATE_CALL = re.compile(r"(?m)^[ \t]*bash scripts/check\.sh((?:[ \t]+[a-z][a-z0-9-]*)*)[ \t]*$")
# Actions that save a cache by themselves, whatever the event (SPEC-038 R2).
CACHE_BY_THEMSELVES = (
    "Swatinem/rust-cache",
    "actions/setup-go",
    "actions/setup-java",
    "actions/setup-python",
    "astral-sh/setup-uv",
    "ruby/setup-ruby",
)


def admitted_runner(name, job=None):
    """The pattern every runner of the workflow `name` must match: a pinned Ubuntu image (SPEC-002
    A9), or, for the one workflow ADMITTED_RUNNERS names, its one runner exactly (SPEC-336 R9), or,
    for a job ADMITTED_JOB_RUNNERS names by file and job, its one runner exactly (SPEC-352 R15).
    `job` is the job whose own `runs-on` the runner is, or None for a value placed anywhere else,
    which only the file's own admission or the Ubuntu pattern can admit."""
    if name in ADMITTED_RUNNERS:
        return f"^{re.escape(ADMITTED_RUNNERS[name])}$"
    if (name, job) in ADMITTED_JOB_RUNNERS:
        return f"^{re.escape(ADMITTED_JOB_RUNNERS[name, job])}$"
    return r"^ubuntu-\d\d\.\d\d$"


def workflow_file_text(path):
    """A workflow file's text as GitHub's parser is given it: the file's bytes, decoded as UTF-8
    strictly and not translated. `Path.read_text` turns a lone carriage return into a line feed
    before the reader sees it, so no test reads a workflow file with it, and `utf-8-sig` would drop
    a byte-order mark the reader refuses by name (SPEC-190 R12)."""
    return Path(path).read_bytes().decode("utf-8")


def workflow_files(directory):
    """The workflow files of a directory: every `.yml` and `.yaml` file in it, as GitHub reads both
    (SPEC-034 R7). A directory with none is VOID, never a pass."""
    return examined(
        "workflow files",
        sorted(path for path in directory.iterdir() if path.suffix in (".yml", ".yaml")),
    )


class WorkflowsAreHardened(unittest.TestCase):
    def setUp(self):
        self.files = workflow_files(WORKFLOWS)

    def test_every_workflow_defaults_to_a_read_only_token(self):
        for path in self.files:
            permissions = read_hardened(path).get("permissions")
            self.assertEqual(
                permissions,
                {"contents": "read"},
                f"{path.name} defaults its token to {permissions}",
            )

    def test_every_action_is_pinned_by_a_full_commit_sha(self):
        uses = [
            (path.name, ref, is_call)
            for path in self.files
            for ref, is_call in marked_uses(read_hardened(path))
        ]
        for name, ref, is_call in examined("action references", uses):
            if is_call and isinstance(ref, str) and LOCAL_CALL.fullmatch(ref):
                continue
            self.assertRegex(ref, PINNED, f"{name} uses {ref}")

    def test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger(self):
        runners = []
        for path in self.files:
            workflow = read_hardened(path)
            code = re.sub(r"(?m)#.*$", "", workflow_file_text(path))
            self.assertNotIn("pull_request_target", code, path.name)
            # Every `runs-on` the workflow holds, each beside the job it is the own runner of, or
            # None when it sits anywhere else, so a nested one is judged as no job's (SPEC-352 R15).
            runners += [(path.name, job, runner) for runner, job in placed_runners(workflow)]
        for name, job, runner in examined("runs-on values", runners):
            # A list or a mapping of labels is read as its text, so the pattern refuses it by name.
            self.assertRegex(str(runner), admitted_runner(name, job), f"{name} runs on {runner}")

    def test_the_admitted_runner_is_admitted_to_its_one_workflow_only(self):
        # SPEC-336 R9: the admitted workflow passes on its runner and is refused on any other, and
        # its runner is refused under any other name (PLANTED_KEYS plants it in the control too).
        for name, admitted in examined("admitted runners", list(ADMITTED_RUNNERS.items())):
            self.assertRegex(admitted, admitted_runner(name))
            for runner in ("macos-15", "ubuntu-24.04", [admitted], "self-hosted"):
                self.assertNotRegex(str(runner), admitted_runner(name))
            for other in ("planted.yml", "ci.yml", name.replace(".yml", ".yaml"), f"x{name}"):
                self.assertNotRegex(admitted, admitted_runner(other))
        self.assertRegex("ubuntu-24.04", admitted_runner("ci.yml"))

    def test_the_macos_runner_is_admitted_to_the_lane_app_jobs_only(self):
        # SPEC-352 A22 (R15; ADR-363): the macOS image is admitted by file and job to the two
        # lanes' `app` jobs, and refused on a lane's `plan` job, on a job named `app` in another
        # file, and to a lane's `app` job on any other image. Every `runs-on` sits in a job, so the
        # runner test, judging each by its job, judges every one.
        lanes = ("testflight-internal.yml", "testflight-release.yml")
        runner = ADMITTED_RUNNERS["xcframework.yml"]
        for path in examined("workflow files", self.files):
            workflow = read_hardened(path)
            jobs = workflow.get("jobs")
            placed = [
                job["runs-on"]
                for job in (jobs if isinstance(jobs, dict) else {}).values()
                if isinstance(job, dict) and "runs-on" in job
            ]
            self.assertEqual(placed, entries(workflow, "runs-on"), path.name)
        for lane in examined("lane files", lanes):
            jobs = load(lane)["jobs"]
            self.assertEqual((jobs.get("app") or {}).get("runs-on"), runner, lane)
            self.assertNotEqual(jobs["plan"].get("runs-on"), runner, lane)
        texts = {lane: workflow_file_text(WORKFLOWS / lane) for lane in lanes}
        app, plan = f"    runs-on: {runner}\n", "    runs-on: ubuntu-24.04\n"
        internal = texts[lanes[0]]
        self.assertEqual((internal.count(app), internal.count(plan)), (1, 1))
        plants = {
            "the two lanes": (texts, None),
            "a plan job on the macOS image": (
                {lanes[0]: internal.replace(plan, app)},
                f"{lanes[0]} runs on {runner}",
            ),
            "an app job in another file": (
                {"planted.yml": internal},
                f"planted.yml runs on {runner}",
            ),
            "an app job on another image": (
                {lanes[0]: internal.replace(app, "    runs-on: macos-15\n")},
                f"{lanes[0]} runs on macos-15",
            ),
        }
        test = "test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger"
        for label, (files, refusal) in examined("planted runners", list(plants.items())):
            with self.subTest(label), tempfile.TemporaryDirectory() as scratch:
                for name, text in files.items():
                    (Path(scratch) / name).write_text(text, encoding="utf-8")
                case = WorkflowsAreHardened(test)
                case.files = workflow_files(Path(scratch))
                if refusal is None:
                    case.test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger()
                    continue
                with self.assertRaisesRegex(AssertionError, re.escape(refusal) + "$"):
                    case.test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger()

    def test_ci_runs_every_stage_of_the_local_gate(self):
        stages = STAGES.search((REPO / "scripts" / "check.sh").read_text()).group(1).split()
        ci = workflow_file_text(WORKFLOWS / "ci.yml")
        for stage in examined("gate stages", stages):
            named = rf"bash scripts/check\.sh [a-z -]*(?<![\w-]){re.escape(stage)}(?![\w-])"
            self.assertRegex(ci, named, stage)

    def test_the_aggregate_check_needs_every_job_and_always_runs(self):
        ci = workflow_file_text(WORKFLOWS / "ci.yml")
        jobs = re.findall(r"(?m)^  ([a-z-]+):\n", ci.split("\njobs:\n", 1)[1])
        aggregate = re.search(r"(?ms)^  ci:\n(.*?)(?=^  [a-z-]+:\n|\Z)", ci).group(1)
        self.assertIn("if: ${{ always() }}", aggregate)
        for job in examined("jobs", [j for j in jobs if j != "ci"]):
            self.assertIn(job, aggregate, f"the aggregate ci job does not need {job}")

    def test_no_workflow_reads_a_secret_or_checks_out_another_repository(self):
        problems, judged = secret_and_checkout_problems(WORKFLOWS)
        self.assertEqual(problems, [])
        examined("workflow expressions", judged["expressions"])
        examined("run steps", judged["run steps"])
        for where, repository in examined("checkouts", judged["checkouts"]):
            self.assertEqual(repository, THIS_REPOSITORY, where)

    def test_each_planted_secret_or_foreign_repository_is_refused_by_name(self):
        problems, _ = secret_and_checkout_problems(PLANTED / "refused")
        # The planted custom shell: it clones another repository before it runs the script.
        custom = (
            'bash -c "git clone https://github.com/example-org/other-repository.git && bash {0}"'
        )
        self.assertEqual(
            problems,
            [
                "another-repository-in-other-forms.yml:jobs.build.steps[0]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[1]: clones a repository: "
                "git -C work clone https://github.com/example-org/other-repository.git",
                "another-repository-in-other-forms.yml:jobs.build.steps[2]: points git at a URL: "
                "git fetch git@example-host:example-org/other-repository.git",
                "another-repository-in-other-forms.yml:jobs.build.steps[3]: points git at a URL: "
                "git fetch github.com:example-org/other-repository.git main",
                "another-repository-in-other-forms.yml:jobs.build.steps[4]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[5]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[6]: checks out from "
                "another server: https://example-host.example",
                "another-repository-in-other-forms.yml:jobs.build.steps[7]: checks out from "
                "another server: https://example-host.example",
                "another-repository-in-other-forms.yml:jobs.build.steps[8]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[9]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[10]: checks out "
                "example-org/other-repository, not this repository",
                "another-repository-in-other-forms.yml:jobs.build.steps[11]: checks out "
                "example-org/other-repository, not this repository",
                "checkout-of-another-repository.yml:jobs.build.steps[0]: checks out "
                "example-org/other-repository, not this repository",
                "checkout-of-another-repository.yml:jobs.build.steps[1]: checks out "
                "${{ github.event.pull_request.head.repo.full_name }}, not this repository",
                "checkout-whose-inputs-are-one-expression.yml:jobs.build.steps[0]: checks out "
                "with inputs the checker does not read",
                "checkout-whose-inputs-are-one-expression.yml:jobs.build.steps[2]: checks out "
                "with inputs the checker does not read",
                "checkout-whose-inputs-are-one-expression.yml:jobs.build.steps[3]: checks out "
                "with inputs the checker does not read",
                "clone-in-a-custom-shell.yml:defaults.run.shell: runs a shell the checker does "
                f"not read: {custom}",
                "clone-in-a-custom-shell.yml:jobs.build.defaults.run.shell: runs a shell the "
                f"checker does not read: {custom}",
                "clone-in-a-custom-shell.yml:jobs.build.steps[0].shell: runs a shell the checker "
                f"does not read: {custom}",
                "clone-in-a-custom-shell.yml:jobs.build.steps[1].parallel[0].shell: runs a shell "
                f"the checker does not read: {custom}",
                "clone-in-a-custom-shell.yml:jobs.build.steps[2].shell: runs a shell the checker "
                "does not read: BASH",
                "clone-in-a-custom-shell.yml:jobs.dynamic.defaults.run: runs a shell the checker "
                "does not read: ${{ fromJSON(needs.build.outputs.defaults) }}",
                "clone-in-a-custom-shell.yml:jobs.unread.defaults: runs a shell the checker does "
                "not read: ${{ fromJSON(vars.PLANTED_DEFAULTS) }}",
                f"clone-in-a-custom-shell.yml:defaults.run.shell: clones a repository: {custom}",
                "clone-in-a-custom-shell.yml:jobs.build.defaults.run.shell: clones a repository: "
                f"{custom}",
                "clone-in-a-custom-shell.yml:jobs.build.steps[0].shell: clones a repository: "
                f"{custom}",
                "clone-in-a-custom-shell.yml:jobs.build.steps[1].parallel[0].shell: clones a "
                f"repository: {custom}",
                "clone-of-another-repository.yml:jobs.build.steps[0]: clones a repository: "
                "git clone --depth 1 https://github.com/example-org/other-repository.git",
                "clone-of-another-repository.yml:jobs.build.steps[1]: clones a repository: "
                "gh repo clone example-org/other-repository",
                "clone-of-another-repository.yml:jobs.build.steps[2]: clones a repository: "
                "git clone https://github.com/example-org/other-repository.git",
                "clone-of-another-repository.yml:jobs.build.steps[0].name: clones a repository: "
                "git clone",
                "clone-of-another-repository.yml:jobs.build.steps[1].name: clones a repository: "
                "gh repo clone",
                "clone-of-another-repository.yml:jobs.build.steps[2].name: clones a repository: "
                "git clone after an empty env",
                "clone-outside-a-run-step.yml:env.BASH_ENV: clones a repository: "
                "$(git clone https://github.com/example-org/other-repository.git)",
                "clone-outside-a-run-step.yml:jobs.build.env.BASH_ENV: clones a repository: "
                "$(git clone https://github.com/example-org/other-repository.git)",
                "clone-outside-a-run-step.yml:jobs.build.steps[0].env.BASH_ENV: clones a "
                "repository: $(git clone https://github.com/example-org/other-repository.git)",
                "clone-outside-a-run-step.yml:jobs.build.steps[1].env.BASH_ENV: points git at a "
                "URL: $(git fetch https://github.com/example-org/other-repository.git main)",
                "environment-the-checker-does-not-read.yml:env: sets an environment the checker "
                "does not read",
                "environment-the-checker-does-not-read.yml:jobs.build.env: sets an environment the "
                "checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.build.container: runs in a "
                "container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.build.steps[0].env: sets an "
                "environment the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.contained.container.env: sets an "
                "environment the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-options.container.options: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.holding-options.container.options: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-image.container.image: runs "
                "in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.holding-image.container.image: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-ports.container.ports: runs "
                "in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.holding-ports.container.ports: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-volumes.container.volumes: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.holding-volumes.container.volumes: "
                "runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-every-property.container."
                "env: sets an environment the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-every-property.container."
                "image: runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-every-property.container."
                "options: runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-every-property.container."
                "ports: runs in a container the checker does not read",
                "environment-the-checker-does-not-read.yml:jobs.given-every-property.container."
                "volumes: runs in a container the checker does not read",
                "every-secret.yml:jobs.build.steps[0].env.CHOSEN: reads the whole secrets "
                "context, or a secret named at run time",
                "every-secret.yml:jobs.build.steps[0].run: reads the whole secrets context, or a "
                "secret named at run time",
                "fetch-of-a-url.yml:jobs.build.steps[1]: points git at a URL: git fetch "
                "https://github.com/example-org/other-repository.git main",
                "fetch-of-a-url.yml:jobs.build.steps[2]: points git at a URL: git pull --ff-only "
                "https://github.com/example-org/other-repository.git main",
                "git-configured-from-the-environment.yml:env.GIT_CONFIG_COUNT: names a git "
                "variable: GIT_CONFIG_COUNT",
                "git-configured-from-the-environment.yml:env.GIT_CONFIG_KEY_0: names a git "
                "variable: GIT_CONFIG_KEY_0",
                "git-configured-from-the-environment.yml:env.GIT_CONFIG_VALUE_0: names a git "
                "variable: GIT_CONFIG_VALUE_0",
                "git-configured-from-the-environment.yml:jobs.build.env.GIT_CONFIG_PARAMETERS: "
                "names a git variable: GIT_CONFIG_PARAMETERS",
                "git-configured-from-the-environment.yml:jobs.build.container.env.GIT_SSH_COMMAND: "
                "names a git variable: GIT_SSH_COMMAND",
                "git-configured-from-the-environment.yml:jobs.build.container.options: names a git "
                "variable: GIT_CONFIG_GLOBAL",
                "git-configured-from-the-environment.yml:jobs.build.steps[0].env.git_ssh_command: "
                "names a git variable: git_ssh_command",
                "git-configured-from-the-environment.yml:jobs.build.steps[1].run: names a git "
                "variable: GIT_ASKPASS",
                "key-the-reader-refuses.yml:line 18: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 20: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 22: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 25: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 31: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:jobs.build.steps[0].: clones a repository: git clone "
                "https://github.com/example-org/other-repository.git",
                "key-the-reader-refuses.yml:jobs.build.steps[1].: clones a repository: git clone "
                "https://github.com/example-org/other-repository.git",
                "key-the-reader-refuses.yml:jobs.build.steps[2].: clones a repository: git clone "
                "https://github.com/example-org/other-repository.git",
                "run-by-alias.yml:line 17: an anchor, alias or tag is not read",
                "run-by-alias.yml:line 19: an anchor, alias or tag is not read",
                "run-by-alias.yml:line 20: an anchor, alias or tag is not read",
                "run-by-alias.yml:line 21: an anchor, alias or tag is not read",
                "second-of-each.yml:jobs.second.steps[0].env.EITHER: reads the secret "
                "EXAMPLE_TOKEN",
                "second-of-each.yml:jobs.second.steps[0].run: reads the secret EXAMPLE_TOKEN",
                "second-of-each.yml:jobs.second.steps[1]: clones a repository: git clone "
                "https://github.com/example-org/other-repository.git",
                "secret-in-a-form-the-reader-refuses.yml:line 20: a quoted value that does not end "
                "at its closing quote is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 24: an anchor, alias or tag is not "
                "read",
                "secret-in-a-form-the-reader-refuses.yml:line 25: a flow list whose items are not "
                "plain is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 28: a flow mapping is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 32: a flow list whose items are not "
                "plain is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 33: a flow list whose items are not "
                "plain is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 34: a flow list whose items are not "
                "plain is not read",
                "secret-in-a-form-the-reader-refuses.yml:line 35: a flow list whose items are not "
                "plain is not read",
                "secret-in-a-form-the-reader-refuses.yml:jobs.build.steps[1].env: sets an "
                "environment the checker does not read",
                "secret-in-a-larger-expression.yml:jobs.build.steps[0].env.EITHER: reads the "
                "secret EXAMPLE_TOKEN",
                "secret-in-a-larger-expression.yml:jobs.build.steps[0].env.FORMATTED: reads the "
                "secret EXAMPLE_KEY",
                "secret-in-a-quoted-value.yml:line 21: a double-quoted value that holds an escape "
                "is not read",
                "secret-in-a-quoted-value.yml:line 22: a double-quoted value that holds an escape "
                "is not read",
                "secret-in-a-quoted-value.yml:jobs.build.steps[0].env.SINGLE: reads the secret "
                "EXAMPLE_TOKEN",
                "secret-in-a-quoted-value.yml:jobs.build.steps[0].env.FORMATTED: reads the secret "
                "EXAMPLE_KEY",
                "secret-in-any-spacing.yml:jobs.build.steps[0].env.SPACED: reads the secret "
                "EXAMPLE_TOKEN",
                "secret-in-any-spacing.yml:jobs.build.steps[0].env.UNSPACED: reads the secret "
                "EXAMPLE_KEY",
                "secret-in-any-spacing.yml:jobs.build.steps[0].env.WIDE: reads the secret "
                "EXAMPLE_VALUE",
                "secret-in-any-spacing.yml:jobs.build.steps[0].env.CAPITALS: reads the secret "
                "Example_Token",
                "secret-in-any-spacing.yml:jobs.build.steps[0].env.HASHED: reads the secret "
                "EXAMPLE_TOKEN",
                "secret-in-brackets.yml:jobs.build.steps[0].env.INDEXED: reads the secret "
                "EXAMPLE_TOKEN",
                "secrets-inherited.yaml:jobs.call.secrets: passes every secret to the workflow "
                "it calls",
                "steps-in-a-parallel-block.yml:jobs.build.steps[0].parallel[0]: checks out "
                "example-org/other-repository, not this repository",
                "steps-in-a-parallel-block.yml:jobs.build.steps[0].parallel[1].parallel[0]: checks "
                "out from another server: https://example-host.example",
                "steps-in-a-parallel-block.yml:jobs.build.steps[0].parallel[2]: clones a "
                "repository: git clone https://github.com/example-org/other-repository.git",
            ],
        )
        # A character outside printable ASCII, planted from a Python escape so no committed file
        # holds one: each line that holds one is refused by its line, and what YAML reads there is
        # judged as well. The checker reads a script's space as a space and its break as the end of
        # a command, so the two kinds name a clone's command apart.
        clone = "#||git clone https://github.com/example-org/other-repository.git"

        def refusal_of(character):
            """A line-break character is refused by its name, any other by the generic message."""
            if character in LINE_BREAKS:
                return LINE_BREAKS[character] + NOT_READ
            return "a character the reader does not read"

        planted = [(name, c, "false ", "planted ") for name, c in PLANTED_SPACES.items()]
        planted += [(name, c, "", "") for name, c in PLANTED_BREAKS.items()]
        for name, character, run, block in planted:
            with self.subTest(name):
                self.assertEqual(
                    planted_problems(PLANTED_CHARACTERS.replace("<C>", character)),
                    [
                        *(
                            f"planted.yml:line {n}: {refusal_of(character)}"
                            for n in (12, 13, 14, 15, 16, 17, 19, 22)
                        ),
                        "planted.yml:line 15: a flow list whose items are not plain is not read",
                        "planted.yml:line 16: a quoted value that does not end at its closing "
                        "quote is not read",
                        "planted.yml:line 19: a key that is not a plain name is not read",
                        "planted.yml:jobs.build.steps[0].env.AFTER: reads the secret EXAMPLE_TOKEN",
                        "planted.yml:jobs.build.steps[0].env.BEFORE: reads the secret "
                        "EXAMPLE_TOKEN",
                        "planted.yml:jobs.build.steps[0].env.COLON: reads the secret EXAMPLE_TOKEN",
                        f"planted.yml:jobs.build.steps[0]: clones a repository: {run}{clone}",
                        f"planted.yml:jobs.build.steps[2]: clones a repository: {block}{clone}",
                        # The clone the refused key holds is still read, as every string is.
                        "planted.yml:jobs.build.steps[1].: clones a repository: git clone "
                        "https://github.com/example-org/other-repository.git",
                    ],
                )
        # A block's end and its indent read only a space or a tab as white space: a line of one of
        # the characters above is refused by its line and is still the block's text, so the secret
        # the block names over two lines keeps the indent that line sets.
        for form, (template, line, secret) in PLANTED_BLOCK_LINES.items():
            for name, character in {**PLANTED_SPACES, **PLANTED_BREAKS}.items():
                with self.subTest(f"{form}: {name}"):
                    self.assertEqual(
                        planted_problems(template.replace("<C>", character)),
                        [
                            f"planted.yml:line {line}: {refusal_of(character)}",
                            "planted.yml:jobs.build.steps[0].run: reads the secret "
                            + secret.replace("<C>", character),
                        ],
                    )
        # A line the reader cannot place refuses the whole file at once, naming the line and why.
        for form, (template, why) in PLANTED_UNPLACED_CHARACTERS.items():
            for name, character in {**PLANTED_SPACES, **PLANTED_BREAKS}.items():
                with self.subTest(f"{form}: {name}"), self.assertRaisesRegex(AssertionError, why):
                    planted_problems(template.replace("<C>", character))
        for form, (text, why) in PLANTED_UNPLACED.items():
            with self.subTest(form), self.assertRaisesRegex(AssertionError, why):
                planted_problems(text)

    def test_the_lanes_credential_reads_are_admitted_and_no_other(self):
        # SPEC-352 A21 (R14; ADR-363): the lanes' reads are admitted by file, job and step, each
        # part in the steps that use it, and every other shape, planted at run time from the live
        # lane text, is refused by name. Each secret's name is read from the live preflight step
        # and every problem is compared with each name replaced by its role word, so no message
        # here carries one.
        lanes = ("testflight-internal.yml", "testflight-release.yml")
        parts = ("KEY", "KEYID", "ISSUER", "CERTIFICATE", "PASSWORD", "PROFILE")
        reads = {
            "preflight": set(parts),
            "sign": {"CERTIFICATE", "PASSWORD", "PROFILE"},
            "upload": {"KEY", "KEYID", "ISSUER"},
        }
        problems, judged = secret_and_checkout_problems(WORKFLOWS)
        self.assertEqual(len(problems), 0, "a live workflow reads a secret the admission refuses")
        roles = {}
        for lane in examined("lane files", lanes):
            steps = (load(lane)["jobs"].get("app") or {}).get("steps") or []
            ids = {f"{lane}:jobs.app.steps[{n}]": step.get("id") for n, step in enumerate(steps)}
            found = {}
            for where, expression in judged["expressions"]:
                variable = re.fullmatch(r"(.*\])\.env\.(\w+)", where)
                for each in secret_reads(expression) if where.startswith(f"{lane}:") else []:
                    step = ids.get(variable.group(1)) if variable else where
                    found.setdefault(step, set()).add(variable.group(2) if variable else where)
                    if step == "preflight":
                        roles[each.removeprefix("reads the secret ")] = variable.group(2)
            self.assertEqual(found, reads, lane)
        self.assertEqual(sorted(roles.values()), sorted(parts))
        secret = {role: name for name, role in roles.items()}

        def masked(found):
            return sorted(
                re.sub(
                    r"reads the secret (\S+)",
                    lambda match: f"reads the {roles.get(match.group(1), 'unadmitted')} part",
                    each,
                )
                for each in found
            )

        def value(part):
            return f"${{{{ secrets.{secret[part]} }}}}"

        lane = lanes[0]
        text = workflow_file_text(WORKFLOWS / lane)
        jobs = load(lane)["jobs"]
        at = {step.get("id"): n for n, step in enumerate(jobs["app"]["steps"])}
        planned = len(jobs["plan"]["steps"]) - 1
        every = [
            f"jobs.app.steps[{at[step]}].env.{part}: reads the {part} part"
            for step, found in reads.items()
            for part in found
        ]
        plan_run = "        run: python3 scripts/ios_lane.py plan --lane internal\n"
        call = "    uses: $/.github/workflows/xcframework.yml\n"
        upload_key = f"          KEY: {value('KEY')}\n"
        job_variable = "      LANE: ${{ needs.plan.outputs.lane }}\n"
        sign_run = "        run: python3 scripts/ios_lane.py sign\n"
        plants = {
            "the lane as it is": (lane, "", "", []),
            "a read in the plan job": (
                lane,
                plan_run,
                f"        env:\n          KEY: {value('KEY')}\n{plan_run}",
                [f"jobs.plan.steps[{planned}].env.KEY: reads the KEY part"],
            ),
            "the lane's reads in a third file": ("planted.yml", "", "", every),
            "a lane a pull request starts": (
                lane,
                "on:\n  workflow_dispatch:\n",
                "on:\n  workflow_dispatch:\n  pull_request:\n",
                every,
            ),
            "the upload step reading the certificate": (
                lane,
                upload_key,
                f"{upload_key}          CERTIFICATE: {value('CERTIFICATE')}\n",
                [f"jobs.app.steps[{at['upload']}].env.CERTIFICATE: reads the CERTIFICATE part"],
            ),
            # The base checker already refuses these two shapes: MUTATION COVERAGE.
            "every secret passed to the call": (
                lane,
                call,
                f"{call}    secrets: inherit\n",
                ["jobs.framework.secrets: passes every secret to the workflow it calls"],
            ),
            "a secret passed to the call": (
                lane,
                call,
                f"{call}    secrets:\n      KEY: {value('KEY')}\n",
                ["jobs.framework.secrets.KEY: reads the KEY part"],
            ),
            "a read in the app job's own variables": (
                lane,
                job_variable,
                f"      KEY: {value('KEY')}\n{job_variable}",
                ["jobs.app.env.KEY: reads the KEY part"],
            ),
            "a read in a script": (
                lane,
                sign_run,
                f'        run: python3 scripts/ios_lane.py sign "{value("PASSWORD")}"\n',
                [f"jobs.app.steps[{at['sign']}].run: reads the PASSWORD part"],
            ),
        }
        for label, (name, old, new, refused) in examined("planted reads", list(plants.items())):
            with self.subTest(label), tempfile.TemporaryDirectory() as scratch:
                if old:
                    self.assertEqual(text.count(old), 1)
                (Path(scratch) / name).write_text(
                    text.replace(old, new) if old else text, encoding="utf-8"
                )
                found, _judged = secret_and_checkout_problems(Path(scratch))
                self.assertEqual(masked(found), sorted(f"{name}:{each}" for each in refused))

    def test_this_repositorys_token_and_checkout_are_admitted(self):
        problems, judged = secret_and_checkout_problems(PLANTED / "admitted")
        self.assertEqual(problems, [])
        read = [text for _, text in examined("workflow expressions", judged["expressions"])]
        for token in (
            "secrets.GITHUB_TOKEN",
            "secrets.github_token",
            "secrets['GITHUB_TOKEN']",
            "github.token",
            # Not the secrets context: a step's output named `secrets`, and a longer word.
            "steps.scan.outputs.secrets",
            "hashFiles('secrets-scan.toml')",
        ):
            self.assertIn(token, read)
        examined("run steps", judged["run steps"])
        for where, repository in examined("checkouts", judged["checkouts"]):
            self.assertEqual(repository, THIS_REPOSITORY, where)

    def test_an_empty_workflow_directory_is_refused(self):
        with tempfile.TemporaryDirectory() as scratch:
            # A file that is not a workflow is not counted: the directory still holds none.
            (Path(scratch) / "README.md").write_text("Not a workflow.\n", encoding="utf-8")
            with self.assertRaisesRegex(AssertionError, "examined 0 workflow files"):
                secret_and_checkout_problems(Path(scratch))

    def test_a_yaml_workflow_is_held_to_the_same_hardening_rules(self):
        # GitHub reads a `.yaml` workflow as it reads a `.yml` one (SPEC-034 R7): each hardening
        # test above, run through its own setUp over the planted workflows, refuses the `.yaml` one
        # by its name, beside a hardened `.yml` control.
        for test in (
            "test_every_workflow_defaults_to_a_read_only_token",
            "test_every_action_is_pinned_by_a_full_commit_sha",
            "test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger",
        ):
            workflows = mock.patch.object(sys.modules[__name__], "WORKFLOWS", PLANTED_HARDENING)
            with self.subTest(test), workflows:
                case = WorkflowsAreHardened(test)
                case.setUp()
                with self.assertRaisesRegex(AssertionError, r"unhardened\.yaml"):
                    getattr(case, test)()
        # The SHA-pin test admits an action only in its plain form: the hardened control, its action
        # written with an empty part, a trailing slash or a backslash, or its SHA cut short, is
        # refused by that reference.
        control = workflow_file_text(PLANTED_HARDENING / "hardened.yml")
        pinned = CONTROL_STEP.split("uses: ", 1)[1]
        for written in (
            pinned.replace("actions/checkout@", "actions//checkout@"),
            pinned.replace("actions/checkout@", "actions/checkout/@"),
            pinned.replace("actions/checkout@", "actions\\checkout@"),
            pinned[: pinned.index("@") + 8],
        ):
            with self.subTest(written), tempfile.TemporaryDirectory() as scratch:
                planted = control.replace(pinned, written)
                (Path(scratch) / "planted.yml").write_text(planted, encoding="utf-8")
                with mock.patch.object(sys.modules[__name__], "WORKFLOWS", Path(scratch)):
                    case = WorkflowsAreHardened("test_every_action_is_pinned_by_a_full_commit_sha")
                    case.setUp()
                    with self.assertRaisesRegex(AssertionError, re.escape(f"uses {written}") + "$"):
                        case.test_every_action_is_pinned_by_a_full_commit_sha()
        # The hardening tests read keys the way the checker does (SPEC-034 R7): the control, one
        # line rewritten by each planted key, is judged beside the live workflows, and the test
        # named refuses it with the refusal named.
        live = [(path.name, path.read_bytes()) for path in workflow_files(WORKFLOWS)]
        for test, line, planted, refusal in PLANTED_KEYS:
            with self.subTest(test=test, planted=planted), tempfile.TemporaryDirectory() as scratch:
                self.assertEqual(control.count(line), 1, line)
                for name, text in live:
                    (Path(scratch) / name).write_bytes(text)
                (Path(scratch) / "planted.yml").write_text(
                    control.replace(line, planted), encoding="utf-8"
                )
                with mock.patch.object(sys.modules[__name__], "WORKFLOWS", Path(scratch)):
                    case = WorkflowsAreHardened(test)
                    case.setUp()
                    with self.assertRaisesRegex(AssertionError, refusal):
                        getattr(case, test)()
        # The walk the SHA-pin and runner tests read through collects every value a key holds,
        # wherever it sits: a `uses` nested in a `uses` is collected after the value that holds it.
        nested = read_workflow(PLANTED_JOB + "      - uses:\n          uses: actions/checkout@v4\n")
        self.assertEqual(
            entries(nested, "uses"), [{"uses": "actions/checkout@v4"}, "actions/checkout@v4"]
        )

    def test_a_call_job_is_a_local_call_or_pinned(self):
        # SPEC-344 A4: the live call jobs are exactly the two callers' calls of the one job body.
        found = []
        for path in workflow_files(WORKFLOWS):
            workflow = read_hardened(path)
            self.assertEqual(
                [ref for ref, _call in marked_uses(workflow)],
                entries(workflow, "uses"),
                f"{path.name}: the marking walk and the census walk disagree",
            )
            jobs = workflow.get("jobs")
            for job_id, job in (jobs if isinstance(jobs, dict) else {}).items():
                if isinstance(job, dict) and "uses" in job:
                    found.append((path.name, job_id, job["uses"]))
        body = "$/.github/workflows/xcframework.yml"
        # The two TestFlight lanes call the same body as their framework job (SPEC-352 R2).
        self.assertEqual(
            sorted(found),
            [
                ("apple-on-change.yml", "apple", body),
                ("apple-on-tag.yml", "apple", body),
                ("testflight-internal.yml", "framework", body),
                ("testflight-release.yml", "framework", body),
            ],
        )
        # The pin test, run by name over planted directories as the test above runs it.
        control = workflow_file_text(PLANTED_HARDENING / "hardened.yml")
        step = CONTROL_STEP.split("uses: ", 1)[1]
        call = "  call:\n    uses: "
        plants = (
            (f"{control}{call}{body}\n", None),
            (f"{control}{call}{body}@dev\n", f"{body}@dev"),
            (
                f"{control}{call}octo-org/other/.github/workflows/x.yml@main\n",
                "octo-org/other/.github/workflows/x.yml@main",
            ),
            (control.replace(step, "./.github/actions/x"), "./.github/actions/x"),
            (control.replace(step, body), body),
        )
        for planted, ref in plants:
            with self.subTest(ref), tempfile.TemporaryDirectory() as scratch:
                (Path(scratch) / "planted.yml").write_text(planted, encoding="utf-8")
                case = WorkflowsAreHardened("test_every_action_is_pinned_by_a_full_commit_sha")
                case.files = workflow_files(Path(scratch))
                if ref is None:
                    case.test_every_action_is_pinned_by_a_full_commit_sha()
                    continue
                with self.assertRaisesRegex(
                    AssertionError, re.escape(f"planted.yml uses {ref}") + "$"
                ):
                    case.test_every_action_is_pinned_by_a_full_commit_sha()


APPLE_PATHS = [
    "crates/ffi/**",
    "crates/engine-core/**",
    "ios/**",
    "Cargo.lock",
    "Cargo.toml",
    "rust-toolchain.toml",
    ".github/workflows/xcframework.yml",
    ".github/workflows/apple-on-change.yml",
]
PULL_REQUEST_ONLY = (
    "github.event.pull_request",
    "github.head_ref",
    "github.base_ref",
    "github.event.number",
    "GITHUB_HEAD_REF",
    "GITHUB_BASE_REF",
)


def pull_request_reads(text):
    """Every line of a workflow that reads a context only a pull request has."""
    return [
        line.strip()
        for line in text.splitlines()
        if not line.lstrip().startswith("#") and any(read in line for read in PULL_REQUEST_ONLY)
    ]


def cargo_commands(text):
    """Every `cargo` command of a workflow's run scripts, backslash continuations joined."""
    joined = re.sub(r"\\\n\s*", " ", text)
    return [
        match.group(1).strip()
        for match in re.finditer(
            r"(?m)^[^#\n]*?\b(cargo (?:rustc|run|build|test|check)\b[^\n]*)", joined
        )
    ]


# The one cargo command that may name a crate type: the umbrella's static library (SPEC-346 R3).
UMBRELLA_STATICLIB = "-p deck-streak-ffi --lib --crate-type staticlib"


def static_library_problems(texts):
    """What the workflows get wrong about the app's one Rust static library (SPEC-346 R3), for
    {workflow file name: its text}: a cargo command naming `--crate-type` that is not a `cargo
    rustc` of the umbrella's static library, one outside xcframework.yml, and a `--crate-type` in a
    file's non-comment lines that `cargo_commands` did not read. Returns the problems and the
    commands judged."""
    problems, judged = [], []
    for name, text in sorted(texts.items()):
        commands = [command for command in cargo_commands(text) if "--crate-type" in command]
        spelled = sum(
            line.count("--crate-type")
            for line in text.splitlines()
            if not line.lstrip().startswith("#")
        )
        seen = sum(command.count("--crate-type") for command in commands)
        if spelled != seen:
            problems.append(
                f"{name}: {spelled} `--crate-type` in its text, {seen} in the cargo commands read"
            )
        for command in commands:
            judged.append(command)
            package = re.search(r"(?:-p|--package)\s+(\S+)", command)
            built = package.group(1) if package else "no package"
            if not command.startswith("cargo rustc ") or UMBRELLA_STATICLIB not in command:
                problems.append(
                    f"{name}: `{command}` builds {built}, not the umbrella's static library"
                )
            elif name != "xcframework.yml":
                problems.append(f"{name}: `{command}` builds it outside xcframework.yml")
    return problems, judged


def watch_problems(paths, closure):
    """Where the change caller's `crates/` globs and the umbrella's build closure differ (SPEC-346
    R6): a crate the umbrella links that no glob watches, and a glob for a crate it does not link.
    `closure` is `umbrella_closure`'s directories."""
    globs = {path for path in paths if path.startswith("crates/")}
    wanted = {f"{directory}/**" for directory in closure}
    unwatched = [
        f"{pattern}: the umbrella links it, and the change caller does not watch it"
        for pattern in sorted(wanted - globs)
    ]
    unlinked = [
        f"{pattern}: the change caller watches it, and the umbrella does not link it"
        for pattern in sorted(globs - wanted)
    ]
    return unwatched + unlinked


class TheAppleBuildRunsFromOneBody(unittest.TestCase):
    """SPEC-344: one job body, xcframework.yml, called by a change caller and a tag caller."""

    def test_the_change_caller_runs_the_build_on_each_apple_path(self):
        self.assertIn("apple-on-change.yml", [p.name for p in workflow_files(WORKFLOWS)])
        caller = load("apple-on-change.yml")
        self.assertEqual(
            caller["on"],
            {
                "pull_request": {
                    "branches": ["dev"],
                    "types": ["opened", "synchronize", "reopened"],
                    "paths": APPLE_PATHS,
                }
            },
        )
        self.assertEqual(caller["jobs"], {"apple": {"uses": "$/.github/workflows/xcframework.yml"}})
        paths = caller["on"]["pull_request"]["paths"]
        runs = (
            "crates/ffi/src/lib.rs",
            "crates/engine-core/src/lib.rs",
            "ios/Harness/Info.plist",
            "Cargo.lock",
            "Cargo.toml",
            "rust-toolchain.toml",
            ".github/workflows/xcframework.yml",
        )
        idle = (
            "crates/api/src/main.rs",
            "docs/specs/x.md",
            "web/app/package.json",
            "Cargo.toml.orig",
            "deploy/host-budget.json",
        )
        for changed in examined("changed paths", runs + idle):
            self.assertEqual(
                any(path_glob(glob).fullmatch(changed) for glob in paths),
                changed in runs,
                changed,
            )

    def test_one_job_body_serves_the_change_and_the_tag(self):
        callee = load("xcframework.yml")
        self.assertEqual(sorted(callee["on"]), ["workflow_call", "workflow_dispatch"])
        self.assertNotIn("concurrency", callee)
        for job_id, job in examined("callee jobs", callee["jobs"].items()):
            self.assertEqual(job["runs-on"], ADMITTED_RUNNERS["xcframework.yml"], job_id)
        text = workflow_file_text(WORKFLOWS / "xcframework.yml")
        self.assertEqual(pull_request_reads(text), [])
        plant = text.replace(
            "    steps:\n", '    steps:\n      - run: echo "${{ github.head_ref }}"\n', 1
        )
        self.assertNotEqual(plant, text)
        self.assertEqual(pull_request_reads(plant), ['- run: echo "${{ github.head_ref }}"'])

    def test_the_apple_build_resolves_only_the_locked_graph(self):
        text = workflow_file_text(WORKFLOWS / "xcframework.yml")
        commands = examined("cargo commands", cargo_commands(text))
        self.assertGreaterEqual(len(commands), 2)
        for command in commands:
            self.assertIn("--locked", command.split(" -- ")[0], command)
        plant = text + "      - run: cargo build -p deck-streak-ffi\n"
        unlocked = [c for c in cargo_commands(plant) if "--locked" not in c.split(" -- ")[0]]
        self.assertEqual(unlocked, ["cargo build -p deck-streak-ffi"])


class TheAppHasOneRustStaticLibrary(unittest.TestCase):
    """SPEC-346: the app links one Rust static library, the umbrella FFI crate. The workflows build
    the umbrella alone as one, the Apple job proves its bindings hold one module and each slice of
    its XCFramework one library, and the change caller watches every crate the umbrella links."""

    def test_only_the_umbrella_is_built_as_a_static_library(self):
        texts = {path.name: workflow_file_text(path) for path in workflow_files(WORKFLOWS)}
        problems, judged = static_library_problems(texts)
        self.assertEqual(problems, [])
        for command in examined("cargo commands naming a crate type", judged):
            self.assertIn(UMBRELLA_STATICLIB, command)
        engine = "cargo rustc --locked -p deck-streak-engine-core --lib --crate-type staticlib"
        plant = texts["xcframework.yml"] + f"      - run: {engine}\n"
        planted, _judged = static_library_problems({**texts, "xcframework.yml": plant})
        self.assertEqual(len(planted), 1, planted)
        self.assertIn("builds deck-streak-engine-core,", planted[0])
        unread = "      - run: echo --crate-type staticlib\n"
        planted, _judged = static_library_problems({**texts, "release.yml": unread})
        self.assertEqual(
            planted, ["release.yml: 1 `--crate-type` in its text, 0 in the cargo commands read"]
        )

    def test_the_bindings_hold_one_module(self):
        steps = load("xcframework.yml")["jobs"]["xcframework"]["steps"]
        names = [step.get("name") for step in steps]
        self.assertIn("the bindings hold one module", names)
        after = names.index("the Swift bindings, in library mode over the device library")
        self.assertEqual(names.index("the bindings hold one module"), after + 1)
        modulemap = (
            'module deck_streak_ffiFFI {\n    header "deck_streak_ffiFFI.h"\n    export *\n}\n'
        )
        umbrella = {
            "bindings/deck_streak_ffi.swift": "",
            "bindings/deck_streak_ffiFFI.h": "",
            "bindings/module.modulemap": modulemap,
        }
        second_crate = {
            **umbrella,
            "bindings/deck_streak_xp.swift": "",
            "bindings/deck_streak_xpFFI.h": "",
        }
        second_module = {
            **umbrella,
            "bindings/module.modulemap": modulemap + modulemap.replace("ffiFFI", "xpFFI"),
        }
        cases = [
            ("the umbrella's three files", umbrella, (0, {"one-module": "pass"})),
            ("a second crate's bindings", second_crate, (1, {"one-module": "fail"})),
            ("a modulemap declaring two modules", second_module, (1, {"one-module": "fail"})),
        ]
        for case, plant, verdict in examined("planted bindings", cases):
            self.assertEqual(run_output_check(steps[after + 1], plant), verdict, case)

    def test_the_xcframework_holds_one_rust_library_per_slice(self):
        steps = load("xcframework.yml")["jobs"]["xcframework"]["steps"]
        names = [step.get("name") for step in steps]
        self.assertIn("the XCFramework holds one Rust library", names)
        after = names.index("the XCFramework, device and simulator slices")
        self.assertEqual(names.index("the XCFramework holds one Rust library"), after + 1)
        device = "DeckStreakFFI.xcframework/ios-arm64/libdeck_streak_ffi.a"
        simulator = "DeckStreakFFI.xcframework/ios-arm64-simulator/libdeck_streak_ffi.a"
        third = "DeckStreakFFI.xcframework/ios-arm64/libxp.a"
        other = "DeckStreakFFI.xcframework/ios-arm64-simulator/libother.a"
        failed = (1, {"one-library": "fail"})
        cases = [
            (
                "the umbrella in each slice",
                {device: "", simulator: ""},
                (0, {"one-library": "pass"}),
            ),
            ("a third library", {device: "", simulator: "", third: ""}, failed),
            ("another library in the simulator slice", {device: "", other: ""}, failed),
            ("a single slice", {device: ""}, failed),
        ]
        for case, plant, verdict in examined("planted XCFrameworks", cases):
            self.assertEqual(run_output_check(steps[after + 1], plant), verdict, case)

    def test_the_change_caller_watches_every_crate_the_umbrella_links(self):
        paths = load("apple-on-change.yml")["on"]["pull_request"]["paths"]
        closure = umbrella_closure(REPO)
        watched = sorted(path for path in paths if path.startswith("crates/"))
        self.assertEqual(
            watched,
            [f"{directory}/**" for directory in closure],
            f"the change caller watches {watched}, the closure is {closure}",
        )
        self.assertEqual(watch_problems(paths, examined("crates the umbrella links", closure)), [])
        with tempfile.TemporaryDirectory() as scratch:
            where = Path(scratch)
            graph = {
                "Cargo.toml": '[workspace]\nmembers = ["crates/*"]\n\n'
                '[workspace.dependencies]\nb = { path = "crates/b" }\n',
                "crates/ffi/Cargo.toml": '[package]\nname = "deck-streak-ffi"\n\n'
                '[dependencies]\na = { path = "../a" }\n',
                "crates/a/Cargo.toml": '[package]\nname = "a"\n\n'
                "[dependencies]\nb.workspace = true\n",
                "crates/b/Cargo.toml": '[package]\nname = "b"\n\n'
                '[dev-dependencies]\nc = { path = "../c" }\n',
                "crates/c/Cargo.toml": '[package]\nname = "c"\n',
            }
            for relative, text in graph.items():
                (where / relative).parent.mkdir(parents=True, exist_ok=True)
                (where / relative).write_text(text, encoding="utf-8")
            planted = umbrella_closure(where)
        self.assertEqual(planted, ["crates/a", "crates/b", "crates/ffi"])
        self.assertEqual(
            watch_problems(["crates/ffi/**"], planted),
            [
                "crates/a/**: the umbrella links it, and the change caller does not watch it",
                "crates/b/**: the umbrella links it, and the change caller does not watch it",
            ],
        )
        self.assertEqual(
            watch_problems(["crates/a/**", "crates/b/**", "crates/c/**", "crates/ffi/**"], planted),
            ["crates/c/**: the change caller watches it, and the umbrella does not link it"],
        )


def triggers(workflow):
    """{event: [branch, ...]} from a workflow's `on:` block, read without a YAML library. A branch
    list may be a flow list (`branches: [dev, main]`) or a block list (`- dev` lines)."""
    block = re.search(r"(?ms)^on:\n(.*?)(?=^\S|\Z)", workflow).group(1)
    found = {}
    for event, body in re.findall(r"(?m)^  ([a-z_]+):\n((?:^    .*\n?)*)", block):
        flow = re.search(r"(?m)^    branches:\s*\[([^\]]*)\]", body)
        listed = re.search(r"(?m)^    branches:\s*\n((?:^      - .*\n?)*)", body)
        if flow:
            names = flow.group(1).split(",")
        elif listed:
            names = [line.strip()[2:] for line in listed.group(1).splitlines()]
        else:
            names = []
        found[event] = [name.strip().strip("'\"") for name in names if name.strip()]
    return found


class CiRunsOnDevAndMain(unittest.TestCase):
    def test_the_ci_workflow_runs_on_pull_requests_into_dev_and_main(self):
        ci = triggers(workflow_file_text(WORKFLOWS / "ci.yml"))
        for event in examined("ci triggers", ["pull_request", "push"]):
            self.assertIn(event, ci, f"ci.yml does not run on {event}")
            for branch in ("dev", "main"):
                self.assertIn(branch, ci[event], f"ci.yml's {event} trigger leaves {branch}")
        # The reader sees a branch leave: a planted workflow whose pull_request drops main.
        planted = (
            "on:\n  pull_request:\n    branches: [dev]\n"
            "  push:\n    branches:\n      - dev\n      - main\npermissions: {}\n"
        )
        self.assertEqual(triggers(planted), {"pull_request": ["dev"], "push": ["dev", "main"]})


# The one form the aggregate job's name takes, as the file writes it (SPEC-034 R8).
AGGREGATE_NAME = "${{ github.event_name == 'pull_request' && 'ci' || 'ci (push)' }}"
NAME_FORM = re.compile(
    r"^\$\{\{ github\.event_name == '([a-z_]+)' && '([^']+)' \|\| '([^']+)' \}\}$"
)
RULESETS = REPO / ".github" / "rulesets"


def required_contexts():
    """Every context a ruleset requires, from both long-lived branches' rulesets."""
    found = set()
    for name in ("dev", "main"):
        ruleset = json.loads((RULESETS / f"{name}.json").read_text(encoding="utf-8"))
        for rule in ruleset["rules"]:
            if rule["type"] == "required_status_checks":
                found |= {c["context"] for c in rule["parameters"]["required_status_checks"]}
    return examined("required contexts", sorted(found))


def job_name_for(job_id, job, event):
    """The name a job's check run carries for `event`: its `name`, or its id when it has none. A
    name that holds an expression is read in exactly one form,
    `github.event_name == '<e>' && '<a>' || '<b>'`, and refused in any other."""
    name = str(job.get("name", job_id))
    if "${{" not in name:
        return name
    form = NAME_FORM.match(name)
    if form is None:
        raise AssertionError(f"{job_id}: a job name in a form this reader does not model: {name!r}")
    matched, then, otherwise = form.groups()
    return then if event == matched else otherwise


def judged_events(on):
    """The events a workflow's `on:` names that are not `pull_request`, in order. `on` is a scalar
    (`on: push`), a list, or a mapping keyed by event."""
    return [e for e in ([on] if isinstance(on, str) else on) if e != "pull_request"]


class TheRequiredCiCheckIsThePullRequestsOwn(unittest.TestCase):
    def test_the_required_ci_check_is_always_the_pull_requests_own_run(self):
        required = required_contexts()
        self.assertIn("ci", required)
        job = load("ci.yml")["jobs"]["ci"]
        self.assertEqual(job.get("name"), AGGREGATE_NAME, "the aggregate job's name")
        self.assertEqual(job_name_for("ci", job, "pull_request"), "ci")
        pushed = job_name_for("ci", job, "push")
        self.assertEqual(pushed, "ci (push)")
        self.assertNotIn(pushed, required)
        # The reader refuses every other form rather than guess at it.
        for other in (
            "${{ github.event_name }}",
            "${{ github.ref && 'ci' || 'x' }}",
            "${{ x }}",
            "${{ github.event_name == 'push' && '' || 'ci' }}",
            "${{ github.event_name == 'push' && 'ci' || '' }}",
        ):
            with self.assertRaises(AssertionError, msg=other):
                job_name_for("ci", {"name": other}, "push")
        self.assertEqual(job_name_for("a", {}, "push"), "a")

    def test_the_events_a_workflow_is_judged_under_are_read_in_every_form(self):
        self.assertEqual(judged_events("push"), ["push"])
        self.assertEqual(judged_events(["pull_request", "push", "schedule"]), ["push", "schedule"])
        self.assertEqual(
            judged_events({"pull_request": {}, "push": {}, "workflow_dispatch": None}),
            ["push", "workflow_dispatch"],
        )
        self.assertEqual(judged_events("pull_request"), [])
        self.assertEqual(judged_events({"pull_request": {"branches": ["dev"]}}), [])

    def test_no_push_run_reports_under_a_required_name(self):
        required = set(required_contexts())
        judged = []
        for path in workflow_files(WORKFLOWS):
            workflow = read_hardened(path)
            on = workflow["on"]
            for event in judged_events(on):
                for job_id, job in workflow["jobs"].items():
                    name = job_name_for(job_id, job, event)
                    judged.append((path.name, event, name))
                    self.assertNotIn(name, required, f"{path.name}: {event} reports {name}")
        self.assertIn(("ci.yml", "push", "ci (push)"), judged)
        examined("job names under a non-pull_request event", judged)


def run_base_is_dev(context):
    """Run base-is-dev's own step from ci.yml under `bash -e`, its env taken from `context`."""
    ci = workflow_file_text(WORKFLOWS / "ci.yml")
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


class TheReaderReadsOnlyItsNamedForms(unittest.TestCase):
    def test_the_reader_reads_only_its_named_forms(self):
        """SPEC-190 R12 part 1's named forms, generated from YAML 1.2.2's own constants: every
        printable ASCII character first in a plain value, a sequence item and a flow item, each
        quoted as its control; every block-scalar header YAML defines, with and without text; flow
        lists with an empty entry at every place and with one trailing comma; each key the core
        schema types; and a comment after each one-line form, a comment line and a blank line. A
        named form reads as YAML reads it; every other form is refused by its line, the one refusal
        naming the form it met."""
        # c-indicator (YAML 1.2.2 5.3): no plain scalar starts with one, but `-`, `?` and `:` may
        # before a character that is not white space (ns-plain-first).
        indicators = "-?:,[]{}#&*!|>'\"%@`"
        printable = [chr(code) for code in range(0x21, 0x7F)]
        unnamed = "is not a form the reader reads"

        def outcome(text):
            try:
                return ("read", read_workflow(text))
            except Unread as why:
                return ("refused", why.refused)
            except AssertionError as why:
                return ("unplaced", str(why).split(":")[0])

        def starts(value):
            first = value[:2] if value[0] in "-?:" else value[0]
            return [f"line 1: a value that starts with {first!r} {unnamed}"]

        def plain(value, context):
            """What YAML reads for `value` first in a plain scalar, as the reader must read it."""
            first, rest = value[0], value[1:]
            if first not in indicators or (first in "-?:" and rest[:1] not in ("", " ")):
                if context == "flow" and first in "?:":
                    return ["line 1: a flow list whose items are not plain is not read"]
                return value
            if first in "&*!":
                return ["line 1: an anchor, alias or tag is not read"]
            if context == "flow" and first in "[]{}'\"#:?":
                return ["line 1: a flow list whose items are not plain is not read"]
            if context == "flow" and first == ",":
                return [f"line 1: a flow list with an empty entry {unnamed}"]
            if first in "'\"":
                return ["line 1: a quoted value that does not end at its closing quote is not read"]
            if first == "[":
                return ["line 1: a flow list whose items are not plain is not read"]
            if first == "{":
                return ["line 1: a flow mapping is not read"]
            if first == "#" and context == "value":
                return None
            if first in "|>":
                if value == "|" and context == "value":
                    return Quoted("")
                return [f"line 1: the block scalar header {value!r} {unnamed}"]
            return starts(value)

        members = []
        for first in printable:
            for value in (first + "x", first + " x", first):
                for context, text, wrap in (
                    ("value", f"k: {value}\n", lambda v: v),
                    ("item", f"k:\n  - {value}\n", lambda v: [v]),
                    ("flow", f"k: [{value}]\n", lambda v: [v]),
                ):
                    expected = plain(value, context)
                    if isinstance(expected, list):
                        if context == "item":
                            expected = [line.replace("line 1:", "line 2:") for line in expected]
                        members.append((f"{context} {value!r}", text, ("refused", expected)))
                    else:
                        members.append(
                            (f"{context} {value!r}", text, ("read", {"k": wrap(expected)}))
                        )
                for context, text, wrap in (
                    ("quoted value", f"k: '{value.replace(chr(39), chr(39) * 2)}'\n", lambda v: v),
                    (
                        "quoted item",
                        f"k:\n  - '{value.replace(chr(39), chr(39) * 2)}'\n",
                        lambda v: [v],
                    ),
                ):
                    members.append((f"{context} {value!r}", text, ("read", {"k": wrap(value)})))
        # A character outside printable ASCII first is refused once, by its line's character scan.
        for first in ("\x80", "\xa0", "\N{EM SPACE}"):
            for context, text, row in (
                ("value", f"k: {first}x\n", 1),
                ("item", f"k:\n  - {first}x\n", 2),
                ("flow", f"k: [{first}x]\n", 1),
            ):
                refusal = [f"line {row}: a character the reader does not read"]
                members.append((f"{context} {first!r}x", text, ("refused", refusal)))
        # Every block-scalar header (c-b-block-header): `|` or `>`, an indentation indicator 1-9
        # and a chomping indicator `-` or `+` in either order or alone, with and without a comment.
        # `|` clips its text to one final line feed and `|-` strips it; any other is refused, and
        # the text under a refused header is a line the reader cannot place.
        bodies = [""]
        bodies += list("123456789") + list("-+")
        bodies += [d + c for d in "123456789" for c in "-+"] + [
            c + d for c in "-+" for d in "123456789"
        ]
        read_as = {"|": ("", "x\n"), "|-": ("", "x")}
        for header in [s + b + c for s in "|>" for b in bodies for c in ("", " # c")]:
            for context, text, row, wrap in (
                ("value", f"k: {header}\n", 1, lambda v: v),
                ("item", f"k:\n  - {header}\n", 2, lambda v: [v]),
            ):
                if context == "value" and header in read_as:
                    empty, full = read_as[header]
                    members.append((f"header {header!r}", text, ("read", {"k": Quoted(empty)})))
                    members.append(
                        (
                            f"header {header!r} with text",
                            text + "  x\n",
                            ("read", {"k": Quoted(full)}),
                        )
                    )
                    continue
                refusal = [f"line {row}: the block scalar header {header!r} {unnamed}"]
                members.append((f"{context} header {header!r}", text, ("refused", refusal)))
                indent = " " * (2 * row)
                members.append(
                    (
                        f"{context} header {header!r} with text",
                        text + indent + "x\n",
                        ("unplaced", f"line {row + 1} was not read"),
                    )
                )
        # A block's trailing blank lines are dropped, but one indented more than its text is text,
        # with the blank lines above it; a blank line that holds a tab is refused.
        for header, clip in (("|", "\n"), ("|-", "")):
            members.append(
                (
                    f"{header} a blank line indented more",
                    f"k: {header}\n  x\n\n    \n\n",
                    ("read", {"k": Quoted("x\n\n  " + clip)}),
                )
            )
            members.append(
                (
                    f"{header} blank lines at its end",
                    f"k: {header}\n  x\n  \n\n",
                    ("read", {"k": Quoted("x" + clip)}),
                )
            )
            for blank in ("\t", "  \t", "    \t", "  \t  "):
                refusal = (
                    "line 3: a tab in the indentation, which the reader does not read"
                    if "\t" in blank[:2]
                    else f"line 3: a blank line of a block scalar that holds a tab {unnamed}"
                )
                members.append(
                    (
                        f"{header} a blank line {blank!r}",
                        f"k: {header}\n  x\n{blank}\n  y\n",
                        ("refused", [refusal]),
                    )
                )
        # Flow lists of plain items: an empty entry at every place, alone and before one trailing
        # comma. YAML reads `[]` as empty and one trailing comma as the list's end, and refuses
        # any other empty entry.
        for count in range(3):
            items = ["x", "y"][:count]
            members.append((f"flow {count}", f"k: [{', '.join(items)}]\n", ("read", {"k": items})))
            if count:
                members.append(
                    (f"flow {count},", f"k: [{', '.join(items)},]\n", ("read", {"k": items}))
                )
            for at in range(count + 1):
                entries = items[:at] + [" "] + items[at:]
                for tail in ("", ","):
                    flow = "[" + ",".join(entries) + tail + "]"
                    if at == count and not tail:
                        expected = ("read", {"k": items})
                    else:
                        expected = (
                            "refused",
                            [f"line 1: a flow list with an empty entry {unnamed}"],
                        )
                    members.append((f"flow {flow}", f"k: {flow}\n", expected))
        # A key YAML types by the core schema is read as that value, not as its text: refused, and
        # read when quoted.
        for key in (
            "true",
            "False",
            "NULL",
            "null",
            "1",
            "-1",
            "0o7",
            "0x1F",
            ".inf",
            "-.Inf",
            ".nan",
            "1.5",
            "1e3",
            ".5",
        ):
            members.append(
                (
                    f"key {key}",
                    f"{key}: x\n",
                    ("refused", ["line 1: a key that is not a plain name is not read"]),
                )
            )
            members.append((f"key '{key}'", f"'{key}': x\n", ("read", {key: "x"})))
        # Comments and blank lines are dropped (l-comment, c-nb-comment-text): a comment after each
        # one-line form, after a space or a tab; a comment line at any indentation; and a blank
        # line. A `#` with no white space before it is text.
        for form, text, value in (
            ("value", "k: x", "x"),
            ("item", "k:\n  - x", ["x"]),
            ("flow list", "k: [x]", ["x"]),
            ("single-quoted value", "k: 'x'", Quoted("x")),
            ("double-quoted value", 'k: "x"', Quoted("x")),
        ):
            for space in (" ", "\t"):
                members.append(
                    (
                        f"a comment after a {form}, after {space!r}",
                        f"{text}{space}# c\n",
                        ("read", {"k": value}),
                    )
                )
        for label, text, value in (
            ("a comment line", "# c\nk: x\n", {"k": "x"}),
            ("an indented comment line", "k:\n  # c\n  - x\n", {"k": ["x"]}),
            ("a comment line after a block", "k: |\n  x\n# c\n", {"k": Quoted("x\n")}),
            ("a `#` inside a value", "k: x#y\n", {"k": "x#y"}),
            ("blank lines", "\nk: x\n\n", {"k": "x"}),
            ("a blank line in a mapping", "k:\n\n  j: x\n", {"k": {"j": "x"}}),
            ("a blank line of spaces in a sequence", "k:\n  \n  - x\n", {"k": ["x"]}),
        ):
            members.append((label, text, ("read", value)))
        members.append(
            ("an empty item", "k:\n  - \n", ("refused", [f"line 2: an empty value {unnamed}"]))
        )
        # A line ends at a line feed, and a carriage return just before one is dropped with it
        # (SPEC-190 R12 part 1). Every other line-break character, and a byte-order mark, is
        # refused by its name and by the line GitHub's parser numbers, wherever it stands.
        characters = (
            ("\r", "a carriage return that does not end a line"),
            ("\x0b", "a vertical tab"),
            ("\x0c", "a form feed"),
            ("\x1c", "the control character U+001C"),
            ("\x1d", "the control character U+001D"),
            ("\x1e", "the control character U+001E"),
            ("\x85", "the next-line character U+0085"),
            ("\u2028", "the line separator U+2028"),
            ("\u2029", "the paragraph separator U+2029"),
            ("\ufeff", "a byte-order mark"),
        )
        suffix = " is not a character the reader reads"
        named_pattern = (
            r"line (\d+): ("
            + "|".join(re.escape(phrase) for _char, phrase in characters)
            + ")"
            + re.escape(suffix)
        )
        places = (
            ("at the end of a value", "k: x{c}j: y\n"),
            ("in a plain value", "k: x{c}y\n"),
            ("in a key", "k{c}j: x\n"),
            ("first in a file", "{c}k: x\n"),
            ("alone in a line", "k: x\n{c} \nj: y\n"),
            ("alone in an indented line", "k:\n  {c} \n  j: x\n"),
            ("in a comment line", "# a{c}b\nk: x\n"),
            ("in a comment after a value", "k: x # a{c}j: y\n"),
            ("in a single-quoted value", "k: 'x{c}y'\n"),
            ("in a double-quoted value", 'k: "x{c}y"\n'),
            ("in a flow list", "k: [x{c}y]\n"),
            ("in a sequence item", "k:\n  - x{c}y\n"),
            ("in a block scalar's text", "k: |\n  x{c}y\n"),
            ("in a block scalar's text under |-", "k: |-\n  x\n  y{c}z\n  w\n"),
            ("in a comment after a block scalar", "k: |\n  x\n# a{c}b\n"),
            ("before a carriage return and a line feed", "k: x{c}\r\nj: y\n"),
            ("twice in a line", "k: x{c}{c}y\n"),
            ("at the end of a file", "k: x{c}"),
            ("after the last line", "k: x\n{c}"),
            ("after a line that ends CRLF", "k: x\r\n{c}j: y\r\n"),
            ("before a line feed", "k: x{c}\nj: y\n"),
        )
        for char, phrase in characters:
            for place, template in places:
                if char == "\r" and "{c}\n" in template:
                    continue  # a carriage return before a line feed ends a line with it
                text = template.replace("{c}", char)
                first = re.search(re.escape(char) + ("(?!\n)" if char == "\r" else ""), text)
                row = text[: first.start()].count("\n") + 1
                members.append((f"{phrase} {place}", text, ("named", [(row, phrase)])))
        for label, text, value in (
            ("CRLF lines", "k: x\r\nj: y\r\n", {"k": "x", "j": "y"}),
            ("LF then CRLF lines", "k: x\nj: y\r\n", {"k": "x", "j": "y"}),
            ("CRLF then LF lines", "k: x\r\nj: y\n", {"k": "x", "j": "y"}),
            ("a CRLF blank line", "k: x\r\n\r\nj: y\r\n", {"k": "x", "j": "y"}),
            ("CRLF block lines", "k: |\r\n  x\r\n  y\r\n", {"k": Quoted("x\ny\n")}),
            ("a CRLF inside a block", "k: |\n  x\r\n  y\n", {"k": Quoted("x\ny\n")}),
        ):
            members.append((f"control: {label}", text, ("read", value)))
        # A tab in any line's indentation is refused, a comment line and a blank line outside a
        # block scalar's text included; a tab past the indentation, in a block's text or a comment's
        # text, is text and is read. A tab after a sequence's dash is refused too.
        tab_refusal = "a tab in the indentation, which the reader does not read"
        for run in ("\t", "\t\t", " \t", "\t "):
            for body, what in (("# c", "a comment line"), ("", "a blank line")):
                line = run + body
                for place, template, row in (
                    ("after a value", "k: x\n{t}\nj: y\n", 2),
                    ("first in a file", "{t}\nk: x\n", 1),
                    ("last in a file", "k: x\n{t}\n", 2),
                    ("in a file of that line alone", "{t}\n", 1),
                    ("between a key and its entries", "k:\n{t}\n  j: x\n", 2),
                    ("after a nested mapping", "k:\n  j: x\n{t}\n  i: y\n", 3),
                    ("inside a nested mapping", "k:\n  j: x\n  {t}\n  i: y\n", 3),
                    ("after a sequence", "k:\n  - x\n{t}\n  - y\n", 3),
                    ("inside a sequence", "k:\n  - x\n  {t}\n  - y\n", 3),
                ):
                    members.append(
                        (
                            f"{what} {run!r} {place}",
                            template.replace("{t}", line),
                            ("refused", [f"line {row}: {tab_refusal}"]),
                        )
                    )
        for run in ("\t", "\t\t"):
            for header in ("|", "|-"):
                for place, template, row in (
                    ("after a block scalar's text", "k: {h}\n  x\n{t}# c\nj: y\n", 3),
                    ("at a block scalar's first line", "k: {h}\n{t}# c\nj: y\n", 2),
                    ("after a nested block scalar", "a:\n  k: {h}\n    x\n{t}# c\n  j: y\n", 4),
                    (
                        "after a block scalar of a sequence item",
                        "s:\n  - run: {h}\n      x\n{t}# c\n  - y\n",
                        4,
                    ),
                ):
                    members.append(
                        (
                            f"a comment line {run!r} {place} under {header}",
                            template.replace("{h}", header).replace("{t}", run),
                            ("refused", [f"line {row}: {tab_refusal}"]),
                        )
                    )
        for label, text, value in (
            ("a tab in a block's text past its width", "k: |\n  \tx\n", {"k": Quoted("\tx\n")}),
            ("a tab inside a block's text", "k: |\n  x\ty\n", {"k": Quoted("x\ty\n")}),
            ("a tab in a comment line's text", "k: x\n# a\tb\n", {"k": "x"}),
            ("a tab in an indented comment's text", "k:\n  # a\tb\n  j: x\n", {"k": {"j": "x"}}),
        ):
            members.append((f"control: {label}", text, ("read", value)))
        for label, text, expected in (
            ("a tab after a dash", "k:\n  -\tx\n", ("unplaced", "line 2 is not a mapping entry")),
            ("a tab after a top dash", "-\tx\n", ("unplaced", "line 1 is not a mapping entry")),
            (
                "a tab after a dash, empty",
                "k:\n  -\t\n  - y\n",
                ("unplaced", "line 2 is not a mapping entry"),
            ),
            (
                "a tab after a dash, a mapping",
                "k:\n  -\tj: v\n",
                (
                    "refused",
                    [
                        "line 2: a tab after a sequence indicator is not read",
                        "line 2: a key that is not a plain name is not read",
                    ],
                ),
            ),
            *(
                (
                    f"a tab after a dash, then a colon: {text!r}",
                    text,
                    ("refused", [f"line {line}: a tab after a sequence indicator is not read"]),
                )
                for text, line in (
                    ("k:\n  -\t: x\n", 2),
                    ("k:\n  -\t :\n", 2),
                    ("k:\n  -\t\t: 'x'\n", 2),
                    ("-\t: x\n", 1),
                    ("k:\n  - j: v\n    -\t: x\n", 3),
                    ("concurrency:\n  group: g\n  -\t: x\n", 3),
                )
            ),
        ):
            members.append((label, text, expected))

        def named(text):
            """The refusals that name a line-break character or a byte-order mark, however the
            reader raised them, as (line, name)."""
            try:
                read_workflow(text)
            except AssertionError as why:
                said = "; ".join(why.refused) if isinstance(why, Unread) else str(why)
                return ("named", [(int(n), p) for n, p in re.findall(named_pattern, said)])
            return ("read", [])

        def judged(text, expected):
            return named(text) if expected[0] == "named" else outcome(text)

        wrong = [
            (label, judged(text, expected), expected)
            for label, text, expected in examined("forms the reader meets", members)
            if judged(text, expected) != expected
        ]
        self.assertGreater(len(members), 2000, "the population of named forms shrank")
        self.assertEqual(wrong, [])


# ------------------------------------------------------------------ reading a workflow (SPEC-038)

# A key the reader reads: a plain name, bare or in matching quotes (SPEC-034 R7).
KEY = re.compile(r"(['\"]?)([\w.-]+)\1")

# The forms the reader reads, each named by the YAML 1.2.2 production it rests on (yaml.org/spec/
# 1.2.2, read 2026-09-30), so the reader is default-deny over its grammar (SPEC-190 R12): a form not
# named here is refused by `_unnamed`, which names the form it met.
# A plain scalar starts with a character that is not a YAML indicator, or with `-`, `?` or `:` before
# one that is not white space (ns-plain-first).
PLAIN_FIRST = re.compile(r"[A-Za-z0-9$()+./;<=\\^_~]|[-?:][^ \t]")
# A block scalar is literal and clipped to one final line feed, `|`, or stripped of it, `|-`
# (c-l+literal, c-chomping-indicator), with nothing after its header.
BLOCK_HEADERS = {"|": "\n", "|-": ""}


class Quoted(str):
    """A scalar YAML reads as a string whatever its text: a quoted one or a block. A plain scalar is
    typed by YAML 1.2's core schema instead (`kind`), so `'false'` is a string and `false` is not."""


# YAML 1.2's core schema, as the `yaml` package resolves a plain scalar for GitHub's workflow parser
# (eemeli/yaml src/schema/core, read 2026-09-30T09:29Z): each kind and the plain text it takes.
CORE_SCHEMA = (
    ("null", r"~|null|Null|NULL"),
    ("boolean", r"true|True|TRUE|false|False|FALSE"),
    ("number", r"[-+]?[0-9]+|0o[0-7]+|0x[0-9a-fA-F]+"),
    ("number", r"[-+]?(?:\.inf|\.Inf|\.INF)|\.nan|\.NaN|\.NAN"),
    ("number", r"[-+]?(?:\.[0-9]+|[0-9]+(?:\.[0-9]*)?)[eE][-+]?[0-9]+"),
    ("number", r"[-+]?(?:\.[0-9]+|[0-9]+\.[0-9]*)"),
)


def kind(value):
    """What GitHub's parser reads a value as: a mapping, a sequence, null, a boolean, a number or a
    string. A quoted or block scalar is a string; a plain one takes the first core-schema kind whose
    text it matches, and is a string when it matches none."""
    if value is None:
        return "null"
    if isinstance(value, dict):
        return "mapping"
    if isinstance(value, list):
        return "sequence"
    if isinstance(value, Quoted):
        return "string"
    return next((k for k, text in CORE_SCHEMA if re.fullmatch(text, value)), "string")


class Unread(AssertionError):
    """A workflow that holds forms the reader does not read, each refusal as `line N: why`. It
    carries the rest of the workflow as read, each refused key or value read as '', so a checker
    can name every refusal and still judge everything else (SPEC-034 R7)."""

    def __init__(self, refused, workflow):
        super().__init__("the reader does not read " + "; ".join(refused))
        self.refused, self.workflow = refused, workflow


# A line ends at a line feed, and a carriage return just before one is dropped with it. Every other
# character either parser could take for a line break, and a byte-order mark, is refused by name,
# wherever it stands in the file (SPEC-190 R12).
LINE_BREAKS = {
    "\r": "a carriage return that does not end a line",
    "\x0b": "a vertical tab",
    "\x0c": "a form feed",
    "\x1c": "the control character U+001C",
    "\x1d": "the control character U+001D",
    "\x1e": "the control character U+001E",
    "\x85": "the next-line character U+0085",
    "\u2028": "the line separator U+2028",
    "\u2029": "the paragraph separator U+2029",
    "\ufeff": "a byte-order mark",
}
NOT_READ = " is not a character the reader reads"


def read_workflow(text):
    """A workflow as dicts, lists and strings, read without a YAML library. It reads the named forms
    and no other (SPEC-190 R12): mappings keyed by plain names that YAML types as strings, `- `
    sequences, `|` and `|-` block scalars, one-line flow lists of plain items with at most one
    trailing comma, plain scalars whose first character is not a YAML indicator (`PLAIN_FIRST`),
    and quoted one-line scalars, a quote doubled inside single quotes read as one. Blank lines,
    comment lines and a ` #` comment after a value are dropped. It ends a line
    only at a line feed, a carriage return just before one dropped with it, as GitHub's parser
    does: a lone carriage return is a character of its line, and the line's character scan
    refuses it. It reads a space or a tab as white space and nothing else. A quoted or block scalar is read as `Quoted`, a string that keeps its style, so `kind`
    types a value as GitHub's parser does.
    It fails closed (SPEC-034 R7). A line that holds a character other than a tab or printable
    ASCII, a byte-order mark and a carriage return that does not end a line included, a
    double-quoted value that holds an escape, a quoted value that does not end at its
    closing quote, an anchor, alias or tag, a flow mapping, a flow list whose items are not plain,
    a key that is not a plain name, a tab in any line's indentation, a comment or blank line
    outside a block scalar's text included, and every form the named forms
    do not hold (`_unnamed`: a value's first character, a block scalar's header, a flow list's
    empty entry, a blank line of a block that holds a tab) are each refused by their line, never
    guessed at, and the file raises Unread once it is read. A key a mapping already holds, read
    without case, is refused by its line too: GitHub's workflow parser refuses a workflow that
    holds one (SPEC-190 R10). A line it cannot place refuses the whole file at once."""
    lines = re.split(r"\r\n|\n", text)
    refused = []
    for at, line in enumerate(lines):
        named = [LINE_BREAKS[c] for c in dict.fromkeys(line) if c in LINE_BREAKS]
        refused += [f"line {at + 1}: {name}{NOT_READ}" for name in named]
        if re.search(r"[^\t\x20-\x7e" + "".join(LINE_BREAKS) + "]", line):
            refused.append(f"line {at + 1}: a character the reader does not read")
    try:
        value, at = _mapping(lines, _skip(lines, 0, refused), 0, refused)
        at = _skip(lines, at, refused)
        if at < len(lines):
            raise AssertionError(f"line {at + 1} was not read: {lines[at]!r}")
    except Unread:
        raise
    except AssertionError as why:
        if refused:
            raise AssertionError(f"{why}; the reader does not read " + "; ".join(refused)) from None
        raise
    if refused:
        raise Unread(refused, value)
    return value


def _indent(line):
    return len(line) - len(line.lstrip(" "))


def _tabbed(lines, at, refused):
    """Refuse a line whose indentation holds a tab: YAML indents with spaces alone, and GitHub's
    parser refuses a mapping or a sequence indented with one ("Tabs are not allowed as indentation").
    A comment or blank line indented with one is refused too, as a form the reader does not read. A
    tab inside a value, or in a block scalar's text, is text and is read."""
    line = lines[at]
    why = f"line {at + 1}: a tab in the indentation, which the reader does not read"
    if "\t" in line[: len(line) - len(line.lstrip(" \t"))] and why not in refused:
        refused.append(why)


def _skip(lines, at, refused):
    """Skip the blank and comment lines, refusing each whose indentation holds a tab: a comment or
    blank line outside a block scalar's text is indented with spaces like any other line."""
    while at < len(lines) and (
        not lines[at].strip(" \t") or lines[at].lstrip(" \t").startswith("#")
    ):
        _tabbed(lines, at, refused)
        at += 1
    return at


def _read(reader, text, at, refused):
    """What `reader` reads of line `at`'s text, or '' with its refusal recorded by the line."""
    try:
        return reader(text)
    except ValueError as why:
        refused.append(f"line {at + 1}: {why}")
        return ""


def _key(text):
    """A plain name, bare or in matching quotes. A bare one YAML types as other than a string
    (`true`, `null`, `1`) is refused: YAML reads it as that value, not as its text."""
    key = KEY.fullmatch(text.strip(" \t"))
    if not key or (not key.group(1) and kind(key.group(2)) != "string"):
        raise ValueError("a key that is not a plain name is not read")
    return key.group(2)


def _unnamed(form):
    """The one refusal for a form the named forms do not hold (SPEC-190 R12): it names the form the
    reader met, so a form is never read because nothing refused it."""
    return ValueError(f"{form} is not a form the reader reads")


def _form(text):
    """The form a value takes that `PLAIN_FIRST` does not name, as its refusal names it: a block
    scalar's header, the character it starts with (`-`, `?` and `:` with the one after), or none."""
    if not text:
        return "an empty value"
    if text[0] in ("|", ">"):
        return f"the block scalar header {text!r}"
    return f"a value that starts with {text[:2] if text[0] in ('-', '?', ':') else text[0]!r}"


def _scalar(text):
    """A one-line scalar or flow list as YAML reads it, or ValueError naming a form the reader does
    not read: an anchor, alias or tag, a flow mapping, a flow list whose items are not plain (a
    quoted or nested item, or a `#`, `:` or `?` inside it), a plain value that holds `: `, which
    YAML reads as a key, and a value whose first character `PLAIN_FIRST` does not name."""
    text = text.strip(" \t")
    if text[:1] in ("&", "*", "!"):
        raise ValueError("an anchor, alias or tag is not read")
    if text[:1] in ("'", '"'):
        return _quoted(text)
    if text[:1] == "{":
        raise ValueError("a flow mapping is not read")
    if text[:1] == "[":
        return _flow(text)
    # A value that starts with a character outside printable ASCII is refused once, by its line's
    # character scan, and read, so what it holds is still judged.
    if not PLAIN_FIRST.match(text) and not re.match(r"[^\t\x20-\x7e]", text):
        raise _unnamed(_form(text))
    text = re.sub(r"[ \t]#.*$", "", text).strip(" \t")
    if re.search(r":(?:[ \t]|$)", text):
        raise ValueError("a key that is not a plain name is not read")
    return text


def _flow(text):
    """A flow list on one line, as YAML reads one (c-flow-sequence): plain items split by commas,
    one trailing comma ending the list, and `[]` empty. An empty entry anywhere else is refused,
    since YAML refuses it."""
    items = re.fullmatch(r"\[([^\[\]{}'\"#:?]*)\](?:[ \t]+#.*)?", text)
    if not items:
        raise ValueError("a flow list whose items are not plain is not read")
    if not items.group(1).strip(" \t"):
        return []
    parts = items.group(1).split(",")
    if len(parts) > 1 and not parts[-1].strip(" \t"):
        parts.pop()
    if any(not part.strip(" \t") for part in parts):
        raise _unnamed("a flow list with an empty entry")
    return [_scalar(part) for part in parts]


def _quoted(text):
    """A quoted one-line scalar that ends at its closing quote, with at most a comment after it. A
    quote doubled inside single quotes is one quote; a double-quoted value is read only when it
    holds no escape, since YAML decodes one there."""
    single = text[0] == "'"
    body = r"'((?:[^']|'')*)'" if single else r'"((?:[^"\\]|\\.)*)"'
    quoted = re.fullmatch(body + r"(?:[ \t]+#.*)?", text)
    if not quoted:
        raise ValueError("a quoted value that does not end at its closing quote is not read")
    if not single and "\\" in quoted.group(1):
        raise ValueError("a double-quoted value that holds an escape is not read")
    return Quoted(quoted.group(1).replace("''", "'") if single else quoted.group(1))


def _item(line, indent):
    """Whether a line holds a sequence item at `indent`: a dash and a space, as YAML reads one.
    `-x: y` is a key, and a bare dash is a line the reader cannot place."""
    return line[indent:].startswith("- ")


def _block(lines, at, indent, refused):
    if _item(lines[at], indent):
        return _sequence(lines, at, indent, refused)
    return _mapping(lines, at, indent, refused)


def _mapping(lines, at, indent, refused):
    found = {}
    while True:
        at = _skip(lines, at, refused)
        if at >= len(lines) or _indent(lines[at]) != indent or _item(lines[at], indent):
            return found, at
        _tabbed(lines, at, refused)
        text = lines[at][indent:]
        if text.startswith("-\t"):
            # A dash and a tab open a sequence item to YAML, whatever follows them; read as a key
            # `-`, the line would be a mapping entry YAML never reads (SPEC-190 R12, condition 2).
            refused.append(f"line {at + 1}: a tab after a sequence indicator is not read")
        if ": " in text:
            key, rest = text.split(": ", 1)
        elif text.endswith(":"):
            key, rest = text[:-1], ""
        else:
            raise AssertionError(f"line {at + 1} is not a mapping entry: {lines[at]!r}")
        key, rest = _read(_key, key, at, refused), rest.strip(" \t")
        if key and key.casefold() in {held.casefold() for held in found}:
            refused.append(f"line {at + 1}: a key the mapping already holds, read without case")
        if rest in BLOCK_HEADERS:
            header, at = rest, at + 1
            body, width = [], None
            while at < len(lines) and (not lines[at].strip(" \t") or _indent(lines[at]) > indent):
                if lines[at].strip(" \t") and width is None:
                    width = _indent(lines[at])
                body.append((at, lines[at], width is None))
                at += 1
            # YAML drops a block's trailing blank lines but keeps one indented more than its
            # text, and the blank lines above it, as text.
            while body and not body[-1][1].strip(" "):
                if width is not None and len(body[-1][1]) > width:
                    break
                body.pop()
            width = width or 0
            for row, line, leading in body:
                # YAML takes a block's indentation from its first line of text: a line of text
                # indented less ends the block, and a blank line above it indented more, or a tab
                # inside the indentation, is an error. A blank line that holds a tab is text to
                # YAML and a blank to this reader, so it is refused. A line the character scan
                # refuses already is refused once, by that scan.
                if re.search(r"[^\t\x20-\x7e]", line):
                    continue
                if "\t" in line[:width]:
                    refused.append(
                        f"line {row + 1}: a tab in the indentation, which the reader does not read"
                    )
                elif not line.strip(" \t") and "\t" in line:
                    refused.append(
                        f"line {row + 1}: "
                        + str(_unnamed("a blank line of a block scalar that holds a tab"))
                    )
                elif line.strip(" \t") and _indent(line) < width:
                    refused.append(f"line {row + 1}: a line indented less than its block's text")
                elif leading and _indent(line) > width:
                    refused.append(
                        f"line {row + 1}: a blank line indented more than its block's text"
                    )
            width = min((_indent(line) for _r, line, _l in body if line.strip(" \t")), default=0)
            text = "".join(line[width:] + "\n" for _row, line, _leading in body)
            found[key] = Quoted((text[:-1] + BLOCK_HEADERS[header]) if text else "")
        elif not rest or rest.startswith("#"):
            child = _skip(lines, at + 1, refused)
            if child < len(lines) and _indent(lines[child]) > indent:
                found[key], at = _block(lines, child, _indent(lines[child]), refused)
            else:
                found[key], at = None, at + 1
        else:
            found[key], at = _read(_scalar, rest, at, refused), at + 1


def _sequence(lines, at, indent, refused):
    found = []
    while True:
        at = _skip(lines, at, refused)
        if at >= len(lines) or _indent(lines[at]) != indent or not _item(lines[at], indent):
            return found, at
        _tabbed(lines, at, refused)
        body = lines[at][indent + 1 :].lstrip(" ")
        inner = len(lines[at]) - len(body)
        if re.match(r"^['\"]?[\w.-]+['\"]?:(?: |$)", body):
            # A mapping item: its first entry sits on the dash's line, its others below it.
            lines[at] = " " * inner + body
            item, at = _mapping(lines, at, inner, refused)
        else:
            item, at = _read(_scalar, body, at, refused), at + 1
        found.append(item)


def load(name):
    return read_workflow(workflow_file_text(WORKFLOWS / name))


def read_hardened(path):
    """A workflow as the hardening tests read it: through `read_workflow`, the checker's reader, so
    they read its keys the way the checker does (SPEC-034 R7). A form the reader does not read, or a
    line it cannot place, fails the test that reads it, named by the file and the line."""
    try:
        return read_workflow(workflow_file_text(path))
    except AssertionError as refused:
        raise AssertionError(f"{path.name}: {refused}") from None


def entries(value, key):
    """Every value a read workflow holds under `key`, in the order it holds them, wherever it sits:
    each `uses`, or each `runs-on`."""
    if isinstance(value, dict):
        return [
            found
            for name, item in value.items()
            for found in ([item] if name == key else []) + entries(item, key)
        ]
    if isinstance(value, list):
        return [found for item in value for found in entries(item, key)]
    return []


def marked_uses(workflow, path=()):
    """Every value a read workflow holds under `uses`, in exactly `entries`' order, each beside
    whether it is a call job's own `uses` (the value at `jobs.<id>.uses`, nothing deeper). A step's
    `uses` can spell the same text, so the mark is the value's place, never its text."""
    if isinstance(workflow, dict):
        return [
            found
            for name, item in workflow.items()
            for found in (
                [(item, name == "uses" and len(path) == 2 and path[0] == "jobs")]
                if name == "uses"
                else []
            )
            + marked_uses(item, path + (name,))
        ]
    if isinstance(workflow, list):
        return [found for item in workflow for found in marked_uses(item, path + (None,))]
    return []


def placed_runners(workflow, path=()):
    """Every value a read workflow holds under `runs-on`, in exactly `entries`' order, each beside
    the id of the job it is the own runner of (the value at `jobs.<id>.runs-on`, nothing deeper),
    or None for a value anywhere else (SPEC-352 R15). The place decides, never the text, so a
    runner nested inside a job is admitted only as no job's."""
    if isinstance(workflow, dict):
        return [
            found
            for name, item in workflow.items()
            for found in (
                [(item, path[1] if len(path) == 2 and path[0] == "jobs" else None)]
                if name == "runs-on"
                else []
            )
            + placed_runners(item, path + (name,))
        ]
    if isinstance(workflow, list):
        return [found for item in workflow for found in placed_runners(item, path + (None,))]
    return []


def path_glob(pattern):
    """A path filter as GitHub matches it: `**` is any run including a slash, `*` any run but a
    slash, `?` one character but a slash, `[..]` a class, and every other character itself."""
    out, at = "", 0
    while at < len(pattern):
        if pattern.startswith("**", at):
            out, at = out + ".*", at + 2
        elif pattern[at] == "*":
            out, at = out + "[^/]*", at + 1
        elif pattern[at] == "?":
            out, at = out + "[^/]", at + 1
        elif pattern[at] == "[" and "]" in pattern[at:]:
            close = pattern.index("]", at)
            out, at = out + pattern[at : close + 1], close + 1
        else:
            out, at = out + re.escape(pattern[at]), at + 1
    return re.compile(out)


def action(step):
    """The action a step uses, without its ref: `actions/cache/restore`, or '' for a run step."""
    return str(step.get("uses", "")).split("@", 1)[0]


def lines_of(value):
    """A block scalar's non-blank lines, stripped: a multi-line `path` or `restore-keys`."""
    return [line.strip() for line in str(value or "").splitlines() if line.strip()]


def paths(step):
    return lines_of((step.get("with") or {}).get("path"))


def gate_stages():
    return STAGES.search((REPO / "scripts" / "check.sh").read_text()).group(1).split()


def stage_calls(workflow):
    """[(job, [stage, ...])] for every `bash scripts/check.sh` a job's steps run. A bare call names
    no stage, which check.sh reads as every stage."""
    calls = []
    for job, body in (workflow.get("jobs") or {}).items():
        for step in (body or {}).get("steps") or []:
            for call in GATE_CALL.finditer(re.sub(r"(?m)#.*$", "", str(step.get("run", "")))):
                calls.append((job, call.group(1).split()))
    return calls


def stage_problems(workflow, stages):
    """Every stage of `stages` that runs in no job or in more than one, and every stage a job names
    that check.sh does not define."""
    jobs = {stage: [] for stage in stages}
    problems = []
    for job, named in stage_calls(workflow):
        for stage in named or stages:
            if stage in jobs:
                jobs[stage].append(job)
            else:
                problems.append(f"{job} runs {stage}, which check.sh does not define")
    for stage, found in jobs.items():
        if len(found) != 1:
            problems.append(f"{stage} runs in {len(found)} job(s): {', '.join(found) or 'none'}")
    return problems


# ------------------------------------------------ GitHub's expression rules (SPEC-038 A2, A10, A11)

EXPRESSION = re.compile(r"\$\{\{\s*(.*?)\s*\}\}")
TOKEN = re.compile(r"\s*(?:(==|!=|&&|\|\||!|\(|\))|'((?:[^']|'')*)'|([A-Za-z_][\w.-]*))")


def truthy(value):
    """GitHub's truthiness: false, null, '', 0 and NaN are falsy; everything else is truthy."""
    if value is None or value is False or value == "":
        return False
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return not math.isnan(value) and value != 0
    return True


def number(value):
    if value is None:
        return 0.0
    if isinstance(value, (bool, int, float)):
        return float(value)
    try:
        return float(value.strip()) if value.strip() else 0.0
    except ValueError:
        return math.nan


def equal(left, right):
    """GitHub's `==`: strings compare without case; other mixed types compare as numbers."""
    if isinstance(left, str) and isinstance(right, str):
        return left.casefold() == right.casefold()
    if type(left) is type(right):
        return left == right
    return number(left) == number(right)


def evaluate(expression, context):
    """The value of one expression (without `${{ }}`) over `context`, {dotted name: value}. `&&`
    and `||` return an operand, as GitHub's do. A `github.` name no scenario models refuses, and a
    step's output nobody set is null. Only the forms the workflows use are read."""
    tokens, at, text = [], 0, expression.strip()
    while at < len(text):
        match = TOKEN.match(text, at)
        if match is None or match.end() == at:
            raise AssertionError(f"this reader models no such expression: {expression!r}")
        operator, string, name = match.groups()
        if operator:
            tokens.append(("op", operator))
        elif string is not None:
            tokens.append(("str", string.replace("''", "'")))
        else:
            tokens.append(("name", name))
        at = match.end()
    position = 0

    def take():
        nonlocal position
        if position >= len(tokens):
            raise AssertionError(f"an expression that ends early: {expression!r}")
        position += 1
        return tokens[position - 1]

    def peek():
        return tokens[position] if position < len(tokens) else (None, None)

    def primary():
        kind, value = take()
        if (kind, value) == ("op", "("):
            inner = disjunction()
            if take() != ("op", ")"):
                raise AssertionError(f"an unclosed parenthesis: {expression!r}")
            return inner
        if (kind, value) == ("op", "!"):
            return not truthy(primary())
        if kind == "str":
            return value
        if kind == "name" and value in ("true", "false", "null"):
            return {"true": True, "false": False, "null": None}[value]
        if kind == "name" and value.startswith("steps."):
            return context.get(value)
        if kind == "name" and value in context:
            return context[value]
        raise AssertionError(f"{expression!r} reads {value}, which no scenario models")

    def comparison():
        left = primary()
        if peek() in (("op", "=="), ("op", "!=")):
            operator = take()[1]
            same = equal(left, primary())
            return same if operator == "==" else not same
        return left

    def conjunction():
        left = comparison()
        while peek() == ("op", "&&"):
            take()
            right = comparison()
            left = right if truthy(left) else left
        return left

    def disjunction():
        left = conjunction()
        while peek() == ("op", "||"):
            take()
            right = conjunction()
            left = left if truthy(left) else right
        return left

    value = disjunction()
    if position != len(tokens):
        raise AssertionError(f"this reader models no such expression: {expression!r}")
    return value


def condition(value, context):
    """Whether a condition holds: `${{ expr }}`, a bare `if:` expression, or a literal."""
    text = str(value).strip()
    whole = re.fullmatch(r"\$\{\{\s*(.*?)\s*\}\}", text)
    return truthy(evaluate(whole.group(1) if whole else text, context))


def rendered(template, context):
    """A value with `${{ }}` inside it, as GitHub renders it."""

    def text(match):
        value = evaluate(match.group(1), context)
        if value is None:
            return ""
        if isinstance(value, bool):
            return "true" if value else "false"
        return str(value)

    return EXPRESSION.sub(text, str(template))


def push(ref, run_id="201"):
    return {
        "github.event_name": "push",
        "github.ref": ref,
        "github.run_id": run_id,
        "github.repository": THIS_REPOSITORY,
        "github.workflow": "ci",
    }


def pull_request(base, number="7", run_id="101", head_repository=THIS_REPOSITORY):
    return {
        "github.event_name": "pull_request",
        "github.ref": f"refs/pull/{number}/merge",
        "github.base_ref": base,
        "github.head_ref": "feat/probe",
        "github.run_id": run_id,
        "github.repository": THIS_REPOSITORY,
        "github.workflow": "ci",
        "github.event.pull_request.number": number,
        "github.event.pull_request.head.repo.full_name": head_repository,
    }


# (scenario, context, whether a cache that missed its key may be saved there)
SAVE_SCENARIOS = [
    ("a push to dev", push("refs/heads/dev"), True),
    ("a push to main", push("refs/heads/main"), True),
    ("a pull request into dev", pull_request("dev"), False),
    ("a pull request into main", pull_request("main"), False),
    ("a fork's pull request into dev", pull_request("dev", head_repository="someone/fork"), False),
    ("a push to another branch", push("refs/heads/feature"), False),
    ("a pushed tag", push("refs/tags/v1.0.0"), False),
]


def cache_problems(name, workflow):
    """Every way a workflow could save a cache other than from a push to dev or main that missed its
    key, and the save steps it examined."""
    problems, saves = [], []
    for job_id, job in (workflow.get("jobs") or {}).items():
        for step in (job or {}).get("steps") or []:
            uses, inputs = action(step), step.get("with") or {}
            where = f"{name}:{job_id}:{step.get('name') or step.get('id') or uses}"
            if uses == "actions/cache":
                problems.append(f"{where}: actions/cache saves in its post step on every event")
            elif uses in CACHE_BY_THEMSELVES:
                problems.append(f"{where}: {uses} saves a cache by itself")
            elif uses == "actions/setup-node" and (
                "cache" in inputs or inputs.get("package-manager-cache") != "false"
            ):
                why = "it has a cache input or package-manager-cache is not false"
                problems.append(f"{where}: setup-node saves a cache, because {why}")
            elif uses == "pnpm/action-setup" and str(inputs.get("cache", "false")) != "false":
                problems.append(f"{where}: pnpm/action-setup saves its store when cache is on")
            elif uses == "actions/cache/save":
                saves.append(where)
                if name == "rust-cache.yml" and runs_only_on_schedule(workflow):
                    problems += scheduled_save_problems(where, step)
                else:
                    problems += save_problems(where, step)
    return problems, saves


def runs_only_on_schedule(workflow):
    """Whether a workflow's events are `schedule` and `workflow_dispatch` and nothing else: a run of
    it executes the default branch's copy and never belongs to a push or a pull request. A workflow
    that adds any other event is judged by the push rule (SPEC-191 R9)."""
    on = workflow.get("on")
    events = [on] if isinstance(on, str) else list(on or [])
    return "schedule" in events and set(events) <= {"schedule", "workflow_dispatch"}


def scheduled_save_problems(where, step):
    """A scheduled workflow's save that runs when a lookup hit (on a schedule or a dispatch), or that
    runs under a key other than a restore step's primary key (SPEC-191 R8, R9). The lookups it depends on are the `cache-hit`
    outputs its condition reads; it must save when they all missed and never when any hit."""
    problems = []
    key = str((step.get("with") or {}).get("key", ""))
    if not re.fullmatch(r"\$\{\{ steps\.[\w-]+\.outputs\.cache-primary-key \}\}", key):
        problems.append(f"{where}: saves under {key}, not a restore step's primary key")
    lookups = sorted(
        set(re.findall(r"steps\.([\w-]+)\.outputs\.cache-hit", str(step.get("if", ""))))
    )
    if not lookups:
        problems.append(f"{where}: saves on a scheduled run whatever a lookup found")
    base = {"github.event_name": "schedule", "github.ref": "refs/heads/main"}
    missed = dict(base, **{f"steps.{lookup}.outputs.cache-hit": "false" for lookup in lookups})
    scenarios = [("a scheduled run that missed its key", missed, True)]
    for lookup in lookups or ["lookup"]:
        for event in ("schedule", "workflow_dispatch"):
            hit = dict(missed, **{f"steps.{lookup}.outputs.cache-hit": "true"})
            hit["github.event_name"] = event
            scenarios.append((f"a {event} run whose {lookup} hit", hit, False))
    for scenario, context, allowed in scenarios:
        saves = "if" not in step or condition(step["if"], context)
        if saves and not allowed:
            problems.append(f"{where}: saves on {scenario}")
        if allowed and not saves:
            problems.append(f"{where}: never saves on {scenario}, so the cache never warms")
    return problems


def save_problems(where, step):
    """A save step that saves where it must not, or never saves where it must."""
    problems = []
    restore = re.fullmatch(
        r"\$\{\{ steps\.([\w-]+)\.outputs\.cache-primary-key \}\}",
        str((step.get("with") or {}).get("key", "")),
    )
    scenarios = list(SAVE_SCENARIOS)
    if restore:
        hit = dict(
            push("refs/heads/dev"), **{f"steps.{restore.group(1)}.outputs.cache-hit": "true"}
        )
        scenarios.append(("a push to dev that hit its key exactly", hit, False))
    for scenario, context, allowed in scenarios:
        saves = "if" not in step or condition(step["if"], context)
        if saves and not allowed:
            problems.append(f"{where}: saves on {scenario}")
        if allowed and not saves:
            problems.append(f"{where}: never saves on {scenario}, so the cache never warms")
    return problems


PLANTED_CACHES = """\
jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
      - name: combined
        uses: actions/cache@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
      - name: node
        uses: actions/setup-node@0123456789abcdef0123456789abcdef01234567
        with:
          cache: pnpm
      - id: restored
        uses: actions/cache/restore@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
      - name: unconditional
        uses: actions/cache/save@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: ${{ steps.restored.outputs.cache-primary-key }}
      - name: any push
        if: ${{ github.event_name == 'push' }}
        uses: actions/cache/save@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
"""


class TheGateRunsInParallelJobs(unittest.TestCase):
    def test_the_gate_runs_in_four_parallel_jobs(self):
        workflow = load("ci.yml")
        layout = {}
        calls = examined("check.sh calls", stage_calls(workflow))
        for job, named in calls:
            layout.setdefault(job, []).extend(named or ["every stage"])
        self.assertEqual(layout, OWNER_LAYOUT)
        self.assertEqual(len(calls), len(OWNER_LAYOUT), "a job calls check.sh more than once")
        for job in OWNER_LAYOUT:
            self.assertIsNone(workflow["jobs"][job].get("needs"), f"{job} waits on another job")
        needs = workflow["jobs"]["ci"]["needs"]
        # SPEC-039 adds the five mutation jobs beside the gate's five, each a need of ci; SPEC-087
        # adds the sixth, the Python runner's shards. SPEC-341 adds the planted card suite.
        mutation = [
            "mutation-plan",
            "mutation-python",
            "mutation-rust",
            "mutation-rows",
            "mutation-verdict",
            "mutation-web",
        ]
        # SPEC-338 adds the web engine's job, which builds the module and holds it to its budget.
        self.assertEqual(
            sorted(needs),
            sorted(
                [
                    *OWNER_LAYOUT,
                    *mutation,
                    "web-engine",
                    "workflow-lint",
                    "base-is-dev",
                    "card-sandbox",
                ]
            ),
        )

    def test_every_stage_runs_in_exactly_one_ci_job(self):
        stages = examined("gate stages", gate_stages())
        self.assertEqual(stage_problems(load("ci.yml"), stages), [])
        # A planted workflow that drops a stage, doubles another and names an unknown one.
        planted = read_workflow(
            "jobs:\n  a:\n    steps:\n      - run: bash scripts/check.sh fmt clippy\n"
            "  b:\n    steps:\n      - run: |\n          bash scripts/check.sh clippy lint\n"
        )
        self.assertEqual(
            stage_problems(planted, ["fmt", "clippy", "test"]),
            [
                "b runs lint, which check.sh does not define",
                "clippy runs in 2 job(s): a, b",
                "test runs in 0 job(s): none",
            ],
        )
        # A bare call runs every stage, so beside one other call it doubles that call's stages.
        bare = read_workflow(
            "jobs:\n  a:\n    steps:\n      - run: bash scripts/check.sh\n"
            "  b:\n    steps:\n      - run: bash scripts/check.sh test\n"
        )
        self.assertEqual(stage_problems(bare, ["fmt", "test"]), ["test runs in 2 job(s): a, b"])

    def test_the_jobs_that_read_history_fetch_all_of_it(self):
        workflow = load("ci.yml")
        readers = [
            (stage, job)
            for job, named in stage_calls(workflow)
            for stage in named
            if stage in ("secrets", "scrub")
        ]
        for stage, job_id in examined("stages that read history", readers):
            job = workflow["jobs"][job_id]
            checkout = next(s for s in job["steps"] if action(s) == "actions/checkout")
            depth = (checkout.get("with") or {}).get("fetch-depth")
            self.assertEqual(depth, "0", f"{job_id} runs {stage} on a shallow checkout")
            if stage == "secrets":
                gate = next(s for s in job["steps"] if GATE_CALL.search(str(s.get("run", ""))))
                env = dict(job.get("env") or {}, **(gate.get("env") or {}))
                self.assertEqual(env.get("CHECK_HISTORY"), "1", f"{job_id} scans no history")


# --- the card sandbox job (SPEC-341 A13, R12)
# The planted card suite runs on every pull request in Chromium and in WebKit, so the job installs
# both with their system libraries, keeps the suite's results whatever its verdict, and the
# aggregate check waits on it.
CARD_JOB = "card-sandbox"
CARD_INSTALL = "pnpm --dir web/app exec playwright install --with-deps --only-shell chromium webkit"
CARD_SUITE = "pnpm --dir web/app test:card"


class TheCardSandboxRunsInBothEngines(unittest.TestCase):
    def test_the_card_sandbox_job_runs_the_planted_suite_in_both_engines(self):
        jobs = load("ci.yml")["jobs"]
        self.assertIn(CARD_JOB, jobs, "ci.yml has no card-sandbox job")
        steps = jobs[CARD_JOB]["steps"]
        runs = [str(step.get("run", "")).strip() for step in steps]
        # both engines installed once, then the suite once, after them
        self.assertEqual(runs.count(CARD_INSTALL), 1, "the job does not install both engines once")
        self.assertEqual(runs.count(CARD_SUITE), 1, "the job does not run the planted suite once")
        self.assertLess(runs.index(CARD_INSTALL), runs.index(CARD_SUITE))
        # one upload of the suite's results after it, whatever its verdict
        uploads = [at for at, step in enumerate(steps) if action(step) == "actions/upload-artifact"]
        self.assertEqual(len(uploads), 1, "the job does not upload the suite's results once")
        upload = steps[uploads[0]]
        self.assertGreater(uploads[0], runs.index(CARD_SUITE))
        self.assertEqual(upload.get("if"), "${{ always() }}")
        self.assertEqual(paths(upload), ["web/app/test-results/"])
        # and the aggregate check waits on it
        self.assertIn(CARD_JOB, jobs["ci"]["needs"])
        examined("card-sandbox steps", steps)


def cache_scan(directory):
    """`cache_problems` over every workflow of a directory: the problems, then the saves found."""
    problems, saves = [], []
    for path in workflow_files(directory):
        found_problems, found = cache_problems(path.name, read_workflow(workflow_file_text(path)))
        problems += found_problems
        saves += found
    return problems, saves


def protoc_pins(directory):
    """(workflow name, digest) for every `PROTOC_SHA256` a workflow of the directory pins."""
    return [
        (path.name, digest)
        for path in workflow_files(directory)
        for digest in re.findall(r"PROTOC_SHA256: ([0-9a-f]+)", workflow_file_text(path))
    ]


class OnlyAPushSavesACache(unittest.TestCase):
    def test_the_cache_scan_reads_a_workflow_saved_with_the_yaml_suffix(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yaml").write_text(PLANTED_CACHES, encoding="utf-8")
            problems, _ = cache_scan(Path(scratch))
        self.assertIn(
            "planted.yaml:build:combined", [problem.split(": ", 1)[0] for problem in problems]
        )

    def test_the_protoc_pin_scan_reads_a_workflow_saved_with_the_yaml_suffix(self):
        with tempfile.TemporaryDirectory() as scratch:
            (Path(scratch) / "planted.yaml").write_text("env:\n  PROTOC_SHA256: 0123abcd\n")
            pins = protoc_pins(Path(scratch))
        self.assertEqual(pins, [("planted.yaml", "0123abcd")])

    def test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile(self):
        workflow = load("ci.yml")
        compiling = sorted(
            {job for job, named in stage_calls(workflow) if COMPILES_RUST & set(named)}
        )
        restored = {}
        for job_id in examined("jobs that compile Rust", compiling):
            steps = workflow["jobs"][job_id]["steps"]
            gate = next(n for n, s in enumerate(steps) if GATE_CALL.search(str(s.get("run", ""))))
            toolchain = [
                n for n, s in enumerate(steps) if str(s.get("run", "")).strip() == "rustup show"
            ]
            self.assertTrue(
                toolchain and toolchain[0] < gate, f"{job_id} installs no pinned toolchain"
            )
            restores = [
                (n, s)
                for n, s in enumerate(steps)
                if action(s) == "actions/cache/restore" and paths(s) == RUST_CACHE
            ]
            self.assertEqual(len(restores), 1, f"{job_id} restores no Rust cache before its stages")
            at, restore = restores[0]
            self.assertLess(at, gate, f"{job_id} restores the Rust cache after its stages")
            key = restore["with"]["key"]
            self.assertIn("${{ hashFiles('rust-toolchain.toml') }}", key)
            self.assertIn("${{ hashFiles('Cargo.lock') }}", key)
            fallback = key.split("${{ hashFiles('Cargo.lock') }}")[0]
            self.assertEqual(lines_of(restore["with"]["restore-keys"]), [fallback])
            restored[job_id] = (gate, restore)
        # One job saves it: the one that builds every target, after its stages.
        saves = [
            (job_id, n, s)
            for job_id, job in workflow["jobs"].items()
            for n, s in enumerate(job.get("steps") or [])
            if action(s) == "actions/cache/save" and paths(s) == RUST_CACHE
        ]
        self.assertEqual(len(saves), 1, f"the Rust cache is saved by {len(saves)} steps")
        job_id, at, save = saves[0]
        self.assertIn(job_id, restored, f"{job_id} saves a Rust cache it never restored")
        gate, restore = restored[job_id]
        self.assertIn("clippy", dict(stage_calls(workflow))[job_id], f"{job_id} builds no target")
        self.assertGreater(at, gate, f"{job_id} saves the Rust cache before its stages")
        primary = "${{ steps." + restore["id"] + ".outputs.cache-primary-key }}"
        self.assertEqual(save["with"]["key"], primary)
        # The workspace's own artifacts are rebuilt from any fresh checkout: clean them first.
        clean = workflow["jobs"][job_id]["steps"][at - 1]
        self.assertEqual(str(clean.get("run", "")).strip(), "cargo clean --workspace")
        self.assertEqual(clean.get("if"), save.get("if"))

    def test_a_cache_is_saved_only_by_a_push_to_dev_or_main(self):
        problems, saves = cache_scan(WORKFLOWS)
        self.assertEqual(problems, [])
        examined("cache saves", saves)
        problems, _ = cache_problems("planted.yml", read_workflow(PLANTED_CACHES))
        self.assertEqual(
            [problem.split(": ", 1)[0] for problem in problems],
            [
                "planted.yml:build:combined",
                "planted.yml:build:node",
                *["planted.yml:build:unconditional"] * 6,
                "planted.yml:build:any push",
                "planted.yml:build:any push",
            ],
        )
        self.assertIn(
            "planted.yml:build:unconditional: saves on a fork's pull request into dev", problems
        )
        self.assertIn("planted.yml:build:any push: saves on a pushed tag", problems)

    def test_only_a_superseded_pull_request_run_is_cancelled(self):
        ci = workflow_file_text(WORKFLOWS / "ci.yml")
        self.assertEqual(
            len(re.findall(r"(?m)^\s*concurrency:", ci)), 1, "a job sets its own group"
        )
        judged = examined("concurrency blocks", [load("ci.yml")["concurrency"]])[0]
        self.assertEqual(concurrency_problems(judged), [])
        # The old block let two pushes to dev share a group, and cancelled no pull-request run.
        old = {"group": "ci-${{ github.ref }}", "cancel-in-progress": "false"}
        self.assertEqual(
            concurrency_problems(old),
            [
                "two pushes to dev share a group, so a third would cancel the pending one",
                "a newer run of a pull request leaves the superseded one running",
            ],
        )

    def test_the_browser_cache_is_keyed_on_the_locked_playwright_version(self):
        workflow = load("ci.yml")
        caches = [
            (job_id, at, step)
            for job_id, job in workflow["jobs"].items()
            for at, step in enumerate(job.get("steps") or [])
            if action(step) == "actions/cache/restore" and paths(step) == BROWSERS
        ]
        lockfile = (REPO / "pnpm-lock.yaml").read_text(encoding="utf-8")
        locked = re.search(
            r"(?m)^      '@playwright/test':\n        specifier: .*\n        version: ([0-9.]+)$",
            lockfile,
        ).group(1)
        for job_id, at, restore in examined("browser caches", caches):
            steps = workflow["jobs"][job_id]["steps"]
            source = re.search(
                r"\$\{\{ steps\.([\w-]+)\.outputs\.([\w-]+) \}\}", restore["with"]["key"]
            )
            self.assertIsNotNone(source, "the browser cache's key reads no step's output")
            step = next(s for s in steps if s.get("id") == source.group(1))
            code, outputs = run_step(step, lockfile)
            self.assertEqual((code, outputs.get(source.group(2))), (0, locked))
            moved = lockfile.replace(f"playwright-core@{locked}", "playwright-core@9.8.7")
            self.assertEqual(run_step(step, moved), (0, {source.group(2): "9.8.7"}))
            # A lockfile that locks no Playwright, or two, fails the step: no key from nothing.
            self.assertEqual(run_step(step, "lockfileVersion: '9.0'\n")[0], 1)
            two = moved + f"\n  playwright-core@{locked}:\n    resolution: {{}}\n"
            self.assertEqual(run_step(step, two)[0], 1)
            installs = [
                s
                for s in steps[at + 1 :]
                if "playwright install chromium --only-shell" in str(s.get("run", ""))
            ]
            self.assertEqual(len(installs), 1, f"{job_id} installs no browser after the restore")
            hit = {f"steps.{restore['id']}.outputs.cache-hit": "true"}
            self.assertTrue(condition(installs[0]["if"], pull_request("dev")))
            self.assertFalse(condition(installs[0]["if"], dict(pull_request("dev"), **hit)))

    def test_every_job_uploads_its_stage_logs_from_a_visible_directory(self):
        workflow = load("ci.yml")
        names = []
        gate_jobs = sorted({job for job, _ in stage_calls(workflow)})
        for job_id in examined("jobs that run check.sh", gate_jobs):
            job = workflow["jobs"][job_id]
            steps = job["steps"]
            at = next(n for n, s in enumerate(steps) if GATE_CALL.search(str(s.get("run", ""))))
            env = dict(job.get("env") or {}, **(steps[at].get("env") or {}))
            logs = str(env.get("CHECK_LOG_DIR", "")).rstrip("/")
            self.assertTrue(logs, f"{job_id} writes its logs to a temporary directory nobody reads")
            uploads = [s for s in steps[at + 1 :] if action(s) == "actions/upload-artifact"]
            self.assertEqual(len(uploads), 1, f"{job_id} uploads no stage logs")
            inputs = uploads[0]["with"]
            hidden = [part for part in logs.split("/") if part.startswith(".")]
            if inputs.get("include-hidden-files") != "true":
                self.assertEqual(hidden, [], f"{job_id}: upload-artifact skips {hidden}")
            uploaded = str(inputs["path"]).rstrip("/").replace("${{ github.workspace }}/", "")
            self.assertEqual(uploaded, logs.replace("${{ github.workspace }}/", ""), job_id)
            self.assertEqual(uploads[0].get("if"), "${{ always() }}", job_id)
            names.append(inputs["name"])
        self.assertEqual(sorted(set(names)), sorted(names), "two jobs upload one artifact name")


class TheEngineBuildsInEveryRustJob(unittest.TestCase):
    def test_every_job_that_compiles_rust_installs_the_pinned_protoc_first(self):
        workflow = load("ci.yml")
        compiling = sorted(
            {job for job, named in stage_calls(workflow) if COMPILES_RUST & set(named)}
        )
        for job_id in examined("jobs that compile Rust", compiling):
            steps = workflow["jobs"][job_id]["steps"]
            gate = next(n for n, s in enumerate(steps) if GATE_CALL.search(str(s.get("run", ""))))
            installs = [
                (n, s)
                for n, s in enumerate(steps)
                if "protoc" in str(s.get("run", "")) and "sha256sum -c" in str(s.get("run", ""))
            ]
            self.assertEqual(len(installs), 1, f"{job_id} installs no checksum-verified protoc")
            at, step = installs[0]
            self.assertLess(at, gate, f"{job_id} installs protoc after its stages")
            self.assertIn(PROTOC_ARCHIVE, step["run"], job_id)
            self.assertEqual((step.get("env") or {}).get("PROTOC_SHA256"), PROTOC_SHA256, job_id)
            self.assertIn('echo "$PROTOC_SHA256 ', step["run"], f"{job_id} checks another digest")
            self.assertIn('>> "$GITHUB_PATH"', step["run"], f"{job_id} puts no protoc on PATH")
        # Every workflow that pins protoc pins ADR-022's digest, engine-measure.yml's cold build too.
        for name, digest in examined("protoc pins", protoc_pins(WORKFLOWS)):
            self.assertEqual(digest, PROTOC_SHA256, name)


def concurrency_problems(block):
    """What a workflow-level concurrency block gets wrong, judged in scenarios: two runs of one pull
    request, two of another, pushes to dev and main, and a pull request beside a push."""
    group = block.get("group", "")
    cancel = block.get("cancel-in-progress", "false")
    first, newer = pull_request("dev", run_id="101"), pull_request("dev", run_id="102")
    other = pull_request("dev", number="8", run_id="103")
    dev, dev_again = push("refs/heads/dev", run_id="201"), push("refs/heads/dev", run_id="202")
    main = push("refs/heads/main", run_id="301")
    problems = []
    if rendered(group, dev) == rendered(group, dev_again):
        problems.append("two pushes to dev share a group, so a third would cancel the pending one")
    if condition(cancel, dev) or condition(cancel, main):
        problems.append("a push cancels the run before it")
    if rendered(group, first) != rendered(group, newer) or not condition(cancel, newer):
        problems.append("a newer run of a pull request leaves the superseded one running")
    if rendered(group, first) == rendered(group, other):
        problems.append("two pull requests share a group, so one cancels the other")
    if rendered(group, first) in (rendered(group, dev), rendered(group, main)):
        problems.append("a pull request shares a push's group")
    return problems


def run_step(step, lockfile):
    """Run a step's own script under GitHub's default bash, in a directory holding `lockfile` as
    pnpm-lock.yaml. Returns its exit code and the outputs it wrote."""
    with tempfile.TemporaryDirectory() as scratch:
        where = Path(scratch)
        (where / "pnpm-lock.yaml").write_text(lockfile, encoding="utf-8")
        output = where / "github-output"
        output.write_text("", encoding="utf-8")
        env = {"PATH": os.environ["PATH"], "GITHUB_OUTPUT": str(output)}
        done = subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", step["run"]],
            cwd=where,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        pairs = [line.split("=", 1) for line in output.read_text().splitlines() if "=" in line]
    return done.returncode, dict(pairs)


def run_output_check(step, plant):
    """Run an output check's own script under GitHub's default bash, in a directory holding
    `plant`, {relative path: text}, with REPORT naming an empty report directory (SPEC-346 R5).
    Returns its exit code and {report file: its text, stripped}."""
    with tempfile.TemporaryDirectory() as scratch:
        where = Path(scratch)
        for relative, text in plant.items():
            (where / relative).parent.mkdir(parents=True, exist_ok=True)
            (where / relative).write_text(text, encoding="utf-8")
        report = where / "report"
        report.mkdir()
        env = {"PATH": os.environ["PATH"], "REPORT": str(report)}
        done = subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", step["run"]],
            cwd=where,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        written = {}
        for each in sorted(report.iterdir()):
            written[each.name] = each.read_text().strip()
    return done.returncode, written


# ------------------------------------------------------- the engine job (SPEC-038 A18, R14)

# The job that runs the engine set, SPEC-022's slow sync and budget tests, beside rust.
ENGINE_JOB = "engine"
# The engine job's timeout, sized from its measured runs (SPEC-038 section 8): a cold run, which
# compiles every dependency before about 140 s of tests, takes about five minutes, so the timeout
# holds at least two cold runs and ends a hung sync test within half an hour, not six hours.
ENGINE_TIMEOUT_MINUTES = range(10, 31)
# The apple harness job's timeout (SPEC-361 A16): the card view's planted suite alone ran about
# fifty minutes on a hosted macOS runner and the whole job projects to 85 to 98 minutes, so the
# band holds about one and a half runs and stays at most half the hosted job's limit.
HARNESS_TIMEOUT_MINUTES = range(150, 181)
# The release job's timeout (SPEC-367 A1): its slowest measured run was cancelled at the old bound
# after 60:26, so the band starts at one and a half times that run, rounded up to the minute, and
# ends at twice it, rounded down to the ten, so a hung test still ends within two hours.
RELEASE_TIMEOUT_MINUTES = range(91, 121)
# The two web mutation legs' timeout (SPEC-379 R1, R2): the weekly battery's `web` job and the
# pull request's `mutation-web` job each run StrykerJS over up to the whole Mini App. The band
# starts at the bound its rule gives, the worst measured cost per mutant times the larger measured
# count, one and a half times, plus the set-up and the report, rounded up to the five, so a leg set
# back to 60 is refused; it ends at twice the projected whole sweep, rounded down to the ten, so a
# hung test still ends within two hours.
WEB_MUTATION_TIMEOUT_MINUTES = range(100, 121)


def engine_job_problems(workflow):
    """What a workflow gets wrong about the engine job (SPEC-038 R14): test-engine run anywhere but
    the engine job alone, an engine job that waits on another or can be skipped, a ci that does not
    need it, no cargo-nextest before its stage, a cache it saves or a Rust cache it does not
    restore, incremental builds, or a timeout outside the measured band."""
    jobs = workflow.get("jobs") or {}
    runs = [(job, named) for job, named in stage_calls(workflow) if "test-engine" in named]
    problems = []
    if runs != [(ENGINE_JOB, ["test-engine"])]:
        problems.append(f"test-engine runs in {runs}, not in the engine job alone")
    job = jobs.get(ENGINE_JOB)
    if job is None:
        return [*problems, "there is no engine job"]
    if job.get("needs") is not None:
        problems.append("the engine job waits on another job")
    if "if" in job:
        problems.append("the engine job can be skipped, and a skipped need fails ci")
    if ENGINE_JOB not in ((jobs.get("ci") or {}).get("needs") or []):
        problems.append("ci does not need the engine job, so the engine's tests are not required")
    steps = job.get("steps") or []
    gate = next((n for n, s in enumerate(steps) if GATE_CALL.search(str(s.get("run", "")))), None)
    nextest = [
        n
        for n, s in enumerate(steps)
        if action(s) == "taiki-e/install-action"
        and "cargo-nextest"
        in [t.strip() for t in str((s.get("with") or {}).get("tool")).split(",")]
    ]
    if gate is None or not nextest or nextest[0] > gate:
        problems.append("the engine job installs no cargo-nextest before its stage")
    caches = [(action(s), paths(s)) for s in steps if action(s).startswith("actions/cache")]
    if caches != [("actions/cache/restore", RUST_CACHE)]:
        problems.append(
            f"the engine job's caches are {[c for c, _ in caches]}: it restores the Rust cache, "
            "and saves none"
        )
    if str((job.get("env") or {}).get("CARGO_INCREMENTAL")) != "0":
        problems.append("the engine job builds incrementally")
    minutes = str(job.get("timeout-minutes") or "")
    if not minutes.isdigit() or int(minutes) not in ENGINE_TIMEOUT_MINUTES:
        band = f"{ENGINE_TIMEOUT_MINUTES.start} to {ENGINE_TIMEOUT_MINUTES.stop - 1}"
        problems.append(f"the engine job's timeout is {minutes or 'unset'}, not {band} minutes")
    return problems


PLANTED_ENGINE = """\
jobs:
  engine:
    runs-on: ubuntu-24.04
    if: ${{ github.event_name == 'push' }}
    needs: [rust]
    steps:
      - uses: actions/cache@0123456789abcdef0123456789abcdef01234567
        with:
          path: target/
          key: build
      - run: bash scripts/check.sh test-engine test
  ci:
    needs: [rust]
"""


class TheEngineRunsBesideRust(unittest.TestCase):
    def test_the_engine_job_runs_the_engine_set_beside_the_rust_job(self):
        workflow = load("ci.yml")
        examined("check.sh calls", stage_calls(workflow))
        self.assertEqual(engine_job_problems(workflow), [])
        # The judge refuses an engine job that breaks every rule at once.
        self.assertEqual(
            engine_job_problems(read_workflow(PLANTED_ENGINE)),
            [
                "test-engine runs in [('engine', ['test-engine', 'test'])], not in the engine job "
                "alone",
                "the engine job waits on another job",
                "the engine job can be skipped, and a skipped need fails ci",
                "ci does not need the engine job, so the engine's tests are not required",
                "the engine job installs no cargo-nextest before its stage",
                "the engine job's caches are ['actions/cache']: it restores the Rust cache, and "
                "saves none",
                "the engine job builds incrementally",
                "the engine job's timeout is unset, not 10 to 30 minutes",
            ],
        )


# How a leg of the engine job names its slice of the engine set to test-engine (SPEC-038 R16): its
# own number, of the matrix's size.
LEG_SLICE = "${{ matrix.slice }}/${{ strategy.job-total }}"


def slice_problems(workflow):
    """What a workflow gets wrong about the engine set's slices (SPEC-038 R16): a matrix that is not
    one `slice` dimension counting 1 to N, with N of at least 2; a leg that does not hand its own
    slice to test-engine as m/N; or one leg's failure cancelling the others."""
    job = (workflow.get("jobs") or {}).get(ENGINE_JOB) or {}
    strategy = job.get("strategy") or {}
    matrix = strategy.get("matrix") or {}
    slices = matrix.get("slice") if isinstance(matrix, dict) else None
    problems = []
    if not isinstance(matrix, dict) or set(matrix) != {"slice"} or not isinstance(slices, list):
        problems.append(f"the engine job's matrix is {matrix}, not one slice dimension")
        slices = []
    if len(slices) < 2 or [str(s) for s in slices] != [str(n) for n in range(1, len(slices) + 1)]:
        problems.append(f"the engine job's slices are {slices}, not 1 to N with N of at least 2")
    if str(strategy.get("fail-fast")) != "false":
        problems.append("one slice's failure cancels the other slices")
    gates = [s for s in job.get("steps") or [] if GATE_CALL.search(str(s.get("run", "")))]
    handed = [(s.get("env") or {}).get("ENGINE_SLICE") for s in gates]
    if handed != [LEG_SLICE]:
        problems.append(f"the engine job hands test-engine the slices {handed}, not [{LEG_SLICE}]")
    return problems


PLANTED_SLICES = """\
jobs:
  engine:
    strategy:
      matrix:
        slice: [1, 1, 3]
    steps:
      - env:
          ENGINE_SLICE: ${{ matrix.slice }}/2
        run: bash scripts/check.sh test-engine
"""


class TheEngineSetRunsInSlices(unittest.TestCase):
    def test_the_engine_job_runs_each_slice_of_the_engine_set_once(self):
        workflow = load("ci.yml")
        self.assertEqual(slice_problems(workflow), [])
        matrix = workflow["jobs"][ENGINE_JOB]["strategy"]["matrix"]
        examined("slices of the engine set", matrix["slice"])
        # The judge refuses a slice run twice and another never, a leg that names a slice of the
        # wrong count, and a failure that cancels the other legs.
        self.assertEqual(
            slice_problems(read_workflow(PLANTED_SLICES)),
            [
                "the engine job's slices are ['1', '1', '3'], not 1 to N with N of at least 2",
                "one slice's failure cancels the other slices",
                "the engine job hands test-engine the slices ['${{ matrix.slice }}/2'], not "
                "[${{ matrix.slice }}/${{ strategy.job-total }}]",
            ],
        )


# ------------------------------------------------- the web engine job (SPEC-338 A16, R8, R9)

# The job that builds the web engine's module, holds it to ADR-336's budget and runs the browser
# tests over it (ADR-349).
WEB_ENGINE_JOB = "web-engine"
# Its three steps, each found by the command its script runs, in the order the job must run them:
# the build, the size gate over the two files the build wrote, then the browser tests over the
# module the gate measured.
WEB_ENGINE_ORDER = (
    ("the build", "bash scripts/web-engine-build.sh"),
    ("the size gate", "python3 scripts/web-engine-size.py"),
    ("the browser tests", "test:engine"),
)
# The job's timeout: a cold wasm32 release build of the engine, the browsers' install and the
# browser tests, with room for a slow run, and a hung browser ended within the hour.
WEB_ENGINE_TIMEOUT_MINUTES = range(20, 61)
# The release archives the job downloads: wasm-bindgen's CLI, whose version is in its URL twice,
# and binaryen's, which carries wasm-opt.
BINDGEN_RELEASE = re.compile(
    r"/wasm-bindgen/releases/download/([^/\s]+)/wasm-bindgen-\1-x86_64-unknown-linux-musl\.tar\.gz"
)
BINARYEN_RELEASE = re.compile(r"/binaryen/releases/download/[^/\s]+/binaryen-[^/\s]+\.tar\.gz")


def bindgen_version():
    """The wasm-bindgen version the root manifest pins exactly (`=X.Y.Z`). The CLI that writes the
    bindings must be that version, or its bindings do not match the module (ADR-349), so the
    manifest is the oracle, not the workflow."""
    manifest = tomllib.loads((REPO / "Cargo.toml").read_text(encoding="utf-8"))
    pin = manifest["workspace"]["dependencies"]["wasm-bindgen"]
    pin = pin if isinstance(pin, str) else str(pin.get("version", ""))
    if not re.fullmatch(r"=\d+\.\d+\.\d+", pin):
        raise AssertionError(f"the root manifest pins wasm-bindgen as {pin!r}, not exactly")
    return pin[1:]


def sha256_checked(step, variable):
    """Whether a step checks a download against the full digest in its own `variable`: the digest
    is 64 hex characters, and a line of its script hands it to `sha256sum -c -`."""
    digest = str((step.get("env") or {}).get(variable, ""))
    check = rf'echo "\${variable}  \S+" \| sha256sum -c -'
    return bool(re.fullmatch(r"[0-9a-f]{64}", digest)) and any(
        re.fullmatch(check, line) for line in lines_of(step.get("run"))
    )


def web_engine_job_problems(workflow):
    """What a workflow gets wrong about the web engine's job (SPEC-338 R8, ADR-349): no such job,
    one that waits on another or can be skipped, a ci that does not need it, the build, the size
    gate and the browser tests missing or out of order, no wasm32 target added before the build, a
    wasm-bindgen that is not the crate's exact version or a download whose digest is not checked, a
    `RUSTFLAGS` or a `.cargo/config` anywhere in it (the build's settings are on its script's
    command line only, ADR-348), incremental builds, or a timeout outside its band."""
    jobs = workflow.get("jobs") or {}
    job = jobs.get(WEB_ENGINE_JOB)
    if job is None:
        return ["there is no web-engine job"]
    problems = []
    if job.get("needs") is not None:
        problems.append("the web-engine job waits on another job")
    if "if" in job:
        problems.append("the web-engine job can be skipped, and a skipped need fails ci")
    if WEB_ENGINE_JOB not in ((jobs.get("ci") or {}).get("needs") or []):
        problems.append(
            "ci does not need the web-engine job, so the module's budget is not required"
        )
    steps = job.get("steps") or []
    runs = [str(step.get("run", "")) for step in steps]
    found = sorted(
        (at, name)
        for name, command in WEB_ENGINE_ORDER
        for at, run in enumerate(runs)
        if command in run
    )
    ran = [name for _, name in found]
    wanted = [name for name, _ in WEB_ENGINE_ORDER]
    missing = [name for name in wanted if name not in ran]
    problems += [f"the web-engine job runs no {name.removeprefix('the ')}" for name in missing]
    if not missing and ran != wanted:
        problems.append(
            f"the web-engine job runs {', '.join(ran)}, in that order, not {', '.join(wanted)}"
        )
    build = next((at for at, name in found if name == "the build"), len(steps))
    if not any("rustup target add wasm32-unknown-unknown" in lines_of(run) for run in runs[:build]):
        problems.append("the web-engine job adds no wasm32-unknown-unknown target before its build")
    version = bindgen_version()
    bindgen = [
        (s, m.group(1)) for s in steps for m in BINDGEN_RELEASE.finditer(str(s.get("run", "")))
    ]
    if not bindgen:
        problems.append("the web-engine job installs no wasm-bindgen release")
    for step, installed in bindgen:
        if installed != version:
            problems.append(
                f"the web-engine job installs wasm-bindgen {installed}, not the crate's {version}"
            )
        if not sha256_checked(step, "WASM_BINDGEN_SHA256"):
            problems.append("the web-engine job does not check wasm-bindgen's sha256")
    binaryen = [s for s in steps if BINARYEN_RELEASE.search(str(s.get("run", "")))]
    if not binaryen:
        problems.append("the web-engine job installs no binaryen release")
    if not all(sha256_checked(step, "BINARYEN_SHA256") for step in binaryen):
        problems.append("the web-engine job does not check binaryen's sha256")
    held = json.dumps([workflow.get("env"), job])
    if "RUSTFLAGS" in held or ".cargo/config" in held:
        problems.append(
            "the web-engine job sets RUSTFLAGS or writes a .cargo/config, where the build takes "
            "its settings on its script's command line only"
        )
    if str((job.get("env") or {}).get("CARGO_INCREMENTAL")) != "0":
        problems.append("the web-engine job builds incrementally")
    minutes = str(job.get("timeout-minutes") or "")
    if not minutes.isdigit() or int(minutes) not in WEB_ENGINE_TIMEOUT_MINUTES:
        band = f"{WEB_ENGINE_TIMEOUT_MINUTES.start} to {WEB_ENGINE_TIMEOUT_MINUTES.stop - 1}"
        problems.append(f"the web-engine job's timeout is {minutes or 'unset'}, not {band} minutes")
    return problems


def planted_job(text, job_id, *changes):
    """A workflow read from `text` with each (old, new) applied in turn inside one job's block. Each
    `old` must occur exactly once in the block when its turn comes, so a plant that changes nothing,
    or changes a line of another job, fails by name instead of passing."""
    block = re.search(rf"(?ms)^  {re.escape(job_id)}:\n.*?(?=^  [a-z-]+:\n|\Z)", text)
    if block is None:
        raise AssertionError(f"the workflow has no {job_id} job to plant in")
    planted = block.group(0)
    for old, new in changes:
        if planted.count(old) != 1:
            raise AssertionError(f"the {job_id} job holds {old!r} {planted.count(old)} times")
        planted = planted.replace(old, new)
    return read_workflow(text[: block.start()] + planted + text[block.end() :])


# The steps the web-engine job runs after its browser tests, each found by the command it runs, in
# the order the job must run them: the app's build, the stage that puts the module and its bindings
# beside it, then the study suite over both (SPEC-350 R13, A21).
WEB_ENGINE_STUDY_ORDER = (
    ("the app build", "pnpm --dir web/app build"),
    ("the stage", "bash scripts/web-engine-stage.sh"),
    ("the study suite", "pnpm --dir web/app test:study"),
)


def web_engine_study_problems(workflow):
    """What a workflow gets wrong about the web-engine job's study suite (SPEC-350 R13, A21): the
    app's build, the stage or the study suite missing or out of order, any of them before the
    browser tests over the module, or an expression inside one of their commands."""
    job = (workflow.get("jobs") or {}).get(WEB_ENGINE_JOB)
    if job is None:
        return ["there is no web-engine job"]
    runs = [str(step.get("run", "")) for step in job.get("steps") or []]
    found = sorted(
        (at, name)
        for name, command in WEB_ENGINE_STUDY_ORDER
        for at, run in enumerate(runs)
        if command in run
    )
    ran = [name for _, name in found]
    wanted = [name for name, _ in WEB_ENGINE_STUDY_ORDER]
    missing = [name for name in wanted if name not in ran]
    problems = [f"the web-engine job runs no {name.removeprefix('the ')}" for name in missing]
    if not missing and ran != wanted:
        problems.append(
            f"the web-engine job runs {', '.join(ran)}, in that order, not {', '.join(wanted)}"
        )
    browsers = next((at for at, run in enumerate(runs) if "test:engine" in run), None)
    if browsers is not None and any(at < browsers for at, _ in found):
        problems.append(
            "the web-engine job builds the app, stages the module or runs the study suite before "
            "its browser tests"
        )
    if any("${{" in runs[at] for at, _ in found):
        problems.append("a study step of the web-engine job holds an expression in its command")
    return problems


class TheWebEngineIsHeldToItsBudget(unittest.TestCase):
    def test_the_web_engine_job_builds_the_module_and_holds_it_to_its_budget(self):
        text = workflow_file_text(WORKFLOWS / "ci.yml")
        workflow = read_workflow(text)
        job = workflow["jobs"].get(WEB_ENGINE_JOB) or {}
        examined("web-engine job steps", job.get("steps") or [])
        self.assertEqual(web_engine_job_problems(workflow), [])
        # Each planted copy of the real job breaks one rule, and the judge refuses it by name.
        version = bindgen_version()
        size, browsers = "python3 scripts/web-engine-size.py", "pnpm --dir web/app test:engine"
        target = "rustup target add wasm32-unknown-unknown\n"
        incremental = 'CARGO_INCREMENTAL: "0"\n'
        flags = (
            "the web-engine job sets RUSTFLAGS or writes a .cargo/config, where the build takes "
            "its settings on its script's command line only"
        )
        plants = {
            "no web-engine job": (
                "web-engine",
                [("  web-engine:\n", "  web-engine-gone:\n")],
                ["there is no web-engine job"],
            ),
            "a job that waits on another": (
                "web-engine",
                [("  web-engine:\n", "  web-engine:\n    needs: [rust]\n")],
                ["the web-engine job waits on another job"],
            ),
            "a job that can be skipped": (
                "web-engine",
                [
                    (
                        "  web-engine:\n",
                        "  web-engine:\n    if: ${{ github.event_name == 'push' }}\n",
                    )
                ],
                ["the web-engine job can be skipped, and a skipped need fails ci"],
            ),
            "a ci that does not need it": (
                "ci",
                [(", web-engine,", ",")],
                ["ci does not need the web-engine job, so the module's budget is not required"],
            ),
            "the size gate after the browser tests": (
                "web-engine",
                [(size, "SWAPPED"), (browsers, size), ("SWAPPED", browsers)],
                [
                    "the web-engine job runs the build, the browser tests, the size gate, in that "
                    "order, not the build, the size gate, the browser tests"
                ],
            ),
            "no size gate": (
                "web-engine",
                [(size, "python3 -c pass")],
                ["the web-engine job runs no size gate"],
            ),
            "no wasm32 target": (
                "web-engine",
                [(target, "rustup target list --installed\n")],
                ["the web-engine job adds no wasm32-unknown-unknown target before its build"],
            ),
            "another wasm-bindgen": (
                "web-engine",
                [
                    (
                        f"download/{version}/wasm-bindgen-{version}-",
                        "download/0.0.1/wasm-bindgen-0.0.1-",
                    )
                ],
                [f"the web-engine job installs wasm-bindgen 0.0.1, not the crate's {version}"],
            ),
            "an unchecked wasm-bindgen": (
                "web-engine",
                [('echo "$WASM_BINDGEN_SHA256  /tmp/wasm-bindgen.tgz" | sha256sum -c -', "true")],
                ["the web-engine job does not check wasm-bindgen's sha256"],
            ),
            "an unchecked binaryen": (
                "web-engine",
                [('echo "$BINARYEN_SHA256  /tmp/binaryen.tgz" | sha256sum -c -', "true")],
                ["the web-engine job does not check binaryen's sha256"],
            ),
            "RUSTFLAGS in the job's env": (
                "web-engine",
                [(incremental, incremental + "      RUSTFLAGS: -Ctarget-feature=+simd128\n")],
                [flags],
            ),
            "a .cargo/config written": (
                "web-engine",
                [(target, target + "          mkdir -p .cargo && touch .cargo/config.toml\n")],
                [flags],
            ),
            "an incremental build": (
                "web-engine",
                [(incremental, 'CARGO_INCREMENTAL: "1"\n')],
                ["the web-engine job builds incrementally"],
            ),
            "a timeout outside the band": (
                "web-engine",
                [(f"timeout-minutes: {job.get('timeout-minutes')}\n", "timeout-minutes: 360\n")],
                ["the web-engine job's timeout is 360, not 20 to 60 minutes"],
            ),
        }
        for name, (job_id, changes, refusal) in examined(
            "planted web-engine defect(s)", plants.items()
        ):
            with self.subTest(name):
                planted = planted_job(text, job_id, *changes)
                self.assertEqual(web_engine_job_problems(planted), refusal)

    def test_the_web_engine_job_runs_the_study_suite(self):
        text = workflow_file_text(WORKFLOWS / "ci.yml")
        workflow = read_workflow(text)
        self.assertEqual(web_engine_study_problems(workflow), [])
        job = workflow["jobs"].get(WEB_ENGINE_JOB) or {}
        examined("web-engine job steps", job.get("steps") or [])
        # Each planted copy of the real job breaks one rule, and the judge refuses it by name.
        build, stage, study = (command for _, command in WEB_ENGINE_STUDY_ORDER)
        browsers = "pnpm --dir web/app test:engine"
        order = (
            "the web-engine job runs {}, in that order, not the app build, the stage, the study "
            "suite"
        )
        early = (
            "the web-engine job builds the app, stages the module or runs the study suite before "
            "its browser tests"
        )
        plants = {
            "no app build": ([(build, "true")], ["the web-engine job runs no app build"]),
            "no stage": ([(stage, "true")], ["the web-engine job runs no stage"]),
            "no study suite": ([(study, "true")], ["the web-engine job runs no study suite"]),
            "the stage before the app build": (
                [(stage, "SWAPPED"), (build, stage), ("SWAPPED", build)],
                [order.format("the stage, the app build, the study suite")],
            ),
            "the study suite before the browser tests": (
                [(study, "SWAPPED"), (browsers, study), ("SWAPPED", browsers)],
                [order.format("the study suite, the app build, the stage"), early],
            ),
            "an expression in a study step": (
                [(study, study + ' -- --project "${{ github.event_name }}"')],
                ["a study step of the web-engine job holds an expression in its command"],
            ),
        }
        for name, (changes, refusal) in examined(
            "planted web-engine study defect(s)", plants.items()
        ):
            with self.subTest(name):
                planted = planted_job(text, WEB_ENGINE_JOB, *changes)
                self.assertEqual(web_engine_study_problems(planted), refusal)


# ------------------------------------------ no secret, no other repository (SPEC-034 A9 to A12)

# The planted workflows: those the checker refuses (A10) and those it admits (A11).
PLANTED = REPO / "scripts" / "tests" / "fixtures" / "secrets-and-checkouts"
PLANTED_HARDENING = REPO / "scripts" / "tests" / "fixtures" / "workflow-hardening"
# The hardening tests read a workflow's keys the way the checker does (SPEC-034 R7). A13 plants each
# of these in the hardened control, beside the live workflows, as (the test that judges it, the
# control's line, the lines planted in its place, the refusal the test raises): keys the reader
# reads, a block read whole, a trigger, and forms the reader does not read, which fail closed.
HARDENING_PIN, HARDENING_RUNNER, HARDENING_TOKEN = (
    "test_every_action_is_pinned_by_a_full_commit_sha",
    "test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger",
    "test_every_workflow_defaults_to_a_read_only_token",
)
CONTROL_STEP = "      - uses: actions/checkout@0123456789abcdef0123456789abcdef01234567"
PLANTED_KEYS = (
    (
        HARDENING_PIN,
        CONTROL_STEP,
        "      - 'uses': actions/checkout@v4",
        r"planted\.yml uses actions/checkout@v4$",
    ),
    (
        HARDENING_PIN,
        CONTROL_STEP,
        '      - "uses": actions/checkout@v4',
        r"planted\.yml uses actions/checkout@v4$",
    ),
    (
        HARDENING_PIN,
        CONTROL_STEP,
        '      - "us\\x65s": actions/checkout@v4',
        r"^planted\.yml: line 17 was not read",
    ),
    (
        HARDENING_PIN,
        CONTROL_STEP,
        '      - name: planted\n        "us\\x65s": actions/checkout@v4',
        r"^planted\.yml: the reader does not read line 17: a key that is not a plain name",
    ),
    (
        HARDENING_RUNNER,
        "    runs-on: ubuntu-24.04",
        "    'runs-on': self-hosted",
        r"planted\.yml runs on self-hosted$",
    ),
    (
        HARDENING_RUNNER,
        "    runs-on: ubuntu-24.04",
        '    "runs-on": [self-hosted, linux]',
        r"planted\.yml runs on \['self-hosted', 'linux'\]$",
    ),
    (
        HARDENING_RUNNER,
        "    runs-on: ubuntu-24.04",
        "    runs-on: macos-26",
        r"planted\.yml runs on macos-26$",
    ),
    (
        HARDENING_RUNNER,
        "  pull_request:",
        '  "pull_request\\x5ftarget":',
        r"^planted\.yml: the reader does not read line 6: a key that is not a plain name",
    ),
    (
        HARDENING_RUNNER,
        "  pull_request:",
        "  pull_request_target:",
        r"unexpectedly found in .* : planted\.yml$",
    ),
    (
        HARDENING_TOKEN,
        "  contents: read",
        "  contents: read\n  pull-requests: write",
        r"planted\.yml defaults its token to \{'contents': 'read', 'pull-requests': 'write'\}$",
    ),
    (
        HARDENING_TOKEN,
        "  contents: read",
        '  contents: read\n  "pull\\x2drequests": write',
        r"^planted\.yml: the reader does not read line 11: a key that is not a plain name",
    ),
)
# The characters the reader refuses (SPEC-034 R7), which A10 plants at test time from these escapes,
# so no committed file holds one: white space that YAML reads as text, and characters that end a
# line of a script where YAML reads none.
PLANTED_SPACES = {
    "no-break space": "\xa0",
    "em space": "\N{EM SPACE}",
    "narrow no-break space": "\N{NARROW NO-BREAK SPACE}",
    "ideographic space": "\N{IDEOGRAPHIC SPACE}",
}
PLANTED_BREAKS = {
    "form feed": "\x0c",
    "line tabulation": "\x0b",
    "file separator": "\x1c",
    "group separator": "\x1d",
    "record separator": "\x1e",
    "next line": "\x85",
    "line separator": "\N{LINE SEPARATOR}",
    "paragraph separator": "\N{PARAGRAPH SEPARATOR}",
}
# A planted job's first lines; its steps follow.
PLANTED_JOB = """\
name: planted
on:
  pull_request:
    branches: [dev]
permissions:
  contents: read
jobs:
  build:
    runs-on: ubuntu-24.04
    steps:
"""
# A planted workflow whose lines each hold <C>, one of the characters above: a secret or a clone
# that YAML reads there, beside a flow list, a quoted value and a key that the character leaves
# unread. The names are synthetic.
PLANTED_CHARACTERS = (
    PLANTED_JOB
    + """\
      - env:
          AFTER: planted<C>#${{ secrets.EXAMPLE_TOKEN }}
          BEFORE: <C>*${{ secrets.EXAMPLE_TOKEN }}
          COLON: ${{ secrets.EXAMPLE_TOKEN }}:<C>
          LISTED: [planted]<C># planted
          QUOTED: 'planted'<C># ${{ secrets.EXAMPLE_TOKEN }}
        run: false<C>#||git clone https://github.com/example-org/other-repository.git
      - name: planted
        run<C>: git clone https://github.com/example-org/other-repository.git
      - run: |
          echo planted
          planted<C>#||git clone https://github.com/example-org/other-repository.git
"""
)
# Planted workflows with a line the reader cannot place, and the refusal each raises: a block it
# does not read, before a clone, and a plain value over two lines, the second a secret. The names
# are synthetic.
PLANTED_UNPLACED = {
    "a folded block": (
        PLANTED_JOB
        + """\
      - run: >
          echo planted
      - run: git clone https://github.com/example-org/other-repository.git
""",
        r"^line 12 was not read",
    ),
    "a plain value over two lines": (
        PLANTED_JOB
        + """\
      - env:
          PLANTED: planted
            ${{ secrets.EXAMPLE_TOKEN }}
        run: echo planted
""",
        r"^line 13 was not read",
    ),
}
# Planted workflows with a line the reader cannot place, and the refusal each raises: a continuation
# line that begins with <C>, and a line of <C> alone in a mapping and in a block.
PLANTED_UNPLACED_CHARACTERS = {
    "a continuation": (
        PLANTED_JOB
        + """\
      - env:
          CONTINUED: planted
            <C># ${{ secrets.EXAMPLE_TOKEN }}
        run: echo planted
""",
        r"^line 13 was not read",
    ),
    "a line in a mapping": (
        PLANTED_JOB
        + """\
      - env:
          PLANTED: planted
          <C>
        run: echo planted
""",
        r"^line 13 is not a mapping entry",
    ),
    "a line in a block": (
        PLANTED_JOB
        + """\
      - run: |
          echo planted
<C>
          git clone https://github.com/example-org/other-repository.git
""",
        r"^line 13 is not a mapping entry",
    ),
}
# Planted `|` blocks that each hold a line of <C> alone, one column less indented than the block's
# other lines, and a secret whose name the block writes over two lines, as (the planted workflow,
# the line of <C>, the secret's name as the checker reports it). The character check refuses the
# line, and the block's end and indent read only a space or a tab as white space, so the line is the
# block's text: at the block's end and inside it, it sets the indent the name keeps. The names are
# synthetic.
PLANTED_BLOCK_LINES = {
    "a line of <C> at a block's end": (
        PLANTED_JOB
        + """\
      - run: |
          echo ${{ secrets['EXAMPLE
          TOKEN'] }}
         <C>
""",
        14,
        "EXAMPLE\n TOKEN",
    ),
    "a line of <C> inside a block": (
        PLANTED_JOB
        + """\
      - run: |
          echo ${{ secrets['EXAMPLE
         <C>
          TOKEN'] }}
""",
        13,
        "EXAMPLE\n<C>\n TOKEN",
    ),
}
# The secrets context in an expression, in any case: `secrets.NAME` (group 1), `secrets['NAME']`
# (group 2), or the context whole, which names no secret: `toJSON(secrets)`, `secrets.*`, or an
# index computed at run time.
SECRET = re.compile(r"(?<![\w.-])secrets(?![\w-])(?:\.([A-Za-z_][\w-]*)|\['([^']*)'\])?", re.I)
# The one table of admitted secret reads (owner ruling #668; SPEC-352 R14; ADR-363): each
# TestFlight lane's `app` job reads each part of the credential only in the step that uses it, by
# the step's id, in that step's own variable: the preflight as a presence test, the signing and
# upload steps as the value. Every other read stays a problem, and so does every read of a lane
# that a pull request or another workflow's run starts (UNADMITTED_TRIGGERS).
ADMITTED_SECRETS = {
    (lane, "app", step): frozenset(reads)
    for lane in ("testflight-internal.yml", "testflight-release.yml")
    for step, reads in (
        (
            "preflight",
            (
                "secrets.TESTFLIGHT_UPLOAD_KEY",
                "secrets.TESTFLIGHT_UPLOAD_KEY_ID",
                "secrets.TESTFLIGHT_UPLOAD_ISSUER_ID",
                "secrets.IOS_DIST_CERTIFICATE",
                "secrets.IOS_DIST_CERTIFICATE_PASSWORD",
                "secrets.IOS_PROVISIONING_PROFILE",
            ),
        ),
        (
            "sign",
            (
                "secrets.IOS_DIST_CERTIFICATE",
                "secrets.IOS_DIST_CERTIFICATE_PASSWORD",
                "secrets.IOS_PROVISIONING_PROFILE",
            ),
        ),
        (
            "upload",
            (
                "secrets.TESTFLIGHT_UPLOAD_KEY",
                "secrets.TESTFLIGHT_UPLOAD_KEY_ID",
                "secrets.TESTFLIGHT_UPLOAD_ISSUER_ID",
            ),
        ),
    )
}
# The re-release check's reads (SPEC-374 R14): the four parts of its own credential, the presence
# test in the one step that tests them, the values in the one step that uses them.
RERELEASE_CHECK = "testflight-rerelease-check.yml"
RERELEASE_PARTS = (
    "secrets.TESTFLIGHT_CHECK_KEY",
    "secrets.TESTFLIGHT_CHECK_KEY_ID",
    "secrets.TESTFLIGHT_CHECK_ISSUER_ID",
    "secrets.TESTFLIGHT_CHECK_APP_ID",
)
ADMITTED_SECRETS[(RERELEASE_CHECK, "rerelease", "preflight")] = frozenset(RERELEASE_PARTS)
ADMITTED_SECRETS[(RERELEASE_CHECK, "rerelease", "read")] = frozenset(RERELEASE_PARTS)
# The triggers under which no read is admitted: a pull request's code, or another run's.
UNADMITTED_TRIGGERS = ("pull_request", "pull_request_target", "workflow_run")
# A command that clones a repository, and a git command given a URL: one with a scheme, or git's
# scp-like form, `user@host:path`, or `host:path` whose host is a dotted name. A refspec, such as
# `main:refs/heads/main` or `v1.0:refs/tags/v1.0`, names no host.
CLONE = re.compile(r"\bgit\b[^;&|]*?\bclone\b|\bgh\s+repo\s+clone\b")
GIT_URL = re.compile(
    r"\bgit\b[^;&|]*?(?:\w+://|(?<![\w/.:@-])"
    r"(?:[\w.-]+@[\w.-]+|[\w-]+(?:\.[\w-]+)*\.[A-Za-z][\w-]*):)"
)
# GitHub's built-in shell keywords, as written. The runner runs any other `shell` as a command
# template, its first word the command and `{0}` the script's path, so it runs a command the
# checker does not read.
SHELLS = ("bash", "sh", "pwsh", "powershell", "python", "cmd")
# A variable git reads, named in any case: git takes configuration, and commands it runs, from
# variables whose names begin with `GIT_`, such as `GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_<n>`,
# `GIT_CONFIG_VALUE_<n>`, `GIT_CONFIG_PARAMETERS` and `GIT_SSH_COMMAND`.
GIT_VARIABLE = re.compile(r"(?<![A-Za-z0-9_])GIT_[A-Za-z0-9_]*", re.I)
# The properties of a job's container, in GitHub's workflow schema, that the runner creates the
# container from, besides its `env`: its `image`, `options`, `ports` and `volumes`. Its registry
# `credentials` are read to pull the image, and no step reads them.
CONTAINER_CREATED_WITH = ("image", "options", "ports", "volumes")


def expressions_in(text):
    """Every `${{ }}` expression in a value, as GitHub delimits one: a `}}` inside a quoted string
    does not close it (an escaped `''` toggles the quote twice), and one never closed runs to the
    value's end."""
    found, at = [], 0
    while (start := text.find("${{", at)) >= 0:
        at, quoted = start + 3, False
        while at < len(text) and (quoted or not text.startswith("}}", at)):
            if text[at] == "'":
                quoted = not quoted
            at += 1
        found.append(text[start + 3 : at].strip())
        at += 2
    return found


def strings(value, where=""):
    """(place, text) for every string a read workflow holds, its place dotted from the root, as in
    `jobs.build.steps[0].env.TOKEN`."""
    if isinstance(value, dict):
        return [
            found
            for key, item in value.items()
            for found in strings(item, f"{where}.{key}" if where else key)
        ]
    if isinstance(value, list):
        return [found for n, item in enumerate(value) for found in strings(item, f"{where}[{n}]")]
    return [(where, value)] if isinstance(value, str) else []


def texts(value, where=""):
    """(place, text) for every key and every string a read workflow holds, in its order, a key
    placed as its value is, as in `jobs.build.steps[0].env.GIT_SSH_COMMAND`."""
    if isinstance(value, dict):
        found = []
        for key, item in value.items():
            place = f"{where}.{key}" if where else str(key)
            found += [(place, str(key)), *texts(item, place)]
        return found
    if isinstance(value, list):
        return [found for n, item in enumerate(value) for found in texts(item, f"{where}[{n}]")]
    return [(where, value)] if isinstance(value, str) else []


def step_inputs(step):
    """A step's `with` inputs, each name in lower case: the runner reads an input's name in any
    case. Inputs that are not a mapping are none here, and the checker refuses a checkout whose
    inputs are not a mapping: GitHub evaluates a `with` that is one `${{ }}` expression when the
    step runs, so no reading of the file names what it holds. An omitted or empty `with` is no
    inputs."""
    given = step.get("with")
    if not isinstance(given, dict):
        return {}
    return {str(name).lower(): value for name, value in given.items()}


def steps_in(steps, where):
    """(place, step) for every step of a list of steps, a step inside a `parallel` block at any
    depth included, each placed as in `jobs.build.steps[0].parallel[1]`: GitHub's workflow schema
    reads a `parallel` step's list as steps, and one of them may be another `parallel` step."""
    for n, step in enumerate(steps if isinstance(steps, list) else []):
        place = f"{where}[{n}]"
        yield place, step
        if isinstance(step, dict):
            yield from steps_in(step.get("parallel"), f"{place}.parallel")


def is_checkout(step):
    """Whether a step uses actions/checkout, its `uses` read as the runner reads it: split at each
    `/` and `\\`, empty parts dropped, and the owner and name in any case. An action at a path
    inside that repository is a checkout too."""
    parts = [
        part for part in re.split(r"[/\\]", str(step.get("uses", "")).split("@", 1)[0]) if part
    ]
    return [part.lower() for part in parts[:2]] == ["actions", "checkout"]


def checked_out(step):
    """The repository a checkout step checks out. An omitted or empty `repository`, and
    `${{ github.repository }}`, are this repository, as actions/checkout defaults it."""
    given = str(step_inputs(step).get("repository") or "").strip()
    if not given or re.fullmatch(r"\$\{\{\s*github\.repository\s*\}\}", given):
        return THIS_REPOSITORY
    return given


def checked_out_from(step):
    """The server a checkout step checks out from when it is another, else ''. An omitted or empty
    `github-server-url`, and `${{ github.server_url }}`, are this server, as actions/checkout
    defaults it."""
    given = str(step_inputs(step).get("github-server-url") or "").strip()
    return "" if re.fullmatch(r"\$\{\{\s*github\.server_url\s*\}\}", given) else given


def secret_reads(expression):
    """What an expression reads from the secrets context, other than GITHUB_TOKEN. GitHub reads a
    secret's name without case, so `secrets.github_token` is the default token too."""
    found = []
    for match in SECRET.finditer(expression):
        name = match.group(1) if match.group(1) is not None else match.group(2)
        if name is None:
            found.append("reads the whole secrets context, or a secret named at run time")
        elif name.upper() != "GITHUB_TOKEN":
            found.append(f"reads the secret {name}")
    return found


def admitted_secret_places(name, workflow):
    """{place: the secrets admitted there} for the workflow file `name`, each place a variable of
    a step's own `env` that ADMITTED_SECRETS names by file, job and step id, spelt as `strings`
    places it. A workflow whose `on`, read as a mapping, a list or one name, holds any of
    UNADMITTED_TRIGGERS has no admitted place."""
    on = workflow.get("on")
    triggers = on if isinstance(on, (dict, list)) else [on]
    if any(str(trigger) in UNADMITTED_TRIGGERS for trigger in triggers):
        return {}
    places = {}
    jobs = workflow.get("jobs")
    for job_id, job in (jobs if isinstance(jobs, dict) else {}).items():
        if not isinstance(job, dict):
            continue
        for where, step in steps_in(job.get("steps"), f"jobs.{job_id}.steps"):
            step_id = step.get("id") if isinstance(step, dict) else None
            admitted = (
                ADMITTED_SECRETS.get((name, job_id, step_id)) if isinstance(step_id, str) else None
            )
            if admitted is not None and isinstance(step.get("env"), dict):
                places.update({f"{where}.env.{key}": admitted for key in step["env"]})
    return places


def admitted_read(admitted, text, expression):
    """Whether the read in `expression`, the value `text` holds, is admitted: at an admitted
    place, the value exactly that one expression, and the expression `secrets.<NAME>` or
    `secrets.<NAME> != ''` for a secret admitted there. Any other form is not admitted."""
    if admitted is None or text != f"${{{{ {expression} }}}}":
        return False
    return expression.removesuffix(" != ''") in admitted


def commands(script):
    """A run script's commands, one per line: a line continued with a backslash is joined to the
    next, and each command's whitespace is collapsed."""
    lines = script.replace("\\\n", " ").splitlines()
    return [" ".join(line.split()) for line in lines if line.strip()]


def reaches(script):
    """Each command of a run script, or of any other string, that clones a repository or gives git a
    URL: a workflow reaches this repository through actions/checkout and origin, and no other."""
    found = []
    for command in commands(script):
        if CLONE.search(command):
            found.append(f"clones a repository: {command}")
        elif GIT_URL.search(command):
            found.append(f"points git at a URL: {command}")
    return found


def shell_problems(given, where):
    """A `shell` that is not one of GitHub's built-in keywords, as written. The runner runs any
    other as a command, so what a custom shell runs is a command the checker does not read. An
    omitted or empty `shell` is none: the runner falls back to the defaults, which are judged
    where they are."""
    if given is None or given == "" or given in SHELLS:
        return []
    return [f"{where}: runs a shell the checker does not read: {given}"]


def defaults_problems(defaults, where):
    """A `defaults.run.shell` that `shell_problems` refuses, and defaults whose shell the checker
    cannot read: a `defaults` or a `defaults.run` that is set and is not a mapping, such as one
    `${{ }}` expression, which GitHub evaluates when the job runs."""
    if defaults is None:
        return []
    if not isinstance(defaults, dict):
        return [f"{where}: runs a shell the checker does not read: {defaults}"]
    run = defaults.get("run")
    if run is None:
        return []
    if not isinstance(run, dict):
        return [f"{where}.run: runs a shell the checker does not read: {run}"]
    return shell_problems(run.get("shell"), f"{where}.run.shell")


def environment_problems(env, where):
    """An environment whose variables the checker cannot read: an `env` that is set and is not a
    mapping, such as one `${{ }}` expression, which GitHub evaluates when the job or the step runs,
    so no reading of the file names a variable whose name begins with `GIT_` there. An omitted or
    empty `env` is no variables."""
    if env is None or isinstance(env, dict):
        return []
    return [f"{where}: sets an environment the checker does not read"]


def container_problems(container, where):
    """A job's container whose environment the checker cannot read: one `${{ }}` expression, an
    `env` that `environment_problems` refuses, or one of `CONTAINER_CREATED_WITH` that is or holds
    a `${{ }}` expression, which GitHub evaluates when the job runs. The steps of a job with a
    container run inside it, in its environment. A container named by its image alone has no `env`
    to read."""
    if container is None or (isinstance(container, str) and "${{" not in container):
        return []
    if isinstance(container, dict):
        problems = environment_problems(container.get("env"), f"{where}.env")
        for name in CONTAINER_CREATED_WITH:
            if "${{" in str(container.get(name, "")):
                problems.append(f"{where}.{name}: runs in a container the checker does not read")
        return problems
    return [f"{where}: runs in a container the checker does not read"]


def git_variables(text):
    """Each variable whose name begins with `GIT_` that a key or a string names, once, in order."""
    return list(dict.fromkeys(GIT_VARIABLE.findall(text)))


def secret_and_checkout_problems(directory):
    """Every read of a secret other than GITHUB_TOKEN and other than a read ADMITTED_SECRETS admits
    (one whole `${{ secrets.<NAME> }}` or `${{ secrets.<NAME> != '' }}` value in a step's own env),
    every `secrets: inherit`, and every checkout, clone or fetch of another repository in the
    workflows of `directory`, each named by its file and its place, with what was judged:
    (problems, {population: [...]}). Every step of a job is judged, a step inside a `parallel`
    block at any depth included, and a checkout whose inputs are
    not a mapping is a problem, as `step_inputs` says. A clone or a fetch is read in every string
    the workflow holds, not only a run step's script. A shell that is not a built-in keyword, git
    configured from the environment, and an environment the checker cannot read are problems too,
    as `shell_problems`, `defaults_problems`, `environment_problems`, `container_problems` and
    `git_variables` say. A form the reader does not read is a problem named by its line, and the
    rest of that file is judged as read. A directory with no workflow file is VOID, never a
    pass."""
    files = workflow_files(directory)
    problems = []
    judged = {"expressions": [], "checkouts": [], "run steps": []}
    for path in files:
        try:
            workflow, refused = read_workflow(workflow_file_text(path)), []
        except Unread as unread:
            workflow, refused = unread.workflow, unread.refused
        problems += [f"{path.name}:{why}" for why in refused]
        admitted = admitted_secret_places(path.name, workflow)
        for where, text in strings(workflow):
            for expression in expressions_in(text):
                judged["expressions"].append((f"{path.name}:{where}", expression))
                if admitted_read(admitted.get(where), text, expression):
                    continue
                problems += [f"{path.name}:{where}: {read}" for read in secret_reads(expression)]
        problems += defaults_problems(workflow.get("defaults"), f"{path.name}:defaults")
        problems += environment_problems(workflow.get("env"), f"{path.name}:env")
        scripts = set()
        for job_id, job in (workflow.get("jobs") or {}).items():
            # A job or step that is not a mapping is not judged: the reader has named its line, or
            # GitHub refuses the workflow.
            if not isinstance(job, dict):
                continue
            if job.get("secrets") == "inherit":
                problems.append(
                    f"{path.name}:jobs.{job_id}.secrets: passes every secret to the workflow it "
                    "calls"
                )
            problems += defaults_problems(
                job.get("defaults"), f"{path.name}:jobs.{job_id}.defaults"
            )
            problems += environment_problems(job.get("env"), f"{path.name}:jobs.{job_id}.env")
            problems += container_problems(
                job.get("container"), f"{path.name}:jobs.{job_id}.container"
            )
            for where, step in steps_in(job.get("steps"), f"{path.name}:jobs.{job_id}.steps"):
                if not isinstance(step, dict):
                    continue
                if is_checkout(step):
                    if step.get("with") is not None and not isinstance(step["with"], dict):
                        problems.append(
                            f"{where}: checks out with inputs the checker does not read"
                        )
                    repository = checked_out(step)
                    judged["checkouts"].append((where, repository))
                    if repository != THIS_REPOSITORY:
                        problems.append(f"{where}: checks out {repository}, not this repository")
                    if server := checked_out_from(step):
                        problems.append(f"{where}: checks out from another server: {server}")
                if "run" in step:
                    judged["run steps"].append(where)
                    scripts.add(f"{where}.run")
                    problems += [f"{where}: {reach}" for reach in reaches(str(step["run"]))]
                problems += shell_problems(step.get("shell"), f"{where}.shell")
                problems += environment_problems(step.get("env"), f"{where}.env")
        # Every other string is read for the same commands: a value a shell runs, such as
        # `BASH_ENV`, which bash expands before a step's script, holds a command as a script does.
        for where, text in strings(workflow):
            if f"{path.name}:{where}" not in scripts:
                problems += [f"{path.name}:{where}: {reach}" for reach in reaches(text)]
        # A variable whose name begins with `GIT_`, set or named anywhere, configures git from the
        # environment: an `env` key at any level, a container's options, or a script that
        # exports one.
        for where, text in texts(workflow):
            problems += [
                f"{path.name}:{where}: names a git variable: {name}" for name in git_variables(text)
            ]
    return problems, judged


def planted_problems(text):
    """What the checker finds in one planted workflow, written to a scratch directory at test time
    (SPEC-034 A10)."""
    with tempfile.TemporaryDirectory() as scratch:
        (Path(scratch) / "planted.yml").write_text(text, encoding="utf-8")
        return secret_and_checkout_problems(Path(scratch))[0]


# The census that keeps every read of a workflow file on the one loader (SPEC-190 R12, A12). It is
# default-deny over a population it derives when it runs, never over a list of modules. The
# population is every module under the test directory that imports the loader's module, directly or
# through any chain of imports and re-exports, every module those import, and `_support.py`. In it,
# every site that can read a file, a stream or a process's output, by any spelling, and every call
# whose callee is not a name, is red unless listed. In the whole directory, every site that can
# import a module, run code or reach a namespace by a name held in data is red unless listed, and so
# is every name, module and attribute the census has not classified. A listed site is bound to its
# module, its qualified name and its own text, and to how many times that text occurs there.
LOADER = (Path(__file__).stem, workflow_file_text.__qualname__)
# Members that read a file, a stream or a socket, matched in any position of a dotted name.
READ_NAMES = frozenset(
    {
        "BufferedRWPair",
        "BufferedRandom",
        "BufferedReader",
        "FileIO",
        "FileInput",
        "FileType",
        "IncrementalNewlineDecoder",
        "StreamReader",
        "StreamReaderWriter",
        "StringIO",
        "TextIOWrapper",
        "copy_file_range",
        "extractfile",
        "fdopen",
        "fileinput",
        "fromfile_prefix_chars",
        "get_data",
        "getline",
        "getlines",
        "getreader",
        "input",
        "linecache",
        "makefile",
        "mmap",
        "open",
        "open_code",
        "pread",
        "preadv",
        "read",
        "read1",
        "read_bytes",
        "read_text",
        "readall",
        "readinto",
        "readline",
        "readlines",
        "readv",
        "recv",
        "recv_into",
        "recvfrom",
        "recvfrom_into",
        "recvmsg",
        "sendfile",
        "sourcehook",
        "splice",
        "stdin",
        "urlopen",
    }
)
# Members that start a process, whose output a module could read; `os`'s own only after `os`.
PROCESS_NAMES = frozenset(
    {"create_subprocess_exec", "create_subprocess_shell", "pty", "subprocess"}
)
OS_PROCESS_NAMES = frozenset(
    {
        "execl",
        "execle",
        "execlp",
        "execlpe",
        "execv",
        "execve",
        "execvp",
        "execvpe",
        "fork",
        "forkpty",
        "popen",
        "posix_spawn",
        "posix_spawnp",
        "spawnl",
        "spawnle",
        "spawnlp",
        "spawnlpe",
        "spawnv",
        "spawnve",
        "spawnvp",
        "spawnvpe",
        "system",
    }
)
# Members that import a module, run code or reach a namespace, matched in any position: the import
# system's own, a frame's and a code object's, and the loaders that take a module by its name.
DYNAMIC_NAMES = frozenset(
    {
        "SourceFileLoader",
        "SourcelessFileLoader",
        "TestProgram",
        "_dot_lookup",
        "_get_target",
        "_importer",
        "ag_code",
        "ag_frame",
        "attrgetter",
        "cr_code",
        "cr_frame",
        "discover",
        "enable_load_extension",
        "exec_module",
        "f_back",
        "f_builtins",
        "f_code",
        "f_globals",
        "f_locals",
        "find_module",
        "find_spec",
        "findTestCases",
        "get_code",
        "get_field",
        "gi_code",
        "gi_frame",
        "import_module",
        "load_extension",
        "load_module",
        "loadTestsFromModule",
        "loadTestsFromName",
        "loadTestsFromNames",
        "methodcaller",
        "module_from_spec",
        "resolve_name",
        "run_module",
        "run_path",
        "source_to_code",
        "spec_from_file_location",
        "spec_from_loader",
        "tb_frame",
    }
)
# Pairs that read or change where an import reads: `sys.path` and the `shlex` lexer.
READ_PAIRS = frozenset({("shlex", "shlex"), ("sys", "path")})
SYS_DYNAMIC = frozenset({"_getframe", "meta_path", "modules", "path_hooks", "path_importer_cache"})
# Builtins that reach an attribute by a name: a constant name is read as that attribute; a name
# held in data is dynamic, and a rebinding of a read or an import member is dynamic too.
ATTRIBUTE_BUILTINS = frozenset({"delattr", "getattr", "setattr"})
REBINDING = frozenset({"delattr", "setattr"})
# Builtins that read, counted wherever they are named, bound or not.
READ_BUILTINS = frozenset({"input", "open"})
# Builtins that import, run code, or reach a namespace by a name held in data.
BARE_DYNAMIC = frozenset(
    {
        "breakpoint",
        "compile",
        "eval",
        "exec",
        "globals",
        "help",
        "locals",
        "vars",
    }
)
# Every other builtin a module may name: a value, a type or a function of what it is given, which
# imports, runs and reads nothing. The exception classes are not named here: `census_problems` derives
# them from the interpreter's builtins and places each by its value, whatever this list names of them.
# A name bound nowhere in its module and in neither is red.
BENIGN_BUILTINS = frozenset(
    {
        "AssertionError",
        "BaseException",
        "Exception",
        "FileNotFoundError",
        "IndexError",
        "KeyboardInterrupt",
        "OSError",
        "PermissionError",
        "ProcessLookupError",
        "SystemExit",
        "TypeError",
        "UnicodeDecodeError",
        "ValueError",
        "abs",
        "all",
        "any",
        "bool",
        "bytearray",
        "bytes",
        "callable",
        "chr",
        "dict",
        "dir",
        "enumerate",
        "float",
        "format",
        "frozenset",
        "hasattr",
        "id",
        "int",
        "isinstance",
        "iter",
        "len",
        "list",
        "map",
        "max",
        "min",
        "next",
        "object",
        "ord",
        "print",
        "property",
        "range",
        "repr",
        "reversed",
        "set",
        "sorted",
        "staticmethod",
        "str",
        "sum",
        "super",
        "tuple",
        "type",
        "zip",
    }
)
# The dunder names a module may use: each names a value, never a namespace or the import system.
BENIGN_DUNDERS = frozenset(
    {"__doc__", "__enter__", "__file__", "__init__", "__name__", "__qualname__"}
)
# The standard modules the census reads through: each imports and runs nothing by a name held in
# data, and reads only through the members named above. Any other module's every use is a site.
VETTED_MODULES = {
    "argparse": "parses the arguments it is given; it reads a file only through `FileType` or `fromfile_prefix_chars`, both named",
    "ast": "parses and walks source it is given as text; it runs nothing",
    "collections": "containers of values",
    "contextlib": "context managers over what it is given; a read under `chdir` is still a named read",
    "copy": "copies a value it is given",
    "dataclasses": "declares classes from names checked as identifiers",
    "datetime": "dates and times",
    "fnmatch": "matches names against patterns",
    "fractions": "exact rational numbers",
    "functools": "wraps a callable it is given; a read it wraps is a named read where it is named",
    "hashlib": "digests bytes it is given, or a file object only an `open` can make",
    "http.server": "serves a directory over a socket; what reaches a module comes through a named read",
    "io": "streams; each that reads or translates a file or text is named",
    "ipaddress": "addresses",
    "itertools": "iterators over what it is given",
    "json": "parses text it is given, or a file object only an `open` can make",
    "math": "numbers",
    "os": "the system's calls; each that reads or starts a process is named",
    "pathlib": "paths; each member that reads is named",
    "plistlib": "parses property-list bytes it is given, or a file object only an `open` can make",
    "posixpath": "path strings",
    "re": "patterns over text it is given",
    "shlex": "splits and quotes text it is given; the `shlex` lexer, which can open a file it names, is named",
    "shutil": "copies and removes files, reading none into the module",
    "signal": "signals",
    "sqlite3": "a database file; a workflow is no database, and loading an extension is named",
    "stat": "file modes",
    "string": "constants and formats; the formatter's `get_field`, which reaches an attribute by name, is named",
    "subprocess": "processes; every member is a named read",
    "sys": "the interpreter; its module table, import hooks, frames, path and stdin are named",
    "tarfile": "archives; `open` and `extractfile` are named",
    "tempfile": "scratch files and directories; a read of one is a named read",
    "textwrap": "text it is given",
    "threading": "threads that run a callable the module names",
    "time": "clocks",
    "tokenize": "tokens of text a named `open` or `readline` gives it",
    "tomllib": "parses text it is given, or a file object only an `open` can make",
    "types": "type objects; a code object to build a function from comes only through a named site",
    "unicodedata": "character properties",
    "unittest": "tests; a loader or patch that takes a module or target by name is named",
    "uuid": "identifiers",
}

Census = collections.namedtuple(
    "Census", "imports aliases bound sites loader scopes assignments declared"
)


def scoped(tree):
    """Every node of `tree` with the qualified name of the function, class or lambda whose body
    holds it, as Python's `__qualname__` spells it, or `<module>`: a decorator, a default and a
    base class belong to the scope that evaluates them. Also each function's, class's and lambda's
    own qualified name."""
    found, own, todo = [], {}, [(tree, "<module>", "")]
    while todo:
        node, qual, prefix = todo.pop()
        found.append((node, qual))
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef, ast.Lambda)):
            name = prefix + ("<lambda>" if isinstance(node, ast.Lambda) else node.name)
            own[node] = name
            inner = name + ("." if isinstance(node, ast.ClassDef) else ".<locals>.")
            body = node.body if isinstance(node.body, list) else [node.body]
            for child in ast.iter_child_nodes(node):
                held = any(child is statement for statement in body)
                todo.append((child, name, inner) if held else (child, qual, prefix))
        else:
            todo.extend((child, qual, prefix) for child in ast.iter_child_nodes(node))
    return found, own


def is_dunder(name):
    """Whether a name is a dunder, `__x__`."""
    return len(name) > 4 and name.startswith("__") and name.endswith("__")


def bindings(node):
    """The names a node binds in its scope, each with how it binds it."""
    if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.ClassDef)):
        return [(node.name, "a definition")]
    if isinstance(node, ast.Name) and not isinstance(node.ctx, ast.Load):
        return [(node.id, "an assignment")]
    if isinstance(node, ast.arg):
        return [(node.arg, "an argument")]
    if isinstance(node, (ast.MatchAs, ast.MatchStar, ast.ExceptHandler)) and node.name:
        return [(node.name, "a capture")]
    if isinstance(node, ast.MatchMapping) and node.rest:
        return [(node.rest, "a capture")]
    if isinstance(node, (ast.TypeVar, ast.ParamSpec, ast.TypeVarTuple)):
        return [(node.name, "a type parameter")]
    if isinstance(node, (ast.Global, ast.Nonlocal)):
        return [(name, "a global or nonlocal") for name in node.names]
    if isinstance(node, (ast.Import, ast.ImportFrom)):
        return [
            (alias.asname or alias.name.split(".")[0], "an import")
            for alias in node.names
            if alias.name != "*"
        ]
    return []


def classified(names):
    """(reads, dynamic) for names in attribute position: whether one reads a file, a stream or a
    process's output, and whether one imports, runs code or reaches a namespace."""
    return (
        any(name in READ_NAMES or name in PROCESS_NAMES for name in names),
        any(
            name in DYNAMIC_NAMES or is_dunder(name) and name not in BENIGN_DUNDERS
            for name in names
        ),
    )


def reference(parts, counted, site, called, builtin):
    """(reads, dynamic) for one dotted reference `parts`. Its names are `counted` from its first
    when that is an import or a builtin, and from its attributes when it is a local the module
    binds: a local holds what its binding read, and the binding is where the census counts it.
    `builtin` is its first name when no import binds it."""
    pairs = set(zip(parts, parts[1:]))
    args = site.args if called else []
    text = [each.value for each in args[1:2] if isinstance(each, ast.Constant)]
    named = bool(text) and isinstance(text[0], str)
    if builtin not in ATTRIBUTE_BUILTINS:
        reads, dynamic = classified(counted)
    elif named:
        reads, dynamic = classified(text)
        reads, dynamic = (False, reads or dynamic) if builtin in REBINDING else (reads, dynamic)
    else:
        reads, dynamic = False, True
    reads = (
        reads
        or builtin in READ_BUILTINS
        or any(("os", name) in pairs for name in OS_PROCESS_NAMES)
        or bool(READ_PAIRS & pairs)
    )
    dynamic = (
        dynamic
        or any(("sys", name) in pairs for name in SYS_DYNAMIC)
        or builtin in BARE_DYNAMIC
        or any(is_dunder(part) and part not in BENIGN_DUNDERS for part in parts)
        or ("patch" in parts and any(isinstance(each, ast.Constant) for each in args[:1]))
        or (
            called
            and parts[-2:] == ["patch", "object"]
            and not (named and not any(classified(text)))
        )
        or (called and parts[:2] == ["unittest", "main"] and bool(site.args or site.keywords))
    )
    return reads, dynamic


def parameters(node):
    """The names a function or a lambda takes."""
    given = node.args
    return {
        each.arg
        for each in [
            *given.posonlyargs,
            *given.args,
            *given.kwonlyargs,
            given.vararg,
            given.kwarg,
        ]
        if each is not None
    }


def assigned(node):
    """(target, value) for each binding a node makes from a value: an assignment, a loop's or a
    comprehension's target, and a `with` item's name."""
    if isinstance(node, ast.Assign):
        return [(target, node.value) for target in node.targets]
    if isinstance(node, (ast.AnnAssign, ast.AugAssign, ast.NamedExpr)) and node.value:
        return [(node.target, node.value)]
    if isinstance(node, (ast.For, ast.AsyncFor, ast.comprehension)):
        return [(node.target, node.iter)]
    if isinstance(node, ast.withitem) and node.optional_vars is not None:
        return [(node.optional_vars, node.context_expr)]
    return []


@functools.lru_cache(maxsize=None)
def module_census(source):
    """One module's source as the census reads it: its import statements as (qualified name, text,
    level, module, names, is a from-import), its aliases as {name: (statement, member)}, the names
    it binds, its sites as (qualified name, text, reads, is dynamic, alias, bare name, the names
    the site's text names, the attributes of its reference), its bindings of the loader's name as
    (how, qualified name, text, statement), its scopes as {qualified name: (is a function, the
    names it binds)} and its bindings from a value as (qualified name, names, attributes, the
    sites in the value, the lambdas the value is)."""
    nodes, own = scoped(ast.parse(source))
    parent = {child: node for node, _qual in nodes for child in ast.iter_child_nodes(node)}
    imports, aliases, bound, sites, loader, references = [], {}, set(), [], [], []
    scopes = collections.defaultdict(set)
    at = collections.defaultdict(list)
    # A method's first argument, and a local bound only from a call of a class the module defines
    # once, name an instance; a member its class defines once and nothing in the module stores is
    # that method, read by what the method's body reads, never by its name.
    classes = {name for node, name in own.items() if isinstance(node, ast.ClassDef)}
    members, selves, taken = collections.defaultdict(set), {}, {}
    stores, made = collections.Counter(), collections.defaultdict(list)
    stored = {
        each.attr
        for each, _qual in nodes
        if isinstance(each, ast.Attribute) and not isinstance(each.ctx, ast.Load)
    }
    declared = set()

    def site(qual, node, reads, dynamic, alias=None, bare=None, chain=()):
        at[node].append(len(sites))
        names = frozenset(each.id for each in ast.walk(node) if isinstance(each, ast.Name))
        sites.append((qual, ast.unparse(node), reads, dynamic, alias, bare, names, tuple(chain)))

    for node, qual in nodes:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda)):
            scopes[own[node]] |= parameters(node)
            taken[own[node]] = parameters(node)
        if (
            isinstance(node, ast.Assign)
            and len(node.targets) == 1
            and isinstance(node.targets[0], ast.Name)
            and isinstance(node.value, ast.Call)
            and isinstance(node.value.func, ast.Name)
        ):
            made[qual, node.targets[0].id].append(node.value.func.id)
        for name, how in bindings(node):
            stores[qual if how != "an argument" else "", name] += 1
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and qual in classes:
            members[qual].add(node.name)
            given = [*node.args.posonlyargs, *node.args.args]
            plain = all(
                isinstance(each, ast.Name) and each.id in ("classmethod", "property")
                for each in node.decorator_list
            )
            if given and plain:
                selves[own[node]] = (qual, given[0].arg)
        if isinstance(node, (ast.Global, ast.Nonlocal)):
            declared.update(node.names)
        if isinstance(node, (ast.Import, ast.ImportFrom)):
            is_from = isinstance(node, ast.ImportFrom)
            level, module = (
                getattr(node, "level", 0),
                getattr(node, "module", None) or "",
            )
            names = tuple(alias.name for alias in node.names)
            imports.append((qual, ast.unparse(node), level, module, names, is_from))
            for alias in node.names:
                member = alias.name if is_from or alias.asname else alias.name.split(".")[0]
                if alias.name != "*":
                    aliases[alias.asname or member] = (len(imports) - 1, member)
                if alias.name == LOADER[1] and alias.asname not in (None, LOADER[1]):
                    loader.append(("an import under another name", qual, imports[-1][1], None))
                elif LOADER[1] in (alias.name, alias.asname):
                    loader.append(("an import", qual, imports[-1][1], len(imports) - 1))
        for name, how in bindings(node):
            bound.add(name)
            if how != "an argument":
                scopes[qual].add(name)
            if name == LOADER[1] and how != "an import":
                loader.append((how, qual, ast.unparse(node).split("\n")[0], None))
        if isinstance(node, ast.Constant) and node.value == LOADER[1]:
            loader.append(("the loader's name as a string", qual, ast.unparse(parent[node]), None))
        if isinstance(node, ast.keyword) and node.arg in READ_NAMES:
            site(qual, parent[node], True, False)
        if isinstance(node, ast.Call) and not isinstance(node.func, (ast.Name, ast.Attribute)):
            site(qual, node, True, False)
        if isinstance(node, (ast.Name, ast.Attribute)) and not (
            isinstance(parent.get(node), ast.Attribute) and parent[node].value is node
        ):
            references.append((node, qual))
    for node, qual in references:
        attributes, first = [], node
        while isinstance(first, ast.Attribute):
            attributes.insert(0, first.attr)
            first = first.value
        if not isinstance(node.ctx, ast.Load):
            if attributes[-1:] == [LOADER[1]]:
                loader.append(("an attribute rebinding", qual, ast.unparse(parent[node]), None))
            if attributes and any(classified(attributes[-1:])):
                site(qual, parent[node], False, True)
            continue
        head = first.id if isinstance(first, ast.Name) else ""
        parts = [head, *attributes]
        if head in aliases:
            index, member = aliases[head]
            _qual, _text, _level, module, _names, is_from = imports[index]
            dotted = f"{module}.{member}" if is_from else member
            parts = [*dotted.split("."), *attributes]
        call = parent.get(node)
        called = isinstance(call, ast.Call) and call.func is node
        builtin = head if head and head not in aliases else None
        owner = selves[qual][0] if qual in selves and head == selves[qual][1] else None
        kinds = made.get((qual, head), [])
        scopes_around = [".".join(qual.split(".")[:end]) for end in range(1, qual.count(".") + 2)]
        if (
            head
            and head not in taken.get(qual, ())
            and head not in declared
            and kinds
            and len(kinds) == stores[qual, head]
            and len(set(kinds)) == 1
            and kinds[0] in classes
            and stores["<module>", kinds[0]] == 1
            and kinds[0] not in declared
            and not any(stores[scope, kinds[0]] for scope in scopes_around)
        ):
            owner = kinds[0]
        own_member = bool(
            owner
            and attributes[:1]
            and attributes[0] in members[owner]
            and attributes[0] not in stored
            and stores[owner, attributes[0]] == 1
        )
        counted = attributes[own_member:] if builtin in bound else parts
        reads, dynamic = reference(
            [part for part in parts if part],
            counted,
            call if called else node,
            called,
            builtin,
        )
        alias = head if head in aliases else None
        site(qual, call if called else node, reads, dynamic, alias, builtin, attributes)
        if called:
            at[node].append(len(sites) - 1)
    assignments = []
    for node, qual in nodes:
        for target, value in assigned(node):
            held = {index for each in ast.walk(value) for index in at.get(each, ())}
            assignments.append(
                (
                    qual,
                    frozenset(each.id for each in ast.walk(target) if isinstance(each, ast.Name)),
                    frozenset(
                        each.attr
                        for each in ast.walk(target)
                        if isinstance(each, ast.Attribute) and not isinstance(each.ctx, ast.Load)
                    ),
                    tuple(sorted(held)),
                    (own[value],) if isinstance(value, ast.Lambda) else (),
                )
            )
    functions = {
        name
        for node, name in own.items()
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef, ast.Lambda))
    }
    return Census(
        tuple(imports),
        aliases,
        frozenset(bound),
        tuple(sites),
        tuple(loader),
        {qual: (qual in functions, frozenset(names)) for qual, names in scopes.items()}
        | {qual: (True, frozenset()) for qual in functions - set(scopes)},
        tuple(assignments),
        frozenset(declared),
    )


def module_sources(directory):
    """Every module under `directory` with its source, by its dotted name there."""
    found = {}
    for path in sorted(directory.rglob("*.py")):
        parts = path.relative_to(directory).with_suffix("").parts
        found[".".join(parts[:-1] if parts[-1] == "__init__" else parts)] = path.read_text(
            encoding="utf-8"
        )
    return found


def enclosing(census, qual):
    """The names bound in the function a qualified name is in and in every function around it:
    its arguments and its locals, never a class body's or the module's."""
    parts = qual.split(".<locals>.")
    found = set()
    for end in range(1, len(parts) + 1):
        is_function, names = census.scopes.get(".<locals>.".join(parts[:end]), (False, ()))
        if is_function:
            found |= names
    return found


def builtin_exception_names():
    """Every exception class the interpreter's builtins define, placed by its value: a name whose
    value there is a class deriving from `BaseException` (an alias is a member by its value). A
    class imports, runs and reads nothing by being named in an `except`, a `raise` or an
    `isinstance`."""
    return frozenset(
        name
        for name in dir(builtins)
        if isinstance(getattr(builtins, name), type)
        and BaseException in type.mro(getattr(builtins, name))
    )


def census_problems(directory):
    """Every problem the census finds under `directory`, each naming its module, its qualified name
    and its text; with how many modules it read, how many are in the population and how many sites
    it counted."""
    modules = {name: module_census(source) for name, source in module_sources(directory).items()}
    leads = [list(directory.parts[start:]) for start in range(len(directory.parts))]

    def spelled(dotted):
        """The module under `directory` a dotted name spells, itself or with a leading run of the
        directory's own path taken off (`scripts.tests.x` is `x`), or None."""
        parts = [part for part in dotted.split(".") if part]
        for lead in [[], *leads]:
            if parts[: len(lead)] == lead and ".".join(parts[len(lead) :]) in modules:
                return ".".join(parts[len(lead) :])
        return None

    def named(importer, statement):
        """The dotted names one import statement names, each package on the way included."""
        _qual, _text, level, module, names, is_from = statement
        if not is_from:
            return [
                ".".join(name.split(".")[:end])
                for name in names
                for end in range(1, len(name.split(".")) + 1)
            ]
        package = importer.split(".")[:-1]
        base = package[: len(package) - level + 1] if level else []
        whole = base + [part for part in module.split(".") if part]
        found = [".".join(whole[:end]) for end in range(1, len(whole) + 1)]
        return found + [".".join([*whole, name]) for name in names if name != "*"]

    def importing(importer, statement):
        """The modules under `directory` one import statement may import."""
        found = set()
        for dotted in named(importer, statement):
            want = dotted.split(".")
            found |= {
                other
                for other in modules
                if other.split(".")[-len(want) :] == want
                or want[-len(other.split(".")) :] == other.split(".")
            }
        return found

    edges = {
        name: set().union(*(importing(name, s) for s in census.imports))
        for name, census in modules.items()
    }
    population = {LOADER[0]}
    while grown := {name for name in modules if edges[name] & population} - population:
        population |= grown
    population |= {name for name in modules if name.split(".")[-1] == "_support"}
    scanned = set(population)
    while grown := set().union(*(edges[name] for name in scanned)) - scanned:
        scanned |= grown
    found, problems, loaders, flags, followed, stars = (
        collections.Counter(),
        [],
        0,
        {},
        {},
        {},
    )
    for name, census in modules.items():
        bound = set(census.bound)
        followed[name] = [any(spelled(each) for each in named(name, s)) for s in census.imports]
        stars[name] = set()
        for statement, here in zip(census.imports, followed[name]):
            qual, text, level, module, names, is_from = statement
            tops = [module.split(".")[0]] if is_from else [each.split(".")[0] for each in names]
            if "*" in names and here:
                stars[name] |= importing(name, statement)
                bound |= set().union(
                    *(modules[other].bound for other in importing(name, statement))
                )
            if not here and (
                level or "*" in names or any(top not in sys.stdlib_module_names for top in tops)
            ):
                found["dynamic", name, qual, text] += 1
        flags[name] = []
        for qual, text, reads, dynamic, alias, bare, _names, _chain in census.sites:
            if alias is not None:
                index, member = census.aliases[alias]
                _q, _t, level, module, _names, is_from = census.imports[index]
                dotted = f"{module}.{member}" if is_from else member
                parts = dotted.split(".")
                prefixes = {".".join(parts[:end]) for end in range(1, len(parts) + 1)}
                if not (followed[name][index] or prefixes & set(VETTED_MODULES)):
                    if level == 0 and parts[0] in sys.stdlib_module_names:
                        dynamic = True
                    else:
                        reads = True
            if (
                bare is not None
                and not (reads or dynamic)
                and bare not in bound
                and bare
                not in BENIGN_BUILTINS
                | ATTRIBUTE_BUILTINS
                | BARE_DYNAMIC
                | builtin_exception_names()
                and not is_dunder(bare)
            ):
                problems.append(f"{name}: {qual}: {text}: a name the census cannot place")
            flags[name].append([reads, dynamic, dynamic])
        for how, qual, text, index in census.loader:
            if (name, qual, how) == (LOADER[0], "<module>", "a definition"):
                loaders += 1
            elif not (how == "an import" and index is not None and followed[name][index]):
                problems.append(f"{name}: {qual}: {text}: {how} of the loader's name")
    if loaders != 1:
        problems.append(f"{LOADER[0]}: {LOADER[1]}: {loaders} definitions of the loader, not one")
    # A function in the population whose read depends on what it is given (an argument, a local,
    # `self`), or that imports or runs code, is a reader: every reference to it in the population
    # is a read, so a call that hands it a workflow is a site. So is every function and every
    # module-level value that reads, in a module the population imports whose reads the census
    # does not count. A value a load returns is held: every use of an attribute of it is a read.
    # All to a fixed point, through aliases, re-exports and stars.
    readers, loads, held, held_attributes = set(), set(), collections.defaultdict(set), set()

    def is_held(name, qual, bare):
        """Whether a name used in a scope is one a load's value is bound to there: bound in that
        scope, a function around it or the module, or anywhere when it is declared global."""
        return any(
            each == bare and (scope in ("<module>", qual) or qual.startswith(scope + "."))
            for scope, each in held[name]
        )

    def state():
        """What the fixed point grows, measured."""
        return (
            len(readers),
            len(loads),
            sum(map(len, held.values())),
            len(held_attributes),
            sum(flag[0] + flag[2] for name in scanned for flag in flags[name]),
        )

    while True:
        before = state()
        methods = [
            (qual.rsplit(".", 1)[-1], group is loads)
            for group in (readers, loads)
            for _name, qual in group
            if "." in qual.split(".<locals>.")[-1]
        ]
        simple = {method for method, _load in methods}
        loading = {method for method, load in methods if load}
        for name in sorted(scanned):
            census = modules[name]
            inside = name in population
            for asname, (index, member) in census.aliases.items():
                _q, _t, _level, module, _names, is_from = census.imports[index]
                target = spelled(module) if is_from else None
                if target and followed[name][index]:
                    for group in (readers, loads):
                        if (target, member) in group:
                            group.add((name, asname))
                    if ("<module>", member) in held[target]:
                        held[name].add(("<module>", asname))
            for star in stars[name]:
                for group in (readers, loads):
                    group |= {(name, qual) for module, qual in list(group) if module == star}
                held[name] |= {each for each in held[star] if each[0] == "<module>"}
            for index, site in enumerate(census.sites):
                qual, _text, _reads, _dynamic, alias, bare, names, chain = site
                flag = flags[name][index]
                local = enclosing(census, qual)
                targets = set()
                if bare is not None:
                    scopes = qual.split(".<locals>.")
                    targets |= {
                        (name, ".<locals>.".join(scopes[:end]) + ".<locals>." + bare)
                        for end in range(1, len(scopes) + 1)
                    }
                    if bare not in local:
                        targets |= {(name, bare)} | {(star, bare) for star in stars[name]}
                    if is_held(name, qual, bare):
                        flag[0] = True
                if alias is not None:
                    at, member = census.aliases[alias]
                    _q, _t, _level, module, _names, is_from = census.imports[at]
                    parts = [*(f"{module}.{member}" if is_from else member).split("."), *chain]
                    for end in range(1, len(parts)):
                        target = spelled(".".join(parts[:end]))
                        if target:
                            targets.add((target, parts[end]))
                            if ("<module>", parts[end]) in held[target]:
                                flag[0] = True
                if targets & readers or any(part in simple for part in chain):
                    flag[0] = True
                if targets & loads or any(part in loading for part in chain):
                    flag[2] = True
                if any(part in held_attributes for part in chain):
                    flag[0] = True
                if census.scopes.get(qual, (False, ()))[0] and (name, qual) != LOADER:
                    if flag[0] and (names & local or not inside):
                        readers.add((name, qual))
                    if flag[1]:
                        readers.add((name, qual))
                        loads.add((name, qual))
            for qual, names, attributes, sites, lambdas in census.assignments:
                if any(flags[name][index][2] for index in sites):
                    held[name] |= {
                        ("<module>" if each in census.declared else qual, each) for each in names
                    }
                    held_attributes.update(attributes)
                if not inside and qual == "<module>" and any(flags[name][i][0] for i in sites):
                    readers.update((name, target) for target in names)
                for each in lambdas:
                    if (name, each) in readers:
                        readers.update((name, target) for target in names)
        if state() == before:
            break
    for name, census in modules.items():
        for (qual, text, *_rest), (reads, dynamic, _load) in zip(census.sites, flags[name]):
            if dynamic or (reads and name in population):
                found["dynamic" if dynamic else "read", name, qual, text] += 1
    listed, counted = collections.Counter(), collections.Counter()
    for table in (NOT_WORKFLOW_READS, DYNAMIC_IMPORTS):
        for key, (count, _why) in table.items():
            listed[key] += count
    for (_kind, name, qual, text), count in found.items():
        counted[name, qual, text] += count
    for key in sorted(set(counted) | set(listed)):
        if counted[key] != listed[key]:
            kinds = "/".join(sorted({kind for kind, *rest in found if tuple(rest) == key}))
            problems.append(
                f"{': '.join(key)}: {counted[key]} {kinds} site(s), {listed[key]} listed"
            )
    return problems, len(modules), len(population), sum(found.values())


def allowed(reason, *sites):
    """{(module, qualified name, text): (count, reason)} for the sites one reason covers, each given
    as (module, qualified name, text, count)."""
    return {(module, qual, text): (count, reason) for module, qual, text, count in sites}


# Every read the census counts in the loader's population, each by its module, its qualified
# name and its text, which holds its receiver, with how many times it occurs and why no
# workflow's text reaches the reader through it, the loader's own read among them.
NOT_WORKFLOW_READS = {
    **allowed(
        "the pnpm lockfile, which a pattern reads for the locked playwright version; never handed to the reader",
        (
            "test_ci_workflows",
            "OnlyAPushSavesACache.test_the_browser_cache_is_keyed_on_the_locked_playwright_version",
            "(REPO / 'pnpm-lock.yaml').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "the root manifest, which bindgen_version reads for the wasm-bindgen pin; never handed to the reader",
        (
            "test_ci_workflows",
            "bindgen_version",
            "(REPO / 'Cargo.toml').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "calls run_step, which runs a cache step's shell over a planted lockfile's text and reads its outputs",
        (
            "test_ci_workflows",
            "OnlyAPushSavesACache.test_the_browser_cache_is_keyed_on_the_locked_playwright_version",
            "run_step(step, \"lockfileVersion: '9.0'\\n\")",
            1,
        ),
        (
            "test_ci_workflows",
            "OnlyAPushSavesACache.test_the_browser_cache_is_keyed_on_the_locked_playwright_version",
            "run_step(step, lockfile)",
            1,
        ),
        (
            "test_ci_workflows",
            "OnlyAPushSavesACache.test_the_browser_cache_is_keyed_on_the_locked_playwright_version",
            "run_step(step, moved)",
            1,
        ),
        (
            "test_ci_workflows",
            "OnlyAPushSavesACache.test_the_browser_cache_is_keyed_on_the_locked_playwright_version",
            "run_step(step, two)",
            1,
        ),
    ),
    **allowed(
        "calls run_base_is_dev, which runs base-is-dev's shell and reads its output",
        (
            "test_ci_workflows",
            "OnlyThisRepositorysDevReachesMain.test_base_is_dev_admits_this_repositorys_dev_into_main",
            "run_base_is_dev(INTO_MAIN)",
            1,
        ),
        (
            "test_ci_workflows",
            "OnlyThisRepositorysDevReachesMain.test_base_is_dev_refuses_a_fork_whose_branch_is_named_dev",
            "run_base_is_dev(fork)",
            1,
        ),
        (
            "test_ci_workflows",
            "OnlyThisRepositorysDevReachesMain.test_base_is_dev_refuses_this_repositorys_other_branches_into_main",
            "run_base_is_dev(dict(INTO_MAIN, **{'github.head_ref': 'feature/probe'}))",
            1,
        ),
    ),
    **allowed(
        "calls required_contexts, whose one read is a ruleset's JSON",
        (
            "test_ci_workflows",
            "TheRequiredCiCheckIsThePullRequestsOwn.test_no_push_run_reports_under_a_required_name",
            "required_contexts()",
            1,
        ),
        (
            "test_ci_workflows",
            "TheRequiredCiCheckIsThePullRequestsOwn.test_the_required_ci_check_is_always_the_pull_requests_own_run",
            "required_contexts()",
            1,
        ),
    ),
    **allowed(
        "the refusal's own initialiser, named like a reader's method by `__init__`; it reads nothing",
        (
            "test_ci_workflows",
            "Unread.__init__",
            "super().__init__('the reader does not read ' + '; '.join(refused))",
            1,
        ),
    ),
    **allowed(
        "the loaders of the modules under test, each pointed at a scratch directory through a patch of its module; each reads through the loader",
        ("test_ci_workflows", "WorkflowFilesAreReadAsBytes.readers", "through_load", 1),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.readers.<locals>.through_load",
            "here",
            1,
        ),
    ),
    **allowed(
        "reads a planted file through each loader `readers` returns; each reads through the loader",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_file_that_ends_its_lines_in_crlf_reads_as_the_same_file_ending_in_lf",
            "read('release.yml')",
            1,
        ),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_file_that_ends_its_lines_in_crlf_reads_as_the_same_file_ending_in_lf",
            "reader",
            1,
        ),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_file_that_ends_its_lines_in_crlf_reads_as_the_same_file_ending_in_lf",
            "self.readers(directory)",
            1,
        ),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_lone_carriage_return_or_a_byte_order_mark_in_a_file_is_refused_by_name",
            "read('release.yml')",
            1,
        ),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_lone_carriage_return_or_a_byte_order_mark_in_a_file_is_refused_by_name",
            "reader",
            1,
        ),
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_a_lone_carriage_return_or_a_byte_order_mark_in_a_file_is_refused_by_name",
            "self.readers(directory)",
            1,
        ),
    ),
    **allowed(
        "calls the census, which reads the test modules' sources through module_sources",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read",
            "census_problems(Path(__file__).parent)",
            1,
        ),
    ),
    **allowed(
        "runs one of the three hardening tests the loop names over a planted directory; each reads through the loader",
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_a_yaml_workflow_is_held_to_the_same_hardening_rules",
            "getattr(case, test)()",
            2,
        ),
    ),
    **allowed(
        "copies the live workflows' bytes into a scratch directory unread; the hardening tests read the copies through the loader",
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_a_yaml_workflow_is_held_to_the_same_hardening_rules",
            "path.read_bytes()",
            1,
        ),
    ),
    **allowed(
        "a patch of this module's WORKFLOWS, taken from sys.modules; entering it points the hardening tests at a planted directory, reading nothing",
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_a_yaml_workflow_is_held_to_the_same_hardening_rules",
            "workflows",
            1,
        ),
    ),
    **allowed(
        "scripts/check.sh, whose STAGES_ALL line a pattern reads; it is a shell script, never handed to the reader",
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_ci_runs_every_stage_of_the_local_gate",
            "(REPO / 'scripts' / 'check.sh').read_text()",
            1,
        ),
        ("test_ci_workflows", "gate_stages", "(REPO / 'scripts' / 'check.sh').read_text()", 1),
    ),
    **allowed(
        "calls module_sources, which reads the test modules' sources for the census",
        ("test_ci_workflows", "census_problems", "module_sources(directory)", 1),
    ),
    **allowed(
        "a test module's source, which ast parses for the census; never handed to the reader",
        ("test_ci_workflows", "module_sources", "path.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "a ruleset's JSON, which json parses for its required contexts; never handed to the reader",
        (
            "test_ci_workflows",
            "required_contexts",
            "(RULESETS / f'{name}.json').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "runs base-is-dev's own shell, cut from ci.yml's loader-read text, under bash; its output is the step's verdict, never a workflow",
        (
            "test_ci_workflows",
            "run_base_is_dev",
            "subprocess.run(['bash', '-e', '-c', script], env=env, capture_output=True, text=True, check=False)",
            1,
        ),
    ),
    **allowed(
        "runs a cache step's own shell under bash in a scratch directory holding a planted lockfile, and reads the GITHUB_OUTPUT it writes; outputs, never a workflow",
        ("test_ci_workflows", "run_step", "output.read_text()", 1),
        (
            "test_ci_workflows",
            "run_step",
            "subprocess.run(['bash', '--noprofile', '--norc', '-e', '-o', 'pipefail', '-c', step['run']], cwd=where, env=env, capture_output=True, text=True, check=False)",
            1,
        ),
    ),
    **allowed(
        "runs an output check's own shell under bash in a scratch directory holding planted bindings or a planted XCFramework, and reads the report files it writes; verdicts, never a workflow",
        ("test_ci_workflows", "run_output_check", "each.read_text()", 1),
        (
            "test_ci_workflows",
            "run_output_check",
            "subprocess.run(['bash', '--noprofile', '--norc', '-e', '-o', 'pipefail', '-c', step['run']], cwd=where, env=env, capture_output=True, text=True, check=False)",
            1,
        ),
    ),
    **allowed(
        "calls run_output_check, which runs an output check's shell over planted files and reads its report",
        (
            "test_ci_workflows",
            "TheAppHasOneRustStaticLibrary.test_the_bindings_hold_one_module",
            "run_output_check(steps[after + 1], plant)",
            1,
        ),
        (
            "test_ci_workflows",
            "TheAppHasOneRustStaticLibrary.test_the_xcframework_holds_one_rust_library_per_slice",
            "run_output_check(steps[after + 1], plant)",
            1,
        ),
    ),
    **allowed(
        "calls umbrella_closure, which reads a workspace's Cargo manifests for the umbrella's build closure; manifests, never a workflow",
        (
            "test_ci_workflows",
            "TheAppHasOneRustStaticLibrary.test_the_change_caller_watches_every_crate_the_umbrella_links",
            "umbrella_closure(REPO)",
            1,
        ),
        (
            "test_ci_workflows",
            "TheAppHasOneRustStaticLibrary.test_the_change_caller_watches_every_crate_the_umbrella_links",
            "umbrella_closure(where)",
            1,
        ),
        (
            "test_testflight_workflows",
            "TheTestflightLanes.test_the_internal_push_filter_watches_every_crate_the_xcframework_links",
            "umbrella_closure(REPO, package)",
            1,
        ),
    ),
    **allowed(
        "the loader's one read: a workflow file's bytes, decoded as strict utf-8 with no byte order mark",
        ("test_ci_workflows", LOADER[1], "Path(path).read_bytes()", 1),
    ),
    **allowed(
        "runs a planted stand-in in a child process over a scratch directory and reads its exit and output; no workflow text reaches the reader",
        (
            "test_dispatch_shards",
            "AStandInThatCannotPlantTheWrapperFailsClosed.run_stand_in",
            "subprocess.run([sys.executable, '-I', str(root / 'stand_in.py'), str(root / 'machine'), str(root / 'memory_scope.py'), '--', 'echo', 'x'], capture_output=True, text=True, env={'PLANT_MARKS': str(root / 'marks'), 'PATH': os.environ['PATH']}, timeout=60)",
            1,
        ),
        (
            "test_dispatch_shards",
            "AStandInThatCannotPlantTheWrapperFailsClosed.test_a_failed_plant_exits_non_zero_names_the_failure_and_runs_no_words",
            "self.run_stand_in(RAISING_PLANT)",
            1,
        ),
        (
            "test_dispatch_shards",
            "AStandInThatCannotPlantTheWrapperFailsClosed.test_a_plant_that_works_runs_the_wrapper_and_not_the_script",
            "self.run_stand_in(PLANT_THAT_WORKS)",
            1,
        ),
    ),
    **allowed(
        "reads the test modules' own Python source to count where the finder is defined; never a workflow file",
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_the_finder_is_defined_once_and_a_planted_copy_is_caught",
            "definitions_of_the_finder(Path(__file__).parent)",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_the_finder_is_defined_once_and_a_planted_copy_is_caught",
            "definitions_of_the_finder(scratch)",
            1,
        ),
        (
            "test_mutation_workflows",
            "definitions_of_the_finder",
            "path.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "definitions_of_the_finder",
            "FINDER.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_renamed_and_rewrapped_copy_of_each_finder_function_is_caught",
            "FINDER.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_renamed_and_rewrapped_copy_of_each_finder_function_is_caught",
            "definitions_of_the_finder(scratch)",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_literal_copy_of_each_finder_function_under_another_name_is_caught",
            "FINDER.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_literal_copy_of_each_finder_function_under_another_name_is_caught",
            "definitions_of_the_finder(scratch)",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_copy_with_changed_logic_is_caught_only_under_the_finders_name",
            "FINDER.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutantsSpellingIsFound.test_a_copy_with_changed_logic_is_caught_only_under_the_finders_name",
            "definitions_of_the_finder(scratch)",
            1,
        ),
    ),
    **allowed(
        "the guard module's own Python source, parsed for its one WRAPPER assignment and never imported; never a workflow file",
        ("test_mutation_workflows", "<module>", "GUARD.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "calls the bash oracle, which runs planted scripts with cargo stubbed and reads the stubs' logs",
        (
            "test_dispatch_shards",
            "AComputedWordBeforeTheBoundsIsRefused.test_every_word_bash_can_expand_to_dashes_before_the_bounds_is_refused",
            "bash_runs(scripts)",
            1,
        ),
        (
            "test_dispatch_shards",
            "EveryMutationCommandKeepsTheGatesBounds.test_every_command_bash_runs_from_a_run_value_is_found_or_refused",
            "bash_runs(scripts)",
            1,
        ),
        (
            "test_dispatch_shards",
            "EveryMutationCommandKeepsTheGatesBounds.test_every_unbounded_command_bash_runs_past_a_hash_is_found",
            "unbounded_by_bash(members)",
            1,
        ),
        (
            "test_dispatch_shards",
            "EveryMutationCommandKeepsTheGatesBounds.test_the_reading_is_declared_what_it_reads_is_found_and_what_it_does_not_is_refused",
            "bash_runs([f'{UNBOUNDED}\\n', f'{LEAD}\\n', *found_texts])",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_every_wrapped_command_bash_runs_without_the_bounds_is_found_or_refused",
            "bash_runs(scripts)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_the_declared_form_around_a_bounded_command_is_found_bounded",
            "bash_runs(scripts)",
            1,
        ),
    ),
    **allowed(
        "runs the verdict's battery over planted reports; its output is the battery's verdict",
        (
            "test_dispatch_shards",
            "TheBatteryRefusesAForeignShardCount.run_battery",
            "subprocess.run([sys.executable, str(VERDICT), 'battery', '--reports', str(reports), '--shards', str(shards), '--package', 'deck-streak-fix', '--listed', str(reports / 'listing' / 'whole.json')], capture_output=True, text=True, check=False, timeout=120, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))",
            1,
        ),
    ),
    **allowed(
        "calls run_battery, which runs the verdict's battery over planted reports",
        (
            "test_dispatch_shards",
            "TheBatteryRefusesAForeignShardCount.test_a_report_beyond_the_count_fails_the_battery",
            "self.run_battery(3, 2)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheBatteryRefusesAForeignShardCount.test_a_report_set_of_exactly_the_count_passes",
            "self.run_battery(2, 2)",
            1,
        ),
    ),
    **allowed(
        "calls size, which runs the verdict's size command over a planted listing and reads its outputs",
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_a_package_that_lists_nothing_or_a_non_listing_is_not_sized",
            "size(None, 'deck-streak-ingest', raw='not json')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_a_package_that_lists_nothing_or_a_non_listing_is_not_sized",
            "size([], 'deck-streak-ingest')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_a_projection_past_the_limit_is_refused_with_its_projection_never_capped",
            "size(entries('deck-streak-ingest', 17000), 'deck-streak-ingest')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_a_small_package_takes_one_shard_and_a_large_one_the_fewest_within_the_bound",
            "size(entries(package, count), package)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_the_sizing_is_the_per_pull_request_plans_own_function",
            "size(listing, package)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheExaminedTotalIsTheListing.test_the_union_of_the_sized_shards_equals_the_union_of_the_thirty_two",
            "size(listing, 'deck-streak-daemon')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_a_listing_beyond_the_ceiling_is_refused_whole",
            "size(entries('deck-streak-ingest', 15913), 'deck-streak-ingest')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_a_listing_beyond_the_ceiling_is_refused_whole",
            "size(entries('deck-streak-ingest', 15912), 'deck-streak-ingest')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_every_sizing_prints_its_headroom",
            "size(entries('deck-streak-daemon', 113), 'deck-streak-daemon')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_every_sizing_prints_its_headroom",
            "size(entries('deck-streak-kernel', 5))",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_whole_tree_is_sized_from_its_listing",
            "size(listing)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_whole_tree_is_sized_from_its_listing",
            "size(None, None, raw='not json')",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_whole_tree_is_sized_from_its_listing",
            "size(entries('a', 3), 'miniapp')",
            1,
        ),
    ),
    **allowed(
        "runs the verdict's shards command over a planted plan, or calls plan_shards, which does; its output and the plan it left are the verdict's, never a workflow",
        (
            "test_dispatch_shards",
            "plan_shards",
            "subprocess.run([sys.executable, str(VERDICT), 'shards', '--plan', str(plan), '--listed', str(whole)], capture_output=True, text=True, check=False, timeout=120, env=env)",
            1,
        ),
        ("test_dispatch_shards", "plan_shards", "plan.read_text('utf-8')", 1),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_release_listing_fits_one_run",
            "plan_shards(listing)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_a_listing_beyond_the_ceiling_is_refused_whole",
            "plan_shards(entries('deck-streak-ingest', 14213))",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_every_sizing_prints_its_headroom",
            "plan_shards(entries('deck-streak-daemon', 113))",
            1,
        ),
    ),
    **allowed(
        "the plan the verdict wrote to a scratch file; JSON, never handed to the reader",
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_the_sizing_is_the_per_pull_request_plans_own_function",
            "plan.read_text('utf-8')",
            1,
        ),
        (
            "test_not_started_legs",
            "planned",
            "(reports / 'mutation-plan' / 'plan.json').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "runs the verdict's shards command over a planted plan; its output is the verdict's",
        (
            "test_dispatch_shards",
            "TheDispatchIsSizedFromItsListing.test_the_sizing_is_the_per_pull_request_plans_own_function",
            "subprocess.run([sys.executable, str(VERDICT), 'shards', '--plan', str(plan), '--listed', str(whole)], capture_output=True, text=True, check=False, timeout=120, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1'))",
            1,
        ),
    ),
    **allowed(
        "the memory-scope wrapper's source, planted with a dropped or added word; never handed to the reader",
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_a_wrapper_that_drops_or_adds_a_word_goes_red",
            "MEMORY_SCOPE.read_text('utf-8')",
            1,
        ),
    ),
    **allowed(
        "a value computed from the wrapper module loaded by path: its options, or the words and status its spies saw; never a workflow",
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_a_wrapper_that_drops_or_adds_a_word_goes_red",
            "cases",
            3,
        ),
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_the_wrapper_runs_exactly_the_words_after_its_separator",
            "options",
            1,
        ),
        ("test_dispatch_shards", "spy_wrong", "argv", 1),
        ("test_dispatch_shards", "spy_wrong", "ran", 3),
        ("test_dispatch_shards", "spy_wrong", "runs", 1),
        ("test_dispatch_shards", "spy_wrong", "status", 2),
        ("test_dispatch_shards", "spy_wrong", "words", 1),
    ),
    **allowed(
        "calls spy_wrong, which runs the wrapper's main() over planted cases and reads its spies' logs",
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_a_wrapper_that_drops_or_adds_a_word_goes_red",
            "spy_wrong(path, cases)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_the_wrapper_runs_exactly_the_words_after_its_separator",
            "spy_wrong(MEMORY_SCOPE, cases)",
            1,
        ),
    ),
    **allowed(
        "calls wrapper_module, which loads scripts/memory_scope.py by its path; a production module, never the reader",
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_a_wrapper_that_drops_or_adds_a_word_goes_red",
            "wrapper_module(MEMORY_SCOPE)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheMemoryScopeRunsTheWordsAfterItsSeparator.test_the_wrapper_runs_exactly_the_words_after_its_separator",
            "wrapper_module(MEMORY_SCOPE)",
            1,
        ),
        ("test_dispatch_shards", "wrapper_options", "wrapper_module(MEMORY_SCOPE)", 1),
    ),
    **allowed(
        "runs the verdict's size command over a planted listing; its output is the verdict's",
        (
            "test_dispatch_shards",
            "TheWeeklySweepNamesItsPackageInLiteralWords.test_a_dash_led_value_selects_nothing_in_the_step_after_the_listing",
            "subprocess.run([sys.executable, str(VERDICT), 'size', '--package', value, '--listed', str(listing)], capture_output=True, text=True, timeout=30)",
            1,
        ),
    ),
    **allowed(
        "calls argv_of, which runs a planted script under bash and reads the stub's log",
        (
            "test_dispatch_shards",
            "TheWeeklySweepNamesItsPackageInLiteralWords.test_the_rewrite_hands_cargo_exactly_the_words_the_head_did",
            "argv_of(new, value)",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheWeeklySweepNamesItsPackageInLiteralWords.test_the_rewrite_hands_cargo_exactly_the_words_the_head_did",
            "argv_of(old, value)",
            1,
        ),
    ),
    **allowed(
        "calls workspace_packages, which reads the crates' Cargo.toml files",
        (
            "test_dispatch_shards",
            "TheWeeklySweepNamesItsPackageInLiteralWords.test_the_rewrite_hands_cargo_exactly_the_words_the_head_did",
            "workspace_packages()",
            1,
        ),
    ),
    **allowed(
        "the memory-scope wrapper's bytes, copied into a scratch directory for bash to run; never handed to the reader",
        ("test_dispatch_shards", "argv_of", "MEMORY_SCOPE.read_bytes()", 1),
        ("test_dispatch_shards", "bash_runs", "MEMORY_SCOPE.read_bytes()", 1),
    ),
    **allowed(
        "a stub's log of the words it was run with, in a scratch directory; never a workflow",
        ("test_dispatch_shards", "argv_of", "log.read_bytes()", 1),
        ("test_dispatch_shards", "bash_runs", "(root / f'done{w}').read_text(encoding='utf-8')", 1),
        ("test_dispatch_shards", "bash_runs", "path.read_text(encoding='utf-8')", 1),
        ("test_dispatch_shards", "spy_runs", "log.read_bytes()", 1),
        ("test_dispatch_shards", "unbounded_by_bash", "log.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "runs a planted script under bash with cargo stubbed; its output is the stub's log",
        (
            "test_dispatch_shards",
            "argv_of",
            "subprocess.run(['bash', str(file)], env=env, capture_output=True, timeout=30, cwd=root)",
            1,
        ),
    ),
    **allowed(
        "runs the bash oracle over planted scripts with cargo stubbed; its output is the stubs' log",
        (
            "test_dispatch_shards",
            "bash_runs",
            "subprocess.Popen([shell, '--noprofile', '--norc', 'oracle.sh'], cwd=root, env=env)",
            1,
        ),
        (
            "test_dispatch_shards",
            "unbounded_by_bash",
            "subprocess.Popen([shell, '--noprofile', '--norc', 'oracle.sh'], cwd=root, env=env)",
            1,
        ),
    ),
    **allowed(
        "runs the verdict's size command and reads the GITHUB_OUTPUT it writes; outputs, never a workflow",
        ("test_dispatch_shards", "size", "sink.read_text(encoding='utf-8')", 1),
        (
            "test_dispatch_shards",
            "size",
            "subprocess.run(args, capture_output=True, text=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', GITHUB_OUTPUT=str(sink)), timeout=120, check=False)",
            1,
        ),
    ),
    **allowed(
        "an empty buffer the wrapper's stdout and stderr are redirected into; it reads nothing",
        ("test_dispatch_shards", "spy_runs", "io.StringIO()", 2),
    ),
    **allowed(
        "calls the planting function WRAPPER_PLANT defines, which loads the wrapper by path with its seams planted",
        (
            "test_dispatch_shards",
            "spy_runs",
            "namespace['plant_wrapper'](script, root / 'machine')",
            1,
        ),
    ),
    **allowed(
        "calls spy_runs, which runs the wrapper's main() over planted cases and reads its spies' logs",
        ("test_dispatch_shards", "spy_wrong", "spy_runs(script, cases)", 1),
    ),
    **allowed(
        "a crate's Cargo.toml, which tomllib parses for its package name; never handed to the reader",
        ("test_dispatch_shards", "workspace_packages", "manifest.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "the memory-scope wrapper's module and spec, loaded by path; a production module, never the reader",
        ("test_dispatch_shards", "wrapper_module", "module", 2),
        ("test_dispatch_shards", "wrapper_module", "spec", 1),
    ),
    **allowed(
        "calls text, which reads a document for its words",
        (
            "test_mutation_python_workflows",
            "TheDocumentsTeachThePythonRun.test_the_builder_brief_and_the_amendments_teach_the_python_run",
            "text(ADR_057)",
            1,
        ),
        (
            "test_mutation_python_workflows",
            "TheDocumentsTeachThePythonRun.test_the_builder_brief_and_the_amendments_teach_the_python_run",
            "text(ADR_073)",
            1,
        ),
        (
            "test_mutation_python_workflows",
            "TheDocumentsTeachThePythonRun.test_the_builder_brief_and_the_amendments_teach_the_python_run",
            "text(BRIEF)",
            1,
        ),
        (
            "test_mutation_python_workflows",
            "TheDocumentsTeachThePythonRun.test_the_builder_brief_and_the_amendments_teach_the_python_run",
            "text(SPEC_039)",
            1,
        ),
        (
            "test_mutation_python_workflows",
            "TheDocumentsTeachThePythonRun.test_the_builder_brief_and_the_amendments_teach_the_python_run",
            "text(TESTING)",
            1,
        ),
    ),
    **allowed(
        "a document's text (a brief, an ADR, a SPEC, TESTING.md) a test reads for its words; never handed to the reader",
        ("test_mutation_python_workflows", "text", "path.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "a tool's configuration file, which the test parses as TOML or JSON; never handed to the reader",
        (
            "test_mutation_workflows",
            "CargoMutantsRunsTheGatesTestTool.test_every_job_that_runs_cargo_mutants_installs_the_test_tool_it_names",
            "(REPO / '.cargo' / 'mutants.toml').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutationCommandRunsTheMutantsProfile.test_every_mutation_command_runs_the_mutants_profile",
            "nextest.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "EveryMutationCommandRunsTheMutantsProfile.test_every_mutation_command_runs_the_mutants_profile",
            "(REPO / '.cargo' / 'mutants.toml').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBriefTeachesTheRecord.test_the_builder_brief_teaches_the_equivalence_record",
            "(REPO / '.cargo' / 'mutants.toml').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBriefTeachesTheRecord.test_the_builder_brief_teaches_the_equivalence_record",
            "(REPO / 'web' / 'app' / 'stryker.config.json').read_text('utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheConfigurationCheckSeesWhatStrykerReads.test_the_configuration_check_refuses_what_stryker_would_read_otherwise",
            "(REPO / 'web/app/stryker.config.json').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheToolsConfigurationsAreValid.test_the_tool_configurations_load_under_their_own_rules",
            "(REPO / 'web/app/package.json').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheToolsConfigurationsAreValid.test_the_tool_configurations_load_under_their_own_rules",
            "(REPO / 'web/app/stryker.config.json').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "calls run, which runs the verdict script and reads its output",
        (
            "test_mutation_workflows",
            "NoExclusionHidesAMutant.test_no_exclusion_hides_a_mutant_from_the_listing",
            "run(str(VERDICT), 'exclusions', '--root', str(REPO))",
            1,
        ),
        (
            "test_mutation_workflows",
            "NoExclusionHidesAMutant.test_no_exclusion_hides_a_mutant_from_the_listing",
            "run(str(VERDICT), 'exclusions', '--root', str(root))",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBatteryTakesAScope.test_a_dispatch_scoped_to_one_package_sweeps_only_its_mutants",
            "run(str(VERDICT), 'battery', '--shards', '5', '--reports', str(reports))",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBatteryTakesAScope.test_a_dispatch_scoped_to_one_package_sweeps_only_its_mutants",
            "run(str(VERDICT), 'battery', '--shards', '5', *args)",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBatteryTakesAScope.test_a_dispatch_scoped_to_one_package_sweeps_only_its_mutants",
            "run(str(VERDICT), 'table', '--root', scratch, *args)",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBriefTeachesTheRecord.test_the_builder_brief_teaches_the_equivalence_record",
            "run(str(VERDICT), 'survivors', '--reports', str(reports), '--out', f'{scratch}/d')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheConfigurationCheckSeesWhatStrykerReads.test_the_configuration_check_refuses_what_stryker_would_read_otherwise",
            "run(str(VERDICT), 'configs', '--root', str(root))",
            2,
        ),
        (
            "test_mutation_workflows",
            "TheToolsConfigurationsAreValid.test_the_tool_configurations_load_under_their_own_rules",
            "run(str(VERDICT), 'configs', '--root', str(REPO))",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheToolsConfigurationsAreValid.test_the_tool_configurations_load_under_their_own_rules",
            "run(str(VERDICT), 'configs', '--root', str(root))",
            2,
        ),
        (
            "test_mutation_workflows",
            "TheVerdictBindsEveryRecord.test_the_verdict_binds_every_record_against_the_whole_listing",
            "run(str(VERDICT), 'survivors', '--reports', str(only), '--out', str(quiet), '--root', str(root))",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheVerdictBindsEveryRecord.test_the_verdict_binds_every_record_against_the_whole_listing",
            "run(str(VERDICT), 'survivors', '--reports', str(reports), '--out', str(drafts), '--root', str(root))",
            1,
        ),
    ),
    **allowed(
        "a draft record the verdict wrote to a scratch directory; JSON or markdown, never handed to the reader",
        (
            "test_mutation_workflows",
            "TheBriefTeachesTheRecord.test_the_builder_brief_teaches_the_equivalence_record",
            "(Path(scratch) / 'd' / 'draft-001.md').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheVerdictBindsEveryRecord.test_the_verdict_binds_every_record_against_the_whole_listing",
            "(drafts / 'drafts.json').read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheVerdictBindsEveryRecord.test_the_verdict_binds_every_record_against_the_whole_listing",
            "(drafts / manifest[0]['body']).read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheVerdictBindsEveryRecord.test_the_verdict_binds_every_record_against_the_whole_listing",
            "(quiet / 'drafts.json').read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "the builder brief, which the test reads for its words; never handed to the reader",
        (
            "test_mutation_workflows",
            "TheBriefTeachesTheRecord.test_the_builder_brief_teaches_the_equivalence_record",
            "BRIEF.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_mutation_workflows",
            "TheBuilderBriefTeachesTheRules.test_the_builder_brief_teaches_the_mutation_rules",
            "BRIEF.read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "runs the verdict script with the arguments it is given; its output is the verdict's",
        (
            "test_mutation_workflows",
            "run",
            "subprocess.run([sys.executable, *args], capture_output=True, text=True, env=dict(os.environ, PYTHONDONTWRITEBYTECODE='1', **env or {}), timeout=300, check=False)",
            1,
        ),
        (
            "test_not_started_legs",
            "verdict",
            "subprocess.run([sys.executable, str(VERDICT), *map(str, args)], capture_output=True, text=True, env=env, timeout=300, check=False)",
            1,
        ),
    ),
    **allowed(
        "calls a helper that runs the verdict script, a step's shell or the verdict module over planted reports; outputs, never a workflow",
        (
            "test_dispatch_shards",
            "TheGatesTimeoutCoversTheCensus.test_the_gates_timeout_covers_the_census_with_its_margin",
            "verdict_module()",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_bound_is_half_of_every_legs_timeout",
            "verdict_module()",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget",
            "verdict_module()",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "judge(reports)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "judge(reports, rows=False)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "legs(merged.out / 'plan.json', 'skipped', 'skipped')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "legs(plan, 'skipped', 'success')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "planned(reports)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "run_shards(merged, None)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "judge(reports)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(diff.out / 'plan.json', 'skipped', 'skipped')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(diff.out / 'plan.json', 'skipped', 'success')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(plan, 'skipped', 'success')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(plan, 'success', 'skipped')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(plan, 'success', 'success')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "legs(plan, result, 'success')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "planned(reports)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "run_shards(diff, None)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "judge(empty)",
            2,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "judge(planted)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "judge(whole)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "planned(whole)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "rewrite(outcomes_of(planted), moved)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_ci_admits_a_not_started_leg_and_no_other_skip",
            "bash(script, env, shell_of(step, aggregate, workflow(CI)))",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_ci_admits_a_skip_from_the_two_legs_and_from_no_other_need",
            "bash(script, env, shell_of(step, aggregate, workflow(CI)))",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "run_shards(fixture, listed)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "verdict_module()",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_verdict_step_fails_on_the_legs_check",
            "bash(script, env, shell_of(step, job, workflow(CI)))",
            1,
        ),
    ),
    **allowed(
        "a Fixture's method, which commits a planted tree or runs the verdict's plan over it; outputs, never a workflow",
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "merged.head({LIB: LIB_TEXT.replace('x * 2', 'x + x')})",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_leg_the_listing_gives_nothing_reads_not_started",
            "merged.plan('--event', 'push', '--subject', MERGED)",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "diff.head({'README.md': 'a fixture, changed\\n'})",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "diff.plan()",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "code.head({LIB: LIB_TEXT.replace('x * 2', 'x + x')})",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "code.plan()",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "constant.head({LIB: LIB_TEXT.replace('pub const LAST_HOUR: u8 = 23;', \"pub const LAST_HOUR: u8 = 23; // the day's last hour\")})",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "constant.plan()",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "other.head({'README.md': 'a fixture, changed\\n'})",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "other.plan()",
            1,
        ),
    ),
    **allowed(
        "a plan or a log the verdict wrote to a scratch file; never handed to the reader",
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
            "plan.read_text(encoding='utf-8')",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_verdict_step_fails_on_the_legs_check",
            "log.read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "an outcomes file the verdict wrote to a scratch directory; never handed to the reader",
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_an_examined_sum_that_differs_from_the_listing_is_refused",
            "outcomes_of(whole).read_text(encoding='utf-8')",
            1,
        ),
    ),
    **allowed(
        "a constant of the verdict module, loaded by path by verdict_module; a production module's table, never the reader",
        (
            "test_dispatch_shards",
            "TheGatesTimeoutCoversTheCensus.test_the_gates_timeout_covers_the_census_with_its_margin",
            "module.CENSUS_SECONDS",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_bound_is_half_of_every_legs_timeout",
            "module.SHARD_BOUND_SECONDS",
            1,
        ),
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget",
            "module.PYTHON_MAX_SHARDS",
            1,
        ),
    ),
    **allowed(
        "the verdict module itself, loaded by path by verdict_module and handed to getattr for one constant; a production module, never the reader",
        (
            "test_dispatch_shards",
            "TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget",
            "module",
            1,
        ),
    ),
    **allowed(
        "a function of the verdict module, loaded by path by verdict_module; a production module, never the reader",
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "module.fewest_shards(module.mutant_costs(packages))",
            1,
        ),
        (
            "test_not_started_legs",
            "ALegWithNothingToExamineIsNotStarted.test_the_plan_writes_how_many_rust_mutants_its_listing_holds",
            "module.mutant_costs(packages)",
            1,
        ),
    ),
    **allowed(
        "runs a step's shell cut from a workflow's loader-read text under the shell argv shell_of resolved from that text; its output is the step's verdict, never a workflow",
        (
            "test_not_started_legs",
            "bash",
            "subprocess.run([*shell, '-c', script], env=env, capture_output=True, text=True, timeout=60, check=False)",
            1,
        ),
    ),
    **allowed(
        "calls verdict, which runs the verdict script and reads its output",
        ("test_not_started_legs", "judge", "verdict(*args)", 1),
        (
            "test_not_started_legs",
            "legs",
            "verdict('legs', '--plan', plan, '--rust-leg', rust, '--rows-leg', rows)",
            1,
        ),
    ),
    **allowed(
        "an outcomes file the verdict wrote to a scratch directory, rewritten for a plant; never handed to the reader",
        ("test_not_started_legs", "rewrite", "path.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "calls git, which runs git in a scratch repository",
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('checkout', '-q', '-b', 'main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('checkout', '-q', '-b', 'topic', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('clone', '-q', str(origin), str(work), cwd=tmp, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('commit', '-q', '--allow-empty', '-m', 'off main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('commit', '-q', '--allow-empty', '-m', 'on main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('init', '-q', '--bare', '-b', 'main', str(origin), cwd=tmp, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('push', '-q', 'origin', 'main', 'v1.0.0', 'v1.1.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('push', '-q', 'origin', 'topic', 'v2.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('rev-parse', f'{tag}^{{commit}}', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('tag', '-a', '-m', 'v1.0.0', 'v1.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('tag', '-a', '-m', 'v2.0.0', 'v2.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "git('tag', 'v1.1.0', cwd=work, env=env)",
            1,
        ),
    ),
    **allowed(
        "runs the release's tag guard, cut from release.yml's loader-read text, under bash in a scratch repository; its output is the guard's verdict",
        (
            "test_release_workflow",
            "TheTagGuardRuns.test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard",
            "subprocess.run(['bash', '-e', str(script)], cwd=work, env={**env, 'GITHUB_REF_NAME': tag, 'GITHUB_REF': f'refs/tags/{tag}', 'GITHUB_SHA': sha}, capture_output=True, text=True)",
            1,
        ),
    ),
    **allowed(
        "calls git, which runs git in a scratch repository",
        (
            "test_release_workflow",
            "scratch_repository",
            "git('init', '-q', '--bare', '-b', 'main', str(origin), cwd=tmp, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('clone', '-q', str(origin), str(work), cwd=tmp, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('checkout', '-q', '-b', 'main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('commit', '-q', '--allow-empty', '-m', 'on main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('tag', '-a', '-m', 'v1.0.0', 'v1.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('tag', 'v1.1.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('commit', '-q', '--allow-empty', '-m', 'later on main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('push', '-q', 'origin', 'main', 'v1.0.0', 'v1.1.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('rev-parse', 'HEAD', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('checkout', '-q', '-b', 'topic', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('commit', '-q', '--allow-empty', '-m', 'off main', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('tag', '-a', '-m', 'v2.0.0', 'v2.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('push', '-q', 'origin', 'topic', 'v2.0.0', cwd=work, env=env)",
            1,
        ),
        (
            "test_release_workflow",
            "scratch_repository",
            "git('rev-parse', f'{tag}^{{commit}}', cwd=work, env=env)",
            1,
        ),
    ),
    **allowed(
        "runs the release's tag guard, cut from release.yml's loader-read text, under bash in a scratch repository; its output is the guard's verdict",
        (
            "test_release_workflow",
            "run_guard",
            "subprocess.run(['bash', '-e', str(script)], cwd=work, env={**env, **github}, capture_output=True, text=True)",
            1,
        ),
    ),
    **allowed(
        "calls a helper that builds a scratch repository with git or runs the release's tag guard in one; it reads no file and no workflow's text reaches a reader",
        (
            "test_release_workflow",
            "TheSecondPathRunsTheTagGuard.test_the_guard_refuses_a_ref_that_is_not_the_tag_it_names",
            "scratch_repository(Path(tmp), env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheSecondPathRunsTheTagGuard.test_the_guard_refuses_a_ref_that_is_not_the_tag_it_names",
            "run_guard(guard, work, env, GITHUB_EVENT_NAME='workflow_dispatch', **names)",
            1,
        ),
        (
            "test_release_workflow",
            "TheSecondPathRunsTheTagGuard.test_both_paths_refuse_a_lightweight_tag_and_a_tag_off_main",
            "scratch_repository(Path(tmp), env)",
            1,
        ),
        (
            "test_release_workflow",
            "TheSecondPathRunsTheTagGuard.test_both_paths_refuse_a_lightweight_tag_and_a_tag_off_main",
            "run_guard(guard, work, env, GITHUB_EVENT_NAME=event, GITHUB_REF=f'refs/tags/{tag}', GITHUB_REF_NAME=tag, GITHUB_SHA=shas[tag])",
            1,
        ),
    ),
    **allowed(
        "runs the release's sync-server build step, cut from release.yml's loader-read text, under bash with cargo stubbed, in planted manifest roots and the tree; its output is the stub's argument line",
        (
            "test_release_workflow",
            "TheReleaseBuildsTheSyncServer.test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry",
            "subprocess.run(['bash', '-e', str(script)], cwd=roots[name], env={**os.environ, 'PATH': path, 'RUNNER_TEMP': str(runner)}, capture_output=True, text=True)",
            1,
        ),
    ),
    **allowed(
        "runs the release's steps from the app's build to the draft, cut from release.yml's loader-read text, and a tar listing of the tarball they write, under bash in planted trees; its output is the steps' own lines and the listing",
        (
            "test_release_workflow",
            "TheReleaseCarriesTheWebEngine.test_the_release_carries_the_module_at_web_engine",
            "subprocess.run(['bash', '-e', str(script)], cwd=root, env=env, capture_output=True, text=True)",
            1,
        ),
    ),
    **allowed(
        "runs git in a scratch repository with the arguments it is given; its output is a sha or nothing",
        (
            "test_release_workflow",
            "git",
            "subprocess.run(['git', *args], cwd=cwd, env=env, capture_output=True, text=True)",
            1,
        ),
    ),
    **allowed(
        "calls one of the variants the dict names, each a planted edit of release.yml's loader-read text; it reads no file",
        (
            "test_workflow_concurrency",
            "EveryReleaseWorkflowQueuesEveryRun.test_the_release_class_is_closed_by_construction",
            "{'root': lambda: release.replace('\\njobs:\\n', f'\\n{variant}: x\\njobs:\\n', 1), 'on': lambda: release.replace('  push:\\n', f'  {variant}:\\n  push:\\n', 1), 'job': lambda: release.replace('  publish:\\n', f'  publish:\\n    {variant}: x\\n', 1), 'push': lambda: release.replace('  push:\\n', f'  push:\\n    {variant}: [v1]\\n', 1), 'release': lambda: release.replace('tags: [v1]\\n', f'tags: [v1]\\n  release:\\n    {variant}: [published]\\n', 1)}[where]()",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        ("test_ci_workflows", "WorkflowFilesAreReadAsBytes.builtin_exceptions", "name", 3),
    ),
    **allowed(
        "runs the census over a scratch copy of this directory that holds one planted module; the copy's modules reach the census as source text and no workflow's text reaches a reader",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.plant_problems",
            "census_problems(copy)",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_builtin_exception_class_is_placed_by_its_value",
            "names",
            5,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_builtin_exception_class_is_placed_by_its_value",
            "self.builtin_exceptions()",
            1,
        ),
    ),
    **allowed(
        "hands a planted module's source to the census's scratch copy; no workflow's text is read",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_builtin_exception_class_is_placed_by_its_value",
            "self.plant_problems(body + '\\n\\ndef plant_control():\\n    return memoryview\\n')",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "name",
            12,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "name.endswith('__')",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "name.isidentifier()",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "name.startswith('__')",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "others",
            6,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "placed",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "self.builtin_exceptions()",
            1,
        ),
    ),
    **allowed(
        "hands a planted module's source to the census's scratch copy; no workflow's text is read",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "self.plant_problems(body)",
            1,
        ),
    ),
    **allowed(
        "a builtin name taken from the interpreter's builtins namespace or a value derived from one; it names no file and no workflow's text reaches a reader through it",
        ("test_ci_workflows", "builtin_exception_names", "name", 3),
    ),
    **allowed(
        "calls the derivation of the exception classes, which reads no file; its value is a set of builtin names",
        ("test_ci_workflows", "census_problems", "builtin_exception_names()", 1),
    ),
    **allowed(
        "calls bash, which runs a step's shell cut from a workflow's loader-read text under the shell argv shell_of resolved from that text, over a shim directory; its output is the step's verdict, never a workflow",
        (
            "test_verdict_folds",
            "Driver.run",
            "bash(script, env, shell_of(self.step, self.job, workflow(CI)))",
            1,
        ),
    ),
    **allowed(
        "a log the step's shim wrote to a scratch file; never handed to the reader",
        ("test_verdict_folds", "Driver.run", "log.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "calls Driver.run, which runs a step's shell over a planted script and shims, and reads the shim's log; outputs, never a workflow",
        (
            "test_verdict_folds",
            "TheCensusJudgesEveryCommandByWhereItSits.test_every_command_form_is_driven_by_the_harness_or_refused_by_name",
            "driver.run(planted_script, key, rc)",
            1,
        ),
        (
            "test_verdict_folds",
            "TheVerdictStepFailsOnEachJudgeAlone.test_the_harness_sees_a_pipe_that_loses_a_recorded_exit",
            "driver.run(mutated, key, rc)",
            1,
        ),
        (
            "test_verdict_folds",
            "TheVerdictStepFailsOnEachJudgeAlone.test_the_step_fails_when_any_one_command_alone_fails_with_any_of_its_exits",
            "driver.run(script, key, rc)",
            1,
        ),
    ),
    **allowed(
        "the verdict tool's own source file, read for its exit constants; a production script, never the reader",
        ("test_verdict_folds", "nonzero_exits", "TOOL.read_text(encoding='utf-8')", 1),
    ),
    **allowed(
        "runs the census's guard test over a planted copy of the test tree; the census reads the copy's module sources through module_sources and reports problems, and no workflow's text reaches another reader through the call",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_the_census_is_red_on_every_planted_site",
            "case.run(result)",
            1,
        ),
    ),
    **allowed(
        "the re-release script's own source, which the lead test parses as a syntax tree to read a constant; a production script, never handed to the reader",
        (
            "test_ci_workflows",
            "TheRereleaseCheck.test_a15_the_rerelease_lead_covers_the_schedule_interval",
            "script.read_text(encoding='utf-8')",
            1,
        ),
    ),
}

# Every site in the test directory that imports, runs code or reaches a namespace by a name held
# in data, each bound the same way, with why it can bring no module of the directory in.
DYNAMIC_IMPORTS = {
    **allowed(
        "runs the audit verdict script by its path; a production script",
        (
            "test_audit_web",
            "TheVerdictScriptStatesItself.test_the_verdict_script_writes_no_bytecode",
            "runpy.run_path(str(self.SCRIPT), run_name='audit_web_verdict_probe')",
            1,
        ),
    ),
    **allowed(
        "runs the threat model reader as a script by its path, in-process; a production script",
        (
            "test_threat_model",
            "scripted",
            "runpy.run_path(str(SCRIPT), run_name='__main__')",
            1,
        ),
    ),
    **allowed(
        "loads a production script by the path the call names; never a module of the test directory",
        ("test_backup_units", "load_backup", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_backup_units",
            "load_backup",
            "importlib.util.spec_from_file_location('deck_streak_backup', BACKUP_SCRIPT)",
            1,
        ),
        ("test_backup_units", "load_backup", "spec.loader.exec_module(module)", 1),
        ("test_web_engine_size", "load_gate", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_web_engine_size",
            "load_gate",
            "importlib.util.spec_from_file_location('web_engine_size', GATE)",
            1,
        ),
        ("test_web_engine_size", "load_gate", "spec.loader.exec_module(module)", 1),
        ("test_dispatch_shards", "wrapper_module", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_dispatch_shards",
            "wrapper_module",
            "importlib.util.spec_from_file_location('memory_scope', script)",
            1,
        ),
        ("test_dispatch_shards", "wrapper_module", "spec.loader.exec_module(module)", 1),
        ("test_memory_scope", "load", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_memory_scope",
            "load",
            "importlib.util.spec_from_file_location('memory_scope', SCRIPT)",
            1,
        ),
        ("test_memory_scope", "load", "spec.loader.exec_module(module)", 1),
        ("test_mutation_python", "runner_module", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_python",
            "runner_module",
            "importlib.util.spec_from_file_location('mutation_python_constants', RUNNER)",
            1,
        ),
        ("test_mutation_python", "runner_module", "spec.loader.exec_module(module)", 1),
        ("test_mutation_python_cli_kills", "runner", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_python_cli_kills",
            "runner",
            "importlib.util.spec_from_file_location('mutation_python_cli_kills', RUNNER)",
            1,
        ),
        ("test_mutation_python_cli_kills", "runner", "spec.loader.exec_module(module)", 1),
        ("test_mutation_python_judge_kills", "load", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_python_judge_kills",
            "load",
            "importlib.util.spec_from_file_location(name, path)",
            1,
        ),
        ("test_mutation_python_judge_kills", "load", "spec.loader.exec_module(module)", 1),
        ("test_mutation_python_lister_kills", "load", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_python_lister_kills",
            "load",
            "importlib.util.spec_from_file_location('mutation_python_under_kill', SCRIPT)",
            1,
        ),
        ("test_mutation_python_lister_kills", "load", "spec.loader.exec_module(module)", 1),
        ("test_mutation_python_verdict", "runner", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_python_verdict",
            "runner",
            "importlib.util.spec_from_file_location('mutation_python_listing', REPO / 'scripts' / 'mutation_python.py')",
            1,
        ),
        ("test_mutation_python_verdict", "runner", "spec.loader.exec_module(module)", 1),
        ("test_mutation_rows_group", "<module>", "SPEC.loader.exec_module(runner)", 1),
        ("test_mutation_rows_group", "<module>", "importlib.util.module_from_spec(SPEC)", 1),
        (
            "test_mutation_rows_group",
            "<module>",
            "importlib.util.spec_from_file_location('mutation_rows', REPO / 'scripts' / 'mutation_rows.py')",
            1,
        ),
        ("test_mutation_verdict", "verdict_module", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_mutation_verdict",
            "verdict_module",
            "importlib.util.spec_from_file_location('mutation_verdict_constants', VERDICT)",
            1,
        ),
        ("test_mutation_verdict", "verdict_module", "spec.loader.exec_module(module)", 1),
        ("test_public_scrub", "scrub_module", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_public_scrub",
            "scrub_module",
            "importlib.util.spec_from_file_location('public_scrub', SCRUB)",
            1,
        ),
        ("test_public_scrub", "scrub_module", "spec.loader.exec_module(module)", 1),
        ("test_rail_contract", "load_guards_check", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_rail_contract",
            "load_guards_check",
            "importlib.util.spec_from_file_location('guards_check', GUARDS)",
            1,
        ),
        ("test_rail_contract", "load_guards_check", "spec.loader.exec_module(module)", 1),
        ("test_slo_evaluator", "load_evaluator", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_slo_evaluator",
            "load_evaluator",
            "importlib.util.spec_from_file_location('slo_evaluate', EVALUATOR)",
            1,
        ),
        ("test_slo_evaluator", "load_evaluator", "spec.loader.exec_module(module)", 1),
        ("test_threat_model", "load", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_threat_model",
            "load",
            "importlib.util.spec_from_file_location('threat_model', SCRIPT)",
            1,
        ),
        ("test_threat_model", "load", "spec.loader.exec_module(module)", 1),
    ),
    **allowed(
        "imports a production module from scripts/, which the test puts on sys.path; never a module of the test directory",
        ("test_band_repeated_key", "<module>", "import mutation_rows", 1),
        (
            "test_host_scrub",
            "Axes.test_an_item_holding_another_device_is_never_digested_or_removed",
            "import apply as apply_tool",
            1,
        ),
        (
            "test_host_scrub",
            "Axes.test_an_item_holding_another_device_is_never_digested_or_removed",
            "import plan as plan_tool",
            1,
        ),
        (
            "test_host_scrub",
            "Axes.test_an_item_that_is_or_holds_a_mount_point_is_refused",
            "import apply as apply_tool",
            1,
        ),
        (
            "test_host_scrub",
            "Axes.test_an_item_that_is_or_holds_a_mount_point_is_refused",
            "import plan as plan_tool",
            1,
        ),
        (
            "test_mutation_python",
            "TheRunnerJudgesEachMutant.test_a_failed_restore_is_the_runs_exit_and_no_outcome_derives_it",
            "import mutation_python",
            1,
        ),
        (
            "test_mutation_python",
            "TheRunnerJudgesEachMutant.test_a_mutant_that_does_not_parse_is_unviable_and_runs_no_test",
            "import mutation_python",
            1,
        ),
        (
            "test_mutation_python",
            "TheRunnerListsItsMutants.test_every_listed_mutant_parses_and_a_comment_changes_no_listing",
            "import mutation_python",
            1,
        ),
        ("test_mutation_rows", "runner_module", "import mutation_rows", 1),
    ),
    **allowed(
        "an import of this repository's own script module by its constant name, `import mutation_rows`, after a sys.path insert of the constant scripts directory; it reads no workflow file",
        ("test_bin_kind_census", "runner_module", "import mutation_rows", 1),
    ),
    **allowed(
        "a thread pool that runs cargo and rustc over generated scratch crates of Rust source at once; it reads no workflow file",
        (
            "test_bin_kind_census",
            "TheBinKindReadsWhatTheCompilerBuilds.test_every_layout_agrees_with_cargo_and_the_compiler_or_is_refused_by_name",
            "concurrent.futures.ThreadPoolExecutor(max_workers=min(16, os.cpu_count() or 1))",
            1,
        ),
    ),
    **allowed(
        "this module, whose WORKFLOWS a patch points at a scratch directory",
        ("test_ci_workflows", "WorkflowFilesAreReadAsBytes.readers", "sys.modules", 1),
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_a_yaml_workflow_is_held_to_the_same_hardening_rules",
            "sys.modules",
            3,
        ),
    ),
    **allowed(
        "this module, whose __file__ a patch points at the plant's copy of the test directory",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_the_census_is_red_on_every_planted_site",
            "sys.modules",
            1,
        ),
    ),
    **allowed(
        "one of the three hardening tests the loop names, by its name",
        (
            "test_ci_workflows",
            "WorkflowsAreHardened.test_a_yaml_workflow_is_held_to_the_same_hardening_rules",
            "getattr(case, test)",
            2,
        ),
    ),
    **allowed(
        "runs the constant WRAPPER_PLANT, which defines the function that loads the wrapper by path with its seams planted",
        ("test_dispatch_shards", "spy_runs", "exec(WRAPPER_PLANT, namespace)", 1),
    ),
    **allowed(
        "runs this module's own tests",
        ("test_github_setup", "<module>", "unittest.main(argv=[sys.argv[0]])", 1),
    ),
    **allowed(
        "a stat result's field, by a key the test lists",
        (
            "test_host_scrub",
            "Axes.test_an_item_holding_another_device_is_never_digested_or_removed.<locals>.across",
            "getattr(st, key)",
            1,
        ),
    ),
    **allowed(
        "the wrapper's run() signature and its empty default, read for the seams' defaults",
        (
            "test_memory_scope",
            "TheWholeValues.test_the_seams_default_to_this_machine",
            "inspect.Parameter.empty",
            2,
        ),
        (
            "test_memory_scope",
            "TheWholeValues.test_the_seams_default_to_this_machine",
            "inspect.signature(self.module.run)",
            1,
        ),
    ),
    **allowed(
        "compiles a mutant of a planted source to prove it parses; it runs nothing",
        (
            "test_mutation_python",
            "TheRunnerListsItsMutants.test_every_listed_mutant_parses_and_a_comment_changes_no_listing",
            "compile(mutant.apply(SITES), 'sites.py', 'exec')",
            1,
        ),
    ),
    **allowed(
        "silences the SyntaxWarning a string constant's own `ast.parse` raises while the stand-in census parses it as text; it imports, runs and reads nothing",
        ("_stand_in_census", "arms_of", "warnings.catch_warnings()", 1),
        ("_stand_in_census", "arms_of", "warnings.simplefilter('ignore')", 1),
    ),
    **allowed(
        "registers the production module loaded by path under its own name, so its dataclasses resolve",
        ("test_mutation_python", "runner_module", "sys.modules", 1),
        ("test_mutation_python_cli_kills", "runner", "sys.modules", 1),
        ("test_mutation_python_judge_kills", "load", "sys.modules", 1),
        ("test_mutation_python_lister_kills", "load", "sys.modules", 1),
        ("test_mutation_python_verdict", "runner", "sys.modules", 1),
        ("test_mutation_rows_group", "<module>", "sys.modules", 1),
        ("test_mutation_verdict", "verdict_module", "sys.modules", 1),
    ),
    **allowed(
        "drops a production module the judge re-imports from a copy, so each test imports it afresh",
        ("test_mutation_python_judge_kills", "Base.setUp", "sys.modules.pop", 1),
        (
            "test_mutation_python_judge_kills",
            "TheJudgeSetsUp.test_the_runner_copy_follows_absolute_imports_and_never_relative_ones",
            "sys.modules.pop",
            1,
        ),
    ),
    **allowed(
        "sets a field of a frozen mutant to prove it refuses",
        (
            "test_mutation_python_lister_kills",
            "TheModuleHoldsItsConstants.test_a_mutant_is_frozen_and_has_slots",
            "setattr",
            1,
        ),
    ),
    **allowed(
        "a member of the lister module, by a name the test lists",
        ("test_mutation_python_lister_kills", "pick", "getattr(m, name)", 1),
    ),
    **allowed(
        "reads the builtins namespace to derive the exception classes; it imports, runs and reads nothing",
        ("test_ci_workflows", "WorkflowFilesAreReadAsBytes.builtin_exceptions", "builtins", 3),
    ),
    **allowed(
        "reads the builtins namespace to derive the exception classes; it imports, runs and reads nothing",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.builtin_exceptions",
            "getattr(builtins, name)",
            2,
        ),
    ),
    **allowed(
        "reads the builtins namespace to derive the exception classes; it imports, runs and reads nothing",
        (
            "test_ci_workflows",
            "WorkflowFilesAreReadAsBytes.test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site",
            "builtins",
            1,
        ),
    ),
    **allowed(
        "reads the builtins namespace to derive the exception classes; it imports, runs and reads nothing",
        ("test_ci_workflows", "builtin_exception_names", "builtins", 3),
    ),
    **allowed(
        "reads the builtins namespace to derive the exception classes; it imports, runs and reads nothing",
        ("test_ci_workflows", "builtin_exception_names", "getattr(builtins, name)", 2),
    ),
    **allowed(
        "patches and calls the module's own loader through its own namespace, by constant names; it imports nothing and runs nothing by a name held in data",
        (
            "test_formal_config",
            "FormalConfig.test_the_loader_refuses_bytes_that_are_not_utf8_wherever_they_sit",
            "globals()",
            2,
        ),
    ),
    **allowed(
        "executes `test_formal_config.py`, a module of this directory the census already reads, again by its constant path under another name; that module never imports the loader's module at any depth",
        ("test_formal_config_presence", "fresh_module", "importlib.util.module_from_spec(spec)", 1),
    ),
    **allowed(
        "executes `test_formal_config.py`, a module of this directory the census already reads, again by its constant path under another name; that module never imports the loader's module at any depth",
        (
            "test_formal_config_presence",
            "fresh_module",
            "importlib.util.spec_from_file_location('formal_config_under_plant', MODULE)",
            1,
        ),
    ),
    **allowed(
        "executes `test_formal_config.py`, a module of this directory the census already reads, again by its constant path under another name; that module never imports the loader's module at any depth",
        ("test_formal_config_presence", "fresh_module", "spec.loader.exec_module(module)", 1),
    ),
    **allowed(
        "a pool of the test's own callables, each running the real program itself; the in-process judge runs before the pool, one at a time; never a workflow file",
        (
            "test_mutation_python_shard_binding",
            "TheInProcessJudgeIsTheProgram.test_the_program_and_its_in_process_judgement_agree",
            "ThreadPoolExecutor(max_workers=8)",
            1,
        ),
    ),
    **allowed(
        "formats the exception the test is handling, to write to the standard error the judge replaces",
        ("test_mutation_python_shard_binding", "judge_in_process", "traceback.format_exc()", 1),
    ),
    **allowed(
        "compiles and executes the production verdict script's own source with edits to the interpolated fields of its messages, in the namespace of the program the test loaded and owns",
        (
            "test_mutation_python_shard_binding",
            "mutating",
            "exec(planted(function, node, kind, positions, value), program.__dict__)",
            1,
        ),
        (
            "test_mutation_python_shard_binding",
            "observed_fields",
            "compile(ast.Module([wrapped], []), str(VERDICT), 'exec')",
            1,
        ),
        (
            "test_mutation_python_shard_binding",
            "observed_fields",
            "exec(compile(ast.Module([wrapped], []), str(VERDICT), 'exec'), program.__dict__)",
            1,
        ),
        (
            "test_mutation_python_shard_binding",
            "planted",
            "compile(ast.Module([mutated], []), str(VERDICT), 'exec')",
            1,
        ),
    ),
    **allowed(
        "reaches the loaded program's own namespace by a constant name, which the test owns",
        ("test_mutation_python_shard_binding", "mutating", "program.__dict__", 3),
        ("test_mutation_python_shard_binding", "observed_fields", "program.__dict__", 3),
        (
            "test_mutation_python_shard_binding",
            "observed_fields",
            "program.__dict__.update(originals)",
            1,
        ),
    ),
    **allowed(
        "loads the production verdict script by its constant path under a name of its own and registers it, so its dataclasses resolve; never a module of the test directory",
        (
            "test_mutation_python_shard_binding",
            "verdict_program",
            "importlib.util.module_from_spec(spec)",
            1,
        ),
        (
            "test_mutation_python_shard_binding",
            "verdict_program",
            "importlib.util.spec_from_file_location('mutation_verdict_in_process', VERDICT)",
            1,
        ),
        (
            "test_mutation_python_shard_binding",
            "verdict_program",
            "spec.loader.exec_module(loaded)",
            1,
        ),
        ("test_mutation_python_shard_binding", "verdict_program", "sys.modules", 1),
        (
            "test_mutation_python_shard_binding",
            "verdict_program",
            "sys.modules.get('mutation_verdict_in_process')",
            1,
        ),
    ),
    **allowed(
        "the missing-tool test's own site census, which parses the runner's source text as a syntax tree: it imports a module by the name a plant gives it, and reaches an attribute by a name a node holds, to decide whether the census would see that reach; it reads no workflow file and brings in no module of the test directory",
        (
            "test_mutation_rows_missing_tool",
            "TheMissingToolPopulation.test_the_census_refuses_every_name_that_reaches_what_it_has_not_read",
            "importlib.import_module(name)",
            2,
        ),
        (
            "test_mutation_rows_missing_tool",
            "TheMissingToolPopulation.test_the_census_refuses_every_name_that_reaches_what_it_has_not_read",
            "vars(module)",
            1,
        ),
        ("test_mutation_rows_missing_tool", "reached_module", "getattr(base, node.attr, None)", 1),
        ("test_mutation_rows_missing_tool", "referenced", "getattr(source, alias.name, None)", 1),
        ("test_mutation_rows_missing_tool", "referenced", "importlib.import_module(name)", 1),
        (
            "test_mutation_rows_missing_tool",
            "referenced",
            "importlib.import_module(node.module)",
            1,
        ),
    ),
    **allowed(
        "an import of this repository's own script module by its constant name, `import mutation_rows`, after a sys.path insert of the constant scripts directory; it reads no workflow file",
        ("test_mutation_rows_refusal", "<module>", "import mutation_rows as runner", 1),
    ),
    **allowed(
        "reads the standard `errno` table and the `__cause__` of an exception the test holds, to compare a refusal's reason with the error that caused it; it reads no file and no workflow",
        ("test_mutation_rows_refusal", "<module>", "errno.errorcode", 1),
        (
            "test_mutation_rows_refusal",
            "TheRefusalIsReadWhole.test_a_spawn_error_that_names_something_else_is_not_a_refusal",
            "errno.ENOENT",
            2,
        ),
        (
            "test_mutation_rows_refusal",
            "TheRefusalIsReadWhole.test_every_errno_at_the_spawn_is_the_same_refusal_naming_the_tool",
            "refused.__cause__",
            1,
        ),
        (
            "test_mutation_rows_refusal",
            "TheRefusalIsReadWhole.test_only_the_two_exits_of_a_program_that_cannot_run_are_refusals_at_every_route",
            "raised.exception.__cause__",
            1,
        ),
        ("test_mutation_rows_refusal", "outcome", "error.__cause__", 2),
    ),
    **allowed(
        "reads the ban filter through configparser, the way the ban service reads it; never a workflow",
        ("test_sync_ban", "failregexes", "configparser.BasicInterpolation()", 1),
        (
            "test_sync_ban",
            "failregexes",
            "configparser.ConfigParser(interpolation=configparser.BasicInterpolation(), inline_comment_prefixes=';')",
            1,
        ),
    ),
    **allowed(
        "probes, then restores, the collation locale the refusal is measured under; reads no file, never a workflow",
        (
            "_support",
            "collation_locale_available",
            "locale.Error",
            1,
        ),
        (
            "_support",
            "collation_locale_available",
            "locale.LC_COLLATE",
            3,
        ),
        (
            "_support",
            "collation_locale_available",
            "locale.setlocale(locale.LC_COLLATE)",
            1,
        ),
        (
            "_support",
            "collation_locale_available",
            "locale.setlocale(locale.LC_COLLATE, name)",
            1,
        ),
        (
            "_support",
            "collation_locale_available",
            "locale.setlocale(locale.LC_COLLATE, collation)",
            1,
        ),
        ("_support", "collation_locale_available", "collation", 1),
    ),
    **allowed(
        "decompresses the image data of the icon a production script writes, to compare its pixels (SPEC-352 A24); bytes in and bytes out, and it imports, runs and reads nothing",
        (
            "test_ios_icon",
            "WhatAnUploadNeeds.test_the_generated_icon_is_an_opaque_square_with_no_text",
            "zlib.decompress(idat)",
            1,
        ),
    ),
    **allowed(
        "names the exception class plistlib's XML parser raises on a malformed property list, to refuse it by name (SPEC-347 A13); a class caught, and it imports, runs and reads nothing",
        ("test_ios_app_tree", "plist_of", "ExpatError", 1),
    ),
    **allowed(
        "splits a settings value the test already holds as text into its scheme and host, to judge the sync endpoint (SPEC-347 A13); text in and parts out, and it imports, runs and reads nothing",
        ("test_ios_app_tree", "settings_problems", "urlsplit(value.replace('$()', ''))", 1),
    ),
    **allowed(
        "loads the re-release production script by the path the call names; never a module of the test directory",
        ("test_testflight_age", "load_script", "importlib.util.module_from_spec(spec)", 1),
        (
            "test_testflight_age",
            "load_script",
            "importlib.util.spec_from_file_location('testflight_age_under_test', SCRIPT)",
            1,
        ),
        ("test_testflight_age", "load_script", "spec.loader.exec_module(module)", 1),
    ),
    **allowed(
        "decodes a base64 segment of a token the test already holds as text, to check no credential part reaches a child or the log; text in and bytes out, and it imports, runs and reads nothing",
        (
            "test_testflight_age",
            "TheCheckDecides.test_a14_no_credential_part_reaches_a_child_or_the_log.<locals>.decode",
            "base64.urlsafe_b64decode(segment + '=' * (-len(segment) % 4))",
            1,
        ),
        (
            "test_testflight_age",
            "TheCheckHoldsItsEdges.test_the_read_is_made_as_specified",
            "base64.urlsafe_b64decode(head + '=' * (-len(head) % 4))",
            1,
        ),
    ),
}


# The census's killer (SPEC-190 A12). A plant is a set of edits to a copy of the test directory,
# each (file, how, text): `create` writes a new file, `append` adds to a file's text, and `replace`
# swaps a text that occurs once in the file for another. The census test runs on the copy: every
# plant is red there and names the module it names, and every control is green.
PLANT_READ = 'WORKFLOWS / "release.yml"'
PLANT_READS = {
    "read_text": f"({PLANT_READ}).read_text(encoding='utf-8')",
    "open() without newline": f"open({PLANT_READ}, encoding='utf-8').read()",
    "io.open": f"io.open({PLANT_READ}, encoding='utf-8').read()",
    "pathlib open": f"({PLANT_READ}).open(encoding='utf-8').read()",
    "codecs.open": f"codecs.open(str({PLANT_READ}), encoding='utf-8').read()",
    "subprocess text=True": (
        f"subprocess.run(['cat', str({PLANT_READ})], capture_output=True, text=True).stdout"
    ),
    "git show text=True": (
        "subprocess.check_output(['git', 'show', 'HEAD:.github/workflows/release.yml'], text=True)"
    ),
    "getattr read_text": f"getattr({PLANT_READ}, 'read_text')(encoding='utf-8')",
    "io.FileIO": f"io.FileIO(str({PLANT_READ})).read().decode('utf-8')",
    "io.TextIOWrapper over a raw file": (
        f"io.TextIOWrapper(io.FileIO(str({PLANT_READ})), encoding='utf-8').read()"
    ),
    "functools.partial read_text": (
        f"functools.partial(pathlib.Path.read_text, encoding='utf-8')({PLANT_READ})"
    ),
}
PLANT_IMPORTS = "import codecs, functools, io, pathlib, subprocess\n"
PLANT_MORE = "import fileinput, linecache, mmap, operator, os, sys, tokenize\n"
PLANT_FEED = "from test_ci_workflows import WORKFLOWS, read_workflow\n"
PLANT_FEEDS = "from test_workflow_concurrency import WORKFLOWS, read_workflow\n"
PLANT_ZZ = "test_zz_plant.py"
PLANT_CONCURRENCY = "test_workflow_concurrency.py"
PLANT_CI = Path(__file__).name
PLANT_GATE = (
    '    return STAGES.search((REPO / "scripts" / "check.sh").read_text()).group(1).split()\n'
)
PLANT_LOADER = '    return Path(path).read_bytes().decode("utf-8")\n'


def plant_site(expression):
    """A function that feeds the reader with `expression`."""
    return f"\n\ndef plant_site():\n    return read_workflow({expression})\n"


def planted(file, how, text, module):
    """A plant of one edit: red naming `module`, or a control, green, when `module` is None."""
    return ((file, how, text),), module


def plant_module(body):
    """A plant that writes a new test module holding `body`."""
    return planted(PLANT_ZZ, "create", '"""A plant."""\n' + body, "test_zz_plant")


def plant_kind(body):
    """A plant appended to a module that feeds the reader, with the modules a site kind names."""
    return planted(
        PLANT_CONCURRENCY,
        "append",
        PLANT_IMPORTS + PLANT_MORE + body,
        "test_workflow_concurrency",
    )


PLANTS = {
    "a control: no plant": ((), None),
    "a control: a site through the loader, in a module that feeds the reader": planted(
        PLANT_CONCURRENCY,
        "append",
        plant_site(f"workflow_file_text({PLANT_READ})"),
        None,
    ),
    "a control: a new module that feeds the reader through the loader": planted(
        PLANT_ZZ,
        "create",
        '"""A plant."""\n'
        "from test_ci_workflows import WORKFLOWS, read_workflow, workflow_file_text\n"
        + plant_site(f"workflow_file_text({PLANT_READ})"),
        None,
    ),
    **{
        f"a module that feeds the reader: {label}": planted(
            PLANT_CONCURRENCY,
            "append",
            PLANT_IMPORTS + plant_site(expression),
            "test_workflow_concurrency",
        )
        for label, expression in PLANT_READS.items()
    },
    **{
        f"a new module importing a re-export: {label}": plant_module(
            PLANT_IMPORTS + PLANT_FEEDS + plant_site(expression)
        )
        for label, expression in PLANT_READS.items()
    },
    "a new module: an import by a computed name": plant_module(
        "import importlib\n\n\ndef plant_site():\n"
        '    ci = importlib.import_module("test_" + "ci_workflows")\n'
        '    return ci.read_workflow((ci.WORKFLOWS / "release.yml").read_text(encoding="utf-8"))\n'
    ),
    "_support.py: a read_text feed": planted(
        "_support.py",
        "append",
        "\n\ndef plant_site():\n    "
        + PLANT_FEED
        + "\n"
        + '    return read_workflow((WORKFLOWS / "release.yml").read_text(encoding="utf-8"))\n',
        "_support",
    ),
    "a second function named like the loader": planted(
        PLANT_CONCURRENCY,
        "append",
        "\n\ndef workflow_file_text(path):\n    return path.read_text(encoding='utf-8')\n"
        + plant_site(f"workflow_file_text({PLANT_READ})"),
        "test_workflow_concurrency",
    ),
    "a site inside an async function": planted(
        PLANT_CONCURRENCY,
        "append",
        f"\n\nasync def plant_site():\n    return read_workflow(({PLANT_READ}).read_text())\n",
        "test_workflow_concurrency",
    ),
    "a site in a lambda at module level": planted(
        PLANT_CONCURRENCY,
        "append",
        f"\n\nPLANT = lambda: read_workflow(({PLANT_READ}).read_text(encoding='utf-8'))\n",
        "test_workflow_concurrency",
    ),
    "a site in a class body": planted(
        PLANT_CONCURRENCY,
        "append",
        f"\n\nclass Plant:\n    TEXT = read_workflow(({PLANT_READ}).read_text(encoding='utf-8'))\n",
        "test_workflow_concurrency",
    ),
    "a listed function gains a feed site": planted(
        PLANT_CI,
        "replace",
        (
            "def gate_stages():\n",
            "def gate_stages():\n    read_workflow((WORKFLOWS / 'release.yml').read_text())\n",
        ),
        "test_ci_workflows",
    ),
    "a listed read swapped for a feed read, the count kept": planted(
        PLANT_CI,
        "replace",
        (
            PLANT_GATE,
            "    read_workflow((WORKFLOWS / 'release.yml').read_text())\n"
            "    return STAGES.search(workflow_file_text(REPO / 'scripts' / 'check.sh'))"
            ".group(1).split()\n",
        ),
        "test_ci_workflows",
    ),
    **{
        f"a site kind: {label}": plant_kind(body)
        for label, body in {
            "os.read of os.open": plant_site(
                f"os.read(os.open(str({PLANT_READ}), os.O_RDONLY), 1 << 20).decode()"
            ),
            "os.fdopen": plant_site(f"os.fdopen(os.open(str({PLANT_READ}), os.O_RDONLY)).read()"),
            "os.popen": plant_site(f"os.popen('cat ' + str({PLANT_READ})).read()"),
            "linecache.getlines": plant_site(f"''.join(linecache.getlines(str({PLANT_READ})))"),
            "fileinput.input": plant_site(f"''.join(fileinput.input(str({PLANT_READ})))"),
            "open bound to another name": "\n\nOPENER = open\n"
            + plant_site(f"''.join(OPENER({PLANT_READ}))"),
            "io.FileIO imported under another name": "from io import FileIO as F\n"
            + plant_site(f"F(str({PLANT_READ})).readall().decode()"),
            "subprocess imported under another name": "import subprocess as sp\n"
            + plant_site(f"sp.check_output(['cat', str({PLANT_READ})]).decode()"),
            "Path.read_text passed to map": plant_site(
                f"''.join(map(pathlib.Path.read_text, [{PLANT_READ}]))"
            ),
            "operator.methodcaller": plant_site(
                f"operator.methodcaller('read_text')({PLANT_READ})"
            ),
            "a vars() subscript": plant_site(f"vars(pathlib.Path)['read_text']({PLANT_READ})"),
            "io.StringIO translating the loader's text": plant_site(
                f"io.StringIO(workflow_file_text({PLANT_READ}), newline=None).read()"
            ),
            "Popen.communicate": plant_site(
                f"subprocess.Popen(['cat', str({PLANT_READ})], stdout=subprocess.PIPE, "
                "text=True).communicate()[0]"
            ),
            "a read in a default argument": f"\n\ndef plant_site(text=({PLANT_READ}).read_text()):"
            "\n    return read_workflow(text)\n",
            "a read in an f-string": plant_site(f"f'{{({PLANT_READ}).read_text()}}'"),
            "a read in a nested function": "\n\ndef plant_site():\n    def inner():\n"
            f"        return ({PLANT_READ}).read_text()\n\n    return read_workflow(inner())\n",
            "sys.stdin": plant_site("sys.stdin.read()"),
            "a lambda as the callee": plant_site(f"(lambda p: p.read_text())({PLANT_READ})"),
            "a subscript as the callee": plant_site(f"[pathlib.Path.read_text][0]({PLANT_READ})"),
            "mmap": plant_site(
                f"mmap.mmap(open({PLANT_READ}).fileno(), 0, access=mmap.ACCESS_READ)[:].decode()"
            ),
            "codecs.open imported under another name": "from codecs import open as copen\n"
            + plant_site(f"copen(str({PLANT_READ})).read()"),
            "tokenize.open": plant_site(f"tokenize.open({PLANT_READ}).read()"),
            "a rebinding of Path.read_bytes": (
                "\n\npathlib.Path.read_bytes = pathlib.Path.read_text\n"
            ),
        }.items()
    },
    "a path in: a helper module a feeding module imports": (
        (
            (
                "_raw.py",
                "create",
                '"""A plant."""\n\n\ndef text(path):\n    return path.read_text()\n',
            ),
            (
                PLANT_CONCURRENCY,
                "append",
                "\nimport _raw\n" + plant_site(f"_raw.text({PLANT_READ})"),
            ),
        ),
        "test_workflow_concurrency",
    ),
    "a path in: a reader the population defines, handed the workflow directory": planted(
        PLANT_CONCURRENCY,
        "append",
        "\nfrom test_ci_workflows import module_sources\n"
        + plant_site("module_sources(WORKFLOWS)['release']"),
        "test_workflow_concurrency",
    ),
    "a method named like a read, called on its instance, that reads": planted(
        PLANT_CONCURRENCY,
        "append",
        f"\n\nclass Plant:\n    def read(self):\n        return ({PLANT_READ}).read_text()\n"
        "\n    def text(self):\n        return read_workflow(self.read())\n",
        "test_workflow_concurrency",
    ),
    "a module a listed load returned, a member of it called anew": planted(
        "test_not_started_legs.py",
        "replace",
        (
            "        module = verdict_module()\n",
            "        module = verdict_module()\n        module.workflow_text(CI)\n",
        ),
        "test_not_started_legs",
    ),
    "a path in: import as": plant_module(
        "import test_workflow_concurrency as c\n\n\ndef plant_site():\n"
        f"    return c.read_workflow((c.{PLANT_READ}).read_text())\n"
    ),
    "a path in: a star import": plant_module(
        "from test_workflow_concurrency import *\n" + plant_site(f"({PLANT_READ}).read_text()")
    ),
    "a path in: an import inside the function": plant_module(
        "\n\ndef plant_site():\n    "
        + PLANT_FEED
        + f"\n    return read_workflow(({PLANT_READ}).read_text())\n"
    ),
    "a path in: an import under try": plant_module(
        "try:\n    "
        + PLANT_FEED
        + "except ImportError:\n    read_workflow = None\n"
        + plant_site(f"({PLANT_READ}).read_text()")
    ),
    "a path in: a relative import": plant_module(
        "from .test_ci_workflows import WORKFLOWS, read_workflow\n"
        + plant_site(f"({PLANT_READ}).read_text()")
    ),
    "a path in: a two-level re-export": (
        (
            ("_mid.py", "create", '"""A plant."""\n' + PLANT_FEED),
            (
                PLANT_ZZ,
                "create",
                '"""A plant."""\nfrom _mid import WORKFLOWS, read_workflow\n'
                + plant_site(f"({PLANT_READ}).read_text()"),
            ),
        ),
        "test_zz_plant",
    ),
    "a path in: a module in a package": planted(
        "pkg/plant.py",
        "create",
        '"""A plant."""\n' + PLANT_FEED + plant_site(f"({PLANT_READ}).read_text()"),
        "pkg.plant",
    ),
    "a path in: a module whose name does not start test_": planted(
        "_feed.py",
        "create",
        '"""A plant."""\n' + PLANT_FEED + plant_site(f"({PLANT_READ}).read_text()"),
        "_feed",
    ),
    "a path in: the repository's dotted spelling": plant_module(
        "import scripts.tests.test_ci_workflows as ci\n\n\ndef plant_site():\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "a path in: the reader imported under another name": plant_module(
        "from test_ci_workflows import WORKFLOWS, read_workflow as rw\n\n\ndef plant_site():\n"
        f"    return rw(({PLANT_READ}).read_text())\n"
    ),
    "dynamic: __import__": plant_module(
        "def plant_site():\n    ci = __import__('test_ci_workflows')\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "dynamic: spec_from_file_location on a test module": plant_module(
        "import importlib.util\nfrom pathlib import Path\n\n\ndef plant_site():\n"
        "    spec = importlib.util.spec_from_file_location(\n"
        "        'ci', Path(__file__).parent / 'test_ci_workflows.py'\n    )\n"
        "    ci = importlib.util.module_from_spec(spec)\n    spec.loader.exec_module(ci)\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "dynamic: runpy.run_path": plant_module(
        "import runpy\n\n\ndef plant_site():\n"
        "    ci = runpy.run_path('test_ci_workflows.py')\n"
        "    return ci['read_workflow']((ci['WORKFLOWS'] / 'release.yml').read_text())\n"
    ),
    "dynamic: a sys.modules lookup": plant_module(
        "import sys\n\n\ndef plant_site():\n    ci = sys.modules['test_ci_workflows']\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "dynamic: exec of a string": plant_module(
        "def plant_site():\n    held = {}\n"
        "    exec('from test_ci_workflows import WORKFLOWS, read_workflow', held)\n"
        "    return held['read_workflow']((held['WORKFLOWS'] / 'release.yml').read_text())\n"
    ),
    "dynamic: mock.patch of a target named in a string": plant_module(
        "from unittest import mock\n\n\ndef plant_site():\n"
        "    with mock.patch('test_ci_workflows.WORKFLOWS') as held:\n        return held\n"
    ),
    "dynamic: getattr of the builtins from globals()": plant_module(
        "def plant_site():\n    load = getattr(globals()['__builtins__'], '__import__')\n"
        "    ci = load('test_ci_workflows')\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "dynamic: a test loader taking a module by name": plant_module(
        "import unittest\n\n\ndef plant_site():\n"
        "    return unittest.defaultTestLoader.loadTestsFromName('test_ci_workflows')\n"
    ),
    "dynamic: pkgutil.resolve_name": plant_module(
        "import pkgutil\n\n\ndef plant_site():\n"
        "    return pkgutil.resolve_name('test_ci_workflows:read_workflow')\n"
    ),
    "dynamic: builtins.__import__": plant_module(
        "import builtins\n\n\ndef plant_site():\n"
        "    return builtins.__import__('test_ci_workflows')\n"
    ),
    "dynamic: gc.get_objects": plant_module(
        "import gc\n\n\ndef plant_site():\n    return [m for m in gc.get_objects() if m]\n"
    ),
    "dynamic: importlib.import_module with a constant name": plant_module(
        "import importlib\n\n\ndef plant_site():\n"
        "    ci = importlib.import_module('test_ci_workflows')\n"
        f"    return ci.read_workflow((ci.{PLANT_READ}).read_text())\n"
    ),
    "the loader: imported under another name": planted(
        PLANT_CONCURRENCY,
        "append",
        "\nfrom test_ci_workflows import workflow_file_text as held\n",
        "test_workflow_concurrency",
    ),
    "the loader: an attribute rebinding": planted(
        PLANT_CONCURRENCY,
        "append",
        "\nimport test_ci_workflows\n\ntest_ci_workflows.workflow_file_text = str\n",
        "test_workflow_concurrency",
    ),
    "the loader: its name in a string": planted(
        PLANT_CONCURRENCY,
        "append",
        "\nimport test_ci_workflows\nfrom unittest import mock\n\n"
        "HELD = mock.patch.object(test_ci_workflows, 'workflow_file_text', str)\n",
        "test_workflow_concurrency",
    ),
    "the loader: its body reads text": planted(
        PLANT_CI,
        "replace",
        (PLANT_LOADER, '    return Path(path).read_text(encoding="utf-8")\n'),
        "test_ci_workflows",
    ),
    "the loader: a second definition as a method": planted(
        PLANT_CONCURRENCY,
        "append",
        "\n\nclass Plant:\n    def workflow_file_text(self, path):\n"
        "        return path.read_bytes().decode()\n",
        "test_workflow_concurrency",
    ),
    "a listed site: a read moved to another function": planted(
        PLANT_CI,
        "replace",
        (
            PLANT_GATE,
            "    return STAGES.search(check_text()).group(1).split()\n\n\n"
            "def check_text():\n"
            '    return (REPO / "scripts" / "check.sh").read_text()\n',
        ),
        "test_ci_workflows",
    ),
    "a listed site: a dynamic import's target swapped": planted(
        "test_mutation_verdict.py",
        "replace",
        (
            'spec_from_file_location("mutation_verdict_constants", VERDICT)',
            'spec_from_file_location("mutation_verdict_constants", Path(__file__))',
        ),
        "test_mutation_verdict",
    ),
    "a listed site: a read's arguments changed": planted(
        PLANT_CI,
        "replace",
        (
            PLANT_GATE,
            '    return STAGES.search((REPO / "scripts" / "check.sh").read_text("utf-8"))'
            ".group(1).split()\n",
        ),
        "test_ci_workflows",
    ),
}


class PlantResult(unittest.TestResult):
    """A test result that keeps a failure's message without its traceback, so a plant's red is
    judged by what the census says, never by a path a traceback names."""

    def addFailure(self, test, err):
        self.failures.append((test, str(err[1])))


class WorkflowFilesAreReadAsBytes(unittest.TestCase):
    """A workflow file reaches the reader as GitHub's parser is given it: its bytes, decoded as
    UTF-8 and not translated (SPEC-190 R12). Every read of a workflow file in the test modules goes
    through one loader, and a census of those modules' file reads is closed."""

    RELEASE = workflow_file_text(WORKFLOWS / "release.yml")
    BLOCK = "jobs:\n  a:\n    steps:\n      - run: |\n          echo a__b\n          echo c\n"

    def said(self, why):
        return "; ".join(why.refused) if isinstance(why, Unread) else str(why)

    def shapes(self):
        """(label, bytes, what a read says): a lone carriage return in a committed-shape workflow,
        one inside a block scalar's text, and a byte-order mark; each refused by name."""
        held = "  cancel-in-progress: false\n  queue: max\n"
        self.assertEqual(self.RELEASE.count(held), 1)
        return (
            (
                "a carriage return inside the concurrency block",
                self.RELEASE.replace(held, held.replace("\n  queue", "\r  queue", 1)).encode(),
                "a carriage return that does not end a line",
            ),
            (
                "a carriage return inside a block scalar",
                self.BLOCK.replace("__", "\r").encode(),
                "a carriage return that does not end a line",
            ),
            ("a byte-order mark", b"\xef\xbb\xbf" + self.RELEASE.encode(), "a byte-order mark"),
        )

    def readers(self, directory):
        """Every loader the test modules read a workflow file with, each as a function of a name in
        `directory`, the module's workflow directory pointed at it."""
        import test_rust_cache_workflow as rust_cache
        import test_workflow_concurrency as concurrency

        here = sys.modules[__name__]

        def through_concurrency(name):
            with mock.patch.object(concurrency, "WORKFLOWS", directory):
                (found,) = concurrency.read_all()
            return found[1]

        def through_rust_cache(name):
            with mock.patch.object(rust_cache, "WORKFLOWS", directory):
                return rust_cache.load(name)

        def through_load(name):
            with mock.patch.object(here, "WORKFLOWS", directory):
                return load(name)

        return (
            ("load", through_load),
            ("read_hardened", lambda name: read_hardened(directory / name)),
            ("read_all", through_concurrency),
            ("the cache tests' load", through_rust_cache),
        )

    def test_a_lone_carriage_return_or_a_byte_order_mark_in_a_file_is_refused_by_name(self):
        for label, raw, phrase in self.shapes():
            with tempfile.TemporaryDirectory() as scratch:
                directory = Path(scratch)
                (directory / "release.yml").write_bytes(raw)
                for reader, read in self.readers(directory):
                    with self.subTest(shape=label, reader=reader):
                        try:
                            read("release.yml")
                        except AssertionError as why:
                            self.assertIn(phrase, self.said(why))
                        else:
                            self.fail("the file was read, not refused")

    def test_a_file_that_ends_its_lines_in_crlf_reads_as_the_same_file_ending_in_lf(self):
        for text in (self.RELEASE, self.BLOCK.replace("__", "")):
            with tempfile.TemporaryDirectory() as scratch:
                directory = Path(scratch)
                (directory / "release.yml").write_bytes(text.replace("\n", "\r\n").encode())
                for reader, read in self.readers(directory):
                    with self.subTest(reader=reader):
                        self.assertEqual(read("release.yml"), read_workflow(text))

    def test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read(self):
        problems, modules, population, sites = census_problems(Path(__file__).parent)
        examined("test modules the census read", range(modules))
        examined("test modules in the loader's population", range(population))
        examined("listed sites the census counted", range(sites))
        self.assertGreater(population, 1, "the loader's module alone is no population")
        self.assertEqual(problems, [])

    def census_sites(self, source):
        """The sites one module's source holds that read or are dynamic, as (qualified name,
        text), in order."""
        return sorted(
            (qual, text)
            for qual, text, reads, dynamic, *_rest in module_census(source).sites
            if reads or dynamic
        )

    def test_the_census_is_red_on_a_read_it_does_not_name(self):
        planted = "def extra(path):\n    return read_workflow(path.read_text(encoding='utf-8'))\n"
        self.assertEqual(
            self.census_sites(planted), [("extra", "path.read_text(encoding='utf-8')")]
        )
        self.assertEqual(
            self.census_sites("TEXT = open('x').read()\n"),
            [("<module>", "open('x')"), ("<module>", "open('x').read()")],
        )
        # The loader's body outside the loader's module is a read, and a definition of its name.
        loader = f"def {LOADER[1]}(path):\n    return path.read_bytes().decode('utf-8')\n"
        self.assertEqual(self.census_sites(loader), [(LOADER[1], "path.read_bytes()")])
        self.assertEqual(
            module_census(loader).loader,
            (("a definition", "<module>", f"def {LOADER[1]}(path):", None),),
        )
        for source, sites in (
            ("TEXT = (lambda: 'x')()\n", [("<module>", "(lambda: 'x')()")]),
            (
                "def f(p, name):\n    return getattr(p, name)()\n",
                [("f", "getattr(p, name)"), ("f", "getattr(p, name)()")],
            ),
            (
                "def f(p):\n    return getattr(p, 'read_text')()\n",
                [("f", "getattr(p, 'read_text')"), ("f", "getattr(p, 'read_text')()")],
            ),
            (
                "import importlib\nM = importlib.import_module('x')\n",
                [("<module>", "importlib.import_module('x')")],
            ),
            ("import sys\nM = sys.modules['x']\n", [("<module>", "sys.modules")]),
            ("exec('x = 1')\n", [("<module>", "exec('x = 1')")]),
            (
                "import subprocess\nT = subprocess.run(['git'], text=True).stdout\n",
                [("<module>", "subprocess.run(['git'], text=True)")],
            ),
            ("class C:\n    T = open('x')\n", [("C", "open('x')")]),
            ("F = lambda p: p.read_text()\n", [("<lambda>", "p.read_text()")]),
            # A method its class defines, called on its instance, is read by its body, not its name;
            # stored over, it is a read again.
            (
                "class S:\n    def read(self):\n        return 1\n\n    def f(self):\n"
                "        s = S()\n        return self.read() + s.read()\n",
                [],
            ),
            (
                "class S:\n    def read(self):\n        return 1\n\n    def f(self):\n"
                "        self.read = print\n        return self.read()\n",
                [("S.f", "self.read = print"), ("S.f", "self.read()")],
            ),
        ):
            with self.subTest(source=source):
                self.assertEqual(self.census_sites(source), sites)
        method = f"class C:\n    def {LOADER[1]}(self, path):\n        return path\n"
        self.assertEqual(
            module_census(method).loader,
            (("a definition", "C", f"def {LOADER[1]}(self, path):", None),),
        )

    def test_the_census_is_red_on_every_planted_site(self):
        tests = Path(__file__).parent
        guard = (
            "test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read"
        )
        for label, (edits, module) in examined("plants", PLANTS.items()):
            with self.subTest(plant=label), tempfile.TemporaryDirectory() as scratch:
                copy = Path(scratch) / "scripts" / "tests"
                for path in tests.rglob("*.py"):
                    (copy / path.relative_to(tests)).parent.mkdir(parents=True, exist_ok=True)
                    (copy / path.relative_to(tests)).symlink_to(path)
                for file, how, text in edits:
                    target = copy / file
                    target.parent.mkdir(parents=True, exist_ok=True)
                    # The original is read through the loader, so the copy holds its bytes.
                    original = "" if how == "create" else workflow_file_text(tests / file)
                    if how == "replace":
                        self.assertEqual(original.count(text[0]), 1, f"{label}: {file}")
                        original, text = original.replace(*text), ""
                    target.unlink(missing_ok=True)
                    target.write_bytes((original + text).encode("utf-8"))
                case = WorkflowFilesAreReadAsBytes(guard)
                case.maxDiff = None
                result = PlantResult()
                with (
                    mock.patch.object(sys.modules[__name__], "__file__", str(copy / PLANT_CI)),
                    contextlib.redirect_stdout(None),
                ):
                    case.run(result)
                said = "\n".join(message for _test, message in result.failures)
                self.assertEqual(result.errors, [], label)
                if module is None:
                    self.assertEqual(result.failures, [], said)
                else:
                    self.assertTrue(result.failures, f"{label}: the census was green")
                    self.assertRegex(said, rf"[\"'\[]{re.escape(module)}[:'\"]")

    def builtin_exceptions(self):
        """The builtin names whose value is an exception class, derived here by its own expression
        and not from the census's."""
        return sorted(
            name
            for name in dir(builtins)
            if isinstance(getattr(builtins, name), type)
            and BaseException in type.mro(getattr(builtins, name))
        )

    def plant_problems(self, body):
        """The problems the census finds under a scratch copy of this directory that holds one new
        module, `test_zz_plant`, which imports the loader's module and then runs `body`; as the
        lines that name that module."""
        tests = Path(__file__).parent
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch) / "scripts" / "tests"
            for path in tests.rglob("*.py"):
                (copy / path.relative_to(tests)).parent.mkdir(parents=True, exist_ok=True)
                (copy / path.relative_to(tests)).symlink_to(path)
            header = '"""A plant."""\nfrom test_ci_workflows import workflow_file_text\n'
            (copy / PLANT_ZZ).write_bytes((header + body).encode("utf-8"))
            problems, _modules, _population, _sites = census_problems(copy)
        return [line for line in problems if line.startswith("test_zz_plant: ")]

    def test_every_builtin_exception_class_is_placed_by_its_value(self):
        names = self.builtin_exceptions()
        examined("builtin exception classes planted", names)
        self.assertGreaterEqual(len(names), 60)
        for member in ("RecursionError", "GeneratorExit", "BaseExceptionGroup", "StopIteration"):
            self.assertIn(member, names)
        # An alias is a member by its value: `IOError` and `EnvironmentError` are `OSError`.
        for alias in ("IOError", "EnvironmentError"):
            self.assertIn(alias, names)
        body = "".join(
            f"\n\ndef plant_except_{name}():\n    try:\n        pass\n    except {name}:\n"
            f"        pass\n"
            f"\n\ndef plant_raise_{name}():\n    raise {name}\n"
            f"\n\ndef plant_isinstance_{name}(value):\n    return isinstance(value, {name})\n"
            for name in names
        )
        # A control in the same module: a name the census does not place stays red, so the module
        # was read and the check is live.
        control = "test_zz_plant: plant_control: memoryview: a name the census cannot place"
        said = self.plant_problems(body + "\n\ndef plant_control():\n    return memoryview\n")
        self.assertIn(control, said)
        refused = sorted({line.split(": ")[1].split("_", 2)[2] for line in said if line != control})
        self.assertEqual(
            refused,
            [],
            f"{len(refused)} builtin exception classes the census cannot place, among them "
            f"{refused[:3]}",
        )

    def test_every_other_builtin_name_stays_red_and_every_read_or_dynamic_builtin_is_a_site(self):
        placed = (
            BENIGN_BUILTINS
            | ATTRIBUTE_BUILTINS
            | READ_BUILTINS
            | BARE_DYNAMIC
            | set(self.builtin_exceptions())
        )
        others = sorted(
            name
            for name in dir(builtins)
            if name not in placed
            and not (name.startswith("__") and name.endswith("__"))
            and name.isidentifier()
            and isinstance(ast.parse(name, mode="eval").body, ast.Name)
        )
        sites = sorted(READ_BUILTINS | BARE_DYNAMIC)
        examined("other builtin names planted", others)
        examined("read or dynamic builtin names planted", sites)
        self.assertGreater(len(others), 0)
        self.assertIn("memoryview", others)
        self.assertIn("slice", others)
        body = "".join(f"\n\ndef plant_{name}():\n    return {name}\n" for name in others + sites)
        said = set(self.plant_problems(body))
        for name in others:
            with self.subTest(name=name):
                self.assertIn(
                    f"test_zz_plant: plant_{name}: {name}: a name the census cannot place", said
                )
        for name in sites:
            with self.subTest(site=name):
                kind = "read" if name in READ_BUILTINS else "dynamic"
                self.assertIn(
                    f"test_zz_plant: plant_{name}: {name}: 1 {kind} site(s), 0 listed", said
                )


# The download step a planted harness job uses; its ref is a placeholder the checker never reads.
PLANTED_DOWNLOAD = "actions/download-artifact@" + "0" * 40


def harness_link_problems(workflow):
    """Each way the `harness` job of a read workflow could link a framework its own run did not
    build, named (SPEC-339 R12, A9): it waits on the `xcframework` job, downloads exactly one
    artifact, that job's `xcframework`, and names no run, token or repository by which its download
    could reach another run's artifact."""
    job = (workflow.get("jobs") or {}).get("harness") or {}
    needs = job.get("needs")
    needs = [needs] if isinstance(needs, str) else list(needs or [])
    problems = []
    if "xcframework" not in needs:
        problems.append("harness: it does not wait on the xcframework job")
    downloads = [s for s in job.get("steps") or [] if action(s) == "actions/download-artifact"]
    if len(downloads) != 1:
        problems.append(f"harness: {len(downloads)} artifact downloads, not one")
    for step in downloads:
        given = step.get("with") or {}
        if given.get("name") != "xcframework":
            problems.append(f"harness: it downloads {given.get('name')!r}, not 'xcframework'")
        for key in ("run-id", "github-token", "repository"):
            if key in given:
                problems.append(f"harness: its download names a {key}, so it can reach another run")
    return problems


class TheHarnessLinksItsOwnRunsFramework(unittest.TestCase):
    def test_the_harness_links_the_framework_its_own_run_built(self):
        """SPEC-339 A9: the harness job links the XCFramework this run's `xcframework` job built,
        never one another run left behind."""
        jobs = load("xcframework.yml")["jobs"]
        self.assertIn("harness", list(jobs))
        examined("harness steps", jobs["harness"].get("steps") or [])
        self.assertEqual(harness_link_problems({"jobs": jobs}), [])

    def test_the_harness_job_timeout_holds_the_planted_suite(self):
        """SPEC-361 A16: the `harness` job's `timeout-minutes` is a digit string inside the band the
        planted suite's measured run needs, so a PR's harness is not cancelled at its own bound."""
        job = load("xcframework.yml")["jobs"]["harness"]
        examined("harness keys", list(job))
        minutes = str(job.get("timeout-minutes") or "")
        band = f"{HARNESS_TIMEOUT_MINUTES.start} to {HARNESS_TIMEOUT_MINUTES.stop - 1}"
        self.assertTrue(
            minutes.isdigit() and int(minutes) in HARNESS_TIMEOUT_MINUTES,
            f"the harness job's timeout is {minutes or 'unset'}, not {band} minutes",
        )

        # The controls: the good job is accepted, and each plant is refused by its rule's name.
        good = {
            "needs": ["xcframework"],
            "steps": [{"uses": PLANTED_DOWNLOAD, "with": {"name": "xcframework"}}],
        }

        def download(**given):
            return {**good, "steps": [{"uses": PLANTED_DOWNLOAD, "with": given}]}

        plants = {
            "the good job": (good, []),
            "a job that waits on nothing": (
                {**good, "needs": []},
                ["harness: it does not wait on the xcframework job"],
            ),
            "a job that waits on another job": (
                {**good, "needs": "harness-wire"},
                ["harness: it does not wait on the xcframework job"],
            ),
            "no download": (
                {**good, "steps": [{"run": "true"}]},
                ["harness: 0 artifact downloads, not one"],
            ),
            "a second download": (
                {**good, "steps": good["steps"] * 2},
                ["harness: 2 artifact downloads, not one"],
            ),
            "another artifact": (
                download(name="harness-fixture"),
                ["harness: it downloads 'harness-fixture', not 'xcframework'"],
            ),
            "another run": (
                download(name="xcframework", **{"run-id": "1"}),
                ["harness: its download names a run-id, so it can reach another run"],
            ),
            "a token": (
                download(name="xcframework", **{"github-token": "planted"}),
                ["harness: its download names a github-token, so it can reach another run"],
            ),
            "another repository": (
                download(name="xcframework", repository="planted/planted"),
                ["harness: its download names a repository, so it can reach another run"],
            ),
        }
        for name, (job, wanted) in examined("planted harness jobs", list(plants.items())):
            with self.subTest(plant=name):
                self.assertEqual(harness_link_problems({"jobs": {"harness": job}}), wanted, name)


# The app's steps in the `harness` job (SPEC-347 R13 and A14, ADR-358 D10; SPEC-348 R20, ADR-359
# D8): its generate line opens its own test step, so the step "the project, generated", which each
# TestFlight lane copies, keeps the harness's one line; the app's tests, the review screen's tests
# and the archive sit, in that order, between the harness's last step and the report.
APP_GENERATE = '"$RUNNER_TEMP/xcodegen/bin/xcodegen" generate --spec ios/app.yml'
HARNESS_GENERATE = '"$RUNNER_TEMP/xcodegen/bin/xcodegen" generate --spec ios/project.yml'
HARNESS_PROJECT = "the project, generated"
HARNESS_LAST = "the required-reason symbols the Release executable imports, against the manifest"
APP_TESTS = "the app's tests, Debug, on the iPhone and then the iPad"
REVIEW_TESTS = "the review screen's tests, Debug, on the iPhone and then the iPad"
REVIEW_ACTIONS_TESTS = "the bury and flag tests, Debug, on the iPhone and then the iPad"
APP_ARCHIVE = "the app, archived unsigned for a device"
HARNESS_REPORT = "the report"
# What the app's test step runs: the app's scheme on both simulators, one after the other, with
# its own derived data and result bundle, signed ad hoc on its command line because a simulator
# refuses an unsigned test host the Keychain (SPEC-347 section 6), and its time written for the
# report.
APP_TEST_NEEDS = (
    "xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug",
    '-destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS"',
    '-destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS"',
    "-disable-concurrent-destination-testing",
    '-derivedDataPath "$RUNNER_TEMP/app-debug"',
    '-resultBundlePath "$RESULTS/app.xcresult"',
    "CODE_SIGN_IDENTITY=-",
    '> "$REPORT/app-seconds"',
)
# What the app's archive step runs: a Release archive for a generic device, then the executable's
# required-reason imports read against the app's own privacy manifest, written for the report.
APP_ARCHIVE_NEEDS = (
    "xcodebuild archive -project ios/DeckStreak.xcodeproj -scheme DeckStreak",
    "-configuration Release",
    "-destination generic/platform=iOS",
    '-archivePath "$RUNNER_TEMP/DeckStreak.xcarchive"',
    "nm -u",
    "ios/App/PrivacyInfo.xcprivacy",
    '"app-symbols"',
)
# The report's rows for the app: its test cases, its test time, and its imports reading; and the
# rows the report gained beside them, the planted suite's cases and time and the first simulator's
# boot (SPEC-382 R1, R4), a superset that keeps every row the report had.
APP_REPORT_NEEDS = (
    'cases_in("app")',
    'minutes("app-seconds")',
    'read("app-symbols")',
    'cases_in("card-probe")',
    'minutes("card-probe-seconds")',
    'minutes("boot-seconds")',
)
# The signing settings only a command line may carry in this job, never its env or a step's.
SIGNING_NAMES = ("CODE_SIGNING_ALLOWED", "CODE_SIGN_IDENTITY", "DEVELOPMENT" + "_TEAM")


def run_joined(step):
    """A run step's text with each shell continuation joined, so a command reads as one line."""
    return re.sub(r"\\\n\s*", " ", str(step.get("run", "")))


def app_steps_problems(workflow):
    """Each way the Apple job body could fail to build and prove the app, named (SPEC-347 R13,
    A14): the `harness` job, on the admitted runner, holds one step of each name; the harness's
    generate step keeps its one line; the app's test step opens with the app's generate line and
    runs its scheme on both simulators, signed ad hoc on its command line; its archive step builds
    Release for a generic device with code signing off on its command line and reads the
    executable's imports against the app's manifest; the app's tests, the review screen's tests
    (SPEC-348 R20) and the archive sit, in order, between the harness's last step and the report,
    which carries their rows; no env of the job or of those steps names a signing setting; and no
    other job of the workflow names the app's spec or scheme."""
    jobs = workflow.get("jobs") or {}
    job = jobs.get("harness") or {}
    steps = job.get("steps") or []
    names = [step.get("name") for step in steps]
    problems = []
    if job.get("runs-on") != ADMITTED_RUNNERS["xcframework.yml"]:
        problems.append(f"harness: it runs on {job.get('runs-on')!r}, not the admitted runner")
    found = {}
    for name in (
        HARNESS_PROJECT,
        HARNESS_LAST,
        APP_TESTS,
        REVIEW_TESTS,
        REVIEW_ACTIONS_TESTS,
        APP_ARCHIVE,
        HARNESS_REPORT,
    ):
        if names.count(name) != 1:
            problems.append(f"harness: {names.count(name)} steps named {name!r}, not one")
        else:
            found[name] = steps[names.index(name)]
    project = found.get(HARNESS_PROJECT)
    if project is not None:
        generates = [
            line.strip()
            for line in str(project.get("run", "")).splitlines()
            if "xcodegen" in line and " generate " in line
        ]
        if generates != [HARNESS_GENERATE]:
            problems.append(
                f"harness: the step {HARNESS_PROJECT!r} does not hold the harness's one generate "
                "line alone"
            )
    tests = found.get(APP_TESTS)
    if tests is not None:
        lines = [line.strip() for line in str(tests.get("run", "")).splitlines() if line.strip()]
        if lines[:1] != [APP_GENERATE]:
            problems.append(
                "harness: the app's test step does not open with the app's generate line"
            )
        run = run_joined(tests)
        problems += [
            f"harness: the app's test step lacks {n}" for n in APP_TEST_NEEDS if n not in run
        ]
    archive = found.get(APP_ARCHIVE)
    if archive is not None:
        run = run_joined(archive)
        problems += [
            f"harness: the app's archive lacks {n}" for n in APP_ARCHIVE_NEEDS if n not in run
        ]
        if "CODE_SIGNING_ALLOWED=NO" not in run:
            problems.append(
                "harness: the app's archive does not turn code signing off on its command line"
            )
    report = found.get(HARNESS_REPORT)
    if report is not None:
        text = str(report.get("run", ""))
        problems += [f"harness: the report lacks {n}" for n in APP_REPORT_NEEDS if n not in text]
    if len(found) == 7:
        order = (HARNESS_LAST, APP_TESTS, REVIEW_TESTS, APP_ARCHIVE, HARNESS_REPORT)
        # The bury and flag step sits right after the review screen's step (SPEC-358 R8).
        order = order[:3] + (REVIEW_ACTIONS_TESTS,) + order[3:]
        at = [names.index(name) for name in order]
        if at != list(range(at[0], at[0] + len(order))):
            problems.append(
                "harness: the app's three steps do not sit, in order, between the harness's last "
                "step and the report"
            )
    envs = [("its env", job.get("env"))] + [
        (f"the env of {step.get('name')!r}", step.get("env"))
        for step in (tests, found.get(REVIEW_TESTS), archive)
        if step is not None
    ]
    for where, env in envs:
        problems += [
            f"harness: {where} names {name}, which only a command line may set"
            for name in SIGNING_NAMES
            if name in str(env or {})
        ]
    for other in sorted(set(jobs) - {"harness"}):
        text = str(jobs[other])
        problems += [
            f"{other}: it names {name}, which only the harness job may"
            for name in ("ios/app.yml", "-scheme DeckStreak")
            if name in text
        ]
    return problems


class TheAppIsGeneratedTestedAndArchived(unittest.TestCase):
    def test_the_app_is_generated_tested_and_archived_in_the_harness_job(self):
        """SPEC-347 A14: the `harness` job generates the app's project in its own step, runs the
        app's tests on both simulators and archives the app for a generic device with code signing
        off, after the harness's steps and before the report, and nothing else changes."""
        jobs = load("xcframework.yml")["jobs"]
        self.assertEqual(app_steps_problems({"jobs": jobs}), [])
        examined("harness steps", jobs["harness"].get("steps") or [])
        examined("jobs", list(jobs))

        # The controls: the good job is accepted, and each plant is refused by its rule's name.
        test_run = (
            APP_GENERATE
            + "\nstarted=$(date +%s)\n"
            + " \\\n  ".join(APP_TEST_NEEDS[:-1])
            + '\necho "$(( $(date +%s) - started ))" '
            + APP_TEST_NEEDS[-1]
            + "\n"
        )
        archive_run = (
            " ".join(APP_ARCHIVE_NEEDS[:4])
            + " CODE_SIGNING_ALLOWED=NO\n"
            + APP_ARCHIVE_NEEDS[4]
            + ' "$app" > "$REPORT/app-imports"\npython3 - '
            + APP_ARCHIVE_NEEDS[5]
            + " "
            + APP_ARCHIVE_NEEDS[6]
            + "\n"
        )
        good_steps = [
            {"uses": PLANTED_DOWNLOAD, "with": {"name": "xcframework"}},
            {"name": HARNESS_PROJECT, "run": HARNESS_GENERATE + "\n"},
            {
                "name": "the tests, Debug, on the iPhone and then the iPad",
                "run": "true\n",
            },
            {"name": HARNESS_LAST, "run": "true\n"},
            {"name": APP_TESTS, "run": test_run},
            {"name": REVIEW_TESTS, "run": "true\n"},
            {"name": REVIEW_ACTIONS_TESTS, "run": "true\n"},
            {"name": APP_ARCHIVE, "run": archive_run},
            {
                "name": HARNESS_REPORT,
                "if": "${{ always() }}",
                "run": "\n".join(APP_REPORT_NEEDS),
            },
        ]

        def harness(steps=None, **keys):
            return {
                "runs-on": ADMITTED_RUNNERS["xcframework.yml"],
                "env": {"REPORT": "harness-report"},
                "steps": good_steps if steps is None else steps,
                **keys,
            }

        def step_with(name, **keys):
            return harness([{**s, **keys} if s.get("name") == name else s for s in good_steps])

        def without(name, piece, instead=""):
            run = next(s["run"] for s in good_steps if s.get("name") == name)
            assert run.count(piece) == 1, f"{piece!r} is not in the planted {name!r} once"
            return step_with(name, run=run.replace(piece, instead))

        def ordered(*names):
            return harness([next(s for s in good_steps if s.get("name") == n) for n in names])

        plants = {
            "the good job": (harness(), []),
            "no app test step": (
                harness([s for s in good_steps if s.get("name") != APP_TESTS]),
                [f"harness: 0 steps named {APP_TESTS!r}, not one"],
            ),
            "a second archive step": (
                harness(good_steps + [next(s for s in good_steps if s.get("name") == APP_ARCHIVE)]),
                [f"harness: 2 steps named {APP_ARCHIVE!r}, not one"],
            ),
            "the app generated in the harness's generate step": (
                step_with(HARNESS_PROJECT, run=HARNESS_GENERATE + "\n" + APP_GENERATE + "\n"),
                [
                    "harness: the step 'the project, generated' does not hold the harness's one "
                    "generate line alone"
                ],
            ),
            "a test step the generate line does not open": (
                step_with(APP_TESTS, run="set -e\n" + test_run),
                ["harness: the app's test step does not open with the app's generate line"],
            ),
            "the iPhone alone": (
                without(APP_TESTS, APP_TEST_NEEDS[2]),
                [f"harness: the app's test step lacks {APP_TEST_NEEDS[2]}"],
            ),
            "both simulators at once": (
                without(APP_TESTS, APP_TEST_NEEDS[3]),
                [f"harness: the app's test step lacks {APP_TEST_NEEDS[3]}"],
            ),
            "code signing off in the test step": (
                without(APP_TESTS, "CODE_SIGN_IDENTITY=-", "CODE_SIGNING_ALLOWED=NO"),
                ["harness: the app's test step lacks CODE_SIGN_IDENTITY=-"],
            ),
            "the harness's result bundle": (
                without(APP_TESTS, "$RESULTS/app.xcresult", "$RESULTS/debug.xcresult"),
                [f"harness: the app's test step lacks {APP_TEST_NEEDS[5]}"],
            ),
            "a Debug archive": (
                without(APP_ARCHIVE, "-configuration Release", "-configuration Debug"),
                ["harness: the app's archive lacks -configuration Release"],
            ),
            "an archive for a simulator": (
                without(
                    APP_ARCHIVE,
                    "-destination generic/platform=iOS",
                    '-destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS"',
                ),
                ["harness: the app's archive lacks -destination generic/platform=iOS"],
            ),
            "signing on in the archive": (
                without(APP_ARCHIVE, " CODE_SIGNING_ALLOWED=NO"),
                ["harness: the app's archive does not turn code signing off on its command line"],
            ),
            "no imports read": (
                without(APP_ARCHIVE, "nm -u"),
                ["harness: the app's archive lacks nm -u"],
            ),
            "the imports against the harness's manifest": (
                without(
                    APP_ARCHIVE,
                    "ios/App/PrivacyInfo.xcprivacy",
                    "ios/Harness/PrivacyInfo.xcprivacy",
                ),
                ["harness: the app's archive lacks ios/App/PrivacyInfo.xcprivacy"],
            ),
            "a report with no app cases": (
                without(HARNESS_REPORT, 'cases_in("app")'),
                ['harness: the report lacks cases_in("app")'],
            ),
            "the app's steps after the report": (
                ordered(
                    HARNESS_PROJECT,
                    HARNESS_LAST,
                    HARNESS_REPORT,
                    APP_TESTS,
                    REVIEW_TESTS,
                    REVIEW_ACTIONS_TESTS,
                    APP_ARCHIVE,
                ),
                [
                    "harness: the app's three steps do not sit, in order, between the harness's "
                    "last step and the report"
                ],
            ),
            "the archive before the tests": (
                ordered(
                    HARNESS_PROJECT,
                    HARNESS_LAST,
                    APP_ARCHIVE,
                    APP_TESTS,
                    REVIEW_TESTS,
                    REVIEW_ACTIONS_TESTS,
                    HARNESS_REPORT,
                ),
                [
                    "harness: the app's three steps do not sit, in order, between the harness's "
                    "last step and the report"
                ],
            ),
            "no review step": (
                harness([s for s in good_steps if s.get("name") != REVIEW_TESTS]),
                [f"harness: 0 steps named {REVIEW_TESTS!r}, not one"],
            ),
            "the review step after the archive": (
                ordered(
                    HARNESS_PROJECT,
                    HARNESS_LAST,
                    APP_TESTS,
                    APP_ARCHIVE,
                    REVIEW_TESTS,
                    REVIEW_ACTIONS_TESTS,
                    HARNESS_REPORT,
                ),
                [
                    "harness: the app's three steps do not sit, in order, between the harness's "
                    "last step and the report"
                ],
            ),
            "a team in the review step's env": (
                step_with(REVIEW_TESTS, env={SIGNING_NAMES[2]: "planted"}),
                [
                    f"harness: the env of {REVIEW_TESTS!r} names {SIGNING_NAMES[2]}, which only a "
                    "command line may set"
                ],
            ),
            "signing in the job's env": (
                harness(env={"REPORT": "harness-report", "CODE_SIGNING_ALLOWED": "NO"}),
                ["harness: its env names CODE_SIGNING_ALLOWED, which only a command line may set"],
            ),
            "a team in the archive's env": (
                step_with(APP_ARCHIVE, env={SIGNING_NAMES[2]: "planted"}),
                [
                    f"harness: the env of {APP_ARCHIVE!r} names {SIGNING_NAMES[2]}, which only a "
                    "command line may set"
                ],
            ),
            "the harness on another runner": (
                harness(**{"runs-on": "macos-latest"}),
                ["harness: it runs on 'macos-latest', not the admitted runner"],
            ),
        }
        for name, (job, wanted) in examined("planted harness jobs", list(plants.items())):
            with self.subTest(plant=name):
                self.assertEqual(app_steps_problems({"jobs": {"harness": job}}), wanted, name)
        other = {
            "steps": [
                {"run": APP_GENERATE},
                {"run": "xcodebuild archive -scheme DeckStreak"},
            ]
        }
        with self.subTest(plant="the app built by another job"):
            self.assertEqual(
                app_steps_problems({"jobs": {"harness": harness(), "xcframework": other}}),
                [
                    "xcframework: it names ios/app.yml, which only the harness job may",
                    "xcframework: it names -scheme DeckStreak, which only the harness job may",
                ],
            )


# The review screen's step in the `harness` job (SPEC-348 R20, ADR-359 D8): the app's scheme on
# both simulators, one after the other, limited to the review screen's three classes, over the app
# step's derived data into its own result bundle, signed ad hoc on its command line as the app's
# step is, and its time written for the report. It runs after a red app step too, as the archive
# does.
REVIEW_TEST_NEEDS = (
    "xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug",
    '-destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS"',
    '-destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS"',
    "-disable-concurrent-destination-testing",
    "-only-testing:DeckStreakTests/ReviewModelTests",
    "-only-testing:DeckStreakTests/ReviewSessionTests",
    "-only-testing:DeckStreakUITests/ReviewFlowTests",
    '-derivedDataPath "$RUNNER_TEMP/app-debug"',
    '-resultBundlePath "$RESULTS/review.xcresult"',
    "CODE_SIGN_IDENTITY=-",
    '> "$REPORT/review-seconds"',
)
# The review fixture the engine's job uploads, copied into the runner's temporary directory and
# named to the test runner on the test command's own line, which xcodebuild hands to the test
# process as `DS_REVIEW_FIXTURE`; never through an env, so no other step's tests see it.
REVIEW_FIXTURE = "TEST_RUNNER_DS_REVIEW_FIXTURE"
REVIEW_FIXTURE_NEEDS = (
    'cp -R engine-artifact/review-fixture "$RUNNER_TEMP/review-fixture"',
    f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-fixture" xcodebuild test',
)
REVIEW_ID = "review-tests"
REVIEW_IF = "${{ !cancelled() && steps.app-tests.outcome != 'skipped' }}"
# The app's step skips the review screen's classes, so each review test runs once, with its fixture.
REVIEW_SKIPS = tuple(
    need.replace("-only-testing:", "-skip-testing:")
    for need in REVIEW_TEST_NEEDS
    if need.startswith("-only-testing:")
)
# The report's rows for the review screen: its test cases, read and listed, and its test time.
REVIEW_REPORT_NEEDS = (
    'cases_in("review")',
    '("Review", review_cases)',
    'minutes("review-seconds")',
)


def review_step_problems(workflow):
    """Each way the `harness` job could fail to run the review screen's tests, named (SPEC-348
    R20, ADR-359 D8): one step of its name, with its id and its condition, runs the app's scheme
    on both simulators limited to the three review classes, over the app's derived data into its
    own result bundle, signed ad hoc on its command line, with the review fixture copied into the
    runner's temporary directory and named on the test command's line, never in an env; the app's
    step skips those classes and names no fixture; and the report carries the step's cases and
    time."""
    job = (workflow.get("jobs") or {}).get("harness") or {}
    steps = job.get("steps") or []
    names = [step.get("name") for step in steps]
    problems = []
    if names.count(REVIEW_TESTS) != 1:
        problems.append(
            f"harness: {names.count(REVIEW_TESTS)} steps named {REVIEW_TESTS!r}, not one"
        )
    review = steps[names.index(REVIEW_TESTS)] if names.count(REVIEW_TESTS) == 1 else None
    if review is not None:
        for key, wanted in (("id", REVIEW_ID), ("if", REVIEW_IF)):
            if review.get(key) != wanted:
                problems.append(
                    f"harness: the review step's {key} is {review.get(key)!r}, not {wanted!r}"
                )
        run = run_joined(review)
        problems += [
            f"harness: the review step lacks {n}"
            for n in (*REVIEW_FIXTURE_NEEDS, *REVIEW_TEST_NEEDS)
            if n not in run
        ]
    envs = [("its env", job.get("env"))]
    if review is not None:
        envs.append((f"the env of {REVIEW_TESTS!r}", review.get("env")))
    problems += [
        f"harness: {where} names {REVIEW_FIXTURE}, which only the review step's command line may set"
        for where, env in envs
        if REVIEW_FIXTURE in str(env or {})
    ]
    if names.count(APP_TESTS) == 1:
        run = run_joined(steps[names.index(APP_TESTS)])
        problems += [
            f"harness: the app's test step lacks {n}" for n in REVIEW_SKIPS if n not in run
        ]
        if REVIEW_FIXTURE in run:
            problems.append(f"harness: the app's test step names {REVIEW_FIXTURE}")
    if names.count(HARNESS_REPORT) == 1:
        text = str(steps[names.index(HARNESS_REPORT)].get("run", ""))
        problems += [f"harness: the report lacks {n}" for n in REVIEW_REPORT_NEEDS if n not in text]
    return problems


class TheReviewScreenIsTestedOnBothSimulators(unittest.TestCase):
    def test_the_review_screen_is_tested_on_both_simulators_in_the_harness_job(self):
        """SPEC-348 R20: the `harness` job runs the review screen's tests on both simulators in a
        step of their own, over the review fixture named on its command line, after the app's
        tests, whose step skips them, and the report carries the step's cases and time."""
        jobs = load("xcframework.yml")["jobs"]
        self.assertEqual(review_step_problems({"jobs": jobs}), [])
        examined("harness steps", jobs["harness"].get("steps") or [])

        # The controls: the good job is accepted, and each plant is refused by its rule's name.
        review_run = (
            REVIEW_FIXTURE_NEEDS[0]
            + "\nstarted=$(date +%s)\n"
            + f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-fixture" '
            + " \\\n  ".join(REVIEW_TEST_NEEDS[:-1])
            + '\necho "$(( $(date +%s) - started ))" '
            + REVIEW_TEST_NEEDS[-1]
            + "\n"
        )
        app_run = (
            APP_GENERATE
            + "\nstarted=$(date +%s)\n"
            + " \\\n  ".join((*APP_TEST_NEEDS[:-1], *REVIEW_SKIPS))
            + '\necho "$(( $(date +%s) - started ))" '
            + APP_TEST_NEEDS[-1]
            + "\n"
        )
        good_steps = [
            {"name": APP_TESTS, "id": "app-tests", "run": app_run},
            {"name": REVIEW_TESTS, "id": REVIEW_ID, "if": REVIEW_IF, "run": review_run},
            {
                "name": HARNESS_REPORT,
                "if": "${{ always() }}",
                "run": "\n".join(REVIEW_REPORT_NEEDS),
            },
        ]

        def harness(steps=None, **keys):
            return {
                "env": {"REPORT": "harness-report"},
                "steps": good_steps if steps is None else steps,
                **keys,
            }

        def step_with(name, **keys):
            return harness([{**s, **keys} if s.get("name") == name else s for s in good_steps])

        def without(name, piece, instead=""):
            run = next(s["run"] for s in good_steps if s.get("name") == name)
            assert run.count(piece) == 1, f"{piece!r} is not in the planted {name!r} once"
            return step_with(name, run=run.replace(piece, instead))

        plants = {
            "the good job": (harness(), []),
            "no review step": (
                harness([s for s in good_steps if s.get("name") != REVIEW_TESTS]),
                [f"harness: 0 steps named {REVIEW_TESTS!r}, not one"],
            ),
            "a review step with no id": (
                step_with(REVIEW_TESTS, id=None),
                [f"harness: the review step's id is None, not {REVIEW_ID!r}"],
            ),
            "a review step that waits on a green app step": (
                step_with(REVIEW_TESTS, **{"if": None}),
                [f"harness: the review step's if is None, not {REVIEW_IF!r}"],
            ),
            "the iPhone alone": (
                without(REVIEW_TESTS, " \\\n  " + REVIEW_TEST_NEEDS[2]),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[2]}"],
            ),
            "both simulators at once": (
                without(REVIEW_TESTS, REVIEW_TEST_NEEDS[3]),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[3]}"],
            ),
            "the flow's class left out": (
                without(REVIEW_TESTS, REVIEW_TEST_NEEDS[6]),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[6]}"],
            ),
            "derived data of its own": (
                without(REVIEW_TESTS, "$RUNNER_TEMP/app-debug", "$RUNNER_TEMP/review-debug"),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[7]}"],
            ),
            "the app's result bundle": (
                without(REVIEW_TESTS, "$RESULTS/review.xcresult", "$RESULTS/app.xcresult"),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[8]}"],
            ),
            "code signing off in the review step": (
                without(REVIEW_TESTS, "CODE_SIGN_IDENTITY=-", "CODE_SIGNING_ALLOWED=NO"),
                ["harness: the review step lacks CODE_SIGN_IDENTITY=-"],
            ),
            "no time recorded": (
                without(REVIEW_TESTS, REVIEW_TEST_NEEDS[-1], '> "$REPORT/app-seconds"'),
                [f"harness: the review step lacks {REVIEW_TEST_NEEDS[-1]}"],
            ),
            "no fixture copied": (
                without(REVIEW_TESTS, REVIEW_FIXTURE_NEEDS[0] + "\n"),
                [f"harness: the review step lacks {REVIEW_FIXTURE_NEEDS[0]}"],
            ),
            "the fixture named in the step's env": (
                harness(
                    [
                        {
                            **s,
                            "run": s["run"].replace(
                                f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-fixture" ', ""
                            ),
                            "env": {REVIEW_FIXTURE: "${{ runner.temp }}/review-fixture"},
                        }
                        if s.get("name") == REVIEW_TESTS
                        else s
                        for s in good_steps
                    ]
                ),
                [
                    f"harness: the review step lacks {REVIEW_FIXTURE_NEEDS[1]}",
                    f"harness: the env of {REVIEW_TESTS!r} names {REVIEW_FIXTURE}, which only the "
                    "review step's command line may set",
                ],
            ),
            "the fixture named in the job's env": (
                harness(env={"REPORT": "harness-report", REVIEW_FIXTURE: "planted"}),
                [
                    f"harness: its env names {REVIEW_FIXTURE}, which only the review step's "
                    "command line may set"
                ],
            ),
            "the app's step runs the review tests": (
                without(APP_TESTS, " \\\n  " + REVIEW_SKIPS[1]),
                [f"harness: the app's test step lacks {REVIEW_SKIPS[1]}"],
            ),
            "the app's step names the fixture": (
                without(
                    APP_TESTS,
                    APP_TEST_NEEDS[0],
                    f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-fixture" ' + APP_TEST_NEEDS[0],
                ),
                [f"harness: the app's test step names {REVIEW_FIXTURE}"],
            ),
            "a report with no review cases": (
                without(HARNESS_REPORT, 'cases_in("review")'),
                ['harness: the report lacks cases_in("review")'],
            ),
            "a report that lists no review cases": (
                without(HARNESS_REPORT, '("Review", review_cases)'),
                ['harness: the report lacks ("Review", review_cases)'],
            ),
        }
        for name, (job, wanted) in examined("planted harness jobs", list(plants.items())):
            with self.subTest(plant=name):
                self.assertEqual(review_step_problems({"jobs": {"harness": job}}), wanted, name)


# The bury and flag step in the `harness` job (SPEC-358 R8 and A14): the app's scheme on the
# iPhone and then the iPad, one after the other, limited to the review's bury and flag class, over
# the app step's derived data into its own result bundle, signed ad hoc on its command line, with a
# copy of the review fixture of its own named on the test command's line, and its time written for
# the report. It runs after a red app step too, as the review screen's step does, and the app's
# step skips its class, so each of its tests runs once, with its fixture.
REVIEW_ACTIONS_TEST_NEEDS = (
    "xcodebuild test -project ios/DeckStreak.xcodeproj -scheme DeckStreak -configuration Debug",
    '-destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS"',
    '-destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS"',
    "-disable-concurrent-destination-testing",
    "-only-testing:DeckStreakUITests/ReviewActionsFlowTests",
    '-derivedDataPath "$RUNNER_TEMP/app-debug"',
    '-resultBundlePath "$RESULTS/review-actions.xcresult"',
    "CODE_SIGN_IDENTITY=-",
    '> "$REPORT/review-actions-seconds"',
)
# A copy of its own: the review screen's step copies the fixture to its own path first, and a copy
# onto an existing directory lands inside it.
REVIEW_ACTIONS_FIXTURE_NEEDS = (
    'cp -R engine-artifact/review-fixture "$RUNNER_TEMP/review-actions-fixture"',
    f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-actions-fixture" xcodebuild test',
)
REVIEW_ACTIONS_ID = "review-actions-tests"
REVIEW_ACTIONS_SKIP = "-skip-testing:DeckStreakUITests/ReviewActionsFlowTests"
# The report's rows for the step: its test cases, read and listed, and its test time.
REVIEW_ACTIONS_REPORT_NEEDS = (
    'cases_in("review-actions")',
    '("Review actions", review_actions_cases)',
    'minutes("review-actions-seconds")',
)


def review_actions_step_problems(workflow):
    """Each way the `harness` job could fail to run the review's bury and flag tests, named
    (SPEC-358 R8, A14): one step of its name, with its id and its condition, runs the app's scheme
    on the iPhone and then the iPad, one after the other, limited to the bury and flag class, over
    the app's derived data into its own result bundle, signed ad hoc on its command line, with a
    copy of the review fixture of its own named on the test command's line, never in its env; the
    app's step skips the class; and the report carries the step's cases and time."""
    job = (workflow.get("jobs") or {}).get("harness") or {}
    steps = job.get("steps") or []
    names = [step.get("name") for step in steps]
    problems = []
    count = names.count(REVIEW_ACTIONS_TESTS)
    if count != 1:
        problems.append(f"harness: {count} steps named {REVIEW_ACTIONS_TESTS!r}, not one")
    step = steps[names.index(REVIEW_ACTIONS_TESTS)] if count == 1 else None
    if step is not None:
        for key, wanted in (("id", REVIEW_ACTIONS_ID), ("if", REVIEW_IF)):
            if step.get(key) != wanted:
                problems.append(
                    f"harness: the bury and flag step's {key} is {step.get(key)!r}, not {wanted!r}"
                )
        run = run_joined(step)
        problems += [
            f"harness: the bury and flag step lacks {n}"
            for n in (*REVIEW_ACTIONS_FIXTURE_NEEDS, *REVIEW_ACTIONS_TEST_NEEDS)
            if n not in run
        ]
        iphone, ipad = (run.find(n) for n in REVIEW_ACTIONS_TEST_NEEDS[1:3])
        if -1 < ipad < iphone:
            problems.append("harness: the bury and flag step names the iPad before the iPhone")
        if REVIEW_FIXTURE in str(step.get("env") or {}):
            problems.append(
                f"harness: the env of {REVIEW_ACTIONS_TESTS!r} names {REVIEW_FIXTURE}, which only "
                "its command line may set"
            )
    if names.count(APP_TESTS) == 1:
        if REVIEW_ACTIONS_SKIP not in run_joined(steps[names.index(APP_TESTS)]):
            problems.append(f"harness: the app's test step lacks {REVIEW_ACTIONS_SKIP}")
    if names.count(HARNESS_REPORT) == 1:
        text = str(steps[names.index(HARNESS_REPORT)].get("run", ""))
        problems += [
            f"harness: the report lacks {n}" for n in REVIEW_ACTIONS_REPORT_NEEDS if n not in text
        ]
    return problems


class TheReviewActionsStepRunsOnTheIphoneAndThenTheIpad(unittest.TestCase):
    def test_the_review_actions_step_runs_on_the_iphone_and_then_the_ipad(self):
        """SPEC-358 A14 (R8): the `harness` job runs the review's bury and flag tests on the
        iPhone and then the iPad in a step of their own, over a copy of the review fixture named on
        its command line, after the app's tests, whose step skips them, and the report carries the
        step's cases and time."""
        jobs = load("xcframework.yml")["jobs"]
        self.assertEqual(review_actions_step_problems({"jobs": jobs}), [])
        examined("harness steps", jobs["harness"].get("steps") or [])

        # The controls: the good job is accepted, and each plant is refused by its rule's name.
        def actions_run(needs):
            return (
                REVIEW_ACTIONS_FIXTURE_NEEDS[0]
                + "\nstarted=$(date +%s)\n"
                + f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-actions-fixture" '
                + " \\\n  ".join(needs[:-1])
                + '\necho "$(( $(date +%s) - started ))" '
                + needs[-1]
                + "\n"
            )

        needs = REVIEW_ACTIONS_TEST_NEEDS
        app_run = (
            APP_GENERATE
            + "\nstarted=$(date +%s)\n"
            + " \\\n  ".join((*APP_TEST_NEEDS[:-1], REVIEW_ACTIONS_SKIP))
            + '\necho "$(( $(date +%s) - started ))" '
            + APP_TEST_NEEDS[-1]
            + "\n"
        )
        actions = {
            "name": REVIEW_ACTIONS_TESTS,
            "id": REVIEW_ACTIONS_ID,
            "if": REVIEW_IF,
            "run": actions_run(needs),
        }
        good_steps = [
            {"name": APP_TESTS, "id": "app-tests", "run": app_run},
            actions,
            {
                "name": HARNESS_REPORT,
                "if": "${{ always() }}",
                "run": "\n".join(REVIEW_ACTIONS_REPORT_NEEDS),
            },
        ]

        def harness(steps=None, **keys):
            return {
                "env": {"REPORT": "harness-report"},
                "steps": good_steps if steps is None else steps,
                **keys,
            }

        def step_with(name, **keys):
            return harness([{**s, **keys} if s.get("name") == name else s for s in good_steps])

        def without(name, piece, instead=""):
            run = next(s["run"] for s in good_steps if s.get("name") == name)
            assert run.count(piece) == 1, f"{piece!r} is not in the planted {name!r} once"
            return step_with(name, run=run.replace(piece, instead))

        plants = {
            "the good job": (harness(), []),
            "no bury and flag step": (
                harness([s for s in good_steps if s.get("name") != REVIEW_ACTIONS_TESTS]),
                [f"harness: 0 steps named {REVIEW_ACTIONS_TESTS!r}, not one"],
            ),
            "a second bury and flag step": (
                harness(good_steps + [actions]),
                [f"harness: 2 steps named {REVIEW_ACTIONS_TESTS!r}, not one"],
            ),
            "a step with no id": (
                step_with(REVIEW_ACTIONS_TESTS, id=None),
                [f"harness: the bury and flag step's id is None, not {REVIEW_ACTIONS_ID!r}"],
            ),
            "a step that waits on a green app step": (
                step_with(REVIEW_ACTIONS_TESTS, **{"if": None}),
                [f"harness: the bury and flag step's if is None, not {REVIEW_IF!r}"],
            ),
            "the iPhone alone": (
                without(REVIEW_ACTIONS_TESTS, " \\\n  " + needs[2]),
                [f"harness: the bury and flag step lacks {needs[2]}"],
            ),
            "the iPad first": (
                step_with(
                    REVIEW_ACTIONS_TESTS,
                    run=actions_run((needs[0], needs[2], needs[1], *needs[3:])),
                ),
                ["harness: the bury and flag step names the iPad before the iPhone"],
            ),
            "both simulators at once": (
                without(REVIEW_ACTIONS_TESTS, needs[3]),
                [f"harness: the bury and flag step lacks {needs[3]}"],
            ),
            "the class left out": (
                without(REVIEW_ACTIONS_TESTS, needs[4]),
                [f"harness: the bury and flag step lacks {needs[4]}"],
            ),
            "derived data of its own": (
                without(
                    REVIEW_ACTIONS_TESTS, "$RUNNER_TEMP/app-debug", "$RUNNER_TEMP/actions-debug"
                ),
                [f"harness: the bury and flag step lacks {needs[5]}"],
            ),
            "the review screen's result bundle": (
                without(
                    REVIEW_ACTIONS_TESTS,
                    "$RESULTS/review-actions.xcresult",
                    "$RESULTS/review.xcresult",
                ),
                [f"harness: the bury and flag step lacks {needs[6]}"],
            ),
            "code signing off in the step": (
                without(REVIEW_ACTIONS_TESTS, "CODE_SIGN_IDENTITY=-", "CODE_SIGNING_ALLOWED=NO"),
                ["harness: the bury and flag step lacks CODE_SIGN_IDENTITY=-"],
            ),
            "no time recorded": (
                without(REVIEW_ACTIONS_TESTS, needs[-1], '> "$REPORT/review-seconds"'),
                [f"harness: the bury and flag step lacks {needs[-1]}"],
            ),
            "no fixture copied": (
                without(REVIEW_ACTIONS_TESTS, REVIEW_ACTIONS_FIXTURE_NEEDS[0] + "\n"),
                [f"harness: the bury and flag step lacks {REVIEW_ACTIONS_FIXTURE_NEEDS[0]}"],
            ),
            "the review screen's fixture copy": (
                step_with(
                    REVIEW_ACTIONS_TESTS,
                    run=actions["run"].replace("review-actions-fixture", "review-fixture"),
                ),
                [
                    f"harness: the bury and flag step lacks {REVIEW_ACTIONS_FIXTURE_NEEDS[0]}",
                    f"harness: the bury and flag step lacks {REVIEW_ACTIONS_FIXTURE_NEEDS[1]}",
                ],
            ),
            "the fixture named in the step's env": (
                step_with(
                    REVIEW_ACTIONS_TESTS,
                    run=actions["run"].replace(
                        f'{REVIEW_FIXTURE}="$RUNNER_TEMP/review-actions-fixture" ', ""
                    ),
                    env={REVIEW_FIXTURE: "${{ runner.temp }}/review-actions-fixture"},
                ),
                [
                    f"harness: the bury and flag step lacks {REVIEW_ACTIONS_FIXTURE_NEEDS[1]}",
                    f"harness: the env of {REVIEW_ACTIONS_TESTS!r} names {REVIEW_FIXTURE}, which "
                    "only its command line may set",
                ],
            ),
            "the app's step runs the class": (
                without(APP_TESTS, " \\\n  " + REVIEW_ACTIONS_SKIP),
                [f"harness: the app's test step lacks {REVIEW_ACTIONS_SKIP}"],
            ),
            "a report with no bury and flag cases": (
                without(HARNESS_REPORT, 'cases_in("review-actions")'),
                ['harness: the report lacks cases_in("review-actions")'],
            ),
            "a report that lists no bury and flag cases": (
                without(HARNESS_REPORT, '("Review actions", review_actions_cases)'),
                ['harness: the report lacks ("Review actions", review_actions_cases)'],
            ),
            "a report with no bury and flag time": (
                without(HARNESS_REPORT, 'minutes("review-actions-seconds")'),
                ['harness: the report lacks minutes("review-actions-seconds")'],
            ),
        }
        for name, (job, wanted) in examined("planted harness jobs", list(plants.items())):
            with self.subTest(plant=name):
                self.assertEqual(
                    review_actions_step_problems({"jobs": {"harness": job}}), wanted, name
                )


# The two simulators the card probe runs on, one after the other, as the harness job names them.
CARD_PROBE_NEEDS = (
    "xcodebuild test",
    '-destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS"',
    '-destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS"',
    "-disable-concurrent-destination-testing",
    '-resultBundlePath "$RESULTS/card-probe.xcresult"',
)
# The actions a planted card-view job uses; their refs are placeholders the checker never reads.
PLANTED_CHECKOUT = "actions/checkout@" + "0" * 40
PLANTED_UPLOAD = "actions/upload-artifact@" + "0" * 40


def card_probe_problems(jobs):
    """Each way the Apple job body could fail to run the card view's proof, named (SPEC-349 R10,
    A9): the `harness` job runs the `CardProbe` scheme's tests on the iPhone and then the iPad and
    uploads its result bundles whatever the outcome; a `card-isolation` job on the admitted runner,
    waiting on nothing, checks out without persisted credentials, runs the package's tests and its
    Swift mutant sweep on the host, and uploads its report whatever the outcome."""
    problems = []
    steps = (jobs.get("harness") or {}).get("steps") or []
    probes = [s for s in steps if "-scheme CardProbe" in str(s.get("run", ""))]
    if len(probes) != 1:
        problems.append(f"harness: {len(probes)} CardProbe test steps, not one")
    for step in probes:
        run = re.sub(r"\\\n\s*", " ", str(step.get("run", "")))
        problems += [
            f"harness: the CardProbe step lacks {n}" for n in CARD_PROBE_NEEDS if n not in run
        ]
    if not any(
        action(s) == "actions/upload-artifact"
        and s.get("if") == "${{ always() }}"
        and "harness-results/" in str((s.get("with") or {}).get("path", ""))
        for s in steps
    ):
        problems.append("harness: no upload of its result bundles runs whatever the outcome")
    job = jobs.get("card-isolation")
    if job is None:
        return problems + ["card-isolation: no such job"]
    if job.get("runs-on") != ADMITTED_RUNNERS["xcframework.yml"]:
        problems.append(
            f"card-isolation: it runs on {job.get('runs-on')!r}, not the admitted runner"
        )
    if job.get("needs"):
        problems.append("card-isolation: it waits on another job")
    steps = job.get("steps") or []
    runs = [str(s.get("run", "")) for s in steps]
    checkouts = [s for s in steps if action(s) == "actions/checkout"]
    if not checkouts or any(
        str((s.get("with") or {}).get("persist-credentials")).lower() != "false" for s in checkouts
    ):
        problems.append("card-isolation: its checkout persists credentials")
    if not any("swift test --package-path ios/CardIsolation" in r for r in runs):
        problems.append("card-isolation: it runs no swift test of ios/CardIsolation")
    if not any(
        'pathlib.Path("ios/CardIsolation")' in r and "swift-mutants.json" in r for r in runs
    ):
        problems.append("card-isolation: it sweeps no mutant of ios/CardIsolation")
    if not any(
        action(s) == "actions/upload-artifact" and s.get("if") == "${{ always() }}" for s in steps
    ):
        problems.append("card-isolation: its report is not uploaded whatever the outcome")
    return problems


class TheCardViewIsProvedOnBothSimulators(unittest.TestCase):
    def test_the_card_probe_suite_runs_on_both_simulators(self):
        """SPEC-349 A9: the card view's planted suite runs on both simulators, and its package's
        tests and Swift mutant sweep run on the host, in the one Apple job body."""
        jobs = load("xcframework.yml")["jobs"]
        self.assertEqual(card_probe_problems(jobs), [])
        examined("harness steps", jobs["harness"].get("steps") or [])
        examined("card-isolation steps", jobs["card-isolation"].get("steps") or [])

        # The controls: the good jobs are accepted, and each plant is refused by its rule's name.
        probe = {
            "run": "xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe \\\n  "
            + " \\\n  ".join(CARD_PROBE_NEEDS[1:])
        }
        upload = {
            "uses": PLANTED_UPLOAD,
            "if": "${{ always() }}",
            "with": {"path": "harness-report/\nharness-results/"},
        }
        sweep = {
            "run": 'package = pathlib.Path("ios/CardIsolation")\n(package / "swift-mutants.json")'
        }
        host = {
            "runs-on": ADMITTED_RUNNERS["xcframework.yml"],
            "steps": [
                {"uses": PLANTED_CHECKOUT, "with": {"persist-credentials": "false"}},
                {"run": "swift test --package-path ios/CardIsolation"},
                sweep,
                {"uses": PLANTED_UPLOAD, "if": "${{ always() }}", "with": {"path": "r/"}},
            ],
        }
        good = {"harness": {"steps": [probe, upload]}, "card-isolation": host}

        def harness(*steps):
            return {**good, "harness": {"steps": list(steps)}}

        def isolation(**changed):
            return {**good, "card-isolation": {**host, **changed}}

        plants = {
            "the good jobs": (good, []),
            "no probe step": (harness(upload), ["harness: 0 CardProbe test steps, not one"]),
            "two probe steps": (
                harness(probe, probe, upload),
                ["harness: 2 CardProbe test steps, not one"],
            ),
            "the iPad left out": (
                harness({"run": probe["run"].replace(CARD_PROBE_NEEDS[2], "")}, upload),
                [f"harness: the CardProbe step lacks {CARD_PROBE_NEEDS[2]}"],
            ),
            "the simulators at once": (
                harness({"run": probe["run"].replace(CARD_PROBE_NEEDS[3], "")}, upload),
                [f"harness: the CardProbe step lacks {CARD_PROBE_NEEDS[3]}"],
            ),
            "no result bundle": (
                harness({"run": probe["run"].replace(CARD_PROBE_NEEDS[4], "")}, upload),
                [f"harness: the CardProbe step lacks {CARD_PROBE_NEEDS[4]}"],
            ),
            "a build in place of the tests": (
                harness(
                    {"run": probe["run"].replace("xcodebuild test", "xcodebuild build")}, upload
                ),
                ["harness: the CardProbe step lacks xcodebuild test"],
            ),
            "the bundles uploaded on success only": (
                harness(probe, {k: v for k, v in upload.items() if k != "if"}),
                ["harness: no upload of its result bundles runs whatever the outcome"],
            ),
            "no host job": (
                {"harness": good["harness"]},
                ["card-isolation: no such job"],
            ),
            "another runner": (
                isolation(**{"runs-on": "macos-latest"}),
                ["card-isolation: it runs on 'macos-latest', not the admitted runner"],
            ),
            "a host job that waits on the build": (
                isolation(needs="xcframework"),
                ["card-isolation: it waits on another job"],
            ),
            "a persisted checkout": (
                isolation(steps=[{"uses": PLANTED_CHECKOUT}, *host["steps"][1:]]),
                ["card-isolation: its checkout persists credentials"],
            ),
            "no package tests": (
                isolation(steps=[host["steps"][0], sweep, host["steps"][3]]),
                ["card-isolation: it runs no swift test of ios/CardIsolation"],
            ),
            "no sweep": (
                isolation(steps=[*host["steps"][:2], host["steps"][3]]),
                ["card-isolation: it sweeps no mutant of ios/CardIsolation"],
            ),
            "a report uploaded on success only": (
                isolation(
                    steps=[*host["steps"][:3], {"uses": PLANTED_UPLOAD, "with": {"path": "r/"}}]
                ),
                ["card-isolation: its report is not uploaded whatever the outcome"],
            ),
        }
        for name, (jobs, wanted) in examined("planted card-view jobs", list(plants.items())):
            with self.subTest(plant=name):
                self.assertEqual(card_probe_problems(jobs), wanted, name)


# SPEC-382: every Apple CI run records its own timing, and uploads it whatever the outcome
# (ADR-393 to ADR-396). The planted suite's step, timed as the other test steps are, and its
# command as the base held it: every line of the step but its comments and its timer (R1).
PLANTED_SUITE = "the card view's planted suite, Debug, on the iPhone and then the iPad"
PLANTED_SUITE_COMMAND = "\n".join(
    (
        "xcodebuild test -project ios/Harness.xcodeproj -scheme CardProbe -configuration Debug \\",
        '  -destination "platform=iOS Simulator,name=$IPHONE_SIM,OS=$SIM_OS" \\',
        '  -destination "platform=iOS Simulator,name=$IPAD_SIM,OS=$SIM_OS" \\',
        "  -disable-concurrent-destination-testing \\",
        '  -derivedDataPath "$RUNNER_TEMP/debug" \\',
        '  -resultBundlePath "$RESULTS/card-probe.xcresult" \\',
        "  CODE_SIGNING_ALLOWED=NO",
    )
)
# How a timed step starts its clock, and the flag that prints a build's timing summary (R2).
TIMER_START = "started=$(date +%s)"
TIMING_SUMMARY = "-showBuildTimingSummary"
# An `xcodebuild` that builds nothing takes no flag: the framework assembly and a version read.
NOT_A_BUILD = ("-create-xcframework", "-version")
BUILD_ACTIONS = ("build", "test", "archive")
# The word `xcodebuild` as a command, never inside a path or another name.
XCODEBUILD = re.compile(r"(?<![\w/.-])xcodebuild(?![\w.-])")
# The static-library step, and the per-target clock inside its loop over the targets (R3).
STATIC_LIBRARIES = "the two static libraries, clocked from the first compile to the second archive"
TARGET_START = "target_started=$(date +%s)"
TARGET_WRITE = 'echo "$(( $(date +%s) - target_started ))" > "$REPORT/build-seconds-$target"'
CARGO_TIMINGS = "target/cargo-timings/"
# The step that boots the first simulator of the first test step, and what it runs (R4): the one
# available device of the pinned iPhone name and OS, booted and waited for.
BOOT = "the iPhone simulator, booted"
BOOT_NEEDS = (
    "xcrun simctl list devices available -j",
    'os.environ["IPHONE_SIM"]',
    'os.environ["SIM_OS"]',
    'xcrun simctl bootstatus "$udid" -b',
)
# Each job's timing record, its upload, and the artifact each job uploads it under (R6).
TIMING_RECORD = "the timing record"
TIMING_UPLOAD = "upload the timing record"
TIMING_UPLOADS = (("harness", "timing-harness"), ("xcframework", "timing-xcframework"))
# A result bundle a step writes into its job's results folder (R1, A7).
BUNDLE = re.compile(r'-resultBundlePath\s+"\$RESULTS/([\w.-]+)\.xcresult"')


def timer_write(name):
    """The line that writes a timed step's elapsed seconds for the report, as each step spells it."""
    return f'echo "$(( $(date +%s) - started ))" > "$REPORT/{name}"'


def job_steps(workflow, job):
    return ((workflow.get("jobs") or {}).get(job) or {}).get("steps") or []


def step_of(workflow, job, name):
    """The one step of a name in a job, or None when the job holds none or several."""
    found = [step for step in job_steps(workflow, job) if step.get("name") == name]
    return found[0] if len(found) == 1 else None


def with_step(workflow, job, name, keys):
    """A copy of a workflow whose job's step of `name` has `keys` set, a None value removing its
    key; a name the job does not hold is appended as a new step. The plants are built on it."""
    copy = json.loads(json.dumps(workflow))
    steps = copy["jobs"][job]["steps"]
    step = next((s for s in steps if s.get("name") == name), None)
    if step is None:
        step = {"name": name}
        steps.append(step)
    for key, value in keys.items():
        if value is None:
            step.pop(key, None)
        else:
            step[key] = value
    return copy


def planted_suite_problems(workflow):
    """Each way the planted suite's step could fail to be timed and read, named (SPEC-382 R1, A1):
    one step of its name in `harness`, its clock started before its command and its seconds written
    after it; the report reading its bundle's tests and its seconds; and its command, every line
    but its comments and its timer, equal to the base's text. The timer is judged first."""
    step = step_of(workflow, "harness", PLANTED_SUITE)
    if step is None:
        return [f"harness: no one step named {PLANTED_SUITE!r}"]
    raw = str(step.get("run", "")).splitlines()
    lines = [line.strip() for line in raw]
    write = timer_write("card-probe-seconds")
    calls = [at for at, line in enumerate(lines) if XCODEBUILD.search(line)]
    problems = []
    if TIMER_START not in lines or write not in lines:
        problems.append("harness: the planted suite's step writes no `card-probe-seconds` timer")
    elif not calls or not lines.index(TIMER_START) < calls[0] < lines.index(write):
        problems.append("harness: the planted suite's timer does not wrap its command")
    report = step_of(workflow, "harness", HARNESS_REPORT)
    text = str((report or {}).get("run", ""))
    problems += [
        f"harness: the report lacks {need}"
        for need in ('cases_in("card-probe")', 'minutes("card-probe-seconds")')
        if need not in text
    ]
    kept = [
        line
        for line in raw
        if line.strip()
        and not line.lstrip().startswith("#")
        and line.strip() not in (TIMER_START, write)
    ]
    if textwrap.dedent("\n".join(kept)) != PLANTED_SUITE_COMMAND:
        problems.append("harness: the planted suite's command is not its base text")
    return problems


def xcodebuild_calls(text):
    """Each `xcodebuild` a run script calls, continuations joined: the words after the name, for
    every line not a comment that names it."""
    joined = re.sub(r"\\\n\s*", " ", text)
    return [
        line[match.end() :].split()
        for line in joined.splitlines()
        if not line.lstrip().startswith("#")
        for match in XCODEBUILD.finditer(line)
    ]


def timing_summary_problems(workflow):
    """Each `xcodebuild` build, test or archive of the Apple job body that does not print its build
    timing summary, named by its job and step (SPEC-382 R2, A2). The planted suite's step is exempt
    by its name, because R1 holds its command; a call naming `-create-xcframework` or `-version`
    builds nothing; and a call whose words name no build, test or archive is refused as unread,
    never passed. Returns the problems and the calls judged, the exempt ones included."""
    problems, judged = [], []
    for job, body in (workflow.get("jobs") or {}).items():
        for step in (body or {}).get("steps") or []:
            name = step.get("name")
            for words in xcodebuild_calls(str(step.get("run", ""))):
                judged.append((job, name))
                if (job, name) == ("harness", PLANTED_SUITE) or any(
                    word in NOT_A_BUILD for word in words
                ):
                    continue
                verbs = [word for word in words if word in BUILD_ACTIONS]
                if not verbs:
                    problems.append(
                        f"{job}: the step {name!r} runs an `xcodebuild` this census cannot read "
                        "as a build, test or archive"
                    )
                elif TIMING_SUMMARY not in words:
                    problems.append(
                        f"{job}: the step {name!r} runs `xcodebuild {verbs[0]}` without "
                        f"{TIMING_SUMMARY}"
                    )
    return problems, judged


def static_library_timing_problems(workflow):
    """Each way the static-library step could fail to time each target, named (SPEC-382 R3, A3):
    one loop over the targets holding one `cargo rustc` that keeps `--locked`, a per-target clock
    started before it and written to `build-seconds-$target` after it, and the total clock around
    the loop kept."""
    step = step_of(workflow, "xcframework", STATIC_LIBRARIES)
    if step is None:
        return [f"xcframework: no one step named {STATIC_LIBRARIES!r}"]
    lines = [line.strip() for line in str(step.get("run", "")).splitlines()]
    loops = [at for at, line in enumerate(lines) if line.startswith("for target in ")]
    if len(loops) != 1 or "done" not in lines[loops[0] :]:
        return ["xcframework: the static-library step compiles its targets in no one loop"]
    start = loops[0]
    end = lines.index("done", start)
    body = lines[start + 1 : end]
    compiles = [at for at, line in enumerate(body) if line.startswith("cargo rustc ")]
    problems = []
    if len(compiles) != 1:
        problems.append(f"xcframework: the targets' loop runs {len(compiles)} compiles, not one")
    elif "--locked" not in body[compiles[0]].split():
        problems.append("xcframework: the targets' compile drops --locked")
    if TARGET_WRITE not in body:
        problems.append(
            "xcframework: the static-library step writes no per-target seconds file, "
            "`build-seconds-$target`"
        )
    elif (
        TARGET_START not in body
        or len(compiles) != 1
        or not body.index(TARGET_START) < compiles[0] < body.index(TARGET_WRITE)
    ):
        problems.append("xcframework: the per-target clock does not wrap the target's compile")
    if TIMER_START not in lines[:start] or timer_write("build-seconds") not in lines[end + 1 :]:
        problems.append(
            "xcframework: the static-library step does not keep its total, `build-seconds`"
        )
    return problems


def boot_problems(workflow):
    """Each way the first simulator could fail to boot in a timed step of its own, named (SPEC-382
    R4, A4): exactly one `harness` step boots a simulator, named for it and placed right after
    "the project, generated"; it picks the one available device of the pinned iPhone name and OS,
    boots it and waits for it, names no iPad, and writes `boot-seconds`, which the report reads."""
    steps = job_steps(workflow, "harness")
    names = [step.get("name") for step in steps]
    booting = [step.get("name") for step in steps if "simctl boot" in str(step.get("run", ""))]
    if not booting:
        return [f"harness: no step boots a simulator after {HARNESS_PROJECT!r}"]
    if booting != [BOOT]:
        return [f"harness: the steps {booting} boot a simulator, not {BOOT!r} alone"]
    problems = []
    if HARNESS_PROJECT not in names or names.index(BOOT) != names.index(HARNESS_PROJECT) + 1:
        problems.append(f"harness: {BOOT!r} does not follow {HARNESS_PROJECT!r}")
    run = str(steps[names.index(BOOT)].get("run", ""))
    problems += [f"harness: the boot step lacks {need}" for need in BOOT_NEEDS if need not in run]
    if "IPAD_SIM" in run:
        problems.append("harness: the boot step boots the iPad, which `xcodebuild` boots")
    lines = [line.strip() for line in run.splitlines()]
    write = timer_write("boot-seconds")
    if write not in lines:
        problems.append("harness: the boot step writes no `boot-seconds`")
    elif TIMER_START not in lines:
        problems.append("harness: the boot step starts no clock")
    elif BOOT_NEEDS[-1] in lines and not (
        lines.index(TIMER_START) < lines.index(BOOT_NEEDS[-1]) < lines.index(write)
    ):
        problems.append("harness: the boot step's clock does not wrap its wait")
    report = step_of(workflow, "harness", HARNESS_REPORT)
    if 'minutes("boot-seconds")' not in str((report or {}).get("run", "")):
        problems.append('harness: the report lacks minutes("boot-seconds")')
    return problems


def developer_directory_problems(workflow):
    """Each job of the Apple job body that does not build with the one pinned Xcode, named
    (SPEC-382 R5, A5): every job's env sets `DEVELOPER_DIR`, the framework job's first, and all of
    them to one path."""
    jobs = workflow.get("jobs") or {}
    if "xcframework" not in jobs:
        return ["xcframework: no such job"]
    pinned = {
        job: str(((body or {}).get("env") or {}).get("DEVELOPER_DIR", ""))
        for job, body in jobs.items()
    }
    order = ["xcframework"] + sorted(set(pinned) - {"xcframework"})
    problems = [
        f"{job}: the job's environment does not set the developer directory"
        for job in order
        if not pinned[job]
    ]
    paths = sorted({path for path in pinned.values() if path})
    if len(paths) > 1:
        problems.append(f"the jobs set {len(paths)} developer directories, not one: {paths}")
    return problems


def timing_upload_problems(workflow):
    """Each way a job could fail to upload its own timing record whatever the outcome, named
    (SPEC-382 R6, A6): `harness`, then `xcframework`, holds one upload of its timing artifact,
    under `always()`, with the action its existing upload uses, after that upload and after one
    step "the timing record", itself under `always()`, that writes `timing.json` in the schema
    `ci-timing/1`; the upload carries the record, and the framework job's carries cargo's timings
    report too."""
    problems = []
    for job, artifact in TIMING_UPLOADS:
        steps = job_steps(workflow, job)
        uploads = [at for at, step in enumerate(steps) if action(step) == "actions/upload-artifact"]
        timing = [
            at for at in uploads if str((steps[at].get("with") or {}).get("name")) == artifact
        ]
        if len(timing) != 1:
            problems.append(f"{job}: {len(timing)} uploads named {artifact!r}, not one")
            continue
        at = timing[0]
        upload = steps[at]
        existing = [n for n in uploads if n != at]
        if upload.get("if") != "${{ always() }}":
            problems.append(f"{job}: the {artifact} upload does not run whatever the outcome")
        if not existing or any(
            str(steps[n].get("uses")) != str(upload.get("uses")) for n in existing
        ):
            problems.append(
                f"{job}: the {artifact} upload does not use the pinned action of the job's "
                "existing upload"
            )
        elif at < max(existing):
            problems.append(f"{job}: the {artifact} upload comes before the job's existing upload")
        carried = lines_of((upload.get("with") or {}).get("path"))
        if not any(line.endswith("timing.json") for line in carried):
            problems.append(f"{job}: the {artifact} upload does not carry timing.json")
        if job == "xcframework" and CARGO_TIMINGS not in carried:
            problems.append(f"{job}: the {artifact} upload does not carry {CARGO_TIMINGS}")
        records = [n for n, step in enumerate(steps) if step.get("name") == TIMING_RECORD]
        if len(records) != 1:
            problems.append(f"{job}: {len(records)} steps named {TIMING_RECORD!r}, not one")
            continue
        record = steps[records[0]]
        run = str(record.get("run", ""))
        if record.get("if") != "${{ always() }}":
            problems.append(f"{job}: the step {TIMING_RECORD!r} does not run whatever the outcome")
        if "timing.json" not in run or "ci-timing/1" not in run:
            problems.append(
                f"{job}: the step {TIMING_RECORD!r} writes no timing.json in ci-timing/1"
            )
        if not existing or not max(existing) < records[0] < at:
            problems.append(
                f"{job}: the step {TIMING_RECORD!r} does not sit between the job's existing upload "
                "and its timing upload"
            )
    return problems


def unread_bundle_problems(workflow):
    """Each result bundle a job uploads and its report does not read, named (SPEC-382 R1, A7): in
    every job that uploads its results folder, each bundle a step writes there is read by the job's
    report with `cases_in`. Returns the problems and the bundles judged."""
    problems, judged = [], []
    for job, body in (workflow.get("jobs") or {}).items():
        results = str(((body or {}).get("env") or {}).get("RESULTS", "")).rstrip("/")
        steps = (body or {}).get("steps") or []
        folder = results.rsplit("/", 1)[-1] + "/"
        if not results or not any(
            action(step) == "actions/upload-artifact"
            and folder in lines_of((step.get("with") or {}).get("path"))
            for step in steps
        ):
            continue
        reports = [step for step in steps if step.get("name") == HARNESS_REPORT]
        text = str(reports[0].get("run", "")) if len(reports) == 1 else ""
        if len(reports) != 1:
            problems.append(f"{job}: {len(reports)} steps named {HARNESS_REPORT!r}, not one")
        for step in steps:
            for label in BUNDLE.findall(run_joined(step)):
                judged.append((job, label))
                if f'cases_in("{label}")' not in text:
                    problems.append(
                        f"{job}: the result bundle {label}.xcresult is uploaded and not read by "
                        "the report"
                    )
    return problems, judged


class TheRunRecordsItsOwnTiming(unittest.TestCase):
    def test_the_planted_suite_is_timed_and_its_command_is_unchanged(self):
        """SPEC-382 A1: the planted suite's step writes its seconds and the report reads its
        bundle's tests and its seconds, while its command stays the base's text."""
        workflow = load("xcframework.yml")
        self.assertIsNotNone(step_of(workflow, "harness", PLANTED_SUITE))
        self.assertIsNotNone(step_of(workflow, "harness", HARNESS_REPORT))
        self.assertEqual(planted_suite_problems(workflow), [])
        examined("harness steps", job_steps(workflow, "harness"))

        # The controls: each plant is refused by its rule's name, and by no other.
        run = step_of(workflow, "harness", PLANTED_SUITE)["run"]
        report = step_of(workflow, "harness", HARNESS_REPORT)["run"]
        changed = "harness: the planted suite's command is not its base text"
        plants = [
            (
                "the timer's write removed",
                PLANTED_SUITE,
                run.replace(timer_write("card-probe-seconds"), ""),
                ["harness: the planted suite's step writes no `card-probe-seconds` timer"],
            ),
            (
                "one token of the command changed",
                PLANTED_SUITE,
                run.replace("-configuration Debug", "-configuration Release"),
                [changed],
            ),
            (
                "the command given the timing summary",
                PLANTED_SUITE,
                run.replace("-configuration Debug", f"-configuration Debug {TIMING_SUMMARY}"),
                [changed],
            ),
            (
                "the report's read of its seconds removed",
                HARNESS_REPORT,
                report.replace('minutes("card-probe-seconds")', '"not measured"'),
                ['harness: the report lacks minutes("card-probe-seconds")'],
            ),
        ]
        for name, step, text, wanted in examined("planted suite plants", plants):
            with self.subTest(plant=name):
                planted = with_step(workflow, "harness", step, {"run": text})
                self.assertEqual(planted_suite_problems(planted), wanted, name)

    def test_every_other_xcodebuild_prints_its_build_timing_summary(self):
        """SPEC-382 A2: every `xcodebuild` build, test or archive of the body other than the
        planted suite's passes the timing summary; the framework assembly and a version read
        build nothing, and a call this census cannot read is refused."""
        workflow = load("xcframework.yml")
        self.assertIsNotNone(step_of(workflow, "harness", PLANTED_SUITE))
        problems, judged = timing_summary_problems(workflow)
        self.assertEqual(problems, [])
        examined("xcodebuild calls", judged)

        # The controls: each plant is refused by its step's name, and an exempt call is not.
        plants = [
            (
                "a test step without the summary",
                "xcodebuild test -project ios/Harness.xcodeproj -scheme Harness \\\n"
                '  -derivedDataPath "$RUNNER_TEMP/planted"\n',
                [
                    "harness: the step 'a planted build' runs `xcodebuild test` without "
                    f"{TIMING_SUMMARY}"
                ],
            ),
            (
                "a call this census cannot read",
                'python3 -c \'import subprocess; subprocess.run(["xcodebuild", "test"])\'\n',
                [
                    "harness: the step 'a planted build' runs an `xcodebuild` this census cannot "
                    "read as a build, test or archive"
                ],
            ),
            ("a version read", 'xcodebuild -version > "$REPORT/planted"\n', []),
        ]
        for name, text, wanted in examined("planted xcodebuild steps", plants):
            with self.subTest(plant=name):
                planted = with_step(workflow, "harness", "a planted build", {"run": text})
                self.assertEqual(timing_summary_problems(planted)[0], wanted, name)

    def test_each_static_library_target_is_timed(self):
        """SPEC-382 A3: each static-library target writes its own seconds inside the loop, the
        total is kept around it, and the compile keeps `--locked`."""
        workflow = load("xcframework.yml")
        self.assertIsNotNone(step_of(workflow, "xcframework", STATIC_LIBRARIES))
        self.assertEqual(static_library_timing_problems(workflow), [])
        run = step_of(workflow, "xcframework", STATIC_LIBRARIES)["run"]
        examined("lines of the static-library step", run.splitlines())

        # The controls: each plant is refused by its rule's name, and by no other.
        plants = [
            (
                "the per-target write removed",
                run.replace(TARGET_WRITE, ""),
                [
                    "xcframework: the static-library step writes no per-target seconds file, "
                    "`build-seconds-$target`"
                ],
            ),
            (
                "the per-target clock started after the compile",
                run.replace(TARGET_START + "\n", "").replace(
                    TARGET_WRITE, TARGET_START + "\n" + TARGET_WRITE
                ),
                ["xcframework: the per-target clock does not wrap the target's compile"],
            ),
            (
                "the total's write removed",
                run.replace(timer_write("build-seconds"), ""),
                ["xcframework: the static-library step does not keep its total, `build-seconds`"],
            ),
            (
                "--locked dropped",
                run.replace("--locked ", ""),
                ["xcframework: the targets' compile drops --locked"],
            ),
        ]
        for name, text, wanted in examined("planted static-library steps", plants):
            with self.subTest(plant=name):
                planted = with_step(workflow, "xcframework", STATIC_LIBRARIES, {"run": text})
                self.assertEqual(static_library_timing_problems(planted), wanted, name)

    def test_the_first_simulator_boots_in_its_own_timed_step(self):
        """SPEC-382 A4: one step right after "the project, generated" boots the pinned iPhone
        alone, waits for it and writes its seconds, which the report reads."""
        workflow = load("xcframework.yml")
        self.assertIsNotNone(step_of(workflow, "harness", HARNESS_PROJECT))
        self.assertEqual(boot_problems(workflow), [])
        examined("harness steps", job_steps(workflow, "harness"))

        # The controls: each plant is refused by its rule's name, and by no other.
        run = step_of(workflow, "harness", BOOT)["run"]
        report = step_of(workflow, "harness", HARNESS_REPORT)["run"]
        moved = json.loads(json.dumps(workflow))
        steps = moved["jobs"]["harness"]["steps"]
        names = [step.get("name") for step in steps]
        steps.insert(names.index(HARNESS_PROJECT), steps.pop(names.index(BOOT)))
        plants = [
            (
                "the wait removed",
                with_step(
                    workflow,
                    "harness",
                    BOOT,
                    {"run": run.replace(BOOT_NEEDS[-1], 'xcrun simctl boot "$udid"')},
                ),
                [f"harness: the boot step lacks {BOOT_NEEDS[-1]}"],
            ),
            (
                "the iPad booted early too",
                with_step(
                    workflow,
                    "harness",
                    BOOT,
                    {"run": run.rstrip("\n") + '\nxcrun simctl boot "$IPAD_SIM"\n'},
                ),
                ["harness: the boot step boots the iPad, which `xcodebuild` boots"],
            ),
            (
                "the boot step before the project",
                moved,
                [f"harness: {BOOT!r} does not follow {HARNESS_PROJECT!r}"],
            ),
            (
                "the report's read of its seconds removed",
                with_step(
                    workflow,
                    "harness",
                    HARNESS_REPORT,
                    {"run": report.replace('minutes("boot-seconds")', '"not measured"')},
                ),
                ['harness: the report lacks minutes("boot-seconds")'],
            ),
        ]
        for name, planted, wanted in examined("planted boot steps", plants):
            with self.subTest(plant=name):
                self.assertEqual(boot_problems(planted), wanted, name)

    def test_the_framework_job_pins_the_developer_directory(self):
        """SPEC-382 A5: the framework job's env sets the developer directory, to the one path
        the other jobs of the body set."""
        workflow = load("xcframework.yml")
        jobs = workflow["jobs"]
        self.assertIn("xcframework", jobs)
        self.assertEqual(developer_directory_problems(workflow), [])
        examined("jobs", list(jobs))

        # The controls: each plant is refused by its rule's name, and by no other.
        env = dict(jobs["xcframework"]["env"])
        other = "/Applications/Xcode.app/Contents/Developer"
        pinned = env["DEVELOPER_DIR"]
        plants = [
            (
                "the entry removed",
                {key: value for key, value in env.items() if key != "DEVELOPER_DIR"},
                ["xcframework: the job's environment does not set the developer directory"],
            ),
            (
                "another Xcode",
                dict(env, DEVELOPER_DIR=other),
                [f"the jobs set 2 developer directories, not one: {sorted([pinned, other])}"],
            ),
        ]
        for name, planted_env, wanted in examined("planted framework envs", plants):
            with self.subTest(plant=name):
                planted = json.loads(json.dumps(workflow))
                planted["jobs"]["xcframework"]["env"] = planted_env
                self.assertEqual(developer_directory_problems(planted), wanted, name)

    def test_each_job_uploads_its_timing_record_whatever_the_outcome(self):
        """SPEC-382 A6: `harness` and the framework job each write their timing record and upload
        it under `always()`, after their existing upload, with the pinned upload action."""
        workflow = load("xcframework.yml")
        self.assertEqual(timing_upload_problems(workflow), [])
        examined("jobs with a timing record", [job for job, _ in TIMING_UPLOADS])

        # The controls: each plant is refused by its rule's name, and by no other.
        harness = step_of(workflow, "harness", TIMING_UPLOAD)
        framework = step_of(workflow, "xcframework", TIMING_UPLOAD)
        plants = [
            (
                "the harness upload's always() removed",
                "harness",
                TIMING_UPLOAD,
                {"if": None},
                ["harness: the timing-harness upload does not run whatever the outcome"],
            ),
            (
                "the harness upload renamed",
                "harness",
                TIMING_UPLOAD,
                {"with": dict(harness["with"], name="timing-planted")},
                ["harness: 0 uploads named 'timing-harness', not one"],
            ),
            (
                "the harness upload on another action",
                "harness",
                TIMING_UPLOAD,
                {"uses": PLANTED_UPLOAD},
                [
                    "harness: the timing-harness upload does not use the pinned action of the "
                    "job's existing upload"
                ],
            ),
            (
                "the harness record's always() removed",
                "harness",
                TIMING_RECORD,
                {"if": None},
                ["harness: the step 'the timing record' does not run whatever the outcome"],
            ),
            (
                "the framework upload without cargo's timings report",
                "xcframework",
                TIMING_UPLOAD,
                {"with": dict(framework["with"], path="xcframework-report/timing.json\n")},
                [f"xcframework: the timing-xcframework upload does not carry {CARGO_TIMINGS}"],
            ),
        ]
        for name, job, step, keys, wanted in examined("planted timing steps", plants):
            with self.subTest(plant=name):
                planted = with_step(workflow, job, step, keys)
                self.assertEqual(timing_upload_problems(planted), wanted, name)

    def test_every_uploaded_bundle_is_read_by_the_report(self):
        """SPEC-382 A7: every result bundle a job uploads is read by that job's report."""
        workflow = load("xcframework.yml")
        problems, judged = unread_bundle_problems(workflow)
        self.assertEqual(problems, [])
        examined("uploaded result bundles", judged)

        # The controls: each plant is refused by its bundle's name, and by no other.
        report = step_of(workflow, "harness", HARNESS_REPORT)["run"]
        plants = [
            (
                "the report's read of the planted suite's bundle removed",
                HARNESS_REPORT,
                report.replace('cases_in("card-probe")', "[]"),
                [
                    "harness: the result bundle card-probe.xcresult is uploaded and not read by "
                    "the report"
                ],
            ),
            (
                "a planted bundle the report does not read",
                "a planted bundle",
                'xcodebuild test -resultBundlePath "$RESULTS/planted.xcresult"\n',
                [
                    "harness: the result bundle planted.xcresult is uploaded and not read by the "
                    "report"
                ],
            ),
        ]
        for name, step, text, wanted in examined("planted bundles", plants):
            with self.subTest(plant=name):
                planted = with_step(workflow, "harness", step, {"run": text})
                self.assertEqual(unread_bundle_problems(planted)[0], wanted, name)


class TheReleaseJobOutlastsItsSlowestRun(unittest.TestCase):
    def test_the_release_job_timeout_holds_its_slowest_measured_run(self):
        """SPEC-367 A1: the `release` job's `timeout-minutes` is a digit string inside the band
        its slowest measured run needs, so a pull request's release job is not cancelled at its
        own bound."""
        job = load("ci.yml")["jobs"]["release"]
        examined("release keys", list(job))
        minutes = str(job.get("timeout-minutes") or "")
        band = f"{RELEASE_TIMEOUT_MINUTES.start} to {RELEASE_TIMEOUT_MINUTES.stop - 1}"
        self.assertTrue(
            minutes.isdigit() and int(minutes) in RELEASE_TIMEOUT_MINUTES,
            f"the release job's timeout is {minutes or 'unset'}, not {band} minutes",
        )


class TheRereleaseCheck(unittest.TestCase):
    def test_a8_the_rerelease_check_runs_scheduled_with_admitted_reads_only(self):
        path = WORKFLOWS / RERELEASE_CHECK
        self.assertTrue(path.is_file(), f"{RERELEASE_CHECK} does not exist")
        workflow = load(RERELEASE_CHECK)
        self.assertEqual(sorted(workflow["on"]), ["schedule", "workflow_dispatch"])
        self.assertEqual(len(workflow["on"]["schedule"]), 1)
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        self.assertEqual(list(workflow["jobs"]), ["rerelease"])
        job = workflow["jobs"]["rerelease"]
        self.assertRegex(job["runs-on"], r"^ubuntu-\d\d\.\d\d$")
        self.assertNotIn("latest", job["runs-on"])
        self.assertEqual(job["environment"], "rerelease-read")
        self.assertEqual(job["permissions"], {"contents": "read", "actions": "write"})
        self.assertEqual(workflow["concurrency"]["group"], "testflight-rerelease-check")
        self.assertEqual(str(workflow["concurrency"]["cancel-in-progress"]).lower(), "false")
        self.assertNotIn("env", job)
        self.assertNotIn("env", workflow)
        pin = None
        for lane_job in load("testflight-internal.yml")["jobs"].values():
            for step in lane_job.get("steps") or []:
                if str(step.get("uses", "")).startswith("actions/checkout@"):
                    pin = step["uses"]
        self.assertIsNotNone(pin, "the internal lane's checkout pin was not found")
        steps = job["steps"]
        self.assertEqual(len(steps), 3)
        checkout, presence, read = steps
        self.assertEqual(checkout["uses"], pin)
        self.assertEqual(str(checkout["with"]["ref"]), "dev")
        self.assertEqual(str(checkout["with"]["fetch-depth"]), "0")
        self.assertEqual(str(checkout["with"]["persist-credentials"]).lower(), "false")
        self.assertEqual(presence["id"], "preflight")
        self.assertEqual(
            presence["env"],
            {
                "KEY": "${{ secrets.TESTFLIGHT_CHECK_KEY != '' }}",
                "KEYID": "${{ secrets.TESTFLIGHT_CHECK_KEY_ID != '' }}",
                "ISSUER": "${{ secrets.TESTFLIGHT_CHECK_ISSUER_ID != '' }}",
                "APPID": "${{ secrets.TESTFLIGHT_CHECK_APP_ID != '' }}",
            },
        )
        self.assertEqual(read["id"], "read")
        self.assertEqual(
            read["env"],
            {
                "KEY": "${{ secrets.TESTFLIGHT_CHECK_KEY }}",
                "KEYID": "${{ secrets.TESTFLIGHT_CHECK_KEY_ID }}",
                "ISSUER": "${{ secrets.TESTFLIGHT_CHECK_ISSUER_ID }}",
                "APPID": "${{ secrets.TESTFLIGHT_CHECK_APP_ID }}",
                "GH_TOKEN": "${{ github.token }}",
                "GH_REPO": "${{ github.repository }}",
            },
        )
        text = workflow_file_text(path)
        names = set(re.findall(r"secrets\.(\w+)", text))
        self.assertEqual(names, {part.removeprefix("secrets.") for part in RERELEASE_PARTS})
        for step in steps:
            run = step.get("run", "")
            self.assertNotIn("${{", run)
            self.assertNotIn("secrets.", run)
            self.assertNotIn("--now", run)
        self.assertEqual(presence["run"].strip(), "python3 scripts/testflight_age.py preflight")
        self.assertEqual(read["run"].strip(), "python3 scripts/testflight_age.py check")

    def test_a15_the_rerelease_lead_covers_the_schedule_interval(self):
        script = REPO / "scripts" / "testflight_age.py"
        lead = None
        for node in ast.parse(script.read_text(encoding="utf-8")).body:
            if isinstance(node, ast.Assign) and any(
                isinstance(target, ast.Name) and target.id == "LEAD_DAYS" for target in node.targets
            ):
                lead = ast.literal_eval(node.value)
        self.assertIsNotNone(lead, "LEAD_DAYS is not a module-level literal of testflight_age.py")
        crons = load(RERELEASE_CHECK)["on"]["schedule"]
        self.assertEqual(len(crons), 1, "the check has not exactly one cron")
        fields = str(crons[0]["cron"]).split()
        self.assertEqual(len(fields), 5, f"cron {fields} has not five fields")
        if fields[2:] == ["*", "*", "*"]:
            interval = 1
        elif fields[2:4] == ["*", "*"] and fields[4].isdigit():
            interval = 7
        else:
            self.fail(f"the cron {fields} is neither daily nor weekly")
        self.assertGreaterEqual(lead, interval)


# ------------------------------------------- the web mutation legs' bound (SPEC-379 R1, R2)

# The jobs that run StrykerJS over the Mini App, each a (workflow file, job) pair: the weekly
# battery's whole sweep, and the pull request's leg, which mutates every changed web file whole.
WEB_MUTATION_LEGS = (("mutation-weekly.yml", "web"), ("ci.yml", "mutation-web"))


def web_mutation_bound_problems(workflows):
    """What the web mutation legs get wrong about their bound (SPEC-379 R1, R2): a leg missing, or
    a leg whose `timeout-minutes` is unset, not a digit string, or outside the band, each named by
    its workflow file and its job. `workflows` maps each file name to the workflow read from it. A
    leg cut at its bound writes no report, so its verdict reads VOID and judges no mutant."""
    band = f"{WEB_MUTATION_TIMEOUT_MINUTES.start} to {WEB_MUTATION_TIMEOUT_MINUTES.stop - 1}"
    problems = []
    for name, job_id in WEB_MUTATION_LEGS:
        job = ((workflows.get(name) or {}).get("jobs") or {}).get(job_id)
        if job is None:
            problems.append(f"{name} has no {job_id} job")
            continue
        minutes = str(job.get("timeout-minutes") or "")
        if not minutes.isdigit() or int(minutes) not in WEB_MUTATION_TIMEOUT_MINUTES:
            problems.append(
                f"{name}: the {job_id} job's timeout is {minutes or 'unset'}, not {band} minutes"
            )
    return problems


class TheWebMutationLegsHoldTheWholeSweep(unittest.TestCase):
    def test_both_web_mutation_legs_hold_the_whole_mini_app_sweep(self):
        """SPEC-379 A1: the weekly battery's `web` job and `ci.yml`'s `mutation-web` job each
        declare a `timeout-minutes` inside the band the whole Mini App's sweep needs, so neither
        leg is cut before its report is written."""
        legs = examined("web mutation legs", WEB_MUTATION_LEGS)
        workflows = {name: load(name) for name, _ in legs}
        self.assertEqual(web_mutation_bound_problems(workflows), [])

    def test_a_web_leg_bound_outside_the_band_is_refused_by_its_leg(self):
        """SPEC-379 A2: the checker accepts both legs at the band's floor and at its ceiling, and
        refuses each leg's bound set back to 60, set one past the ceiling, or removed, naming that
        leg alone. Every copy is planted from the workflow files with both bounds set here, so the
        test pins the checker, not the tree's own bounds."""
        texts = {name: workflow_file_text(WORKFLOWS / name) for name, _ in WEB_MUTATION_LEGS}
        live = {
            name: load(name)["jobs"][job_id].get("timeout-minutes")
            for name, job_id in WEB_MUTATION_LEGS
        }

        def leg(name, job_id, minutes):
            line = "" if minutes is None else f"    timeout-minutes: {minutes}\n"
            old = f"    timeout-minutes: {live[name]}\n"
            return planted_job(texts[name], job_id, (old, line))

        floor = WEB_MUTATION_TIMEOUT_MINUTES.start
        ceiling = WEB_MUTATION_TIMEOUT_MINUTES.stop - 1
        band = f"{floor} to {ceiling}"
        for minutes in (floor, ceiling):
            with self.subTest(control=minutes):
                both = {name: leg(name, job_id, minutes) for name, job_id in WEB_MUTATION_LEGS}
                self.assertEqual(web_mutation_bound_problems(both), [])
        plants = [
            (
                name,
                job_id,
                minutes,
                f"{name}: the {job_id} job's timeout is {shown}, not {band} minutes",
            )
            for name, job_id in WEB_MUTATION_LEGS
            for minutes, shown in ((60, "60"), (ceiling + 1, str(ceiling + 1)), (None, "unset"))
        ]
        for name, job_id, minutes, refusal in examined("planted web leg bounds", plants):
            with self.subTest(leg=f"{name} {job_id}", minutes=minutes):
                workflows = {other: leg(other, job, floor) for other, job in WEB_MUTATION_LEGS}
                workflows[name] = leg(name, job_id, minutes)
                self.assertEqual(web_mutation_bound_problems(workflows), [refusal])


if __name__ == "__main__":
    unittest.main()
