"""The TestFlight lane's steps, run as child processes over fixture repositories (SPEC-352 A1 to
A6, ADR-363 D2 and D7).

`scripts/ios_lane.py` is never imported: each test runs it with `sys.executable`, in a fixture
checkout, with an environment the test builds from `PATH`, a scratch `HOME` and the GitHub
variables the step reads, so nothing of the session reaches it. Every number a test expects is a
literal the fixture's history fixes, never one the script computed, and every refusal is matched
as one whole line of the script's error output.

The fixture's history, every commit's workspace version in brackets:

    dev:   R [0.0.1] - A [0.1.0] - B [0.1.5] ----------------------- D
                                    \\- S1 [0.1.5] - S2 [0.2.0] -/     \\- T [0.4.0]  (topic)
    main:  R ------- M1 (A merged) ------------------------------- M2 (D merged)

`dev` holds six commits, four on its first-parent chain; `main`'s first-parent chain is M2, M1,
R. Annotated tags: `v0.1.0` on M1, `v0.2.0` on M2, `v0.1.5` on B (reached from main only through
M2's second parent), `v0.4.0` on T (not on main at all), and, on M2, `v0.3.0` (another version)
and `v0.2`, `v0.02.0` and `v0.2.0-rc.1` (not SemVer). `v0.0.1` on R is a lightweight tag.
"""

import os
import subprocess
import sys
import tempfile
import unittest
import uuid
from pathlib import Path

from _support import REPO, examined

LANE = REPO / "scripts" / "ios_lane.py"
IDENTITY = ("-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid")
SHALLOW = "the checkout is shallow; the build number needs the full history"


def cargo(version):
    """A root `Cargo.toml` whose workspace version is `version`."""
    return f'[workspace]\nmembers = []\n\n[workspace.package]\nversion = "{version}"\n'


def git(cwd, env, *args):
    """Run git in `cwd` under the fixture's environment and return its output, stripped."""
    run = subprocess.run(
        ["git", *IDENTITY, *args], cwd=cwd, env=env, capture_output=True, text=True, check=True
    )
    return run.stdout.strip()


