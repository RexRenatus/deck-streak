"""Vendoring refuses excluded and deny-listed files by construction (SPEC-037 A1 to A6), and scans
with the public scrub's own composition of its rules, holding no copy of it (SPEC-054 A6).

Each test runs scripts/vendor-packs.py against a fixture built at run time in a temporary
directory: an upstream git repository with one commit, shaped like phoenix-v2 in miniature (a few
`scripts/*.py`, packs under `skills/packs/`, the subscription-proxy pack with its reference client,
and the scanner), and a DeckStreak root beside it that holds what an earlier run vendored, with its
`.packs/VENDORED.json` and `methodology.json`. The fixture's exclusions are the real manifest's, so
A1 proves them against the names upstream ships today. The planted address and the planted private
literal are assembled at run time, so this file itself holds neither. Nothing here reads phoenix-v2.
"""

import ast
import hashlib
import importlib.util
import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

SCRIPT = REPO / "scripts" / "vendor-packs.py"
SCRUB = REPO / "scripts" / "public-scrub.py"
REAL_MANIFEST = REPO / ".packs" / "VENDORED.json"
# The packs whose deny lists the scrub composes: a second composition would have to name them.
COMPOSED_PACKS = ("persona-core", "privacy-gdpr")
# Assembled at run time, so this file carries no address and no literal for the scrub to find.
PLANTED_ADDRESS = ".".join(["10", "20", "30", "40"])
PLANTED_LITERAL = "-".join(["xq7", "vendor", "mark"])
# A compiled-cache shape: a NUL byte and bytes that are not UTF-8.
BINARY = bytes([0, 255, 254]) + b"compiled" + bytes(8)
# The commit the fixture's earlier run recorded.
EARLIER_COMMIT = "0" * 40
# Every upstream file an exclusion names carries this token, so a copy under any name is found.
TOKEN = "excluded-upstream-file"
# Temporary repositories are isolated from the machine's git configuration, hooks and ignores.
GIT = [
    "git",
    "-c",
    "user.name=vendor-test",
    "-c",
    "user.email=vendor-test@example.invalid",
    "-c",
    "commit.gpgsign=false",
    "-c",
    "core.hooksPath=/dev/null",
    "-c",
    "core.excludesFile=/dev/null",
    "-c",
    "core.fileMode=true",
    "-c",
    "init.defaultBranch=main",
]

# The upstream commit.
UPSTREAM = {
    "scripts/alpha-probe.py": "print('alpha probe, version 2')\n",
    "scripts/sdd-probe.py": "print('sdd probe')\n",
    "scripts/unlisted-tool.py": "print('a tool DeckStreak does not vendor')\n",
    "scripts/proxy-client-scan.py": f"{TOKEN}: the scanner, which names {PLANTED_LITERAL}\n",
    "skills/packs/alpha/SKILL.md": "# alpha\n",
    "skills/packs/alpha/checks.json": '{\n  "rows": ["alpha-one", "alpha-two"]\n}\n',
    "skills/packs/alpha/rows/new-row.md": "a row alpha gained upstream\n",
    "skills/packs/alpha/run-rows.sh": "#!/bin/sh\necho alpha rows\n",
    "skills/packs/alpha/examples/demo.md": f"{TOKEN}: a worked example\n",
    "skills/packs/beta/SKILL.md": "# beta\n",
    "skills/packs/gamma/SKILL.md": "# gamma, a pack DeckStreak has not vendored\n",
    "skills/packs/subscription-proxy/SKILL.md": f"{TOKEN}: the proxy pack's body\n",
    "skills/packs/subscription-proxy/checks.json": f"{TOKEN}: the proxy pack's rows\n",
    "skills/packs/subscription-proxy/client/run-headless.sh": (
        f"{TOKEN}: the reference client, which names {PLANTED_LITERAL}\n"
    ),
}
EXECUTABLE = {"skills/packs/alpha/run-rows.sh"}
EXCLUDED = sorted(source for source, text in UPSTREAM.items() if text.startswith(TOKEN))

