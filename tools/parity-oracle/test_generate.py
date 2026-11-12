"""The generator writes strict goldens from a registry kept per SPEC (SPEC-029, SPEC-002 A4).

Every test builds a synthetic stand-in of the predecessor and its own registry in a temporary
directory, and removes both when it ends (R11). No test imports the predecessor.
"""

import contextlib
import datetime as dt
import hashlib
import importlib.util
import io
import json
import platform
import re
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("parity_generate", HERE / "generate.py")
generate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generate)


def examined(what, items):
    """Print how many items a check examined and refuse zero (the tdd pack's contract)."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


#: The stand-in predecessor: a package the tests reach through the registry, as the real one is.
STAND_IN = {
    "__init__.py": "",
    "xp.py": """
        def review_xp(ease, maturity):
            return round(10 * ease * maturity)

        def bad(value):
            return float("nan")
    """,
    "day.py": """
        from dataclasses import dataclass
        from datetime import date, datetime, timedelta, timezone

        @dataclass(frozen=True)
        class Clock:
            offset_minutes: int

        def local_day(instant_ms, clock):
            zone = timezone(timedelta(minutes=clock.offset_minutes))
            return datetime.fromtimestamp(instant_ms / 1000, zone).date()

        def epoch_plus(days):
            return date(1970, 1, 1) + timedelta(days=days)

        def instant(instant_ms):
            return datetime.fromtimestamp(instant_ms / 1000, timezone.utc)
    """,
    "constants.py": """
        ROLLOVER_HOUR = 4
        RETRY_ATTEMPTS = 3

        class Limits:
            LOSS_CAP = 90
    """,
}

REVIEW_XP = """
    def cases(rng):
        return [("tie", {"ease": 0.5, "maturity": 1.3})] + [
            (None, {"ease": rng.choice([0.5, 1.0, 1.2]), "maturity": 2.0}) for _ in range(3)
        ]

    FUNCTIONS = {"review_xp": {"kind": "function", "function": "xp.review_xp", "cases": cases}}
"""

LOCAL_DAY_NOTE = "Builds a Clock from offset_minutes and passes instant_ms through unchanged."
LOCAL_DAY = """
    def with_clock(local_day, predecessor, *, instant_ms, offset_minutes):
        return local_day(instant_ms, predecessor("day.Clock")(offset_minutes=offset_minutes))

    def cases(rng):
        return [("offset", {"instant_ms": 0, "offset_minutes": -60}),
                (None, {"instant_ms": 0, "offset_minutes": 0})]

    FUNCTIONS = {
        "local_day": {
            "kind": "adapter",
            "function": "day.local_day",
            "adapter": with_clock,
            "note": "Builds a Clock from offset_minutes and passes instant_ms through unchanged.",
            "cases": cases,
        },
    }
"""

CONSTANTS = """
    FUNCTIONS = {
        "stand.constants": {
            "kind": "constants",
            "names": ["constants.ROLLOVER_HOUR", "constants.RETRY_ATTEMPTS",
                      "constants.Limits.LOSS_CAP"],
        },
    }
"""

EPOCH_PLUS = """
    def cases(rng):
        return [(None, {"days": 1}), ("negative", {"days": -1}), (None, {"days": 20000})]

    FUNCTIONS = {"epoch_plus": {"kind": "function", "function": "day.epoch_plus", "cases": cases}}
"""

INSTANT = """
    def cases(rng):
        return [(None, {"instant_ms": 1}), ("negative", {"instant_ms": -1}),
                (None, {"instant_ms": 1234567})]

    FUNCTIONS = {"instant": {"kind": "function", "function": "day.instant", "cases": cases}}
"""

NEEDS = """
    def cases(rng):
        return [(None, {})]

    FUNCTIONS = {"needs": {"kind": "function", "function": "needs.f", "cases": cases}}
"""

ONE_XP = """
    def cases(rng):
        return [(None, {"ease": 1.0, "maturity": 2.0})]

    FUNCTIONS = {"one_xp": {"kind": "function", "function": "xp.review_xp", "cases": cases}}
