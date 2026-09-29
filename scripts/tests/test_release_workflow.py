"""The release workflow builds a tagged release once, attests it and publishes it last (SPEC-062 R1,
R2; ADR-062, ADR-017, ADR-034). It runs on a push of a SemVer tag only, proves the tag's commit is
on `main` on a full-history checkout, creates a draft, attaches one tarball with its digests and a
build-provenance attestation, and publishes after the last upload (A8). Its token is read-only
except in its one job, every action is pinned, and no step saves a cache (A9)."""

import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_ci_workflows import PINNED, action, entries, read_hardened

RELEASE = REPO / ".github" / "workflows" / "release.yml"
JOB_WRITES = {"contents": "write", "id-token": "write", "attestations": "write"}
ATTEST = "actions/attest-build-provenance"


def tag_glob(pattern):
    """A workflow tag filter as GitHub matches it: `[..]` a class, `+` one or more of the atom
    before it, `*` any run but a slash, every other character itself."""
    out, at = "", 0
    while at < len(pattern):
        char = pattern[at]
        if char == "[":
            end = pattern.index("]", at)
            out += pattern[at : end + 1]
            at = end
        elif char in "+":
            out += "+"
        elif char == "*":
            out += "[^/]*"
        else:
            out += re.escape(char)
        at += 1
    return re.compile(out)


def read_release():
    assert RELEASE.is_file(), f"{RELEASE.relative_to(REPO)} does not exist"
    return read_hardened(RELEASE)


def steps_of(workflow):
    jobs = workflow["jobs"]
    assert len(jobs) == 1, f"the release runs {len(jobs)} jobs, not one"
    return list(jobs.values())[0]["steps"]


def index_of(steps, needle, key="run"):
    for at, step in enumerate(steps):
        if needle in str(step.get(key, "")):
            return at
    raise AssertionError(f"no step has `{needle}` in its {key}")


class TheReleaseRunsOnSemverTags(unittest.TestCase):
    def test_the_release_runs_on_semver_tags_and_publishes_last(self):
        workflow = read_release()
        triggers = workflow["on"]
        self.assertEqual(list(triggers), ["push"], "the release runs on a push, and only on one")
        self.assertEqual(list(triggers["push"]), ["tags"], "the push filter is tags alone")
        globs = [tag_glob(pattern) for pattern in triggers["push"]["tags"]]
        admitted = ["v1.2.3", "v0.0.1", "v10.20.30"]
        refused = ["v1.2", "v1.2.3.4", "latest", "1.2.3", "v1.2.3-rc1", "v1.x.3"]
        for name in examined("tag names", admitted + refused):
            matched = any(glob.fullmatch(name) for glob in globs)
            self.assertEqual(matched, name in admitted, f"the tag filter and `{name}`")
        job = list(workflow["jobs"].values())[0]
        steps = steps_of(workflow)
        checkout = next(s for s in steps if action(s) == "actions/checkout")
        self.assertEqual(checkout["with"]["fetch-depth"], "0", "a full-history checkout")
        ancestor = index_of(steps, "merge-base --is-ancestor")
        self.assertIn("origin/main", steps[ancestor]["run"])
        self.assertIn(
            'merge-base --is-ancestor "$GITHUB_SHA" origin/main',
            steps[ancestor]["run"],
            "the commit the tag names is the one proved to be on main",
        )
        self.assertLess(steps.index(checkout), ancestor)
        build = index_of(steps, "cargo build")
        self.assertIn("--release", steps[build]["run"])
        self.assertIn("deckstreakd", steps[build]["run"])
        web = index_of(steps, "pnpm")
        self.assertIn("build", "".join(str(s.get("run", "")) for s in steps[web:]))
        self.assertLess(ancestor, build, "the tag is proved on main before anything is built")
        create = index_of(steps, "gh release create")
        self.assertIn("--draft", steps[create]["run"])
        attest = next(at for at, s in enumerate(steps) if action(s) == ATTEST)
        upload = index_of(steps, "gh release upload")
        publish = index_of(steps, "--draft=false")
        self.assertEqual(publish, len(steps) - 1, "the publish is the last step")
        self.assertLess(create, upload)
        self.assertLess(attest, upload)
        self.assertLess(upload, publish)
        self.assertLess(build, create)
        self.assertLess(index_of(steps, "SHA256SUMS"), upload)
        text = "\n".join(str(s.get("run", "")) for s in steps)
        for member in ("deckstreakd", "deploy", "agent", "ai-safety.json"):
            self.assertIn(member, text, f"the tarball holds {member}")
        self.assertNotIn("probe", text, "the tarball holds no pack probe")
        self.assertRegex(text, r"GITHUB_STEP_SUMMARY", "the unpacked size is reported")
        self.assertRegex(text, r"\bdu\b|\bwc\b|stat ", "the unpacked size is measured")
        self.assertIn(".tar.gz", steps[attest]["with"]["subject-path"])
        self.assertEqual(job["runs-on"], "ubuntu-24.04")


