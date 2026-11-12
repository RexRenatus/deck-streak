"""The stage step puts the web engine's module and its bindings beside the built app, where the app's
Worker loads them from `/engine/`, and refuses, naming the file, when either is missing (SPEC-350
R13, A22; ADR-361).

Every case runs the script as CI's `web-engine` job does, from a planted tree's root: the two files
`scripts/web-engine-build.sh` writes under `target/web-engine/`, and a built app under
`web/app/build/`. The expected bytes are the planted ones, never read back from the script."""

import os
import subprocess
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

STAGE = REPO / "scripts" / "web-engine-stage.sh"
# The two files the build writes, named here apart from the script.
MODULE = "deck_streak_web_engine_bg.wasm"
BINDINGS = "deck_streak_web_engine.js"


def run_stage(planted):
    """Run the stage from a planted tree's root. `planted` maps each of the two files to the bytes to
    plant, or None to leave it missing. Returns the exit code, the output, and every file under the
    staged directory by name, with its bytes."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        built = root / "target" / "web-engine"
        built.mkdir(parents=True)
        (root / "web" / "app" / "build").mkdir(parents=True)
        (root / "web" / "app" / "build" / "index.html").write_text("<!doctype html>\n")
        for name, data in planted.items():
            if data is not None:
                (built / name).write_bytes(data)
        out = subprocess.run(
            ["bash", str(STAGE)],
            cwd=root,
            capture_output=True,
            text=True,
            env={**os.environ, "LC_ALL": "C"},
            check=False,
        )
        staged = root / "web" / "app" / "build" / "engine"
        files = (
            {path.name: path.read_bytes() for path in sorted(staged.iterdir())}
            if staged.is_dir()
            else {}
        )
        return out.returncode, out.stdout + out.stderr, files


class TheStageCopiesTheModule(unittest.TestCase):
    def test_the_stage_copies_the_module_and_its_bindings(self):
        module = b"\0asm\x01\0\0\0" + os.urandom(4096)
        bindings = b"export function open() { return wasm.open(); }\n" * 40
        code, output, files = run_stage({MODULE: module, BINDINGS: bindings})
        # both files, byte for byte, and nothing else
        self.assertEqual((code, files), (0, {BINDINGS: bindings, MODULE: module}), output)
        examined("staged files", list(files))

    def test_the_stage_refuses_a_missing_module(self):
        cases = {
            "the module missing": ({MODULE: None, BINDINGS: b"export {};\n"}, MODULE),
            "the bindings missing": ({MODULE: b"\0asm\x01\0\0\0", BINDINGS: None}, BINDINGS),
            "both missing": ({MODULE: None, BINDINGS: None}, BINDINGS),
        }
        for name, (planted, missing) in examined("planted missing files", cases.items()):
            with self.subTest(name):
                code, output, files = run_stage(planted)
                # refused, naming the missing file, and nothing staged
                self.assertEqual((code, files), (1, {}), output)
                self.assertIn(f"target/web-engine/{missing} is missing", output)


if __name__ == "__main__":
    unittest.main()
