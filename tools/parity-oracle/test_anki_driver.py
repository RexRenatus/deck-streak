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
import math
import os
import shlex
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


def drive(directory, *options):
    """main() over the small registry and the stand-in wrapped as one executable, as the sandbox
    runs it: the exit code, what main wrote on stderr, the output folder and the registry."""
    source = Path(directory) / "harness"
    source.mkdir()
    (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
    registry = small_registry(directory)
    binary = Path(directory) / "harness-binary"
    words = "".join(f" {shlex.quote(option)}" for option in options)
    binary.write_text(
        f'#!/bin/sh\nexec "{sys.executable}" "{STAND_IN}"{words} "$@"\n', encoding="utf-8"
    )
    binary.chmod(0o755)
    out = Path(directory) / "out"
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
        contextlib.redirect_stdout(io.StringIO()),
        contextlib.redirect_stderr(io.StringIO()) as err,
    ):
        code = generate.main(argv, pin=generate.tree_digest(source))
    return code, err.getvalue(), out, registry


def strict_golden():
    """A golden strict_refusals accepts, to plant one fault in at a time."""
    return {
        "schema": generate.SCHEMA,
        "kind": "engine",
        "function": "anki.timing.day",
        "source_commit": generate.ENGINE_COMMIT,
        "generator": generate.GENERATOR,
        "generator_sha256": "ab" * 32,
        "registry": f"{generate.PUBLISHED_REGISTRY}/spec_498.py",
        "registry_sha256": "cd" * 32,
        "inputs": "synthetic",
        "seed": 498,
        "note": "day",
        "cases": [{"input": {"now": 1}, "output": {"day": 0}, "class": "rollover"}],
    }


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

    def test_the_driver_starts_the_harness_in_utc(self):
        """A14 (R18 (b)): the driver starts the harness with TZ=UTC beside PATH and LC_ALL, so the
        host's zone never reaches a golden. Popen is patched, so nothing is started."""
        seen = {}

        class Started(Exception):
            pass

        def popen(argv, **kwargs):
            seen.update(kwargs.get("env") or {})
            raise Started

        with mock.patch.object(generate.subprocess, "Popen", popen):
            with self.assertRaises(Started):
                generate.Harness(["planted-harness"], Path("planted-work"))
        keys = examined("harness environment key(s)", sorted(seen))
        self.assertEqual(keys, ["LC_ALL", "PATH", "TZ"], f"the harness env keys are {keys}")
        self.assertEqual(seen["TZ"], "UTC")


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
            with (
                quiet as err,
                mock.patch.object(generate, "writable_refusals", return_value=[]),
                mock.patch.object(generate, "HARNESS_SOURCE_SHA256", None),
            ):
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


