"""The web engine's size gate holds the module plus its JS bindings to 8000000 bytes `gzip -9`,
records each file's brotli size beside it, and reads VOID, never a pass, when a file is missing or
empty or a compressor is absent (SPEC-338 A3 to A5, R8; ADR-336's budget, ADR-349's arithmetic).

Every case runs the gate as CI's `web-engine` job does, over planted files. The expected sizes are
measured here by the compressors themselves, never read from the gate. The over-budget control is
two files each under the budget alone and over it together, so a gate that judged one file, or the
larger, would pass it."""

import importlib.util
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
# Brotli's figure as the gate records it: quality 11, a 2^24-byte window (the brotli CLI's own
# defaults), computed by node's zlib, which the job's runner carries.
BROTLI = (
    "const z = require('zlib'), fs = require('fs');"
    "const data = fs.readFileSync(process.argv[1]);"
    "const out = z.brotliCompressSync(data, {params: {"
    "[z.constants.BROTLI_PARAM_QUALITY]: 11, [z.constants.BROTLI_PARAM_LGWIN]: 24,"
    "[z.constants.BROTLI_PARAM_SIZE_HINT]: data.length}});"
    "process.stdout.write(String(out.length));"
)
# A small module and bindings shaped like the real ones: a wasm header and repeated text.
SMALL_MODULE = b"\0asm\x01\0\0\0" + b"deck streak engine section " * 4000 + os.urandom(20_000)
SMALL_BINDINGS = b"export function open() { return wasm.open(); }\n" * 300


def gzip_size(path):
    """The `gzip -9` size of a file, measured by gzip itself, with no name in the header."""
    out = subprocess.run(["gzip", "-9", "-n", "-c", str(path)], capture_output=True, check=True)
    return len(out.stdout)


def brotli_size(path):
    """The brotli size of a file at quality 11, measured by node's zlib."""
    out = subprocess.run(
        ["node", "-e", BROTLI, str(path)], capture_output=True, check=True, text=True
    )
    return int(out.stdout)


def run_gate(module, bindings, path=None):
    """Run the gate over planted files. `module` and `bindings` are bytes to plant, or None to
    leave that file missing. Returns the exit code, stdout, and the two paths."""
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        files = {
            "module": root / "deck_streak_web_engine_bg.wasm",
            "bindings": root / "deck_streak_web_engine.js",
        }
        for name, data in (("module", module), ("bindings", bindings)):
            if data is not None:
                files[name].write_bytes(data)
        env = dict(os.environ)
        if path is not None:
            env["PATH"] = path
        done = subprocess.run(
            [
                sys.executable,
                str(GATE),
                "--module",
                str(files["module"]),
                "--bindings",
                str(files["bindings"]),
            ],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )
        sizes = {
            name: (len(data), gzip_size(files[name]), brotli_size(files[name]))
            for name, data in (("module", module), ("bindings", bindings))
            if data
        }
        return done.returncode, done.stdout + done.stderr, files, sizes


def load_gate():
    """The gate as a module, for its arithmetic at the budget's edge."""
    spec = importlib.util.spec_from_file_location("web_engine_size", GATE)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def call_measure(function, tool, path):
    """One of the gate's measuring functions, called in its own process so a tool's output that
    reaches the process's own stdout cannot reach the test runner's. Its last line is the figure,
    or VOID when the gate refused."""
    program = (
        "import importlib.util, sys\n"
        "spec = importlib.util.spec_from_file_location('web_engine_size', sys.argv[1])\n"
        "gate = importlib.util.module_from_spec(spec)\n"
        "spec.loader.exec_module(gate)\n"
        "from pathlib import Path\n"
        "try:\n"
        "    figure = getattr(gate, sys.argv[2])(sys.argv[3], Path(sys.argv[4]))\n"
        "except gate.Void:\n"
        "    figure = 'VOID'\n"
        "sys.stdout.flush()\n"
        "sys.stdout.write('\\n' + str(figure) + '\\n')\n"
    )
    done = subprocess.run(
        [sys.executable, "-c", program, str(GATE), function, str(tool), str(path)],
        capture_output=True,
        check=False,
    )
    return done.stdout.decode("utf-8", "replace").strip().split("\n")[-1]


