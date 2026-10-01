"""CI runs the whole gate on hosted runners with read-only tokens and pinned actions (SPEC-002 A9),
on pull requests into dev and main and pushes to both (SPEC-030 A1), and only this repository's dev
reaches main (SPEC-034 A5 to A7). The gate runs in parallel jobs, each stage in exactly one, the
engine's slow tests in a job of their own, a cache is saved only by a push to dev or main, and every
job that compiles Rust installs the protoc Anki's engine needs (SPEC-038). No workflow reads a
secret but the default token, or checks out or fetches another repository (SPEC-034 A9 to A12), and
a `.yaml` workflow is held to the hardening rules as a `.yml` one is, the hardening tests reading
keys the way the checker does (A13)."""

import collections
import contextlib
import json
import math
import os
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

from _support import REPO, examined

WORKFLOWS = REPO / ".github" / "workflows"
PINNED = re.compile(r"^[\w.-]+/[\w.-]+(?:/[\w.-]+)*@[0-9a-f]{40}$")
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
# The owner's layout of the gate (SPEC-038 R3): each job and the stages it runs, in order. The engine
# job, which runs the engine set beside rust, joined it by the amendment of section 8 (R14).
OWNER_LAYOUT = {
    "rust": ["fmt", "clippy", "test", "doctest", "audit-rust"],
    "engine": ["test-engine"],
    "web": ["web", "audit-web"],
    "hygiene": ["python", "scrub", "secrets"],
}
# What the Rust cache holds (SPEC-038 R1): the crates Cargo downloaded, and the build.
RUST_CACHE = ["~/.cargo/registry/index/", "~/.cargo/registry/cache/", "~/.cargo/git/db/", "target/"]
BROWSERS = ["~/.cache/ms-playwright"]
# The stages that compile Rust: python's among them, because a guard test builds the ingest crate
# twice (SPEC-055 A2), and test-engine, which builds the engine set's tests (R13).
COMPILES_RUST = {"clippy", "test", "doctest", "python", "test-engine"}
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
            (path.name, ref) for path in self.files for ref in entries(read_hardened(path), "uses")
        ]
        for name, ref in examined("action references", uses):
            self.assertRegex(ref, PINNED, f"{name} uses {ref}")

    def test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger(self):
        runners = []
        for path in self.files:
            workflow = read_hardened(path)
            code = re.sub(r"(?m)#.*$", "", workflow_file_text(path))
            self.assertNotIn("pull_request_target", code, path.name)
            runners += [(path.name, runner) for runner in entries(workflow, "runs-on")]
        for name, runner in examined("runs-on values", runners):
            # A list or a mapping of labels is read as its text, so the pattern refuses it by name.
            self.assertRegex(str(runner), r"^ubuntu-\d\d\.\d\d$", f"{name} runs on {runner}")

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
    """Refuse a line whose indentation holds a tab: YAML indents with spaces alone, and the `yaml`
    package GitHub's parser reads with refuses the file ("Tabs are not allowed as indentation"). A
    tab inside a value, or in a block scalar's text, is text and is read."""
    line = lines[at]
    why = f"line {at + 1}: a tab in the indentation, which YAML refuses"
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
                    refused.append(f"line {row + 1}: a tab in the indentation, which YAML refuses")
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
        # adds the sixth, the Python runner's shards.
        mutation = [
            "mutation-plan",
            "mutation-python",
            "mutation-rust",
            "mutation-rows",
            "mutation-verdict",
            "mutation-web",
        ]
        self.assertEqual(
            sorted(needs), sorted([*OWNER_LAYOUT, *mutation, "workflow-lint", "base-is-dev"])
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


# ------------------------------------------------------- the engine job (SPEC-038 A18, R14)

# The job that runs the engine set, SPEC-022's slow sync and budget tests, beside rust.
ENGINE_JOB = "engine"
# The engine job's timeout, sized from its measured runs (SPEC-038 section 8): a cold run, which
# compiles every dependency before about 140 s of tests, takes about five minutes, so the timeout
# holds at least two cold runs and ends a hung sync test within half an hour, not six hours.
ENGINE_TIMEOUT_MINUTES = range(10, 31)


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
    """Every read of a secret other than GITHUB_TOKEN, every `secrets: inherit`, and every checkout,
    clone or fetch of another repository in the workflows of `directory`, each named by its file
    and its place, with what was judged: (problems, {population: [...]}). Every step of a job is
    judged, a step inside a `parallel` block at any depth included, and a checkout whose inputs are
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
        for where, text in strings(workflow):
            for expression in expressions_in(text):
                judged["expressions"].append((f"{path.name}:{where}", expression))
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


def read_primitives(source):
    """Every call that reads a file in a module's source, as (enclosing function, receiver text)
    pairs: `read_text`, `read_bytes` and `open`, the loader's own body left out. A call outside any
    function is named `<module>`, so no read is one the census cannot place."""
    import ast

    tree = ast.parse(source)
    parent = {child: node for node in ast.walk(tree) for child in ast.iter_child_nodes(node)}
    found = []
    for node in ast.walk(tree):
        if not isinstance(node, ast.Call):
            continue
        func = node.func
        name = func.attr if isinstance(func, ast.Attribute) else getattr(func, "id", "")
        if name not in ("read_text", "read_bytes", "open"):
            continue
        scope = node
        while scope in parent and not isinstance(scope, ast.FunctionDef):
            scope = parent[scope]
        inside = scope.name if isinstance(scope, ast.FunctionDef) else "<module>"
        if inside != LOADER:
            found.append(
                (inside, ast.unparse(func.value) if isinstance(func, ast.Attribute) else name)
            )
    return found


LOADER = "workflow_file_text"
# The reads of a file that is no workflow: a script, a lock file, a configuration, a brief, a
# ruleset, a log, a plan. Each is (module, function): how many reads that function makes.
NOT_WORKFLOW_READS = {
    (
        "test_ci_workflows",
        "test_every_file_read_in_the_test_modules_is_the_loader_or_a_named_non_workflow_read",
    ): 2,
    ("test_ci_workflows", "gate_stages"): 1,
    ("test_ci_workflows", "required_contexts"): 1,
    ("test_ci_workflows", "run_step"): 1,
    ("test_ci_workflows", "test_a_yaml_workflow_is_held_to_the_same_hardening_rules"): 1,
    ("test_ci_workflows", "test_ci_runs_every_stage_of_the_local_gate"): 1,
    ("test_ci_workflows", "test_the_browser_cache_is_keyed_on_the_locked_playwright_version"): 1,
    (
        "test_mutation_workflows",
        "test_every_job_that_runs_cargo_mutants_installs_the_test_tool_it_names",
    ): 1,
    ("test_mutation_workflows", "test_the_builder_brief_teaches_the_equivalence_record"): 4,
    ("test_mutation_workflows", "test_the_builder_brief_teaches_the_mutation_rules"): 1,
    (
        "test_mutation_workflows",
        "test_the_configuration_check_refuses_what_stryker_would_read_otherwise",
    ): 1,
    ("test_mutation_workflows", "test_the_tool_configurations_load_under_their_own_rules"): 2,
    ("test_mutation_workflows", "test_the_verdict_binds_every_record_against_the_whole_listing"): 3,
    ("test_not_started_legs", "planned"): 1,
    ("test_not_started_legs", "rewrite"): 1,
    (
        "test_not_started_legs",
        "test_a_listed_leg_that_is_missing_or_not_started_is_refused_by_name",
    ): 1,
    ("test_not_started_legs", "test_an_examined_sum_that_differs_from_the_listing_is_refused"): 1,
    ("test_not_started_legs", "test_the_verdict_step_fails_on_the_legs_check"): 1,
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
        sites = collections.Counter()
        modules = 0
        for path in sorted(Path(__file__).parent.glob("test_*.py")):
            source = path.read_text(encoding="utf-8")
            if path.stem != Path(__file__).stem and "test_ci_workflows" not in source:
                continue
            modules += 1
            for function, _receiver in read_primitives(source):
                sites[(path.stem, function)] += 1
        examined("test modules that read workflows", range(modules))
        self.assertIn(f"def {LOADER}(", Path(__file__).read_text(encoding="utf-8"))
        self.assertEqual(dict(sites), NOT_WORKFLOW_READS)

    def test_the_census_is_red_on_a_read_it_does_not_name(self):
        planted = "def extra(path):\n    return read_workflow(path.read_text(encoding='utf-8'))\n"
        self.assertEqual(read_primitives(planted), [("extra", "path")])
        self.assertEqual(read_primitives("TEXT = open('x').read()\n"), [("<module>", "open")])
        loader = f"def {LOADER}(path):\n    return path.read_bytes().decode('utf-8')\n"
        self.assertEqual(read_primitives(loader), [])

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


if __name__ == "__main__":
    unittest.main()