class HarnessPin(unittest.TestCase):
    def test_a_harness_source_holding_a_symbolic_link_is_refused(self):
        """A1 (R11): a link in the harness's source is refused, because the pin covers regular
        files only and a link would build the harness from a file the digest never read."""
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "harness"
            source.mkdir()
            (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            (source / "lib.rs").symlink_to(source / "main.rs")
            with self.assertRaisesRegex(generate.Refused, "lib.rs is a symbolic link"):
                generate.tree_digest(source)

    def test_a_harness_source_with_no_file_is_refused(self):
        """A1 (R11): a source tree that holds no regular file has nothing to pin."""
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "harness"
            (source / "src").mkdir(parents=True)
            with self.assertRaisesRegex(generate.Refused, "holds no file to pin"):
                generate.tree_digest(source)

    def test_the_harness_digest_covers_each_file_name(self):
        """A1 (R11): the same bytes under another name give another digest, and a copy of the
        tree gives the same one."""
        digests = {}
        with tempfile.TemporaryDirectory() as scratch:
            for tree, name in (("one", "main.rs"), ("copy", "main.rs"), ("renamed", "lib.rs")):
                root = Path(scratch) / tree
                root.mkdir()
                (root / name).write_text("fn main() {}\n", encoding="utf-8")
                digests[tree] = generate.tree_digest(root)
        self.assertEqual(digests["one"], digests["copy"])
        self.assertNotEqual(digests["one"], digests["renamed"])


class SourceGuards(unittest.TestCase):
    def test_a_source_that_is_not_a_directory_is_refused(self):
        """A1 (R11, PC-1): a source path that is missing, or is a file, is refused by name."""
        with tempfile.TemporaryDirectory() as scratch:
            missing = Path(scratch) / "no-such-checkout"
            single = Path(scratch) / "lib.rs"
            single.write_text("\n", encoding="utf-8")
            refusals = generate.writable_refusals([missing, single], lambda path: False)
        self.assertEqual(
            refusals, [f"{missing} is not a directory", f"{single} is not a directory"]
        )

    def test_the_default_probe_asks_for_write_access(self):
        """A1 (R11, PC-1): with no probe given, each path is probed for write access, not for
        its existence, so a tree the run cannot write to passes."""
        with tempfile.TemporaryDirectory() as scratch:
            source = Path(scratch) / "engine"
            (source / "src").mkdir(parents=True)
            (source / "src" / "lib.rs").write_text("\n", encoding="utf-8")
            modes = []

            def access(path, mode):
                modes.append(mode)
                return False

            with mock.patch.object(generate.os, "access", side_effect=access):
                refusals = generate.writable_refusals([source])
        self.assertEqual(refusals, [])
        self.assertEqual(set(examined("probes", modes)), {os.W_OK})

    def test_the_engine_source_is_probed_as_well_as_the_harness_source(self):
        """A1 (R11, PC-1): main probes the engine's checkout too: a writable engine source beside
        a harness source the run cannot write to is refused, naming the engine's path."""
        with tempfile.TemporaryDirectory() as scratch:
            harness = Path(scratch) / "harness"
            harness.mkdir()
            (harness / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            engine = Path(scratch) / "engine"
            (engine / "src").mkdir(parents=True)
            (engine / "src" / "lib.rs").write_text("\n", encoding="utf-8")
            out = Path(scratch) / "out"
            argv = [
                "--harness",
                str(STAND_IN),
                "--harness-source",
                str(harness),
                "--engine-source",
                str(engine),
                "--registry",
                str(small_registry(scratch)),
                "--out",
                str(out),
            ]

            def access(path, mode):
                return Path(path).is_relative_to(engine)

            with (
                mock.patch.object(generate.os, "access", side_effect=access),
                contextlib.redirect_stderr(io.StringIO()) as err,
            ):
                code = generate.main(argv, pin="0" * 64)
            self.assertEqual(code, 2)
            self.assertIn(f"{engine} is writable", err.getvalue())
            self.assertNotIn(f"{harness} is writable", err.getvalue())
            self.assertFalse(out.exists())


class Census(unittest.TestCase):
    def test_the_census_reads_lines_inside_lists(self):
        """A5 (R6, PC-10): a stock line inside a list, at any depth, is found as surely as one
        in a string or an object."""
        line = "A line of the engine's own text"
        hashes = generate.stock_hashes([line])
        golden = {
            "cases": [
                {"input": {"rows": ["x"]}, "output": {"texts": ["short", f"  {line} "]}},
                {"input": {"rows": []}, "output": [["deeper", [line]]]},
            ]
        }
        refusals = generate.census_refusals("anki.timing.day", golden, hashes)
        self.assertEqual(len(refusals), 2, refusals)
        self.assertIn("case 0 holds stock line", refusals[0])
        self.assertIn("case 1 holds stock line", refusals[1])


class Strict(unittest.TestCase):
    def planted(self, **faults):
        """strict_refusals of the strict golden with `faults` planted, once the golden alone
        reads clean."""
        self.assertEqual(generate.strict_refusals("anki.timing.day", strict_golden()), [])
        return generate.strict_refusals("anki.timing.day", dict(strict_golden(), **faults))

    def test_a_source_commit_that_is_not_hex_is_refused(self):
        """A1 (R1): a source_commit that is a string but not 40 lowercase hex digits is
        refused."""
        commits = ["z" * 40, generate.ENGINE_COMMIT.upper(), "abc"]
        for commit in examined("commits", commits):
            with self.subTest(commit=commit):
                self.assertEqual(
                    self.planted(source_commit=commit),
                    ["anki.timing.day: source_commit is not a 40-hex commit"],
                )

    def test_a_digest_that_is_not_hex_is_refused(self):
        """A1 (R1, R7): a generator or registry digest that is not 64 lowercase hex digits is
        refused, naming its key."""
        for key in examined("digest keys", ["generator_sha256", "registry_sha256"]):
            for digest in ("AB" * 32, "ab" * 31, 64):
                with self.subTest(key=key, digest=digest):
                    self.assertEqual(
                        self.planted(**{key: digest}),
                        [f"anki.timing.day: {key} is not a sha256"],
                    )

    def test_a_golden_with_no_case_is_refused(self):
        """A1 (R1): a golden whose cases are empty, or not a list, is refused."""
        for cases in examined("case lists", [[], {}, None]):
            with self.subTest(cases=cases):
                self.assertEqual(self.planted(cases=cases), ["anki.timing.day: holds no case"])

    def test_a_case_that_is_not_input_output_and_class_is_refused(self):
        """A1 (R1): a case without its input or its output, or with a key beyond the optional
        class, is refused, naming the case."""
        faults = [
            {"input": {}},
            {"output": {}},
            {"input": {}, "output": {}, "note": "x"},
            ["input", "output"],
        ]
        for case in examined("cases", faults):
            with self.subTest(case=case):
                self.assertEqual(
                    self.planted(cases=[{"input": {}, "output": {}}, case]),
                    ["anki.timing.day: case 1 is not input, output and an optional class"],
                )


class ProvenanceFaults(unittest.TestCase):
    def test_a_golden_naming_a_file_not_committed_is_refused(self):
        """A1 (R7, T-G13): a golden whose generator names no committed file is refused by name,
        and nothing is read in its place."""
        registry = f"{generate.PUBLISHED_REGISTRY}/spec_498.py"
        golden = {
            "generator": "tools/parity-oracle/anki/no-such-driver.py",
            "generator_sha256": "ab" * 32,
            "registry": registry,
            "registry_sha256": generate.sha256(ROOT / registry),
        }
        self.assertEqual(
            generate.provenance_refusals("anki.timing.day", golden, ROOT),
            [
                "anki.timing.day: generator 'tools/parity-oracle/anki/no-such-driver.py' "
                "is not a committed file"
            ],
        )

    def test_a_registry_digest_that_differs_is_refused(self):
        """A1 (R7, T-G13): a golden whose registry_sha256 is not the committed registry
        module's digest is refused, as one whose generator digest differs is."""
        registry = f"{generate.PUBLISHED_REGISTRY}/spec_498.py"
        golden = {
            "generator": generate.GENERATOR,
            "generator_sha256": generate.sha256(ROOT / generate.GENERATOR),
            "registry": registry,
            "registry_sha256": generate.sha256(ROOT / registry),
        }
        self.assertEqual(generate.provenance_refusals("anki.timing.day", golden, ROOT), [])
        planted = dict(golden, registry_sha256="0" * 64)
        self.assertEqual(
            generate.provenance_refusals("anki.timing.day", planted, ROOT),
            [f"anki.timing.day: registry_sha256 differs from the committed {registry}"],
        )


class Protocol(unittest.TestCase):
    def test_a_harness_that_cannot_start_fails_the_run(self):
        """A1 (R11): a harness that cannot start fails the run with the reason, not a trace."""
        with tempfile.TemporaryDirectory() as scratch:
            registered = generate.load_registry(small_registry(scratch))
            seeds = {m: generate.load_module(m).SEED for m, _ in registered.values()}
            missing = Path(scratch) / "no-such-harness"
            with self.assertRaisesRegex(generate.RunFailed, "the harness cannot start"):
                generate.run([str(missing)], registered, seeds.__getitem__, "ab" * 32)

    def test_an_answer_that_is_not_an_object_fails_the_run(self):
        """A1 (R11): a call answered with a JSON value that is not an object fails the run."""
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(generate.RunFailed, "answered call with list"):
                run_small(scratch, "--fault", "not-object")

    def test_a_hello_with_other_keys_fails_the_run(self):
        """A1 (R11): a hello that carries a key beyond the engine commit and the stock texts
        fails the run."""
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(generate.RunFailed, "hello carries"):
                run_small(scratch, "--fault", "hello-keys")

    def test_stock_texts_that_are_not_a_list_of_texts_fail_the_run(self):
        """A1 (R11) and A5 (R6): stock texts sent as one text, not a list of texts, fail the
        run rather than reach the census character by character."""
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(generate.RunFailed, "stock_texts is not a list of texts"):
                run_small(
                    scratch, "--fault", "stock-type", "--stock", "A line of the engine's own text"
                )


class Main(unittest.TestCase):
    def test_a_call_missing_a_required_argument_is_refused(self):
        """A1 (R11): main refuses (exit 2) a call missing any of the four required arguments,
        before it reads a source."""
        required = ["--harness", "--harness-source", "--engine-source", "--out"]
        for missing in examined("required arguments", required):
            with self.subTest(missing=missing), tempfile.TemporaryDirectory() as scratch:
                source = Path(scratch) / "harness"
                source.mkdir()
                (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
                out = Path(scratch) / "out"
                given = {
                    "--harness": str(STAND_IN),
                    "--harness-source": str(source),
                    "--engine-source": str(source),
                    "--out": str(out),
                }
                del given[missing]
                argv = ["--registry", str(small_registry(scratch))]
                for option, value in given.items():
                    argv += [option, value]
                with contextlib.redirect_stderr(io.StringIO()) as err:
                    code = generate.main(argv, pin="0" * 64)
                self.assertEqual(code, 2)
                self.assertIn(
                    "--harness, --harness-source, --engine-source and --out are required",
                    err.getvalue(),
                )
                self.assertFalse(out.exists())

    def test_a_run_that_fails_exits_3_and_writes_nothing(self):
        """A1 (R11): a run that fails after the harness starts, here on a harness of another
        engine, exits 3 and makes no output folder."""
        with tempfile.TemporaryDirectory() as scratch:
            code, err, out, _ = drive(scratch, "--commit", "1" * 40)
            self.assertEqual(code, 3)
            self.assertIn("not the pinned", err)
            self.assertFalse(out.exists())

    def test_the_written_goldens_record_the_registry_seed(self):
        """A1 (R1): each golden main writes records its registry module's integer SEED, and its
        inputs are the ones that seed draws."""
        with tempfile.TemporaryDirectory() as scratch:
            code, _, out, registry = drive(scratch)
            self.assertEqual(code, 0)
            module = generate.load_module(registry / "spec_498.py")
            for path in examined("goldens", sorted((out / "goldens").glob("*.json"))):
                golden = json.loads(path.read_text(encoding="utf-8"))
                name = golden["function"]
                drawn = generate.covered(name, module.FUNCTIONS[name], module.SEED)
                inputs = [json.loads(json.dumps(arguments)) for _, arguments in drawn]
                self.assertEqual(golden["seed"], module.SEED, name)
                self.assertEqual([case["input"] for case in golden["cases"]], inputs, name)

    def test_the_written_goldens_record_the_driver_digest(self):
        """A1 (R7, T-G13): each golden main writes carries the digest of the driver file that
        wrote it and of the registry module that registered it."""
        with tempfile.TemporaryDirectory() as scratch:
            code, _, out, registry = drive(scratch)
            self.assertEqual(code, 0)
            driver = generate.sha256(generate.HERE)
            module = generate.sha256(registry / "spec_498.py")
            for path in examined("goldens", sorted((out / "goldens").glob("*.json"))):
                golden = json.loads(path.read_text(encoding="utf-8"))
                self.assertEqual(golden["generator_sha256"], driver, path.name)
                self.assertEqual(golden["registry_sha256"], module, path.name)

    def test_the_run_writes_no_bytecode_beside_what_it_loads(self):
        """A1 (R11): the run never writes in a tree it reads, not even bytecode: the registry
        main loads is left with no cache beside it, even where the interpreter would write one."""
        with tempfile.TemporaryDirectory() as scratch:
            registry = small_registry(scratch)
            source = Path(scratch) / "harness"
            source.mkdir()
            (source / "main.rs").write_text("fn main() {}\n", encoding="utf-8")
            argv = [
                "--harness",
                str(STAND_IN),
                "--harness-source",
                str(source),
                "--engine-source",
                str(source),
                "--registry",
                str(registry),
                "--out",
                str(Path(scratch) / "out"),
            ]
            with (
                mock.patch.object(sys, "dont_write_bytecode", False),
                mock.patch.object(sys, "pycache_prefix", None),
                mock.patch.object(generate, "writable_refusals", return_value=[]),
                contextlib.redirect_stderr(io.StringIO()) as err,
            ):
                code = generate.main(argv, pin="0" * 64)
                guarded = sys.dont_write_bytecode
            self.assertEqual(code, 2)
            self.assertIn("is not the pinned", err.getvalue())
            self.assertTrue(guarded)
            self.assertEqual(sorted(p.name for p in registry.iterdir()), ["spec_498.py"])


class Draw(unittest.TestCase):
    def test_an_input_that_is_not_an_object_is_refused(self):
        """A1 (R1): a case's input is a JSON object, so a golden can hold it as one."""
        for arguments in examined("inputs", [[1, 2], "now", 7, None]):
            with self.subTest(arguments=arguments):
                with self.assertRaisesRegex(generate.Refused, "not an object"):
                    generate.draw("anki.timing.day", lambda rng, a=arguments: [(None, a)], 498)

    def test_an_input_that_is_not_strict_json_is_refused(self):
        """A1 (R1): an input holding NaN or an infinity is not strict JSON and is refused."""
        for value in examined("values", [math.inf, -math.inf, math.nan]):
            with self.subTest(value=value):
                with self.assertRaisesRegex(generate.Refused, "not strict JSON"):
                    generate.draw("anki.timing.day", lambda rng, v=value: [(None, {"x": v})], 498)


if __name__ == "__main__":
    unittest.main()
