"""The bench plan (SPEC-618, ADR-640): A1 to A7.

Each test drives `scripts/perf-plan.py` as a child process over planted files in a temporary
directory: a changed-path list, a cargo metadata document and the two output files. The git arm
reads this repository's own history (an empty diff, and the diff from the empty tree), so no test
makes a repository, runs cargo or reaches GitHub.
"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "perf-plan.py"
METADATA = REPO / "scripts" / "tests" / "fixtures" / "perf" / "metadata.json"
OWN = "RexRenatus/deck-streak"
FORKED = "someone/deck-streak"
EMPTY_TREE = "4b825dc642cb6eb9a060e54bf8d69288fbee4904"
PUSH = "a push to dev or main measures the baseline"
FORK = "fork PR (GitHub mints no OIDC token for a fork; the push to dev measures the merged change)"
REACHING = {
    "rust": ["crates/core/src/queue.rs"],
    "web": ["web/app/src/lib/card/frame-document.ts"],
}


def run(*args):
    return subprocess.run(
        [sys.executable, str(SCRIPT), *args],
        capture_output=True,
        text=True,
        check=False,
    )


def plan(lane, changed, head_repo=OWN):
    """A pull request run of one lane over a planted changed-path list."""
    with tempfile.TemporaryDirectory() as tmp:
        listing = Path(tmp) / "changed.txt"
        listing.write_text("".join(path + "\n" for path in changed), encoding="utf-8")
        args = ["--lane", lane, "--event", "pull_request", "--repo", OWN, "--head-repo", head_repo]
        args += ["--changed", str(listing), "--metadata", str(METADATA)]
        return run(*args)


def outputs(done):
    """The key=value lines a run printed, by key."""
    pairs = [line.split("=", 1) for line in done.stdout.splitlines()]
    return {pair[0]: pair[1] for pair in pairs if len(pair) == 2 and "bench-" not in pair[0]}


def expected(lane, applicable, upload, reason):
    flags = {True: "true", False: "false"}
    said = "applicable" if upload else "not applicable"
    return (
        f"applicable={flags[applicable]}\nupload={flags[upload]}\nreason={reason}\n"
        f"bench-{lane}: {said}: {reason}\n"
    )


class TheForkArm(unittest.TestCase):
    def test_a_fork_pull_request_builds_but_never_uploads(self):
        for lane, changed in examined("fork lanes", REACHING.items()):
            done = plan(lane, changed, head_repo=FORKED)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertEqual(outputs(done)["upload"], "false", done.stdout)
            self.assertEqual(done.stdout, expected(lane, True, False, FORK))
        unreached = plan("web", ["docs/README.md"], head_repo=FORKED)
        self.assertEqual(unreached.stdout, expected("web", False, False, FORK))
        deleted = plan("rust", ["Cargo.lock"], head_repo="")
        self.assertEqual(deleted.stdout, expected("rust", True, False, FORK))


class TheControls(unittest.TestCase):
    def test_a_same_repository_pull_request_uploads_what_it_builds(self):
        for lane, changed in examined("same-repository lanes", REACHING.items()):
            done = plan(lane, changed)
            self.assertEqual(done.returncode, 0, done.stderr)
            self.assertEqual(outputs(done)["applicable"], "true", done.stdout)
            self.assertEqual(outputs(done)["upload"], "true", done.stdout)

    def test_a_push_always_applies_and_uploads(self):
        for lane in examined("lanes", ("rust", "web")):
            for head_repo in ("", FORKED, OWN):
                done = run(
                    "--lane", lane, "--event", "push", "--repo", OWN, "--head-repo", head_repo
                )
                self.assertEqual(done.returncode, 0, done.stderr)
                self.assertEqual(outputs(done)["applicable"], "true", done.stdout)
                self.assertEqual(outputs(done)["upload"], "true", done.stdout)


class TheLanes(unittest.TestCase):
    def assert_lane(self, lane, reached, unreached):
        none = f"the diff reaches no {lane} bench (examined 1 changed paths)"
        for path in examined(f"{lane} paths that reach no bench", unreached):
            done = plan(lane, [path])
            self.assertEqual(outputs(done)["applicable"], "false", path + ": " + done.stdout)
            self.assertEqual(done.stdout, expected(lane, False, False, none))
        for path, why in examined(f"{lane} paths that reach a bench", reached.items()):
            done = plan(lane, ["docs/README.md", path])
            self.assertEqual(done.returncode, 0, done.stderr)
            reason = f"the diff changes {path}, {why}"
            self.assertEqual(done.stdout, expected(lane, True, True, reason))
        first, second = list(reached)[-1], list(reached)[0]
        done = plan(lane, [*unreached, first, second])
        reason = f"the diff changes {first}, {reached[first]}"
        self.assertEqual(done.stdout, expected(lane, True, True, reason))

    def test_the_rust_lane_applies_only_where_the_diff_reaches_a_rust_bench(self):
        closure = "a Rust file in the bench closure"
        reached = {
            "Cargo.lock": "a Cargo manifest or the lockfile",
            "crates/bot/Cargo.toml": "a Cargo manifest or the lockfile",
            "rust-toolchain.toml": "the toolchain pin",
            ".github/workflows/ci.yml": "the ci workflow",
            "crates/ffi/benches/grading.rs": "a bench file",
            "crates/ffi/benches/inputs/cards.json": "a bench file",
            "benches/startup.rs": "a bench file",
            "crates/ffi/benches.rs": closure + " (ffi)",
            "crates/core/src/queue.rs": closure + " (core)",
            "crates/fsrs/src/replay.rs": closure + " (fsrs)",
            "crates/helper/src/lib.rs": closure + " (helper)",
            "src/lib.rs": closure + " (app)",
        }
        unreached = [
            "crates/bot/src/lib.rs",
            "crates/ffi/inner/src/lib.rs",
            "crates/core/README.md",
            "crates/core/Cargo.toml.orig",
            "tools/rust-toolchain.toml",
            ".github/workflows/release.yml",
            "web/app/src/lib/card/frame-document.ts",
            "docs/README.md",
        ]
        self.assert_lane("rust", reached, unreached)

    def test_the_web_lane_applies_only_where_the_diff_reaches_a_web_bench(self):
        manifest = "a web manifest or the lockfile"
        reached = {
            "web/app/src/lib/card/frame-document.bench.ts": "a bench file",
            "web/bench/session.bench.ts": "a bench file",
            "vitest.bench.config.ts": "the bench config",
            "web/app/package.json": manifest,
            "package.json": manifest,
            "pnpm-lock.yaml": manifest,
            "pnpm-workspace.yaml": manifest,
            "web/app/src/lib/study/review-templates.golden.json": "web app source",
            "web/app/src/lib/card/frame-document.ts": "web app source",
        }
        unreached = [
            "web/app/src/lib/study/review.test.ts",
            "web/app/src/lib/card/frame-host.spec.ts",
            "web/app/static/icon.svg",
            "web/app/vite.config.ts",
            "web/other/package.json",
            "vitest.config.ts",
            "crates/core/src/queue.rs",
            "docs/README.md",
        ]
        self.assert_lane("web", reached, unreached)


class TheOutputs(unittest.TestCase):
    def test_the_reason_is_printed_and_written_as_job_outputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp) / "output"
            summary = Path(tmp) / "summary"
            changed = Path(tmp) / "changed.txt"
            out.write_text("earlier=kept\n", encoding="utf-8")
            summary.write_text("# earlier\n", encoding="utf-8")
            changed.write_text("docs/README.md\n", encoding="utf-8")
            files = ["--github-output", str(out), "--step-summary", str(summary)]
            none = "the diff reaches no web bench (examined 1 changed paths)"
            done = run(
                "--lane",
                "web",
                "--event",
                "pull_request",
                "--repo",
                OWN,
                "--head-repo",
                OWN,
                "--changed",
                str(changed),
                *files,
            )
            first = f"applicable=false\nupload=false\nreason={none}\n"
            self.assertEqual(out.read_text(encoding="utf-8"), "earlier=kept\n" + first)
            self.assertEqual(
                summary.read_text(encoding="utf-8"),
                f"# earlier\nbench-web: not applicable: {none}\n",
            )
            self.assertEqual(done.stdout, expected("web", False, False, none))
            done = run(
                "--lane", "rust", "--event", "push", "--repo", OWN, "--head-repo", "", *files
            )
            self.assertEqual(done.stdout, expected("rust", True, True, PUSH))
            self.assertEqual(
                out.read_text(encoding="utf-8"),
                f"earlier=kept\n{first}applicable=true\nupload=true\nreason={PUSH}\n",
            )
            self.assertEqual(
                summary.read_text(encoding="utf-8"),
                f"# earlier\nbench-web: not applicable: {none}\nbench-rust: applicable: {PUSH}\n",
            )


class TheChangedPaths(unittest.TestCase):
    def test_the_changed_paths_come_from_git_between_base_and_head(self):
        pull = ["--lane", "web", "--event", "pull_request", "--repo", OWN, "--head-repo", OWN]
        done = run(*pull, "--base", "HEAD", "--head", "HEAD")
        self.assertEqual(outputs(done)["applicable"], "false", done.stdout + done.stderr)
        none = "the diff reaches no web bench (examined 0 changed paths)"
        self.assertEqual(done.stdout, expected("web", False, False, none))
        self.assertEqual(done.returncode, 0, done.stderr)
        done = run(*pull, "--base", EMPTY_TREE, "--head", "HEAD")
        self.assertEqual(done.returncode, 0, done.stderr)
        self.assertEqual(outputs(done)["applicable"], "true", done.stdout)
        self.assertEqual(outputs(done)["upload"], "true", done.stdout)
        self.assertTrue(outputs(done)["reason"].startswith("the diff changes "), done.stdout)
        done = run(*pull, "--base", "no-such-revision", "--head", "HEAD")
        self.assertEqual(done.returncode, 1, done.stdout)
        self.assertEqual(
            done.stdout, "bench-web: VOID git diff --name-only no-such-revision HEAD exited 128\n"
        )


class TheRefusals(unittest.TestCase):
    def test_an_input_the_plan_cannot_read_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            changed = Path(tmp) / "changed.txt"
            changed.write_text("crates/core/src/queue.rs\n", encoding="utf-8")
            absent = str(Path(tmp) / "absent")
            planted = {
                "broken": "{",
                "listed": "[]",
                "nameless": json.dumps(
                    {"workspace_root": "/w", "packages": [{"manifest_path": "/w/Cargo.toml"}]}
                ),
            }
            for name, text in planted.items():
                (Path(tmp) / name).write_text(text, encoding="utf-8")
            pull = ["--event", "pull_request", "--repo", OWN, "--head-repo", OWN]
            rust = ["--lane", "rust", *pull, "--changed", str(changed)]
            web = ["--lane", "web", *pull]
            needs = "bench-web: VOID a pull request needs --changed, or --base and --head"
            metadata = "bench-rust: VOID cannot read the cargo metadata at "
            cases = [
                (
                    ["--lane", "rust", "--event", "push", "--repo", "", "--head-repo", ""],
                    "bench-rust: VOID --repo names no repository",
                ),
                (
                    ["--lane", "web", "--event", "merge_group", "--repo", OWN, "--head-repo", OWN],
                    "bench-web: VOID an event the plan does not read: merge_group",
                ),
                (web, needs),
                ([*web, "--base", "HEAD"], needs),
                ([*web, "--head", "HEAD"], needs),
                (
                    [*web, "--changed", absent],
                    "bench-web: VOID cannot read the changed paths at " + absent,
                ),
                (rust, "bench-rust: VOID the rust lane needs --metadata on a pull request"),
                ([*rust, "--metadata", absent], metadata + absent),
            ]
            for name in planted:
                path = str(Path(tmp) / name)
                cases.append(([*rust, "--metadata", path], metadata + path))
            for args, line in examined("refused inputs", cases):
                done = run(*args)
                self.assertEqual(done.returncode, 1, line + ": " + done.stdout + done.stderr)
                self.assertEqual(done.stdout, line + "\n")
        full = ["--lane", "web", "--event", "push", "--repo", OWN, "--head-repo", OWN]
        for at in examined("required options", range(0, len(full), 2)):
            done = run(*full[:at], *full[at + 2 :])
            self.assertEqual(done.returncode, 2, full[at] + ": " + done.stdout)
        self.assertEqual(run("--lane", "go", *full[2:]).returncode, 2)
