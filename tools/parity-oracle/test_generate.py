"""The generator writes strict goldens from a registry kept per SPEC (SPEC-029, SPEC-002 A4).

Every test builds a synthetic stand-in of the predecessor and its own registry in a temporary
directory, and removes both when it ends (R11). No test imports the predecessor.
"""

import contextlib
import hashlib
import importlib.util
import io
import json
import platform
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("parity_generate", HERE / "generate.py")
generate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generate)

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


if __name__ == "__main__":
    unittest.main()