class TheSizeGate(unittest.TestCase):
    def test_a_module_over_the_budget_fails_the_gate(self):
        # Each file is under the budget alone; together they are over it.
        half = os.urandom(BUDGET // 2 + 64)
        code, out, files, sizes = run_gate(half, os.urandom(BUDGET // 2 + 64))
        total = sizes["module"][1] + sizes["bindings"][1]
        self.assertGreater(total, BUDGET, "the planted control is not over the budget")
        self.assertLess(max(sizes["module"][1], sizes["bindings"][1]), BUDGET)
        self.assertEqual(code, 1, out)
        self.assertIn(
            f"web-engine-size: OVER BUDGET: total gzip-9 {total} > {BUDGET} by {total - BUDGET}",
            out,
        )
        self.assertNotIn("PASS", out)
        # At the edge: the budget itself passes, one byte more fails.
        gate = load_gate()
        edges = examined("budget edge(s)", [(BUDGET - 1, 0), (BUDGET, 0), (BUDGET + 1, 1)])
        for total, expected in edges:
            self.assertEqual(gate.judge(total), expected, f"a total of {total}")

    def test_a_module_under_the_budget_passes_and_records_brotli(self):
        code, out, files, sizes = run_gate(SMALL_MODULE, SMALL_BINDINGS)
        self.assertEqual(code, 0, out)
        for name in examined("shipped file(s)", ["module", "bindings"]):
            raw, gzipped, brotli = sizes[name]
            self.assertIn(
                f"web-engine-size: {name} {files[name].name}: raw {raw}, gzip-9 {gzipped}, "
                f"brotli-11 {brotli}",
                out,
            )
        total = sizes["module"][1] + sizes["bindings"][1]
        brotli_total = sizes["module"][2] + sizes["bindings"][2]
        self.assertIn(
            f"web-engine-size: PASS: total gzip-9 {total} <= {BUDGET}, "
            f"headroom {BUDGET - total}; total brotli-11 {brotli_total}",
            out,
        )

    def test_a_missing_or_empty_file_is_void_not_a_pass(self):
        cases = [
            ("module", None, SMALL_BINDINGS, "is missing"),
            ("bindings", SMALL_MODULE, None, "is missing"),
            ("module", b"", SMALL_BINDINGS, "is empty"),
            ("bindings", SMALL_MODULE, b"", "is empty"),
        ]
        for name, module, bindings, why in examined("missing or empty case(s)", cases):
            code, out, files, _ = run_gate(module, bindings)
            self.assertEqual(code, 2, f"{name} {why}: {out}")
            self.assertIn(f"web-engine-size: VOID: the {name} {files[name]} {why}", out)
            self.assertNotIn("PASS", out)

    def test_a_missing_compressor_is_void_not_a_pass(self):
        # A PATH holding one compressor and not the other: the figure the absent one would give is
        # never printed as measured.
        cases = [("gzip", "node"), ("node", "gzip")]
        for present, absent in examined("missing compressor case(s)", cases):
            with tempfile.TemporaryDirectory() as tmp:
                found = shutil.which(present)
                self.assertIsNotNone(found, f"{present} is not on this PATH")
                (Path(tmp) / present).symlink_to(found)
                code, out, _, _ = run_gate(SMALL_MODULE, SMALL_BINDINGS, path=tmp)
            self.assertEqual(code, 2, f"without {absent}: {out}")
            self.assertIn(f"web-engine-size: VOID: {absent} is not on PATH", out)
            self.assertNotIn("PASS", out)

    def test_the_gate_keeps_python_from_writing_bytecode(self):
        before = sys.dont_write_bytecode
        sys.dont_write_bytecode = False
        try:
            load_gate()
            self.assertTrue(sys.dont_write_bytecode)
        finally:
            sys.dont_write_bytecode = before

    def test_a_compressor_that_fails_or_prints_nothing_is_void(self):
        with tempfile.TemporaryDirectory() as tmp:
            planted = Path(tmp) / "planted"
            planted.write_bytes(SMALL_BINDINGS)
            # Each fake tool is (what it does, its exit, its output): a failure that still
            # printed something, and a success that printed nothing, are each no measurement.
            cases = [("exits 3 after printing", 3, "5"), ("exits 0 and prints nothing", 0, "")]
            for why, code, output in examined("fake compressor case(s)", cases):
                fake = Path(tmp) / f"fake-{code}"
                fake.write_text(f"#!/bin/sh\nprintf '%s' '{output}'\nexit {code}\n")
                fake.chmod(0o700)
                self.assertEqual(call_measure("gzip_size", fake, planted), "VOID", f"gzip {why}")
                if output:
                    self.assertEqual(
                        call_measure("brotli_size", fake, planted), "VOID", f"node {why}"
                    )

    def test_a_brotli_figure_is_read_as_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            planted = Path(tmp) / "planted"
            planted.write_bytes(SMALL_BINDINGS)
            fake = Path(tmp) / "fake-node"
            # The Arabic-Indic digit three: a digit as text, and no digit as bytes.
            fake.write_text("#!/bin/sh\nprintf '\\331\\243'\n")
            fake.chmod(0o700)
            self.assertEqual(call_measure("brotli_size", fake, planted), "3")

    def test_the_command_line_names_itself_and_requires_both_files(self):
        helped = subprocess.run(
            [sys.executable, str(GATE), "--help"], capture_output=True, text=True, check=False
        )
        words = " ".join(helped.stdout.split())
        self.assertEqual(helped.returncode, 0, helped.stderr)
        for line in examined(
            "help line(s)",
            [
                "usage: web-engine-size",
                "the web engine's size gate",
                "the module a browser loads",
                "its JS bindings",
            ],
        ):
            self.assertIn(line, words)
        for given in examined(
            "missing option case(s)", [[], ["--module", "m"], ["--bindings", "b"]]
        ):
            done = subprocess.run(
                [sys.executable, str(GATE), *given], capture_output=True, text=True, check=False
            )
            self.assertEqual(done.returncode, 2, f"{given}: {done.stderr}")
            self.assertIn("required", done.stderr)


if __name__ == "__main__":
    unittest.main()
