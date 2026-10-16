"""The gate's ruff is one pinned, hashed release, and `ruff.toml` holds the settled rule set
(SPEC-424 A1 and A2). The pin file names one `ruff==<version>` requirement and a sha256 for every
wheel of that release, and `ruff.toml` requires exactly that version, so a ruff off its pin refuses
the configuration wherever it runs. The `[lint]` table adds the settled families and rules to ruff's
default set, with its two kinds of ignore, its per-file table and its banned names."""

import re
import tomllib
import unittest

from _support import REPO, examined

CONFIG = REPO / "ruff.toml"
PIN_FILE = REPO / "scripts" / "ruff-requirements.txt"
# The pin's requirement line, its hash lines and the last of them (SPEC-424 R1).
REQUIREMENT = re.compile(r"^ruff==([0-9]+(?:\.[0-9]+)*) \\$")
HASH_LINE = re.compile(r"^    --hash=sha256:([0-9a-f]{64}) \\$")
LAST_HASH_LINE = re.compile(r"^    --hash=sha256:([0-9a-f]{64})$")
# The settled selection (SPEC-424 R2, ADR-438 D2), each list in the order ruff.toml writes it.
EXTEND_SELECT = ["S", "B", "ASYNC", "PGH", "PLE", "FIX", "T10", "DTZ", "RUF034", "TID251"]
PERMANENT_IGNORES = ["S603", "S607", "S105", "S106", "S107"]
NOT_ADMITTED = ["S101", "S102", "S108", "S311", "B904", "B905", "B023", "B007", "B008", "DTZ001"]
PER_FILE_IGNORES = {
    "scripts/tests/**": ["S104", "S604"],
    "scripts/tests/test_mutation_rows_refusal.py": ["TID251"],
    "crates/agent/tests/fixtures/fake-probe.py": ["PLE2515"],
}
BANNED = [
    "os.system",
    "os.popen",
    "subprocess.getoutput",
    "subprocess.getstatusoutput",
    "pickle",
    "marshal",
    "shelve",
    "tempfile.mktemp",
    "telnetlib",
    "ftplib",
    "os.chdir",
]
# The parity oracle's generator and registry, whose bytes their goldens pin (SPEC-029).
EXCLUDES = ["tools/parity-oracle/generate.py", "tools/parity-oracle/registry"]


def config():
    return tomllib.loads(CONFIG.read_text(encoding="utf-8"))


def keys_named(value, names, where=""):
    """Every place in a parsed TOML document whose key is one of `names`, at any depth."""
    found = []
    if isinstance(value, dict):
        for key, item in value.items():
            place = f"{where}.{key}" if where else key
            if key in names:
                found.append(place)
            found += keys_named(item, names, place)
    return found


class TheRuffPinIsOneHashedRelease(unittest.TestCase):
    def test_ruff_toml_requires_exactly_the_version_the_hashed_pin_names(self):
        settings = config()
        self.assertIn("required-version", settings, "ruff.toml requires no ruff version")
        lines = [
            line
            for line in PIN_FILE.read_text(encoding="utf-8").splitlines()
            if line.strip() and not line.startswith("#")
        ]
        requirement = REQUIREMENT.match(lines[0] if lines else "")
        self.assertIsNotNone(requirement, f"the pin's first line is {lines[:1]}, not ruff==<v> \\")
        hashes = lines[1:]
        self.assertTrue(hashes, "the pin hashes no wheel")
        for line in hashes[:-1]:
            self.assertRegex(line, HASH_LINE, "a hash line that is not the pin's last")
        self.assertRegex(hashes[-1], LAST_HASH_LINE, "the pin's last hash line")
        digests = examined(
            "pinned wheel digests",
            [(HASH_LINE.match(line) or LAST_HASH_LINE.match(line)).group(1) for line in hashes],
        )
        self.assertEqual(len(set(digests)), len(digests), "a digest pinned twice")
        self.assertEqual(settings["required-version"], f"=={requirement.group(1)}")


class TheLintTableIsTheSettledSet(unittest.TestCase):
    def test_the_lint_table_is_exactly_the_settled_selection(self):
        settings = config()
        lint = settings.get("lint", {})
        self.assertEqual(lint.get("extend-select"), EXTEND_SELECT)
        self.assertEqual(lint.get("ignore"), PERMANENT_IGNORES + NOT_ADMITTED)
        self.assertEqual(lint.get("per-file-ignores"), PER_FILE_IGNORES)
        banned = lint.get("flake8-tidy-imports", {}).get("banned-api", {})
        self.assertEqual(list(banned), BANNED)
        for name, entry in examined("banned names", list(banned.items())):
            with self.subTest(name=name):
                self.assertEqual(list(entry), ["msg"])
                self.assertTrue(str(entry["msg"]).strip(), f"{name} is banned with no message")
        self.assertEqual(lint.get("unfixable"), ["F401", "F841"])
        self.assertEqual(keys_named(settings, {"select", "preview"}), [])
        self.assertEqual(settings.get("extend-exclude"), EXCLUDES)
        self.assertEqual(
            sorted(lint),
            ["extend-select", "flake8-tidy-imports", "ignore", "per-file-ignores", "unfixable"],
        )
        self.assertEqual(list(lint["flake8-tidy-imports"]), ["banned-api"])


if __name__ == "__main__":
    unittest.main()
