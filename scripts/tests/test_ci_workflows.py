"""CI runs the whole gate on hosted runners with read-only tokens and pinned actions (SPEC-002 A9),
on pull requests into dev and main and pushes to both (SPEC-030 A1), and only this repository's dev
reaches main (SPEC-034 A5 to A7). The gate runs in parallel jobs, each stage in exactly one, the
engine's slow tests in a job of their own, a cache is saved only by a push to dev or main, and every
job that compiles Rust installs the protoc Anki's engine needs (SPEC-038). No workflow reads a
secret but the default token, or checks out or fetches another repository (SPEC-034 A9 to A12), and
a `.yaml` workflow is held to the hardening rules as a `.yml` one is (A13)."""

import math
import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

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
            named = rf"bash scripts/check\.sh [a-z -]*(?<![\w-]){re.escape(stage)}(?![\w-])"
            self.assertRegex(ci, named, stage)

    def test_the_aggregate_check_needs_every_job_and_always_runs(self):
        ci = (WORKFLOWS / "ci.yml").read_text()
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
                "checkout-of-another-repository.yml:jobs.build.steps[0]: checks out "
                "example-org/other-repository, not this repository",
                "checkout-of-another-repository.yml:jobs.build.steps[1]: checks out "
                "${{ github.event.pull_request.head.repo.full_name }}, not this repository",
                "clone-of-another-repository.yml:jobs.build.steps[0]: clones a repository: "
                "git clone --depth 1 https://github.com/example-org/other-repository.git",
                "clone-of-another-repository.yml:jobs.build.steps[1]: clones a repository: "
                "gh repo clone example-org/other-repository",
                "every-secret.yml:jobs.build.steps[0].env.CHOSEN: reads the whole secrets "
                "context, or a secret named at run time",
                "every-secret.yml:jobs.build.steps[0].run: reads the whole secrets context, or a "
                "secret named at run time",
                "fetch-of-a-url.yml:jobs.build.steps[1]: points git at a URL: git fetch "
                "https://github.com/example-org/other-repository.git main",
                "fetch-of-a-url.yml:jobs.build.steps[2]: points git at a URL: git pull --ff-only "
                "https://github.com/example-org/other-repository.git main",
                "key-the-reader-refuses.yml:line 18: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 20: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 22: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 25: a key that is not a plain name is not read",
                "key-the-reader-refuses.yml:line 31: a key that is not a plain name is not read",
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
                "secret-in-brackets.yml:jobs.build.steps[0].env.INDEXED: reads the secret "
                "EXAMPLE_TOKEN",
                "secrets-inherited.yaml:jobs.call.secrets: passes every secret to the workflow "
                "it calls",
            ],
        )
        # A character outside printable ASCII, planted from a Python escape so no committed file
        # holds one: each line that holds one is refused by its line, and what YAML reads there is
        # judged as well. The checker reads a script's space as a space and its break as the end of
        # a command, so the two kinds name a clone's command apart.
        clone = "#||git clone https://github.com/example-org/other-repository.git"
        planted = [(name, c, "false ", "planted ") for name, c in PLANTED_SPACES.items()]
        planted += [(name, c, "", "") for name, c in PLANTED_BREAKS.items()]
        for name, character, run, block in planted:
            with self.subTest(name):
                self.assertEqual(
                    planted_problems(PLANTED_CHARACTERS.replace("<C>", character)),
                    [
                        *(
                            f"planted.yml:line {n}: a character the reader does not read"
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
                    ],
                )
        # A line the reader cannot place refuses the whole file at once, naming the line and why.
        for form, (template, why) in PLANTED_UNPLACED_CHARACTERS.items():
            for name, character in {**PLANTED_SPACES, **PLANTED_BREAKS}.items():
                with self.subTest(f"{form}: {name}"), self.assertRaisesRegex(AssertionError, why):
                    planted_problems(template.replace("<C>", character))

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
        # GitHub reads a `.yaml` workflow as it reads a `.yml` one (SPEC-034 R7): each hardening test
        # above refuses the planted `.yaml` workflow by its name, beside a hardened `.yml` control.
        planted = workflow_files(PLANTED_HARDENING)
        for test in (
            "test_every_workflow_defaults_to_a_read_only_token",
            "test_every_action_is_pinned_by_a_full_commit_sha",
            "test_no_workflow_uses_a_self_hosted_runner_or_a_privileged_trigger",
        ):
            case = WorkflowsAreHardened(test)
            case.files = planted
            with self.subTest(test), self.assertRaisesRegex(AssertionError, r"unhardened\.yaml"):
                getattr(case, test)()


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
        ci = triggers((WORKFLOWS / "ci.yml").read_text(encoding="utf-8"))
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


# ------------------------------------------------------------------ reading a workflow (SPEC-038)

# A key the reader reads: a plain name, bare or in matching quotes (SPEC-034 R7).
KEY = re.compile(r"(['\"]?)([\w.-]+)\1")


class Unread(AssertionError):
    """A workflow that holds forms the reader does not read, each refusal as `line N: why`. It
    carries the rest of the workflow as read, each refused key or value read as '', so a checker
    can name every refusal and still judge everything else (SPEC-034 R7)."""

    def __init__(self, refused, workflow):
        super().__init__("the reader does not read " + "; ".join(refused))
        self.refused, self.workflow = refused, workflow


def read_workflow(text):
    """A workflow as dicts, lists and strings, read without a YAML library. It reads the block YAML
    the workflows here use: mappings keyed by plain names, `- ` sequences, `|` block scalars, flow
    lists of plain items, and plain or quoted one-line scalars, a quote doubled inside single
    quotes read as one. Blank lines, comment lines and a ` #` comment after a value are dropped.
    It ends a line only at a line feed or a carriage return, and reads a space or a tab as white
    space and nothing else, as YAML does.
    It fails closed (SPEC-034 R7). A line that holds a character other than a tab or printable
    ASCII, a double-quoted value that holds an escape, a quoted value that does not end at its
    closing quote, an anchor, alias or tag, a flow mapping, a flow list whose items are not plain,
    and a key that is not a plain name are each refused by their line, never guessed at, and the
    file raises Unread once it is read. A line it cannot place refuses the whole file at once."""
    lines = re.split(r"\r\n|\r|\n", text)
    refused = [
        f"line {at + 1}: a character the reader does not read"
        for at, line in enumerate(lines)
        if re.search(r"[^\t\x20-\x7e]", line)
    ]
    value, at = _mapping(lines, _skip(lines, 0), 0, refused)
    at = _skip(lines, at)
    if at < len(lines):
        raise AssertionError(f"line {at + 1} was not read: {lines[at]!r}")
    if refused:
        raise Unread(refused, value)
    return value


def _indent(line):
    return len(line) - len(line.lstrip(" "))


def _skip(lines, at):
    while at < len(lines) and (
        not lines[at].strip(" \t") or lines[at].lstrip(" \t").startswith("#")
    ):
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
    key = KEY.fullmatch(text.strip(" \t"))
    if not key:
        raise ValueError("a key that is not a plain name is not read")
    return key.group(2)


def _scalar(text):
    """A one-line scalar or flow list as YAML reads it, or ValueError naming a form the reader does
    not read: an anchor, alias or tag, a flow mapping, a flow list whose items are not plain (a
    quoted or nested item, or a `#`, `:` or `?` inside it), and a plain value that holds `: `, which
    YAML reads as a key."""
    text = text.strip(" \t")
    if text[:1] in ("&", "*", "!"):
        raise ValueError("an anchor, alias or tag is not read")
    if text[:1] in ("'", '"'):
        return _quoted(text)
    if text[:1] == "{":
        raise ValueError("a flow mapping is not read")
    if text[:1] == "[":
        items = re.fullmatch(r"\[([^\[\]{}'\"#:?]*)\](?:[ \t]+#.*)?", text)
        if not items:
            raise ValueError("a flow list whose items are not plain is not read")
        return [_scalar(part) for part in items.group(1).split(",") if part.strip()]
    text = re.sub(r"[ \t]#.*$", "", text).strip(" \t")
    if re.search(r":(?:[ \t]|$)", text):
        raise ValueError("a key that is not a plain name is not read")
    return text


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
    return quoted.group(1).replace("''", "'") if single else quoted.group(1)


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
        at = _skip(lines, at)
        if at >= len(lines) or _indent(lines[at]) != indent or _item(lines[at], indent):
            return found, at
        text = lines[at][indent:]
        if ": " in text:
            key, rest = text.split(": ", 1)
        elif text.endswith(":"):
            key, rest = text[:-1], ""
        else:
            raise AssertionError(f"line {at + 1} is not a mapping entry: {lines[at]!r}")
        key, rest = _read(_key, key, at, refused), rest.strip(" \t")
        if rest in ("|", "|-"):
            at += 1
            body = []
            while at < len(lines) and (not lines[at].strip(" \t") or _indent(lines[at]) > indent):
                body.append(lines[at])
                at += 1
            while body and not body[-1].strip(" \t"):
                body.pop()
            width = min((_indent(line) for line in body if line.strip(" \t")), default=0)
            found[key] = "".join(line[width:] + "\n" for line in body)
        elif not rest or rest.startswith("#"):
            child = _skip(lines, at + 1)
            if child < len(lines) and _indent(lines[child]) > indent:
                found[key], at = _block(lines, child, _indent(lines[child]), refused)
            else:
                found[key], at = None, at + 1
        else:
            found[key], at = _read(_scalar, rest, at, refused), at + 1


def _sequence(lines, at, indent, refused):
    found = []
    while True:
        at = _skip(lines, at)
        if at >= len(lines) or _indent(lines[at]) != indent or not _item(lines[at], indent):
            return found, at
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
    return read_workflow((WORKFLOWS / name).read_text(encoding="utf-8"))


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
                problems += save_problems(where, step)
    return problems, saves


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
        # SPEC-039 adds the five mutation jobs beside the gate's five, each a need of ci.
        mutation = [
            "mutation-plan",
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


class OnlyAPushSavesACache(unittest.TestCase):
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
        saves = []
        for path in examined("workflow files", sorted(WORKFLOWS.glob("*.yml"))):
            problems, found = cache_problems(path.name, load(path.name))
            self.assertEqual(problems, [], path.name)
            saves += found
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
        ci = (WORKFLOWS / "ci.yml").read_text(encoding="utf-8")
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
        pins = [
            (path.name, digest)
            for path in sorted(WORKFLOWS.glob("*.yml"))
            for digest in re.findall(r"PROTOC_SHA256: ([0-9a-f]+)", path.read_text())
        ]
        for name, digest in examined("protoc pins", pins):
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


def step_inputs(step):
    """A step's `with` inputs, each name in lower case: the runner reads an input's name in any
    case. Inputs that are not a mapping are none: the reader has named their line, or GitHub
    refuses the workflow."""
    given = step.get("with")
    if not isinstance(given, dict):
        return {}
    return {str(name).lower(): value for name, value in given.items()}


def checked_out(step):
    """The repository a checkout step checks out. An omitted or empty `repository`, and
    `${{ github.repository }}`, are this repository, as actions/checkout defaults it."""
    given = str(step_inputs(step).get("repository") or "").strip()
    if not given or re.fullmatch(r"\$\{\{\s*github\.repository\s*\}\}", given):
        return THIS_REPOSITORY
    return given


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
    """Each command of a run script that clones a repository or gives git a URL: a workflow reaches
    this repository through actions/checkout and origin, and no other."""
    found = []
    for command in commands(script):
        if CLONE.search(command):
            found.append(f"clones a repository: {command}")
        elif GIT_URL.search(command):
            found.append(f"points git at a URL: {command}")
    return found


def secret_and_checkout_problems(directory):
    """Every read of a secret other than GITHUB_TOKEN, every `secrets: inherit`, and every checkout,
    clone or fetch of another repository in the workflows of `directory`, each named by its file
    and its place, with what was judged: (problems, {population: [...]}). A form the reader does
    not read is a problem named by its line, and the rest of that file is judged as read. A
    directory with no workflow file is VOID, never a pass."""
    files = workflow_files(directory)
    problems = []
    judged = {"expressions": [], "checkouts": [], "run steps": []}
    for path in files:
        try:
            workflow, refused = read_workflow(path.read_text(encoding="utf-8")), []
        except Unread as unread:
            workflow, refused = unread.workflow, unread.refused
        problems += [f"{path.name}:{why}" for why in refused]
        for where, text in strings(workflow):
            for expression in expressions_in(text):
                judged["expressions"].append((f"{path.name}:{where}", expression))
                problems += [f"{path.name}:{where}: {read}" for read in secret_reads(expression)]
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
            for n, step in enumerate(job.get("steps") or []):
                if not isinstance(step, dict):
                    continue
                where = f"{path.name}:jobs.{job_id}.steps[{n}]"
                # GitHub reads an action's owner and name in any case.
                if action(step).lower() == "actions/checkout":
                    repository = checked_out(step)
                    judged["checkouts"].append((where, repository))
                    if repository != THIS_REPOSITORY:
                        problems.append(f"{where}: checks out {repository}, not this repository")
                if "run" in step:
                    judged["run steps"].append(where)
                    problems += [f"{where}: {reach}" for reach in reaches(str(step["run"]))]
    return problems, judged


def planted_problems(text):
    """What the checker finds in one planted workflow, written to a scratch directory at test time
    (SPEC-034 A10)."""
    with tempfile.TemporaryDirectory() as scratch:
        (Path(scratch) / "planted.yml").write_text(text, encoding="utf-8")
        return secret_and_checkout_problems(Path(scratch))[0]


if __name__ == "__main__":
    unittest.main()
