"""The second oracle's driver: one harness session, strict goldens with their provenance, a
read-only source and the stock-text census (SPEC-498 R1, R6, R7, R11, ADR-516).

Every test drives a stand-in for the harness (`anki/fixtures/stand_in_harness.py`), which speaks
the protocol and computes nothing of the engine's, over a registry in a temporary directory or
the committed one. No test starts the engine, cargo or the network.
"""

import contextlib
import hashlib
import importlib.util
import io
import json
import shutil
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
ANKI = HERE / "anki"
STAND_IN = ANKI / "fixtures" / "stand_in_harness.py"
ROOT = HERE.parent.parent


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


generate = load("anki_generate", ANKI / "generate.py")


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


#: A two-function registry over the committed corpus, for the tests that need a small run.
SMALL = """
    import hashlib
    import importlib.util
    import sys
    from pathlib import Path

    _path = Path({corpus!r})
    _spec = importlib.util.spec_from_file_location("anki_corpus_small", _path)
    CORPUS = importlib.util.module_from_spec(_spec)
    sys.modules["anki_corpus_small"] = CORPUS
    _spec.loader.exec_module(CORPUS)
    CORPUS_SHA256 = hashlib.sha256(_path.read_bytes()).hexdigest()
    SEED = CORPUS.SEED


    def day(rng):
        return [("rollover", CORPUS.timing(rng, "rollover"))] + [
            (None, CORPUS.timing(rng)) for _ in range(3)
        ]


    def render(rng):
        nt = CORPUS.basic()
        return [("empty", {{"notetype": nt, "fields": ["", "x"], "ord": 0}})] + [
            (None, {{"notetype": nt, "fields": [CORPUS.words(rng), "y"], "ord": 0}})
        ]


    FUNCTIONS = {{
        "anki.timing.day": {{"id": "F4", "classes": ["rollover"], "note": "day", "cases": day}},
        "anki.render.card": {{"id": "R-1", "classes": {classes}, "note": "card", "cases": render}},
    }}
"""


def small_registry(directory, classes='["empty"]', name="spec_498.py"):
    folder = Path(directory) / "registry"
    folder.mkdir()
    text = SMALL.format(corpus=str(ANKI / "corpus.py"), classes=classes)
    (folder / name).write_text(textwrap.dedent(text), encoding="utf-8")
    return folder


def stand_in(*options):
    return [sys.executable, str(STAND_IN), *options]


def run_small(directory, *options):
    registered = generate.load_registry(small_registry(directory))
    seeds = {module: generate.load_module(module).SEED for module, _ in registered.values()}
    return generate.run(stand_in(*options), registered, seeds.__getitem__, "ab" * 32)


class Registry(unittest.TestCase):
    def test_edge_classes_covered_refuses_a_declared_class_with_no_case(self):
        """T-G16's control: a registration that declares a class its builder never draws is
        refused before anything runs, naming the class."""
        with tempfile.TemporaryDirectory() as scratch:
            folder = small_registry(scratch, classes='["empty", "large"]')
            with self.assertRaisesRegex(generate.Refused, "no case: large"):
                generate.load_registry(folder)

    def test_a_case_class_the_registration_does_not_declare_is_refused(self):
        with tempfile.TemporaryDirectory() as scratch:
            folder = small_registry(scratch, classes="[]")
            with self.assertRaisesRegex(generate.Refused, "does not declare: empty"):
                generate.load_registry(folder)

    def test_a_module_not_named_by_its_spec_is_refused(self):
        with tempfile.TemporaryDirectory() as scratch:
            folder = small_registry(scratch, name="extra.py")
            with self.assertRaisesRegex(generate.Refused, "spec_NNN.py"):
                generate.load_registry(folder)

    def test_a_stale_corpus_pin_is_refused(self):
        """A corpus edit without the registry's pin edit is refused, so no golden can record a
        corpus its registry digest does not cover."""
        with tempfile.TemporaryDirectory() as scratch:
            folder = small_registry(scratch)
            module = folder / "spec_498.py"
            text = module.read_text(encoding="utf-8").replace(
                "CORPUS_SHA256 = hashlib", 'CORPUS_SHA256 = "0" * 64 or hashlib'
            )
            module.write_text(text, encoding="utf-8")
            with self.assertRaisesRegex(generate.Refused, "CORPUS_SHA256"):
                generate.load_registry(folder)