"""

#: One NaN object, so a builder's two draws compare equal and only strict JSON can refuse it.
NAN = float("nan")


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


class GeneratorWritesGoldensFromTheRegistry(unittest.TestCase):
    def setUp(self):
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        self.addCleanup(self.forget_stand_in)
        self.tmp = Path(scratch.name)
        self.checkout = self.tmp / "checkout"
        package = self.checkout / "v9stand"
        package.mkdir(parents=True)
        for name, source in STAND_IN.items():
            (package / name).write_text(textwrap.dedent(source), encoding="utf-8")
        subprocess.run(["git", "init", "-q", str(self.checkout)], check=True)
        subprocess.run(["git", "-C", str(self.checkout), "add", "-A"], check=True)
        subprocess.run(
            [
                "git",
                "-C",
                str(self.checkout),
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@example.com",
                "commit",
                "-qm",
                "stand-in",
            ],
            check=True,
        )
        self.commit = subprocess.run(
            ["git", "-C", str(self.checkout), "rev-parse", "HEAD"],
            capture_output=True,
            text=True,
            check=True,
        ).stdout.strip()
        self.registry = self.tmp / "registry"
        self.registry.mkdir()
        self.out = self.tmp / "goldens"

    @staticmethod
    def forget_stand_in():
        for name in [m for m in sys.modules if m == "v9stand" or m.startswith("v9stand.")]:
            del sys.modules[name]

    def register(self, module: str, source: str) -> Path:
        path = self.registry / module
        path.write_text(textwrap.dedent(source), encoding="utf-8")
        return path

    def generate(self) -> tuple[int, str, str]:
        stdout, stderr = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(stdout), contextlib.redirect_stderr(stderr):
            code = generate.main(
                [
                    "--source-checkout",
                    str(self.checkout),
                    "--package",
                    "v9stand",
                    "--registry",
                    str(self.registry),
                    "--out",
                    str(self.out),
                ]
            )
        return code, stdout.getvalue(), stderr.getvalue()

    def golden(self, name: str) -> dict:
        path = self.out / f"{name}.json"
        self.assertTrue(path.is_file(), f"the generator wrote no golden {path.name}")
        return json.loads(path.read_text(encoding="utf-8"))

    def test_a_golden_records_its_provenance_and_rounds_half_to_even(self):
        self.register("spec_001.py", REVIEW_XP)
        code, stdout, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("review_xp")
        self.assertIn("review_xp.json", stdout)
        self.assertEqual(golden["schema"], "phx.parity-golden.v1")
        self.assertEqual(golden["kind"], "function")
        self.assertEqual(golden["function"], "xp.review_xp")
        self.assertEqual(golden["inputs"], "synthetic")
        self.assertEqual(golden["seed"], 20)
        self.assertEqual(golden["source_commit"], self.commit)
        # Python's round() sends 6.5 to 6: the tie a port using f64::round would get wrong.
        self.assertEqual(
            golden["cases"][0],
            {"class": "tie", "input": {"ease": 0.5, "maturity": 1.3}, "output": 6},
        )

    def test_a_golden_records_the_digests_of_its_generator_and_registry(self):
        module = self.register("spec_001.py", REVIEW_XP)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("review_xp")
        self.assertEqual(golden["generator"], "tools/parity-oracle/generate.py")
        self.assertEqual(golden["generator_sha256"], sha256(HERE / "generate.py"))
        self.assertEqual(golden["registry"], "tools/parity-oracle/registry/spec_001.py")
        self.assertEqual(golden["registry_sha256"], sha256(module))
        # Strict and canonical: sorted keys, a two-space indent, one final newline, no NaN.
        self.assertEqual(
            (self.out / "review_xp.json").read_text(encoding="utf-8"),
            json.dumps(golden, indent=2, sort_keys=True, allow_nan=False) + "\n",
        )

    def test_a_non_json_number_is_refused(self):
        self.register(
            "spec_001.py",
            """
            FUNCTIONS = {"bad": {"kind": "function", "function": "xp.bad",
                                 "cases": lambda rng: [(None, {"value": 1})]}}
        """,
        )
        with self.assertRaises(ValueError):
            self.generate()

    def test_two_registrations_of_one_golden_name_are_refused(self):
        self.register("spec_001.py", REVIEW_XP)
        self.register("spec_002.py", REVIEW_XP)
        code, _, stderr = self.generate()
        self.assertEqual(code, 2)
        self.assertIn("registry/spec_001.py", stderr)
        self.assertIn("registry/spec_002.py", stderr)
        self.assertFalse((self.out / "review_xp.json").exists())

    def test_an_adapter_golden_names_the_function_it_drives(self):
        self.register("spec_001.py", LOCAL_DAY)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("local_day")
        self.assertEqual(golden["kind"], "adapter")
        self.assertEqual(golden["function"], "day.local_day")
        self.assertEqual(golden["adapter"], "with_clock")
        interpreter = f"{platform.python_implementation()} {platform.python_version()}"
        self.assertEqual(golden["note"], f"{LOCAL_DAY_NOTE[:-1]}; cases drawn under {interpreter}.")
        # The stand-in's day at instant 0: the last day before the epoch at UTC-1, day 0 at UTC.
        self.assertEqual([case["output"] for case in golden["cases"]], [-1, 0])
        self.assertEqual(golden["cases"][0]["class"], "offset")

    def test_a_constants_golden_holds_each_named_value(self):
        self.register("spec_001.py", CONSTANTS)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("stand.constants")
        self.assertEqual(golden["kind"], "constants")
        self.assertEqual(golden["function"], "stand.constants")
        self.assertEqual(
            golden["cases"],
            [
                {"input": {"name": "constants.ROLLOVER_HOUR"}, "output": 4},
                {"input": {"name": "constants.RETRY_ATTEMPTS"}, "output": 3},
                {"input": {"name": "constants.Limits.LOSS_CAP"}, "output": 90},
            ],
        )

    def test_a_returned_date_is_written_as_its_epoch_day(self):
        self.register("spec_001.py", EPOCH_PLUS)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("epoch_plus")
        self.assertEqual([case["output"] for case in golden["cases"]], [1, -1, 20000])

    def test_a_returned_datetime_is_written_as_epoch_milliseconds(self):
        self.register("spec_001.py", INSTANT)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        golden = self.golden("instant")
        self.assertEqual([case["output"] for case in golden["cases"]], [1, -1, 1234567])

    def test_a_registration_of_an_unknown_kind_is_refused(self):
        self.register(
            "spec_001.py",
            """
            FUNCTIONS = {"review_xp": {"kind": "formula", "function": "xp.review_xp"}}
        """,
        )
        code, _, stderr = self.generate()
        self.assertEqual(code, 2)
        self.assertIn("registry/spec_001.py", stderr)
        self.assertIn("'formula'", stderr)

    def test_a_registry_module_not_named_for_its_spec_is_refused(self):
        self.register("helpers.py", REVIEW_XP)
        code, _, stderr = self.generate()
        self.assertEqual(code, 2)
        self.assertIn("registry/helpers.py", stderr)

    def test_a_builder_that_draws_outside_its_seed_is_refused(self):
        self.register(
            "spec_001.py",
            """
            import random

            def cases(rng):
                return [(None, {"ease": random.random(), "maturity": 1.0})]

            FUNCTIONS = {"review_xp": {"kind": "function", "function": "xp.review_xp",
                                       "cases": cases}}
        """,
        )
        with self.assertRaisesRegex(ValueError, "review_xp"):
            self.generate()

    def test_the_generator_writes_no_bytecode_into_the_checkout(self):
        self.register("spec_001.py", REVIEW_XP)
        code, _, _ = self.generate()
        self.assertEqual(code, 0)
        self.assertEqual(self.golden("review_xp")["function"], "xp.review_xp")
        # The stand-in's modules all live in its one package, so bytecode could land only there.
        self.assertFalse((self.checkout / "v9stand" / "__pycache__").exists())

    def test_a_dependency_the_predecessor_lacks_is_raised_by_its_name(self):
        """A11: a predecessor module that imports a module nobody has raises that module's name,
        never the generator's own refusal that no module holds the path."""
        (self.checkout / "v9stand" / "needs.py").write_text(
            "import parity_missing_dependency\n\n\ndef f():\n    return 1\n", encoding="utf-8"
        )
        self.register("spec_001.py", NEEDS)
        with self.assertRaises(ModuleNotFoundError) as raised:
            self.generate()
        self.assertEqual(
            (raised.exception.name, str(raised.exception)),
            ("parity_missing_dependency", "No module named 'parity_missing_dependency'"),
        )

    def test_the_checkout_is_imported_before_any_other_path_and_gets_no_bytecode(self):
        """A16: a package of the stand-in's name ahead of every other path loses to the checkout,
        whose `review_xp` the golden records; and with the bytecode prefix cleared and bytecode
        on, the generator still writes nothing into the checkout's package."""
        decoy = self.tmp / "decoy" / "v9stand"
        decoy.mkdir(parents=True)
        (decoy / "__init__.py").write_text("", encoding="utf-8")
        (decoy / "xp.py").write_text(
            "def review_xp(ease, maturity):\n    return -1\n", encoding="utf-8"
        )
        self.register("spec_001.py", ONE_XP)
        with (
            mock.patch.object(sys, "path", [str(decoy.parent), *sys.path]),
            mock.patch.object(sys, "pycache_prefix", None),
            mock.patch.object(sys, "dont_write_bytecode", False),
        ):
            code, _, _ = self.generate()
        self.assertEqual(code, 0)
        self.assertEqual(
            self.golden("one_xp")["cases"],
            [{"input": {"ease": 1.0, "maturity": 2.0}, "output": 20}],
        )
        self.assertEqual(
            sorted(path.name for path in (self.checkout / "v9stand").iterdir()),
            ["__init__.py", "constants.py", "day.py", "xp.py"],
        )


