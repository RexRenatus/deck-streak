"""The `fragment` check passes a Dependabot pull request that changes only the files the
ecosystems of `.github/dependabot.yml` update, and reads every other pull request as it did
(SPEC-190 R13 and R14, A13 to A18, #1020).

Each test runs the `fragment` job's own step from `changelog.yml` under `bash -e`, as the runner
does, in a planted repository whose base commit and head commit stand for a pull request, the step's
env taken from the event payload of the scenario. Nothing here runs GitHub.
"""

import os
import re
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path

from _support import REPO, examined
from test_dependabot import CONFIG, ecosystems

WORKFLOW = REPO / ".github" / "workflows" / "changelog.yml"
BASE_SHA = "github.event.pull_request.base.sha"
AUTHOR = "github.event.pull_request.user.login"
BOT = "dependabot[bot]"
PERSON = "someone"
REFUSED = "Add a fragment under changelog.d/ (see changelog.d/README.md).\n"
PASSED = "A Dependabot pull request that changes only dependency files needs no fragment.\n"
# The ecosystems `.github/dependabot.yml` updates, each with its directory (R13): the root of the
# repository, and for pip the coverage requirements file of SPEC-592 (R11).
ECOSYSTEMS = {"cargo": "/", "github-actions": "/", "npm": "/", "pip": "/tools/coverage"}
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
# A planted tree: each kind of file an ecosystem updates, a source file, a manifest no ecosystem
# updates, the configuration of Dependabot itself, the changelog and the README of the fragments.
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
    "pnpm-workspace.yaml": "packages:\n  - web/app\n",
    "web/app/package.json": "{}\n",
    "tools/coverage/requirements.txt": "coverage\n",
    "vendor/deepmerge/package.json": "{}\n",
    ".github/dependabot.yml": "version: 2\n",
    ".github/workflows/ci.yml": "name: ci\n",
    ".github/workflows/release.yml": "name: release\n",
    ".github/workflows/lint.yaml": "name: lint\n",
}


def fragment_step():
    """The `fragment` job's step in changelog.yml: its env as (name, expression) pairs and its
    script, read up to the next step. The file is read as its bytes, decoded as UTF-8."""
    text = WORKFLOW.read_bytes().decode("utf-8")
    job = re.search(r"(?ms)^  fragment:\n(.*?)(?=^  [a-z-]+:\n|\Z)", text).group(1)
    names = re.findall(r"(?m)^          ([A-Z_]+): \$\{\{ ([a-z_.]+) \}\}$", job)
    block = re.search(r"(?ms)^        run: \|\n(.*?)(?=^      - |\Z)", job)
    return names, textwrap.dedent(block.group(1))


def environment(home):
    """PATH from the environment of the test, HOME at `home` and no system git configuration, so
    no configuration of the machine reaches a planted repository."""
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
    """Each of `paths` from the planted tree with a line added, as an update writes it."""
    return {path: TREE[path] + "bumped\n" for path in paths}