# What the fixture's earlier run vendored: destination -> (source path, the bytes it copied).
EARLIER = {
    ".packs/scripts/alpha-probe.py": (
        "scripts/alpha-probe.py",
        "print('alpha probe, version 1')\n",
    ),
    "scripts/sdd-probe.py": ("scripts/sdd-probe.py", "print('sdd probe')\n"),
    ".packs/skills/packs/alpha/SKILL.md": ("skills/packs/alpha/SKILL.md", "# alpha\n"),
    ".packs/skills/packs/alpha/checks.json": (
        "skills/packs/alpha/checks.json",
        '{\n  "rows": ["alpha-one"]\n}\n',
    ),
    ".packs/skills/packs/beta/SKILL.md": ("skills/packs/beta/SKILL.md", "# beta\n"),
}
# Listed, as the old vendoring's own lists named them, though an exclusion now matches them.
LISTED_EXCLUDED = {
    ".packs/scripts/proxy-client-scan.py": "scripts/proxy-client-scan.py",
    ".packs/skills/packs/subscription-proxy/checks.json": (
        "skills/packs/subscription-proxy/checks.json"
    ),
}
# What a clean run vendors from UPSTREAM over EARLIER: destination -> source path.
CLEAN = {
    ".packs/scripts/alpha-probe.py": "scripts/alpha-probe.py",
    "scripts/sdd-probe.py": "scripts/sdd-probe.py",
    ".packs/skills/packs/alpha/SKILL.md": "skills/packs/alpha/SKILL.md",
    ".packs/skills/packs/alpha/checks.json": "skills/packs/alpha/checks.json",
    ".packs/skills/packs/alpha/rows/new-row.md": "skills/packs/alpha/rows/new-row.md",
    ".packs/skills/packs/alpha/run-rows.sh": "skills/packs/alpha/run-rows.sh",
    ".packs/skills/packs/beta/SKILL.md": "skills/packs/beta/SKILL.md",
}


def sha256(content):
    data = content if isinstance(content, bytes) else content.encode("utf-8")
    return hashlib.sha256(data).hexdigest()