class TheReleaseWorkflowIsHardened(unittest.TestCase):
    def test_the_release_workflow_is_read_only_and_pinned(self):
        workflow = read_release()
        self.assertEqual(workflow["permissions"], {"contents": "read"})
        job = list(workflow["jobs"].values())[0]
        self.assertEqual(job["permissions"], JOB_WRITES)
        self.assertEqual(job["runs-on"], "ubuntu-24.04")
        refs = examined("action references", entries(workflow, "uses"))
        for ref in refs:
            self.assertRegex(ref, PINNED, f"release.yml uses {ref}")
        steps = steps_of(workflow)
        for step in steps:
            self.assertNotEqual(action(step), "actions/cache", "a step that saves a cache")
            self.assertNotIn("cache", step.get("with", {}), f"{action(step)} caches by itself")
            if action(step) == "actions/setup-node":
                self.assertEqual(step["with"].get("package-manager-cache"), "false")
            self.assertNotIn("rust-cache", action(step))
        text = RELEASE.read_text(encoding="utf-8")
        secrets = re.findall(r"secrets\.([A-Za-z_]+)", text)
        self.assertEqual(sorted(set(secrets) - {"GITHUB_TOKEN"}), [], "a secret but the token")
        self.assertNotRegex(text, r"(?m)^\s*(pull_request|workflow_run|schedule)")
        self.assertNotIn("actions/cache/save", text)
        attested = [s for s in steps if action(s) == ATTEST]
        self.assertEqual(len(attested), 1, "one attestation step")


def git(*args, cwd, env):
    done = subprocess.run(["git", *args], cwd=cwd, env=env, capture_output=True, text=True)
    assert done.returncode == 0, f"git {args}: {done.stderr}"
    return done.stdout.strip()


class TheTagGuardRuns(unittest.TestCase):
    def test_the_release_refuses_a_tag_off_main_or_lightweight_by_running_its_guard(self):
        steps = steps_of(read_release())
        guard = steps[index_of(steps, "merge-base --is-ancestor")]["run"]
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            env = {
                **os.environ,
                "GIT_CONFIG_GLOBAL": "/dev/null",
                "GIT_CONFIG_SYSTEM": "/dev/null",
                "GIT_AUTHOR_NAME": "t",
                "GIT_AUTHOR_EMAIL": "t@example.org",
                "GIT_COMMITTER_NAME": "t",
                "GIT_COMMITTER_EMAIL": "t@example.org",
            }
            origin, work = tmp / "origin.git", tmp / "work"
            git("init", "-q", "--bare", "-b", "main", str(origin), cwd=tmp, env=env)
            git("clone", "-q", str(origin), str(work), cwd=tmp, env=env)
            git("checkout", "-q", "-b", "main", cwd=work, env=env)
            git("commit", "-q", "--allow-empty", "-m", "on main", cwd=work, env=env)
            git("tag", "-a", "-m", "v1.0.0", "v1.0.0", cwd=work, env=env)
            git("tag", "v1.1.0", cwd=work, env=env)
            git("push", "-q", "origin", "main", "v1.0.0", "v1.1.0", cwd=work, env=env)
            git("checkout", "-q", "-b", "topic", cwd=work, env=env)
            git("commit", "-q", "--allow-empty", "-m", "off main", cwd=work, env=env)
            git("tag", "-a", "-m", "v2.0.0", "v2.0.0", cwd=work, env=env)
            git("push", "-q", "origin", "topic", "v2.0.0", cwd=work, env=env)
            script = tmp / "guard.sh"
            script.write_text(guard, encoding="utf-8")
            verdicts = {}
            for tag in ("v1.0.0", "v1.1.0", "v2.0.0"):
                sha = git("rev-parse", f"{tag}^{{commit}}", cwd=work, env=env)
                done = subprocess.run(
                    ["bash", "-e", str(script)],
                    cwd=work,
                    env={**env, "GITHUB_REF_NAME": tag, "GITHUB_SHA": sha},
                    capture_output=True,
                    text=True,
                )
                verdicts[tag] = done.returncode
            self.assertEqual(sorted(verdicts), ["v1.0.0", "v1.1.0", "v2.0.0"])
            self.assertEqual(verdicts["v1.0.0"], 0, f"an annotated tag on main: {verdicts}")
            self.assertNotEqual(verdicts["v1.1.0"], 0, f"a lightweight tag: {verdicts}")
            self.assertNotEqual(verdicts["v2.0.0"], 0, f"a tag off main: {verdicts}")


if __name__ == "__main__":
    unittest.main()