def verdict(author, head, base=None, moves=()):
    """The run of the step over a planted pull request by `author`: a base commit that holds `base`
    (the planted tree by default), then a head commit that writes `head` and moves each (old, new)
    of `moves` unchanged."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch) / "repo"
        root.mkdir()
        git(root, "init", "-q", "-b", "dev")
        write(root, TREE if base is None else base)
        git(root, "add", "-A")
        git(root, "commit", "-q", "-m", "base")
        context = {BASE_SHA: git(root, "rev-parse", "HEAD"), AUTHOR: author}
        write(root, head)
        for old, new in moves:
            (root / new).parent.mkdir(parents=True, exist_ok=True)
            git(root, "mv", old, new)
        git(root, "add", "-A")
        git(root, "commit", "-q", "--allow-empty", "-m", "head")
        names, script = fragment_step()
        env = environment(scratch)
        for name, expression in names:
            if expression not in context:
                raise AssertionError(f"fragment reads {expression}, which no scenario models")
            env[name] = context[expression]
        return subprocess.run(
            ["bash", "-e", "-c", script],
            cwd=root,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )


def dependency_files():
    """Every file in this tree that an ecosystem of `.github/dependabot.yml` updates (R13): the
    workspace manifest, the lockfile and the manifest of each member of the Cargo workspace; the
    root manifest, the lockfile, the workspace file and the manifest of the one pnpm package; the
    coverage requirements file (SPEC-592 R11); and every workflow file."""
    found = [
        "Cargo.toml",
        "Cargo.lock",
        "package.json",
        "pnpm-lock.yaml",
        "pnpm-workspace.yaml",
        "web/app/package.json",
        "tools/coverage/requirements.txt",
    ]
    found += sorted(path.relative_to(REPO).as_posix() for path in REPO.glob("crates/*/Cargo.toml"))
    found += sorted(
        path.relative_to(REPO).as_posix()
        for path in (REPO / ".github" / "workflows").iterdir()
        if path.suffix in (".yml", ".yaml")
    )
    return found


class ADependabotDependencyPullRequestNeedsNoFragment(unittest.TestCase):
    def test_the_author_is_read_from_the_event_pull_request(self):
        names, script = fragment_step()
        self.assertEqual(dict(names), {"BASE": BASE_SHA, "AUTHOR": AUTHOR})
        self.assertNotIn("${{", script)

    def test_a_dependabot_pull_request_that_changes_only_dependency_files_needs_no_fragment(self):
        updates = {
            "cargo: the lockfile": bumped("Cargo.lock"),
            "cargo: the workspace manifest and the lockfile": bumped("Cargo.toml", "Cargo.lock"),
            "cargo: the manifest of a member and the lockfile": bumped(
                "crates/kernel/Cargo.toml", "Cargo.lock"
            ),
            "npm: the manifest of the package and the lockfile": bumped(
                "web/app/package.json", "pnpm-lock.yaml"
            ),
            "npm: the root manifest and the lockfile": bumped("package.json", "pnpm-lock.yaml"),
            "npm: the catalog and the lockfile": bumped("pnpm-workspace.yaml", "pnpm-lock.yaml"),
            "github-actions: two workflow files": bumped(
                ".github/workflows/ci.yml", ".github/workflows/release.yml"
            ),
            "github-actions: a workflow saved with the yaml suffix": bumped(
                ".github/workflows/lint.yaml"
            ),
            "pip: the coverage requirements file": bumped("tools/coverage/requirements.txt"),
        }
        for label, head in examined("Dependabot updates", updates.items()):
            with self.subTest(update=label):
                done = verdict(BOT, head)
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertEqual(done.stdout, PASSED)

    def test_every_file_an_ecosystem_updates_in_this_tree_passes_alone(self):
        found = ecosystems(CONFIG.read_text(encoding="utf-8"))
        self.assertEqual(sorted(found), list(ECOSYSTEMS))
        for name, directory in ECOSYSTEMS.items():
            self.assertRegex(found[name], rf"(?m)^    directory: {re.escape(directory)}$", name)
        cargo = (REPO / "Cargo.toml").read_text(encoding="utf-8")
        self.assertRegex(cargo, r"(?m)^members = \[\"crates/\*\"\]$")
        pnpm = (REPO / "pnpm-workspace.yaml").read_text(encoding="utf-8")
        self.assertRegex(pnpm, r"(?m)^packages:\n  - web/app\n(?!  - )")
        for path in examined("files an ecosystem updates in this tree", dependency_files()):
            with self.subTest(path=path):
                self.assertTrue((REPO / path).is_file(), path)
                planted = {"README.md": "planted\n", path: "base\n"}
                done = verdict(BOT, {path: "head\n"}, base=planted)
                self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
                self.assertEqual(done.stdout, PASSED)


class EveryOtherPullRequestReadsAsItDid(unittest.TestCase):
    def test_every_other_author_reads_as_before(self):
        fragment = {"changelog.d/bump.md": "### Changed\n\n- A bump.\n"}
        source = bumped("crates/kernel/src/lib.rs")
        cases = {
            "a dependency update by a person, with no fragment": (
                PERSON,
                bumped("Cargo.lock"),
                REFUSED,
            ),
            "a coverage requirements update by a person, with no fragment": (
                PERSON,
                bumped("tools/coverage/requirements.txt"),
                REFUSED,
            ),
            "a source change by a person, with no fragment": (PERSON, source, REFUSED),
            "a source change by a person, with a fragment": (PERSON, {**source, **fragment}, ""),
            "a compiled changelog by a person": (PERSON, bumped("CHANGELOG.md"), ""),
            "a source change by Dependabot, with a fragment": (BOT, {**source, **fragment}, ""),
        }
        for label, (author, head, said) in examined("earlier verdicts", cases.items()):
            with self.subTest(case=label):
                done = verdict(author, head)
                self.assertEqual(done.returncode, 1 if said else 0, done.stdout + done.stderr)
                self.assertEqual(done.stdout, said)

    def test_a_dependabot_pull_request_that_changes_another_path_is_refused(self):
        moved = (("crates/kernel/src/lib.rs", "crates/moved/Cargo.toml"),)
        cases = {
            "a source file beside the lockfile": (
                bumped("Cargo.lock", "crates/kernel/src/lib.rs"),
                (),
            ),
            "a manifest no ecosystem updates": (bumped("vendor/deepmerge/package.json"), ()),
            "the configuration of Dependabot": (bumped(".github/dependabot.yml"), ()),
            "a file below the workflow directory": (
                {".github/workflows/nested/ci.yml": "name: ci\n"},
                (),
            ),
            "a source file beside the coverage requirements file": (
                bumped("tools/coverage/requirements.txt", "crates/kernel/src/lib.rs"),
                (),
            ),
            "a file that only resembles the coverage requirements file": (
                {"tools/coverage/requirements.txt.bak": "coverage\n"},
                (),
            ),
            "a requirements file below the coverage directory": (
                {"tools/coverage/x/requirements.txt": "coverage\n"},
                (),
            ),
            "a source file moved onto the manifest path of a member": ({}, moved),
            "no changed path": ({}, ()),
        }
        for label, (head, moves) in examined("Dependabot pull requests", cases.items()):
            with self.subTest(case=label):
                done = verdict(BOT, head, moves=moves)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertEqual(done.stdout, REFUSED)

    def test_a_login_that_only_resembles_dependabots_is_refused(self):
        logins = (
            "dependabot",
            "Dependabot[bot]",
            "dependabott",
            "dependabot[bot]x",
            " dependabot[bot]",
            "dependabot[bot] ",
            "dependabot-preview[bot]",
            "renovate[bot]",
            "github-actions[bot]",
            "",
        )
        for login in examined("logins that are not the Dependabot login", logins):
            with self.subTest(login=login):
                done = verdict(login, bumped("Cargo.lock"))
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                self.assertEqual(done.stdout, REFUSED)


if __name__ == "__main__":
    unittest.main()