class LanePlan(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        scratch = tempfile.TemporaryDirectory()
        cls.addClassCleanup(scratch.cleanup)
        cls.scratch = Path(scratch.name)
        (cls.scratch / "home").mkdir()
        cls.env = {"PATH": os.environ["PATH"], "HOME": str(cls.scratch / "home"), "LANG": "C.UTF-8"}
        cls.origin = cls.scratch / "origin"
        cls.origin.mkdir()
        cls.sha = {}

        def commit(name, version=None):
            if version is not None:
                (cls.origin / "Cargo.toml").write_text(cargo(version), encoding="utf-8")
            (cls.origin / "notes.txt").write_text(f"{name}\n", encoding="utf-8")
            run("add", "-A")
            run("commit", "-q", "-m", name)
            cls.sha[name] = run("rev-parse", "HEAD")

        def merge(name, branch):
            run("merge", "-q", "--no-ff", "-m", name, branch)
            cls.sha[name] = run("rev-parse", "HEAD")

        def run(*args):
            return git(cls.origin, cls.env, *args)

        run("init", "-q", "-b", "dev")
        commit("R", "0.0.1")
        run("tag", "v0.0.1")
        run("branch", "main")
        commit("A", "0.1.0")
        run("checkout", "-q", "main")
        merge("M1", "dev")
        run("tag", "-a", "v0.1.0", "-m", "v0.1.0")
        run("checkout", "-q", "dev")
        commit("B", "0.1.5")
        run("tag", "-a", "v0.1.5", "-m", "v0.1.5")
        run("checkout", "-q", "-b", "side")
        (cls.origin / "side.txt").write_text("S1\n", encoding="utf-8")
        commit("S1")
        commit("S2", "0.2.0")
        run("checkout", "-q", "dev")
        merge("D", "side")
        run("checkout", "-q", "main")
        merge("M2", "dev")
        for tag in ("v0.2.0", "v0.3.0", "v0.2", "v0.02.0", "v0.2.0-rc.1"):
            run("tag", "-a", tag, "-m", tag)
        run("checkout", "-q", "-b", "topic", "dev")
        commit("T", "0.4.0")
        run("tag", "-a", "v0.4.0", "-m", "v0.4.0")
        run("checkout", "-q", "dev")

    def clone(self, *options, at=None):
        """A clone of the fixture in a fresh directory, detached at `at` when it names a tag."""
        checkout = self.scratch / f"checkout-{uuid.uuid4().hex}"
        git(self.scratch, self.env, "clone", "-q", *options, f"file://{self.origin}", str(checkout))
        if at is not None and not options:
            git(checkout, self.env, "checkout", "-q", "--detach", f"refs/tags/{at}")
        return checkout

    def single(self, version):
        """A one-commit repository on `dev` whose workspace version is `version`, and its commit."""
        root = self.scratch / f"single-{uuid.uuid4().hex}"
        root.mkdir()
        (root / "Cargo.toml").write_text(cargo(version), encoding="utf-8")
        git(root, self.env, "init", "-q", "-b", "dev")
        git(root, self.env, "add", "-A")
        git(root, self.env, "commit", "-q", "-m", "the workspace")
        return root, git(root, self.env, "rev-parse", "HEAD")

    def plan(self, checkout, lane, event, ref, sha):
        """Run the plan step as the runner would, and return the run and the outputs it wrote."""
        output = self.scratch / f"output-{uuid.uuid4().hex}"
        output.touch()
        env = dict(
            self.env,
            GITHUB_EVENT_NAME=event,
            GITHUB_REF=ref,
            GITHUB_REF_TYPE="tag" if ref.startswith("refs/tags/") else "branch",
            GITHUB_REF_NAME=ref.split("/", 2)[2],
            GITHUB_SHA=sha,
            GITHUB_OUTPUT=str(output),
        )
        run = subprocess.run(
            [sys.executable, str(LANE), "plan", "--lane", lane],
            cwd=checkout,
            env=env,
            capture_output=True,
            text=True,
            check=False,
        )
        lines = output.read_text(encoding="utf-8").splitlines()
        return run, dict(line.split("=", 1) for line in lines if line)

    def assertRefused(self, run, outputs, message):
        self.assertIn(message, run.stderr.splitlines())
        self.assertEqual(run.returncode, 1)
        self.assertEqual(outputs, {})

    def test_the_internal_plan_refuses_any_event_or_ref_but_a_dispatch_on_dev(self):
        checkout = self.clone()
        dispatch_only = "the internal lane runs on workflow_dispatch only, not on {}"
        dev_only = "the internal lane builds refs/heads/dev only, not {}"
        refused = [
            ("push", "refs/heads/dev", dispatch_only.format("push")),
            ("pull_request", "refs/pull/1/merge", dispatch_only.format("pull_request")),
            ("workflow_dispatch", "refs/heads/main", dev_only.format("refs/heads/main")),
            ("workflow_dispatch", "refs/tags/v0.2.0", dev_only.format("refs/tags/v0.2.0")),
        ]
        for event, ref, message in examined("refused internal starts", refused):
            with self.subTest(event=event, ref=ref):
                run, outputs = self.plan(checkout, "internal", event, ref, self.sha["D"])
                self.assertRefused(run, outputs, message)
        run, outputs = self.plan(
            checkout, "internal", "workflow_dispatch", "refs/heads/dev", self.sha["D"]
        )
        self.assertEqual(outputs.get("lane"), "internal")
        self.assertEqual(run.returncode, 0)

    def test_the_plan_refuses_a_shallow_checkout(self):
        starts = [
            ("internal", "dev", "workflow_dispatch", "refs/heads/dev", "D"),
            ("release", "v0.2.0", "push", "refs/tags/v0.2.0", "M2"),
        ]
        for lane, branch, event, ref, commit in examined("shallow checkouts", starts):
            with self.subTest(lane=lane):
                shallow = self.clone("--depth", "1", "--branch", branch)
                self.assertEqual(
                    git(shallow, self.env, "rev-parse", "--is-shallow-repository"), "true"
                )
                run, outputs = self.plan(shallow, lane, event, ref, self.sha[commit])
                self.assertRefused(run, outputs, SHALLOW)

    def test_the_internal_build_number_is_devs_first_parent_count(self):
        checkout = self.clone()
        run, outputs = self.plan(
            checkout, "internal", "workflow_dispatch", "refs/heads/dev", self.sha["D"]
        )
        # dev holds six commits, four of them on its first-parent chain: R, A, B and D.
        self.assertEqual(outputs.get("number"), "4")
        self.assertEqual(outputs, {"lane": "internal", "number": "4", "version": "0.2.0"})
        self.assertEqual(run.returncode, 0)
        self.assertEqual(git(checkout, self.env, "rev-list", "--count", "HEAD"), "6")

    def test_the_marketing_version_is_the_workspace_version_in_three_integers(self):
        root, sha = self.single("3.14.15")
        run, outputs = self.plan(root, "internal", "workflow_dispatch", "refs/heads/dev", sha)
        self.assertEqual(outputs.get("version"), "3.14.15")
        self.assertEqual(outputs, {"lane": "internal", "number": "1", "version": "3.14.15"})
        self.assertEqual(run.returncode, 0)
        for version in examined("refused workspace versions", ["1.2", "1.2.3-rc.1"]):
            with self.subTest(version=version):
                root, sha = self.single(version)
                run, outputs = self.plan(
                    root, "internal", "workflow_dispatch", "refs/heads/dev", sha
                )
                self.assertRefused(
                    run,
                    outputs,
                    f"the workspace version {version} is not three dot-separated integers",
                )

    def test_the_release_plan_admits_only_an_annotated_semver_tag_on_mains_first_parent_chain(
        self,
    ):
        tagged = {"v0.0.1": "R", "v0.1.5": "B", "v0.4.0": "T"}
        refused = [
            ("v0.0.1", "the tag v0.0.1 is a lightweight tag; a release is an annotated tag"),
            ("v0.2", "the tag v0.2 is not a SemVer release tag"),
            ("v0.02.0", "the tag v0.02.0 is not a SemVer release tag"),
            ("v0.2.0-rc.1", "the tag v0.2.0-rc.1 is not a SemVer release tag"),
            ("v0.4.0", "the commit of v0.4.0 is not on main"),
            ("v0.1.5", "the commit of v0.1.5 is not on main's first-parent chain"),
            ("v0.3.0", "the tag v0.3.0 names 0.3.0, but the workspace version is 0.2.0"),
        ]
        for tag, message in examined("refused release tags", refused):
            with self.subTest(tag=tag):
                checkout = self.clone(at=tag)
                sha = self.sha[tagged.get(tag, "M2")]
                run, outputs = self.plan(checkout, "release", "push", f"refs/tags/{tag}", sha)
                self.assertRefused(run, outputs, message)
        tag_push_only = "the release lane runs on a tag push only, not on {} of {}"
        starts = [("push", "refs/heads/main"), ("workflow_dispatch", "refs/tags/v0.2.0")]
        for event, ref in examined("refused release starts", starts):
            with self.subTest(event=event, ref=ref):
                checkout = self.clone(at="v0.2.0")
                run, outputs = self.plan(checkout, "release", event, ref, self.sha["M2"])
                self.assertRefused(run, outputs, tag_push_only.format(event, ref))
        checkout = self.clone(at="v0.2.0")
        run, outputs = self.plan(checkout, "release", "push", "refs/tags/v0.2.0", self.sha["M2"])
        self.assertEqual(outputs, {"lane": "release", "number": "3", "version": "0.2.0"})
        self.assertEqual(run.returncode, 0)

    def test_the_release_build_number_is_mains_first_parent_count_at_the_tag(self):
        # main's first-parent chain is R, M1, M2: two commits at v0.1.0 and three at v0.2.0, where
        # M1 reaches three commits and M2 eight.
        releases = [("v0.1.0", "M1", "2", "0.1.0"), ("v0.2.0", "M2", "3", "0.2.0")]
        for tag, commit, number, version in examined("release tags", releases):
            with self.subTest(tag=tag):
                checkout = self.clone(at=tag)
                run, outputs = self.plan(
                    checkout, "release", "push", f"refs/tags/{tag}", self.sha[commit]
                )
                self.assertEqual(outputs.get("number"), number)
                self.assertEqual(outputs, {"lane": "release", "number": number, "version": version})
                self.assertEqual(run.returncode, 0)


if __name__ == "__main__":
    unittest.main()
