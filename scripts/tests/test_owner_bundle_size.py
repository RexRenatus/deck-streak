"""The size gate sums every owner file it is given beside the module and its bindings (SPEC-511
C17, R11; ADR-529 D5): the owner's page downloads the public module and bindings plus the
companion's module and weights, so a budget that counted two of the four would pass a page over it.

Every case runs the gate as a command over planted files. The expected sizes are measured here by
gzip itself, never read from the gate. Brotli's figure is printed and never gated, so a stand-in
`node` on the gate's PATH answers it, and no test here starts node."""

import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

GATE = REPO / "scripts" / "web-engine-size.py"
# ADR-336's budget, written here apart from the gate.
BUDGET = 8_000_000
# The four files, each under a quarter of the budget once compressed, so only their sum is over it.
NAMES = ("module", "bindings", "companion module", "companion weights")


def gzip_size(path):
    """The `gzip -9 -n` size of a file, measured by gzip itself."""
    out = subprocess.run(["gzip", "-9", "-n", "-c", str(path)], capture_output=True, check=True)
    return len(out.stdout)


def tool_path(root):
    """A PATH holding gzip and a stand-in `node` that answers one brotli figure, made once under
    `root`."""
    tools = Path(root) / "tools"
    if tools.is_dir():
        return str(tools)
    tools.mkdir()
    gzip = shutil.which("gzip")
    if gzip is None:
        raise AssertionError("gzip is not on this PATH")
    (tools / "gzip").symlink_to(gzip)
    node = tools / "node"
    node.write_text("#!/bin/sh\nprintf '%s' 1\n")
    node.chmod(0o700)
    return str(tools)


def run_gate(root, files, owners):
    """Run the gate over `files` (module, bindings) and each of `owners` as an owner file."""
    command = [sys.executable, str(GATE), "--module", str(files[0]), "--bindings", str(files[1])]
    for owner in owners:
        command += ["--owner-file", str(owner)]
    env = dict(os.environ, PATH=tool_path(root))
    done = subprocess.run(command, capture_output=True, text=True, env=env, check=False)
    return done.returncode, done.stdout + done.stderr


def planted(root, sizes, pools=None):
    """Four files of incompressible bytes, of the given raw sizes, in `root`: each a prefix of its
    own pool of random bytes when `pools` is given, so a file's bytes stay put as its size moves."""
    paths = []
    for index, (name, size) in enumerate(zip(NAMES, sizes)):
        path = Path(root) / f"{name.replace(' ', '-')}.bin"
        path.write_bytes(pools[index][:size] if pools else os.urandom(size))
        paths.append(path)
    return paths


def sized(root, target):
    """Four planted files whose `gzip -9 -n` sizes sum to exactly `target`, and that sum."""
    sizes = [target // 4 - 2_000] * 4
    # Room for the last file to grow by every other file's overhead, which is under 1 KiB each.
    pools = [os.urandom(target // 4 + 64_000) for _ in NAMES]
    seen = set()
    for _ in range(16):
        paths = planted(root, sizes, pools)
        total = sum(gzip_size(path) for path in paths)
        if total == target:
            return paths, total
        # Random bytes compress to themselves plus a small overhead, so the last file's raw size
        # moves its compressed size by as much. A stored block's header adds its bytes at once, so
        # the last file can skip the value; a total seen before moves the third file instead.
        if total in seen:
            sizes[2] += 7
        seen.add(total)
        sizes[3] += target - total
    raise AssertionError(f"no four files sum to {target} bytes gzip -9")


class TheGateSumsEveryOwnerFile(unittest.TestCase):
    def test_a_set_one_byte_over_fails_and_the_same_set_at_the_budget_passes(self):
        with tempfile.TemporaryDirectory() as root:
            over, total = sized(root, BUDGET + 1)
            self.assertEqual(total, BUDGET + 1)
            for path in examined("planted file(s)", over):
                self.assertLess(gzip_size(path), BUDGET // 2, f"{path.name} alone is small")
            code, out = run_gate(root, over[:2], over[2:])
            self.assertEqual(code, 1, out)
            self.assertIn(f"OVER BUDGET: total gzip-9 {BUDGET + 1} > {BUDGET} by 1", out)
            self.assertIn("examined 4 file(s)", out)
            self.assertNotIn("PASS", out)
            # The module and bindings alone, the two files the gate took before, are under it.
            code, out = run_gate(root, over[:2], [])
            self.assertEqual(code, 0, out)
            self.assertIn("examined 2 file(s)", out)
        with tempfile.TemporaryDirectory() as root:
            at, total = sized(root, BUDGET)
            code, out = run_gate(root, at[:2], at[2:])
            self.assertEqual(code, 0, out)
            self.assertIn(f"PASS: total gzip-9 {BUDGET} <= {BUDGET}, headroom 0", out)

    def test_each_owner_file_is_measured_on_its_own_line(self):
        with tempfile.TemporaryDirectory() as root:
            files = planted(root, [4_000, 3_000, 2_000, 1_000])
            code, out = run_gate(root, files[:2], files[2:])
            self.assertEqual(code, 0, out)
            for path in examined("owner file(s)", files[2:]):
                self.assertIn(
                    f"web-engine-size: owner-file {path.name}: raw {path.stat().st_size}, "
                    f"gzip-9 {gzip_size(path)}, brotli-11 1",
                    out,
                )

    def test_a_missing_or_empty_owner_file_is_void_not_a_pass(self):
        with tempfile.TemporaryDirectory() as root:
            files = planted(root, [4_000, 3_000, 2_000, 1_000])
            empty = Path(root) / "empty.bin"
            empty.write_bytes(b"")
            missing = Path(root) / "missing.bin"
            cases = [(missing, "is missing"), (empty, "is empty")]
            for path, why in examined("missing or empty owner file case(s)", cases):
                code, out = run_gate(root, files[:2], [files[2], path])
                self.assertEqual(code, 2, f"{path.name} {why}: {out}")
                self.assertIn(f"web-engine-size: VOID: the owner-file {path} {why}", out)
                self.assertNotIn("PASS", out)


if __name__ == "__main__":
    unittest.main()
