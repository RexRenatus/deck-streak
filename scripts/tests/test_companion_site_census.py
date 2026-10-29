"""The census of a built site refuses every file the owner route serves (SPEC-511 C18, R12;
ADR-529 D5): a file named `module.wasm` or `weights.bin`, any file under `api/`, a `manifest.json`
that names either, and a WebAssembly module outside `engine/`, where the public engine is staged.
It prints how many files it examined, fails by path, and reads VOID for a missing or empty site.

Every case runs the census as a command over a planted site."""

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

CENSUS = REPO / "scripts" / "companion-site-census.py"
# The first bytes of every WebAssembly module, written here apart from the census.
WASM = b"\0asm\x01\0\0\0"


def run_census(root):
    """Run the census over `root`. Returns its exit code and its output."""
    done = subprocess.run(
        [sys.executable, str(CENSUS), "--root", str(root)],
        capture_output=True,
        text=True,
        check=False,
    )
    return done.returncode, done.stdout + done.stderr


def site(root, files):
    """Plant `files`, {relative path: bytes}, under `root`."""
    for relative, data in files.items():
        path = Path(root) / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


# A built site as the web-engine job stages it: the app's pages and assets, its web manifest, and
# the public engine's module and bindings under engine/.
PUBLIC = {
    "index.html": b"<!doctype html><title>DeckStreak</title>",
    "study.html": b"<!doctype html><title>Study</title>",
    "_app/immutable/entry/start.js": b"export const start = 1;",
    "_app/immutable/assets/app.css": b"body { margin: 0 }",
    "manifest.webmanifest": b'{"name": "DeckStreak", "icons": []}',
    "engine/deck_streak_web_engine_bg.wasm": WASM + b"public engine",
    "engine/deck_streak_web_engine.js": b"export default function init() {}",
}

# Each finding the census must refuse, planted alone beside the public site, with the path the
# census names.
FINDINGS = [
    ("a module.wasm", {"owner/module.wasm": WASM + b"owner"}, "owner/module.wasm"),
    ("a weights.bin", {"weights.bin": b"\x00\x01weights"}, "weights.bin"),
    ("a file under api/", {"api/companion/v1/state.bin": b"state"}, "api/companion/v1/state.bin"),
    (
        "a manifest.json naming the module",
        {"_app/manifest.json": b'{"module": "module.wasm"}'},
        "_app/manifest.json",
    ),
    (
        "a WebAssembly module outside engine/",
        {"_app/immutable/assets/companion-1a2b.wasm": WASM + b"owner"},
        "_app/immutable/assets/companion-1a2b.wasm",
    ),
]


class TheSiteCensus(unittest.TestCase):
    def test_a_public_site_passes_and_counts_its_files(self):
        with tempfile.TemporaryDirectory() as root:
            site(root, PUBLIC)
            # A manifest.json that names neither file is the app's own, and passes.
            site(root, {"manifest.json": b'{"name": "DeckStreak"}'})
            code, out = run_census(root)
            self.assertEqual(code, 0, out)
            self.assertIn(f"companion-site-census: examined {len(PUBLIC) + 1} file(s)", out)
            self.assertIn("companion-site-census: PASS", out)

    def test_each_planted_finding_fails_by_path(self):
        for what, planted, path in examined("planted finding(s)", FINDINGS):
            with tempfile.TemporaryDirectory() as root:
                site(root, PUBLIC)
                site(root, planted)
                code, out = run_census(root)
                self.assertEqual(code, 1, f"{what}: {out}")
                self.assertIn(f"companion-site-census: FINDING {path}:", out, what)
                self.assertIn("companion-site-census: FAIL: 1 finding(s)", out, what)
                self.assertNotIn("PASS", out, what)
        self.assertEqual(len(FINDINGS), 5)

    def test_every_finding_is_named_when_several_are_planted(self):
        with tempfile.TemporaryDirectory() as root:
            site(root, PUBLIC)
            for _, planted, _ in FINDINGS:
                site(root, planted)
            code, out = run_census(root)
            self.assertEqual(code, 1, out)
            for _, _, path in FINDINGS:
                self.assertIn(f"companion-site-census: FINDING {path}:", out)
            self.assertIn(f"companion-site-census: FAIL: {len(FINDINGS)} finding(s)", out)

    def test_a_missing_or_empty_site_is_void(self):
        with tempfile.TemporaryDirectory() as root:
            empty = Path(root) / "build"
            empty.mkdir()
            (empty / "engine").mkdir()
            cases = [(Path(root) / "absent", "is missing"), (empty, "holds no file")]
            for path, why in examined("void site(s)", cases):
                code, out = run_census(path)
                self.assertEqual(code, 2, f"{path} {why}: {out}")
                self.assertIn(f"companion-site-census: VOID: the site {path} {why}", out)
                self.assertNotIn("PASS", out)


if __name__ == "__main__":
    unittest.main()