class EachOperandOfTheGeneratorIsHeld(unittest.TestCase):
    """SPEC-437 R6 (A9, A10 and A12 to A15): the default registry, each refusal of a
    registration, a date nested in a value, a case's class, a NaN input, and the two required
    flags with every help text, each observed through what the generator returns, raises or
    prints. No test reads the generator's own bytes."""

    def test_the_default_registry_names_every_committed_golden(self):
        """A9: the registry the generator reads by default registers exactly the goldens
        committed beside it, one golden file per registered name; the engine's goldens are the
        second oracle's (SPEC-498), held by test_goldens.py."""
        committed = sorted(
            path.stem
            for path in (HERE / "goldens").glob("*.json")
            if not path.name.startswith("anki.")
        )
        registered = sorted(generate.load_registry(generate.REGISTRY))
        self.assertEqual(registered, examined("committed goldens", committed))

    def test_a_registration_is_refused_for_each_operand_alone(self):
        """A10: each operand of `check`'s conditions alone refuses a registration that is whole
        but for it, by the refusal's whole message; the whole registrations are the control."""
        where = "registry/spec_001.py"

        def build(rng):
            return [(None, {})]

        def glue(function, predecessor, **given):
            return function(**given)

        function = {"kind": "function", "function": "xp.review_xp", "cases": build}
        adapter = {
            "kind": "adapter",
            "function": "day.local_day",
            "adapter": glue,
            "note": "Builds a Clock.",
            "cases": build,
        }
        constants = {"kind": "constants", "names": ["constants.ROLLOVER_HOUR"]}
        named = f"{where}: x names no attribute"
        builder = f"{where}: x needs a predecessor function and a case builder"
        glued = f"{where}: x needs a callable adapter and a note"
        cases = (
            (
                "a bad name string",
                "Not A Name",
                function,
                f"{where}: 'Not A Name' is not a golden name (a-z, 0-9, '_' and '.')",
            ),
            ("an empty names list", "x", {**constants, "names": []}, named),
            ("a names list holding a non-string", "x", {**constants, "names": ["a", 1]}, named),
            ("a names string", "x", {**constants, "names": "abc"}, named),
            ("a function that is not a string", "x", {**function, "function": 5}, builder),
            ("cases that are not callable", "x", {**function, "cases": 5}, builder),
            ("an adapter that is not callable", "x", {**adapter, "adapter": 5}, glued),
            ("a note that is not a string", "x", {**adapter, "note": 5}, glued),
            ("a blank note", "x", {**adapter, "note": "   "}, glued),
        )
        for label, name, registration, refusal in examined("refused operands", cases):
            with self.subTest(label):
                with self.assertRaises(generate.RegistryError) as refused:
                    generate.check(where, name, registration)
                self.assertEqual(str(refused.exception), refusal)
        for registration in (function, adapter, constants):
            self.assertIsNone(generate.check(where, "x", registration))

    def test_a_date_inside_a_dict_or_a_list_is_written_as_its_epoch_day(self):
        """A12: a date is its epoch day wherever it sits, as a dict's value and as a list's or a
        tuple's item, and the containers keep their keys and order."""
        value = {
            "day": dt.date(1970, 1, 2),
            "days": [dt.date(1969, 12, 31), (dt.date(1970, 1, 1), 7)],
        }
        self.assertEqual(generate.plain(value), {"day": 1, "days": [-1, [0, 7]]})

    def test_a_case_whose_class_is_the_empty_name_is_refused(self):
        """A13: a case's class is a name or None; the empty name is refused by name, and a named
        and an unnamed case are the control."""

        def empty(rng):
            return [("", {"ease": 1.0})]

        def named(rng):
            return [("tie", {"ease": 1.0}), (None, {"ease": 2.0})]

        with self.assertRaises(ValueError) as refused:
            generate.draw("x", empty)
        self.assertEqual(str(refused.exception), "x: a case's class is '', not a name or None")
        self.assertEqual(generate.draw("x", named), [("tie", {"ease": 1.0}), (None, {"ease": 2.0})])

    def test_a_case_input_holding_nan_is_refused_before_the_predecessor_runs(self):
        """A14: a NaN in a case's input is refused as strict JSON refuses it, after the
        predecessor's function is resolved and before it is called."""
        resolved, ran = [], []

        def function(**given):
            ran.append(given)
            return 0

        def predecessor(path):
            resolved.append(path)
            return function

        def build(rng):
            return [(None, {"value": NAN})]

        registration = {"kind": "function", "function": "xp.bad", "cases": build}
        with self.assertRaises(ValueError) as refused:
            generate.cases_of("bad", registration, predecessor)
        self.assertEqual(
            (str(refused.exception).split(":")[0], resolved, ran),
            ("Out of range float values are not JSON compliant", ["xp.bad"], []),
        )

    def test_main_requires_its_checkout_and_package_and_states_each_help_text(self):
        """A15: without `--source-checkout`, and without `--package`, `main` exits 2 naming the
        flag before it reads any registry; its help states each flag's help text."""
        scratch = tempfile.TemporaryDirectory()
        self.addCleanup(scratch.cleanup)
        empty = Path(scratch.name)
        rest = ["--registry", str(empty), "--out", str(empty / "goldens")]
        cases = (
            (["--package", "v9stand", *rest], "--source-checkout"),
            (["--source-checkout", str(empty), *rest], "--package"),
        )
        for argv, flag in examined("required flags", cases):
            with self.subTest(flag):
                err = io.StringIO()
                with contextlib.redirect_stderr(err), self.assertRaises(SystemExit) as stopped:
                    generate.main(argv)
                said = err.getvalue().splitlines()[-1].split(": error: ", 1)[-1]
                self.assertEqual(
                    (stopped.exception.code, said),
                    (2, f"the following arguments are required: {flag}"),
                )
        out = io.StringIO()
        with contextlib.redirect_stdout(out), self.assertRaises(SystemExit) as helped:
            generate.main(["--help"])
        options = " ".join(out.getvalue().split()).split(" show this help message and exit ")[-1]
        stated = re.fullmatch(
            r"--source-checkout SOURCE_CHECKOUT (.+) --package PACKAGE (.+) "
            r"--registry REGISTRY (.+) --out OUT (.+)",
            options,
        )
        texts = (
            "the predecessor's private checkout",
            "the predecessor's top-level package, which registry paths are relative to",
            "the registry directory (default: registry/)",
            "the goldens directory",
        )
        self.assertEqual(
            (helped.exception.code, stated and stated.groups()),
            (0, tuple(examined("help texts", texts))),
        )


if __name__ == "__main__":
    unittest.main()
