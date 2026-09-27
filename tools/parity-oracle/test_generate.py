"""The oracle's generator writes strict, provenance-carrying goldens (SPEC-002 A4)."""

import importlib.util
import json
import random
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("parity_generate", HERE / "generate.py")
generate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(generate)

STAND_IN = '''
def review_xp(ease, maturity):
    return round(10 * ease * maturity)

def bad(value):
    return float("nan")
'''


def cases(rng: random.Random):
    return [("tie", {"ease": 0.5, "maturity": 1.3})] + [
        (None, {"ease": rng.choice([0.5, 1.0, 1.2]), "maturity": 2.0}) for _ in range(3)
    ]


class GeneratorWritesStrictGoldens(unittest.TestCase):
    def setUp(self):
        self.tmp = Path(tempfile.mkdtemp())
        pkg = self.tmp / "v9stand"
        pkg.mkdir()
        (pkg / "__init__.py").write_text("")
        (pkg / "xp.py").write_text(STAND_IN)
        subprocess.run(["git", "init", "-q", str(self.tmp)], check=True)
        subprocess.run(["git", "-C", str(self.tmp), "add", "-A"], check=True)
        subprocess.run(
            ["git", "-C", str(self.tmp), "-c", "user.name=t", "-c", "user.email=t@example.com",
             "commit", "-qm", "stand-in"],
            check=True,
        )
        self.saved = dict(generate.FUNCTIONS)
        generate.FUNCTIONS.clear()

    def tearDown(self):
        generate.FUNCTIONS.clear()
        generate.FUNCTIONS.update(self.saved)
        for name in [m for m in sys.modules if m.startswith("v9stand")]:
            del sys.modules[name]

    def test_a_golden_records_its_provenance_and_rounds_half_to_even(self):
        generate.FUNCTIONS["v9stand.xp.review_xp"] = cases
        out = self.tmp / "goldens"
        self.assertEqual(generate.main(["--source-checkout", str(self.tmp), "--out", str(out)]), 0)
        golden = json.loads((out / "review_xp.json").read_text())
        self.assertEqual(golden["schema"], "phx.parity-golden.v1")
        self.assertEqual(golden["inputs"], "synthetic")
        self.assertEqual(golden["seed"], 20)
        self.assertEqual(len(golden["source_commit"]), 40)
        self.assertEqual(golden["generator_sha256"], generate.hashlib.sha256(
            (HERE / "generate.py").read_bytes()).hexdigest())
        # Python's round() sends 6.5 to 6: the tie a port using f64::round would get wrong.
        self.assertEqual(golden["cases"][0], {"class": "tie", "input": {"ease": 0.5, "maturity": 1.3}, "output": 6})

    def test_a_non_json_number_is_refused(self):
        generate.FUNCTIONS["v9stand.xp.bad"] = lambda rng: [(None, {"value": 1})]
        out = self.tmp / "goldens"
        with self.assertRaises(ValueError):
            generate.main(["--source-checkout", str(self.tmp), "--out", str(out)])


if __name__ == "__main__":
    unittest.main()
