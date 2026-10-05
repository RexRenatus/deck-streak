"""The release workflow builds a tagged release once, attests it and publishes it last (SPEC-062 R1,
R2; ADR-062, ADR-017, ADR-034). It runs on a push of a SemVer tag only, proves the tag's commit is
on `main` on a full-history checkout, creates a draft, attaches one tarball with its digests and a
build-provenance attestation, and publishes after the last upload (A8). Its token is read-only
except in its release job, every action is pinned, and no step saves a cache (A9). It builds the
sync server from the fork and the commit the engine's patch entry pins, and ships it in the same
tarball (SPEC-337 A1, A2), once a job before it has audited that server's dependency graph (SPEC-340
A9)."""

import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_ci_workflows import PINNED, action, entries, read_hardened, workflow_file_text

RELEASE = REPO / ".github" / "workflows" / "release.yml"
CI = REPO / ".github" / "workflows" / "ci.yml"
JOB_WRITES = {"contents": "write", "id-token": "write", "attestations": "write"}
ATTEST = "actions/attest-build-provenance"
UPSTREAM = "https://github.com/ankitects/anki.git"
# The job that audits the sync server's dependency graph before the release builds it (SPEC-340 R7;
# ADR-351 D5), and the script it runs.
AUDIT_JOB = "audit-sync-server"
AUDIT_SCRIPT = "bash scripts/audit-sync-server.sh"
INSTALL = "taiki-e/install-action"


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
    """The release job's steps. The workflow's other job, the sync server's audit, builds nothing
    and publishes nothing (SPEC-340 R7)."""
    jobs = workflow["jobs"]
    assert sorted(jobs) == sorted([AUDIT_JOB, "release"]), f"the release runs {sorted(jobs)}"
    return jobs["release"]["steps"]


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
        job = workflow["jobs"]["release"]
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
        job = workflow["jobs"]["release"]
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
        text = workflow_file_text(RELEASE)
        secrets = re.findall(r"secrets\.([A-Za-z_]+)", text)
        self.assertEqual(sorted(set(secrets) - {"GITHUB_TOKEN"}), [], "a secret but the token")
        self.assertNotRegex(text, r"(?m)^\s*(pull_request|workflow_run|schedule)")
        self.assertNotIn("actions/cache/save", text)
        attested = [s for s in steps if action(s) == ATTEST]
        self.assertEqual(len(attested), 1, "one attestation step")

    def test_the_token_reaches_the_three_gh_release_steps_alone(self):
        workflow = read_release()
        job = workflow["jobs"]["release"]
        self.assertNotIn("GH_TOKEN", workflow.get("env") or {}, "the workflow env holds it")
        self.assertNotIn("GH_TOKEN", job.get("env") or {}, "the job env holds it")
        steps = steps_of(workflow)
        holders = [str(s.get("run", "")) for s in steps if "GH_TOKEN" in (s.get("env") or {})]
        releases = [str(s.get("run", "")) for s in steps]
        releases = [r for r in releases if r.startswith("gh release ")]
        self.assertEqual(len(releases), 3, releases)
        self.assertEqual(holders, releases, "only the gh release steps hold the token")
        text = workflow_file_text(RELEASE)
        self.assertEqual(text.count("github.token"), 3, "the token is named three times")


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