def write(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(content if isinstance(content, bytes) else content.encode("utf-8"))


def git(*args):
    return subprocess.run([*GIT, *args], capture_output=True, check=True).stdout


def methodology(commit):
    """methodology.json in the repository's own shape, pinned to `commit`."""
    return (
        "{\n"
        f'  "vendored_from": "{commit}",\n'
        '  "sdd": {},\n'
        '  "ddd": {"dependency_prefix": "deck-streak-"},\n'
        '  "tdd": {}\n'
        "}\n"
    )


def real_exclusions():
    """The `excluded` entries of the real manifest, exactly as it records them."""
    return json.loads(REAL_MANIFEST.read_text(encoding="utf-8"))["excluded"]


def private_list(directory, literals):
    deny = Path(directory) / "private.json"
    deny.write_text(
        json.dumps(
            {
                "schema": "phx.persona.deny.v1",
                "key_markers": [],
                "patterns": [],
                "literals": list(literals),
                "journal_paths": [],
            }
        ),
        encoding="utf-8",
    )
    return deny


def lines(done):
    return done.stdout.splitlines()


def last_line(done):
    found = lines(done)
    return found[-1] if found else ""


def summary(examined_files, changed, new, excluded, commit):
    """The one line a run ends with (SPEC-037 R6)."""
    return (
        f"vendor-packs: examined {examined_files} file(s), changed {changed}, new {new}, "
        f"excluded {excluded}, from {commit}"
    )


class Fixture:
    """An upstream repository with one commit, and a DeckStreak root an earlier run vendored."""

    def __init__(self, tmp, upstream=None, earlier=None, listed_excluded=None):
        base = Path(tmp)
        self.upstream = base / "upstream"
        self.root = base / "deckstreak"
        for source, content in (UPSTREAM if upstream is None else upstream).items():
            write(self.upstream / source, content)
            if source in EXECUTABLE:
                (self.upstream / source).chmod(0o755)
        git("init", "-q", str(self.upstream))
        git("-C", str(self.upstream), "add", "-A")
        git("-C", str(self.upstream), "commit", "-q", "-m", "the upstream commit")
        self.head = git("-C", str(self.upstream), "rev-parse", "HEAD").decode("ascii").strip()
        entries = []
        for destination, (source, content) in (EARLIER if earlier is None else earlier).items():
            write(self.root / destination, content)
            entries.append({"path": destination, "from": source, "sha256": sha256(content)})
        for destination, source in (listed_excluded or {}).items():
            earlier_copy = f"an earlier copy of {source}\n"
            entries.append({"path": destination, "from": source, "sha256": sha256(earlier_copy)})
        self.manifest = {
            "schema": "deckstreak.vendored-packs.v1",
            "source": "a fixture upstream",
            "vendored_from": EARLIER_COMMIT,
            "why": "the fixture's own vendoring",
            "excluded": real_exclusions(),
            "files": sorted(entries, key=lambda entry: entry["path"]),
        }
        write(self.root / ".packs" / "VENDORED.json", json.dumps(self.manifest, indent=2) + "\n")
        write(self.root / "methodology.json", methodology(EARLIER_COMMIT))

    def vendor(self, *extra, env_deny=None):
        env = {key: value for key, value in os.environ.items() if key != "PERSONA_CORE_DENY_LIST"}
        env["PYTHONDONTWRITEBYTECODE"] = "1"
        if env_deny is not None:
            env["PERSONA_CORE_DENY_LIST"] = str(env_deny)
        command = [sys.executable, str(SCRIPT), "--source", str(self.upstream)]
        return subprocess.run(
            [*command, "--root", str(self.root), *extra],
            capture_output=True,
            text=True,
            check=False,
            env=env,
        )

    def snapshot(self):
        """Every file under the root, by path, with its bytes and whether it is executable."""
        files = examined("fixture files", sorted(p for p in self.root.rglob("*") if p.is_file()))
        return {
            path.relative_to(self.root).as_posix(): (path.read_bytes(), os.access(path, os.X_OK))
            for path in files
        }

    def manifest_now(self):
        return json.loads((self.root / ".packs" / "VENDORED.json").read_text(encoding="utf-8"))

    def landed(self):
        """Each excluded upstream file the root holds: at the path it would be vendored to, or
        under any name, found by its token."""
        at_path = {
            f".packs/{source}" for source in EXCLUDED if (self.root / ".packs" / source).exists()
        }
        by_token = {path for path, (data, _) in self.snapshot().items() if TOKEN.encode() in data}
        return sorted(at_path | by_token)


class VendoringRefusesByConstruction(unittest.TestCase):
    def test_an_excluded_upstream_file_is_never_written_into_the_tree(self):
        # Both routes the old vendoring took: files the manifest lists (the scanner, the proxy
        # pack's rows), and new files of a vendored pack (the client, an example). The client and
        # the scanner name a private literal that the run is given, so reading either would refuse
        # the run: an exclusion drops a file before it is read.
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp, listed_excluded=LISTED_EXCLUDED)
            done = fixture.vendor("--deny-list", str(private_list(tmp, [PLANTED_LITERAL])))
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            self.assertEqual(fixture.landed(), [], done.stdout)
            new_row = fixture.root / ".packs/skills/packs/alpha/rows/new-row.md"
            self.assertEqual(
                new_row.read_text(encoding="utf-8"), UPSTREAM["skills/packs/alpha/rows/new-row.md"]
            )
            self.assertEqual(last_line(done), summary(7, 2, 2, 5, fixture.head))

    def test_a_planted_address_upstream_refuses_the_run_and_the_tree_is_unchanged(self):
        checks = '{\n  "rows": ["alpha-one", "alpha-two"],\n  "probe_host": "'
        checks += PLANTED_ADDRESS + '"\n}\n'
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp, upstream={**UPSTREAM, "skills/packs/alpha/checks.json": checks})
            before = fixture.snapshot()
            done = fixture.vendor()
            self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
            self.assertIn("vendor-packs: skills/packs/alpha/checks.json:3: ipv4", lines(done))
            self.assertNotIn(PLANTED_ADDRESS, done.stdout)
            self.assertEqual(fixture.snapshot(), before)

    def test_a_private_literal_upstream_refuses_the_run_by_index_never_its_value(self):
        notes = "line one\nthe reference names " + PLANTED_LITERAL.upper() + " here\n"
        for route in ("--deny-list", "PERSONA_CORE_DENY_LIST"):
            with self.subTest(route=route), tempfile.TemporaryDirectory() as tmp:
                upstream = {**UPSTREAM, "skills/packs/beta/notes.md": notes}
                fixture = Fixture(tmp, upstream=upstream)
                deny = private_list(tmp, [PLANTED_LITERAL])
                before = fixture.snapshot()
                if route == "--deny-list":
                    done = fixture.vendor("--deny-list", str(deny))
                else:
                    done = fixture.vendor(env_deny=deny)
                self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
                finding = "vendor-packs: skills/packs/beta/notes.md:2: private literal #0"
                self.assertIn(finding, lines(done))
                self.assertNotIn(PLANTED_LITERAL.casefold(), done.stdout.casefold())
                self.assertEqual(fixture.snapshot(), before)

    def test_a_binary_upstream_file_refuses_the_run(self):
        with tempfile.TemporaryDirectory() as tmp:
            upstream = {**UPSTREAM, "skills/packs/alpha/data/table.bin": BINARY}
            fixture = Fixture(tmp, upstream=upstream)
            before = fixture.snapshot()
            done = fixture.vendor()
            self.assertEqual(done.returncode, 1, done.stdout + done.stderr)
            self.assertIn("vendor-packs: skills/packs/alpha/data/table.bin: binary", lines(done))
            self.assertEqual(fixture.snapshot(), before)

    def test_a_listed_file_the_source_lacks_refuses_the_run_and_nothing_is_deleted(self):
        upstream = {source: text for source, text in UPSTREAM.items() if "/beta/" not in source}
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp, upstream=upstream)
            before = fixture.snapshot()
            done = fixture.vendor()
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            missing = "vendor-packs: skills/packs/beta/SKILL.md: listed, but the commit lacks it"
            self.assertIn(missing, lines(done))
            self.assertEqual(fixture.snapshot(), before)
            kept = fixture.root / ".packs/skills/packs/beta/SKILL.md"
            self.assertEqual(kept.read_text(encoding="utf-8"), "# beta\n")

    def test_a_clean_upstream_revendors_with_its_digests_and_commit(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp)
            done = fixture.vendor()
            self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
            manifest = fixture.manifest_now()
            self.assertEqual(manifest["vendored_from"], fixture.head)
            pin = (fixture.root / "methodology.json").read_text(encoding="utf-8")
            self.assertEqual(pin, methodology(fixture.head))
            expected = sorted(
                (
                    {"path": destination, "from": source, "sha256": sha256(UPSTREAM[source])}
                    for destination, source in CLEAN.items()
                ),
                key=lambda entry: entry["path"],
            )
            self.assertEqual(
                manifest, {**fixture.manifest, "vendored_from": fixture.head, "files": expected}
            )
            for destination, source in CLEAN.items():
                written = (fixture.root / destination).read_bytes()
                self.assertEqual(written, UPSTREAM[source].encode("utf-8"), destination)
            script = fixture.root / ".packs/skills/packs/alpha/run-rows.sh"
            self.assertTrue(os.access(script, os.X_OK), "the executable bit follows upstream")
            self.assertEqual(last_line(done), summary(7, 2, 2, 1, fixture.head))
            left_out = (
                "vendor-packs: left out, not vendored (a new pack is a wiring decision): gamma"
            )
            self.assertIn(left_out, lines(done))
            whole = "vendor-packs: excluded whole by the manifest: subscription-proxy"
            self.assertIn(whole, lines(done))
            self.assertEqual(fixture.landed(), [])
            self.assertFalse((fixture.root / ".packs/skills/packs/gamma").exists())

    def test_a_second_run_changes_nothing_and_leaves_the_tree_byte_identical(self):
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp)
            first = fixture.vendor()
            self.assertEqual(first.returncode, 0, first.stdout + first.stderr)
            after_first = fixture.snapshot()
            second = fixture.vendor()
            self.assertEqual(second.returncode, 0, second.stdout + second.stderr)
            self.assertEqual(last_line(second), summary(7, 0, 0, 1, fixture.head))
            self.assertEqual(fixture.snapshot(), after_first)

    def test_a_run_that_examines_nothing_is_void_and_changes_nothing(self):
        with tempfile.TemporaryDirectory() as tmp:
            only = {".packs/scripts/proxy-client-scan.py": "scripts/proxy-client-scan.py"}
            fixture = Fixture(tmp, earlier={}, listed_excluded=only)
            before = fixture.snapshot()
            done = fixture.vendor()
            self.assertEqual(done.returncode, 3, done.stdout + done.stderr)
            self.assertEqual(last_line(done), summary(0, 0, 0, 1, fixture.head))
            self.assertEqual(fixture.snapshot(), before)


