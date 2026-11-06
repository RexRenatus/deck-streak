"""The release job keeps a release-profile dependency cache of its own (SPEC-631 A1).

The `release` job runs the workspace's tests in the release profile, and the debug key it used to
restore holds `target/debug`, which that profile never reads. These tests read `ci.yml` as text,
the `release` job's steps only, and check the job restores a `rust-release-` key under the id
`release-cache` and no other Rust cache key, cleans with `cargo clean --workspace --release` and
saves that primary key under the `rust` job's push-to-dev-or-main-on-a-miss condition, and prints
`df -h /` before the restore and after the stage. Nothing here runs GitHub: the live proof is the
first pull request run after the saving push, named in the SPEC.
"""

import re
import unittest

from _support import REPO, examined

CI = REPO / ".github" / "workflows" / "ci.yml"
JOB = "release"
RUST_CACHE = ["~/.cargo/registry/index/", "~/.cargo/registry/cache/", "~/.cargo/git/db/", "target/"]
KEY = (
    "rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}"
    "-${{ hashFiles('Cargo.lock') }}"
)
FALLBACK = "rust-release-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml') }}-"
PRIMARY = "${{ steps.release-cache.outputs.cache-primary-key }}"
ON_A_MISS = (
    "${{ github.event_name == 'push' && (github.ref == 'refs/heads/dev' || "
    "github.ref == 'refs/heads/main') && steps.release-cache.outputs.cache-hit != 'true' }}"
)
STEP_START = re.compile(r"^      - ")
JOB_START = re.compile(r"^  [A-Za-z0-9_-]+:\s*$")


def job_steps(name):
    """The steps of one ci.yml job, each as its lines with the leading `- ` turned into spaces, so
    every key of a step sits at eight spaces whatever line it opens on."""
    lines = CI.read_bytes().decode("utf-8").splitlines()
    if f"  {name}:" not in lines:
        raise AssertionError(f"ci.yml has no job {name}")
    body = []
    for line in lines[lines.index(f"  {name}:") + 1 :]:
        if JOB_START.match(line):
            break
        body.append(line)
    steps = []
    for line in body[body.index("    steps:") + 1 :]:
        if STEP_START.match(line):
            steps.append([line.replace("      - ", "        ", 1)])
        elif steps:
            steps[-1].append(line)
    return examined(f"steps of the {name} job", steps)


def field(step, key, indent=8):
    """A step's one-line value for `key`, or '' when the step has none."""
    for line in step:
        match = re.match(rf"^ {{{indent}}}{re.escape(key)}: (.*)$", line)
        if match:
            return match.group(1).strip()
    return ""


def block(step, key, indent):
    """The stripped non-blank lines of the block scalar `key: |` at `indent`."""
    out = []
    taking = False
    for line in step:
        if taking:
            if line.strip() and len(line) - len(line.lstrip()) <= indent:
                break
            if line.strip():
                out.append(line.strip())
        elif re.match(rf"^ {{{indent}}}{re.escape(key)}: [|>]", line):
            taking = True
    return out


def run_of(step):
    one = field(step, "run")
    return one if one not in ("|", "|-") else "\n".join(block(step, "run", 8))


def uses_of(step):
    return field(step, "uses").split("@", 1)[0]


class TheReleaseJobCache(unittest.TestCase):
    def test_the_release_job_restores_and_saves_its_own_profile_key(self):
        steps = job_steps(JOB)
        restores = [at for at, s in enumerate(steps) if uses_of(s) == "actions/cache/restore"]
        self.assertEqual(len(restores), 1, f"the release job restores {len(restores)} caches")
        restore = steps[restores[0]]
        self.assertEqual(
            field(restore, "key", 10),
            KEY,
            "the release job restores a key other than the release-profile key",
        )
        self.assertEqual(field(restore, "id"), "release-cache", "the restore's id")
        self.assertEqual(block(restore, "path", 10), RUST_CACHE, "the restore's paths")
        self.assertEqual(block(restore, "restore-keys", 10), [FALLBACK])
        stage = next(at for at, s in enumerate(steps) if "check.sh test-release" in run_of(s))
        self.assertLess(restores[0], stage, "the restore comes after the stage")
        saves = [at for at, s in enumerate(steps) if uses_of(s) == "actions/cache/save"]
        self.assertEqual(len(saves), 1, f"the release job saves {len(saves)} caches")
        save = steps[saves[0]]
        self.assertGreater(saves[0], stage, "the save comes before the stage")
        self.assertEqual(block(save, "path", 10), RUST_CACHE, "the save's paths")
        self.assertEqual(field(save, "key", 10), PRIMARY)
        self.assertEqual(field(save, "if"), ON_A_MISS, "the save's condition")
        clean = steps[saves[0] - 1]
        self.assertEqual(run_of(clean), "cargo clean --workspace --release")
        self.assertEqual(field(clean, "if"), ON_A_MISS, "the clean's condition")
        self.assertGreater(saves[0] - 1, stage, "the clean comes before the stage")
        disks = [at for at, s in enumerate(steps) if run_of(s) == "df -h /"]
        examined("df steps", disks)
        self.assertTrue([at for at in disks if at < restores[0]], "no df before the restore")
        self.assertTrue(
            [at for at in disks if stage < at < saves[0] - 1],
            "no df between the stage and the clean",
        )


if __name__ == "__main__":
    unittest.main()