class Run(unittest.TestCase):
    def test_every_golden_is_strict_synthetic_and_seeded(self):
        """dm.oracle: every golden the committed registry gives is strict phx.parity-golden.v1
        with closed keys, the pinned engine commit, an integer seed and synthetic inputs."""
        registered = generate.load_registry(generate.REGISTRY)
        seeds = {module: generate.load_module(module).SEED for module, _ in registered.values()}
        goldens, _ = generate.run(stand_in(), registered, seeds.__getitem__, "ab" * 32)
        refusals = [
            r
            for name, golden in examined("goldens", goldens.items())
            for r in generate.strict_refusals(name, golden)
        ]
        self.assertEqual(refusals, [])
        self.assertEqual({g["source_commit"] for g in goldens.values()}, {generate.ENGINE_COMMIT})
        self.assertEqual(set(goldens), set(registered))

    def test_a_golden_that_is_not_strict_is_refused(self):
        with tempfile.TemporaryDirectory() as scratch:
            goldens, _ = run_small(scratch)
        golden = dict(goldens["anki.timing.day"])
        self.assertEqual(generate.strict_refusals("anki.timing.day", golden), [])
        planted = dict(golden, diverges="x", seed="20", inputs="owner")
        refusals = generate.strict_refusals("anki.timing.day", planted)
        self.assertEqual(len(refusals), 3, refusals)

    def test_the_driver_records_what_the_harness_answered(self):
        with tempfile.TemporaryDirectory() as scratch:
            goldens, _ = run_small(scratch, "--refuse", "anki.render.card=empty_front")
        day = examined("cases", goldens["anki.timing.day"]["cases"])
        for case in day:
            digest = hashlib.sha256(json.dumps(case["input"], sort_keys=True).encode()).hexdigest()
            self.assertEqual(case["output"], {"stand_in": digest[:16]})
        self.assertEqual(day[0]["class"], "rollover")
        card = goldens["anki.render.card"]["cases"]
        self.assertEqual({json.dumps(c["output"]) for c in card}, {'{"refused": "empty_front"}'})

    def test_a_harness_of_another_engine_is_refused(self):
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(generate.RunFailed, "not the pinned"):
                run_small(scratch, "--commit", "1" * 40)

    def test_a_protocol_fault_fails_the_run(self):
        faults = {
            "extra-key": "answered with",
            "not-json": "not JSON",
            "words": "never the engine's words",
            "exit": "exited 1",
            "close": "closed its output",
        }
        for fault, reason in examined("faults", faults.items()):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory() as scratch:
                options = ["--fault", fault, "--refuse", "anki.timing.day=no"]
                with self.assertRaisesRegex(generate.RunFailed, reason):
                    run_small(scratch, *options)