def calls_and_strings(path):
    """Every call `path` makes, as `owner.name` or `name`, and every string constant it holds: a
    static read of its syntax tree, so nothing in it runs."""
    called, strings = [], []
    for node in ast.walk(ast.parse(path.read_text(encoding="utf-8"), filename=str(path))):
        if isinstance(node, ast.Call):
            func = node.func
            if isinstance(func, ast.Attribute):
                owner = func.value.id if isinstance(func.value, ast.Name) else "?"
                called.append(f"{owner}.{func.attr}")
            elif isinstance(func, ast.Name):
                called.append(func.id)
        elif isinstance(node, ast.Constant) and isinstance(node.value, str):
            strings.append(node.value)
    return called, strings


def load_scrub():
    """scripts/public-scrub.py as a module, loaded without writing bytecode into the tree."""
    writes, sys.dont_write_bytecode = sys.dont_write_bytecode, True
    try:
        spec = importlib.util.spec_from_file_location("public_scrub_under_test", SCRUB)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
    finally:
        sys.dont_write_bytecode = writes
    return module


class VendoringReusesTheScrubsRules(unittest.TestCase):
    def test_the_vendoring_scans_with_the_scrubs_own_rules_and_holds_no_copy(self):
        called, strings = calls_and_strings(SCRIPT)
        examined("call(s) in vendor-packs.py", called)
        # The composition is the scrub's: the vendoring asks for it once and builds none itself.
        self.assertEqual([name for name in called if name.endswith(".rules")], ["scrub.rules"])
        builders = ("load_deny", "load_persona_core", "Scan")
        copies = [name for name in called if name.rsplit(".", 1)[-1] in builders]
        self.assertEqual(copies, [], "the vendoring composes the deny lists itself")
        self.assertEqual([text for text in strings if text in COMPOSED_PACKS], [])
        # That one composition holds both packs' shapes and the private list's literals.
        with tempfile.TemporaryDirectory() as tmp:
            scan = load_scrub().rules(private_list(tmp, [PLANTED_LITERAL]))
        rules = {rule for rule, _pattern in scan.rows}
        self.assertLessEqual({"email", "ipv4"}, rules, "persona-core's shapes")
        self.assertLessEqual({"ipv6", "home-directory"}, rules, "privacy-gdpr's shapes")
        self.assertIn(("private", PLANTED_LITERAL), scan.literals)
        # Its refusals read as the vendoring's always did, and nothing is written.
        with tempfile.TemporaryDirectory() as tmp:
            fixture = Fixture(tmp)
            before = fixture.snapshot()
            missing = Path(tmp) / "no-such-list.json"
            done = fixture.vendor("--deny-list", str(missing))
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            self.assertEqual(
                lines(done),
                [f"vendor-packs: the private list {missing} is not a file; nothing was written"],
            )
            malformed = Path(tmp) / "malformed.json"
            malformed.write_text("{ a list that is not JSON\n", encoding="utf-8")
            done = fixture.vendor("--deny-list", str(malformed))
            self.assertEqual(done.returncode, 2, done.stdout + done.stderr)
            refusal = last_line(done)
            self.assertTrue(
                refusal.startswith("vendor-packs: a deny list cannot be read: "), refusal
            )
            self.assertTrue(refusal.endswith("; nothing was written"), refusal)
            self.assertEqual(fixture.snapshot(), before)


if __name__ == "__main__":
    unittest.main()
