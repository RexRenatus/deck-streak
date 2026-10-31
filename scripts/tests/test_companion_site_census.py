"""The census of a built site refuses every file the owner route serves (SPEC-511 C18, R12;
ADR-529 D5): a file named `module.wasm` or `weights.bin`, any file under `api/`, a `manifest.json`
that names either, and a WebAssembly module outside `engine/`, where the public engine is staged.
It prints how many files it examined, fails by path, and reads VOID for a missing or empty site.

Every case runs the census as a command over a planted site, but two that load it to read what a
command's exit cannot show: the bytecode setting its load leaves, and the code `main` returns."""

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_mutation_python_judge_kills import load

CENSUS = REPO / "scripts" / "companion-site-census.py"
# The first bytes of every WebAssembly module, written here apart from the census.
WASM = b"\0asm\x01\0\0\0"
# How much of a manifest.json the census reads for a served name.
MEBIBYTE = 1 << 20

companion_census = load(CENSUS, "companion_site_census")


def command(*args):
    """Run the census with `args`, on a terminal wide enough that argparse wraps no line. Returns
    its exit code and its output."""
    done = subprocess.run(
        [sys.executable, str(CENSUS), *args],
        capture_output=True,
        text=True,
        check=False,
        env={**os.environ, "COLUMNS": "200"},
    )
    return done.returncode, done.stdout + done.stderr


def run_census(root):
    """Run the census over `root`. Returns its exit code and its output."""
    return command("--root", str(root))


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

    def test_a_manifest_that_does_not_read_is_judged_by_its_path_alone(self):
        # A link to an absent file is listed as a file and cannot be opened: the census reads it as
        # holding nothing, so a manifest.json it cannot read names no served file.
        with tempfile.TemporaryDirectory() as root:
            site(root, PUBLIC)
            os.symlink("absent.json", Path(root) / "manifest.json")
            code, out = run_census(root)
            self.assertEqual(code, 0, out)
            self.assertIn(f"companion-site-census: examined {len(PUBLIC) + 1} file(s)", out)
            self.assertIn("companion-site-census: PASS", out)
            self.assertNotIn("Traceback", out)

    def test_a_manifest_is_read_to_its_first_mebibyte_and_no_further(self):
        name = b"weights.bin"
        cases = [
            ("a served name whose last byte is the mebibyte's last", MEBIBYTE - len(name), 1),
            ("a served name whose last byte is one past the mebibyte", MEBIBYTE - len(name) + 1, 0),
        ]
        for what, offset, expected in examined("manifest bound(s)", cases):
            with self.subTest(what), tempfile.TemporaryDirectory() as root:
                site(root, PUBLIC)
                site(root, {"_app/manifest.json": b" " * offset + name})
                code, out = run_census(root)
                self.assertEqual(code, expected, f"{what}: {out}")
                found = "companion-site-census: FINDING _app/manifest.json:" in out
                self.assertEqual(found, expected == 1, f"{what}: {out}")

    def test_the_help_names_the_census_its_purpose_and_its_root(self):
        code, out = command("--help")
        self.assertEqual(code, 0, out)
        shown = " ".join(out.split())
        self.assertIn("usage: companion-site-census [-h] --root ROOT", shown)
        self.assertIn(
            "companion-site-census: the built site holds no file the owner route serves "
            "(SPEC-511 R12).",
            shown,
        )
        self.assertIn("--root ROOT the built site's root", shown)

    def test_a_census_without_a_root_is_a_usage_error(self):
        code, out = command()
        self.assertEqual(code, 2, out)
        self.assertIn(
            "companion-site-census: error: the following arguments are required: --root", out
        )
        self.assertNotIn("Traceback", out)

    def test_loading_the_census_turns_bytecode_writing_off(self):
        # A copy in a scratch folder, so the bytecode its own load caches lands there, never beside
        # the tree's script.
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch) / CENSUS.name
            copy.write_bytes(CENSUS.read_bytes())
            kept = sys.dont_write_bytecode
            sys.dont_write_bytecode = False
            try:
                load(copy, "companion_site_census_copy")
                written = sys.dont_write_bytecode
            finally:
                sys.dont_write_bytecode = kept
        self.assertIs(written, True)

    def test_main_returns_zero_for_a_public_site(self):
        shown = io.StringIO()
        with tempfile.TemporaryDirectory() as root:
            site(root, PUBLIC)
            with contextlib.redirect_stdout(shown):
                code = companion_census.main(["--root", root])
        self.assertEqual(code, 0, shown.getvalue())
        self.assertIn("companion-site-census: PASS", shown.getvalue())


if __name__ == "__main__":
    unittest.main()