class Provenance(unittest.TestCase):
    def test_golden_provenance(self):
        """T-G13: each golden's generator_sha256 equals the committed generator's digest and its
        registry_sha256 the registry module's; a one-byte edit of the generator in a temporary
        copy turns every golden red."""
        registered = generate.load_registry(generate.REGISTRY)
        seeds = {module: generate.load_module(module).SEED for module, _ in registered.values()}
        digest = generate.sha256(generate.HERE)
        goldens, _ = generate.run(stand_in(), registered, seeds.__getitem__, digest)
        items = examined("goldens", goldens.items())
        for name, golden in items:
            self.assertEqual(generate.provenance_refusals(name, golden, ROOT), [], name)
        with tempfile.TemporaryDirectory() as scratch:
            copy = Path(scratch)
            for relative in (generate.GENERATOR, f"{generate.PUBLISHED_REGISTRY}/spec_498.py"):
                (copy / relative).parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(ROOT / relative, copy / relative)
            edited = copy / generate.GENERATOR
            data = bytearray(edited.read_bytes())
            data[-2] ^= 0x01
            edited.write_bytes(bytes(data))
            for name, golden in items:
                refusals = generate.provenance_refusals(name, golden, copy)
                self.assertEqual(len(refusals), 1, name)
                self.assertIn("generator_sha256 differs", refusals[0])

    def test_the_harness_pin_is_held(self):
        """The driver starts no harness unless its source's digest is the pinned one."""
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "harness"
            source.mkdir()
            (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            pin = generate.tree_digest(source)
            common = [
                "--harness",
                str(STAND_IN),
                "--harness-source",
                str(source),
                "--engine-source",
                str(source),
                "--out",
                str(Path(scratch) / "out"),
            ]
            quiet = contextlib.redirect_stderr(io.StringIO())
            with quiet as err, mock.patch.object(generate, "writable_refusals", return_value=[]):
                self.assertEqual(generate.main(common, pin="0" * 64), 2)
                self.assertEqual(generate.main(common, pin=None), 2)
            refusals = err.getvalue().splitlines()
            self.assertEqual(len(refusals), 2, refusals)
            self.assertIn(f"digest {pin} is not the pinned {'0' * 64}", refusals[0])
            self.assertIn("the harness is not pinned", refusals[1])
            self.assertFalse((Path(scratch) / "out").exists())


class ReadOnly(unittest.TestCase):
    def test_a_writable_source_is_refused(self):
        """PC-1: a source the run could write to is refused before the harness starts, and the
        same tree read-only is accepted."""
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "engine"
            (source / "src").mkdir(parents=True)
            (source / "src" / "lib.rs").write_text("\n", encoding="utf-8")
            refusals = generate.writable_refusals([source])
            self.assertEqual(len(refusals), 1)
            self.assertIn("is writable", refusals[0])
            read_only = examined("paths", [source, source / "src", source / "src" / "lib.rs"])
            probed = []

            def never(path):
                probed.append(path)
                return False

            self.assertEqual(generate.writable_refusals([source], never), [])
            self.assertEqual(sorted(probed), sorted(read_only))
            with contextlib.redirect_stderr(io.StringIO()) as err:
                code = generate.main(
                    [
                        "--harness",
                        str(STAND_IN),
                        "--harness-source",
                        str(source),
                        "--engine-source",
                        str(source),
                        "--out",
                        str(Path(scratch) / "out"),
                    ],
                    pin=generate.tree_digest(source),
                )
            self.assertEqual(code, 2)
            self.assertIn("is writable", err.getvalue())
            self.assertFalse((Path(scratch) / "out").exists())

    def test_the_run_writes_only_in_its_output_directory(self):
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "harness"
            source.mkdir()
            (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            registry = small_registry(scratch)
            # The built harness is one executable; the stand-in is a script, so wrap it.
            binary = Path(scratch) / "harness-binary"
            binary.write_text(
                f'#!/bin/sh\nexec "{sys.executable}" "{STAND_IN}" "$@"\n', encoding="utf-8"
            )
            binary.chmod(0o755)
            before = sorted(p.relative_to(scratch) for p in Path(scratch).rglob("*"))
            out = Path(scratch) / "out"
            argv = [
                "--harness",
                str(binary),
                "--harness-source",
                str(source),
                "--engine-source",
                str(source),
                "--registry",
                str(registry),
                "--out",
                str(out),
            ]
            with (
                mock.patch.object(generate, "writable_refusals", return_value=[]),
                contextlib.redirect_stdout(io.StringIO()) as printed,
            ):
                code = generate.main(argv, pin=generate.tree_digest(source))
            self.assertEqual(code, 0)
            after = sorted(p.relative_to(scratch) for p in Path(scratch).rglob("*"))
            new = [p for p in after if p not in before and "__pycache__" not in p.parts]
            self.assertEqual(
                [p.as_posix() for p in new],
                [
                    "out",
                    "out/goldens",
                    "out/goldens/anki.render.card.json",
                    "out/goldens/anki.timing.day.json",
                    "out/stock-lines.sha256",
                ],
            )
            self.assertIn("examined 6 case(s) of 2 function(s)", printed.getvalue())


class StockText(unittest.TestCase):
    def test_a_golden_holding_a_stock_line_is_refused(self):
        """PC-10: when a line the harness reports as the engine's stock text appears in a golden,
        the run fails and writes nothing; the census keeps only the lines' digests."""
        css = "font-family: system-ui; font-size: 18px; margin: 12px;"
        stock = f"Stock heading\n  .card {{ {css} }}  \nshort"
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(generate.RunFailed, "stock-text census refuses"):
                run_small(scratch, "--stock", stock)
        with tempfile.TemporaryDirectory() as scratch:
            goldens, hashes = run_small(scratch, "--stock", "A line of the engine's own text")
        self.assertEqual(hashes, {hashlib.sha256(b"A line of the engine's own text").hexdigest()})
        golden = goldens["anki.timing.day"]
        self.assertEqual(generate.census_refusals("anki.timing.day", golden, hashes), [])
        planted = json.loads(json.dumps(golden))
        planted["cases"][1]["output"] = {"text": "x\n  A line of the engine's own text \n"}
        refusals = generate.census_refusals("anki.timing.day", planted, hashes)
        self.assertEqual(len(refusals), 1)
        self.assertIn("case 1 holds stock line", refusals[0])

    def test_short_lines_are_not_counted(self):
        self.assertEqual(generate.stock_hashes(["{{Front}}\n}\n\n"]), set())
        self.assertEqual(len(generate.stock_hashes(["twelve chars", "eleven char"])), 1)


if __name__ == "__main__":
    unittest.main()