class TheReleaseBuildsTheSyncServer(unittest.TestCase):
    """SPEC-337 R1 (ADR-347 D1): the release builds the sync server from the fork and the commit
    the engine's patch entry pins, after the tag guard and the protobuf compiler, and ships it in
    the tarball that the manifest, the digests and the attestation cover."""

    def test_the_sync_server_is_built_after_the_guard_and_shipped_in_the_tarball(self):
        steps = steps_of(read_release())
        ancestor = index_of(steps, "merge-base --is-ancestor")
        protoc = index_of(steps, "protoc", key="name")
        server = index_of(steps, "cargo install")
        tarball = index_of(steps, "MANIFEST.sha256")
        create = index_of(steps, "gh release create")
        self.assertLess(ancestor, server, "the tag is proved on main before the server is built")
        self.assertLess(protoc, server, "the engine needs the protobuf compiler")
        self.assertLess(server, tarball, "the server is built before the tarball is assembled")
        self.assertLess(tarball, create, "the tarball is assembled before the draft")
        run = str(steps[server]["run"])
        self.assertIn("--locked", run, "the fork's own lockfile builds the server")
        self.assertIn('"$RUNNER_TEMP/sync-server"', run, "the server is installed outside the tree")
        self.assertNotRegex(run, r"[0-9a-f]{40}", "the step holds no second copy of the commit")
        packed = str(steps[tarball]["run"])
        line = (
            'install -m 0755 "$RUNNER_TEMP/sync-server/bin/anki-sync-server" '
            '"$stage/bin/anki-sync-server"'
        )
        self.assertIn(line, packed, "the tarball carries the server as bin/anki-sync-server")
        self.assertLess(
            packed.index(line),
            packed.index("MANIFEST.sha256"),
            "the manifest is written after the server is staged, so its digests cover it",
        )

    def test_the_build_step_reads_the_fork_and_the_commit_from_the_engines_patch_entry(self):
        steps = steps_of(read_release())
        run = str(steps[index_of(steps, "cargo install")]["run"])
        fork = "https://example.org/planted/anki.git"
        rev = "0123456789abcdef0123456789abcdef01234567"
        patch = f'[patch."{UPSTREAM}"]\nanki = {{ git = "{fork}", rev = "%s" }}\n'
        manifests = {
            "pinned": patch % rev,
            "unpatched": "[workspace]\nmembers = []\n",
            "a branch": patch % "main",
            "a short rev": patch % rev[:12],
        }
        with tempfile.TemporaryDirectory() as tmp:
            tmp = Path(tmp)
            stub = tmp / "bin" / "cargo"
            stub.parent.mkdir()
            stub.write_text('#!/bin/sh\necho "cargo $*"\n', encoding="utf-8")
            stub.chmod(0o755)
            script = tmp / "build.sh"
            script.write_text(run, encoding="utf-8")
            roots = {"the tree": REPO}
            for name, manifest in manifests.items():
                roots[name] = tmp / name.replace(" ", "-")
                roots[name].mkdir()
                (roots[name] / "Cargo.toml").write_text(manifest, encoding="utf-8")
            runner = tmp / "runner"
            path = os.pathsep.join([str(stub.parent), os.environ["PATH"]])
            outcomes = {}
            for name in examined("manifest roots", roots):
                done = subprocess.run(
                    ["bash", "-e", str(script)],
                    cwd=roots[name],
                    env={**os.environ, "PATH": path, "RUNNER_TEMP": str(runner)},
                    capture_output=True,
                    text=True,
                )
                outcomes[name] = (done.returncode, done.stdout.strip())
        installed = f"--root {runner}/sync-server anki-sync-server"
        self.assertEqual(
            outcomes["pinned"],
            (0, f"cargo install --locked --git {fork} --rev {rev} {installed}"),
            "the step passes the patch entry's fork and commit to cargo",
        )
        for name in ("unpatched", "a branch", "a short rev"):
            code, out = outcomes[name]
            self.assertNotEqual(code, 0, f"{name}: the step refuses it")
            self.assertEqual(out, "", f"{name}: refused before cargo runs")
        code, out = outcomes["the tree"]
        self.assertEqual(code, 0, f"the tree's own manifest: {out}")
        self.assertRegex(
            out,
            rf"^cargo install --locked --git https://\S+ --rev [0-9a-f]{{40}} {re.escape(installed)}$",
        )
        self.assertNotIn(UPSTREAM, out, "the tree's server is the fork's, not upstream's")

    def test_the_release_waits_on_the_server_audit(self):
        """SPEC-340 A9 (R7; ADR-351 D5): a job with read-only permissions audits the sync
        server's dependency graph before the release job builds it, and the release job needs that
        job; the release job's own permissions are unchanged."""
        jobs = read_release()["jobs"]
        self.assertEqual(jobs["release"].get("needs"), [AUDIT_JOB], "the release waits on it")
        audit = jobs[AUDIT_JOB]
        self.assertEqual(audit["permissions"], {"contents": "read"})
        self.assertEqual(jobs["release"]["permissions"], JOB_WRITES)
        self.assertEqual(audit["runs-on"], "ubuntu-24.04")
        steps = audit["steps"]
        # The release job's own pinned checkout, and ci.yml's pinned installer for cargo-deny.
        checkout = [s for s in steps if action(s) == "actions/checkout"]
        released = [s for s in jobs["release"]["steps"] if action(s) == "actions/checkout"]
        self.assertEqual([s["uses"] for s in checkout], [released[0]["uses"]])
        install = [s for s in steps if action(s) == INSTALL]
        pinned = {
            ref for ref in entries(read_hardened(CI), "uses") if ref.startswith(f"{INSTALL}@")
        }
        self.assertEqual(len(pinned), 1, pinned)
        self.assertEqual([s["uses"] for s in install], sorted(pinned))
        self.assertEqual(install[0]["with"]["tool"], "cargo-deny")
        audited = index_of(steps, AUDIT_SCRIPT)
        self.assertLess(steps.index(checkout[0]), audited)
        self.assertLess(steps.index(install[0]), audited)


if __name__ == "__main__":
    unittest.main()
