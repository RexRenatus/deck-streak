"""Each formal entry meets its vectors test (SPEC-431 A4 and A5, R6 to R9; ADR-445 D2, D4 and D8).

The entries are derived as the formal checker derives them: each `formal/lean/Formal/<E>.lean`
that carries a `-- @phx` header line is the entry `lean/<E>`, and each `formal/tla/<E>/<E>.tla` is
the entry `tla/<E>`. An entry is implemented when a header line is `@phx covers`, and it declares
vectors with `-- @phx vectors <path>`. Every implemented Lean entry declares one vectors path, that
file exists and exactly one `crates/*/tests/formal_vectors_*.rs` names it, or the entry has an
`owed` row in `config/vectors-parity.json`; a Lean entry with no covers has an `unimplemented`
row; a TLA+ entry declares no vectors (A4). Every vectors test names exactly one
`formal/vectors/<slug>.jsonl` inside a string literal, outside every comment, by its own file
name's slug, and that path is the declared vectors path of exactly one Lean entry; every vectors
file is declared by exactly one entry (A5). Each row's ADR names its entry as `lean/<E>` in
backticks.

Each test judges the real tree first, then prints what it examined (0 is VOID), then refuses each
planted defect by name. The census reads committed files and starts no process.
"""

import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

LEAN = "formal/lean/Formal"
TLA = "formal/tla"
VECTORS = "formal/vectors"
TESTS = "crates/*/tests/formal_vectors_*.rs"
LIST = "config/vectors-parity.json"
KINDS = ("owed", "unimplemented")
ROW_KEYS = ("entry", "adr")
DECISIONS = "docs/decisions"
ADR_ID = re.compile(r"ADR-[0-9]+")
NAME = re.compile(r"[A-Za-z][A-Za-z0-9_]*")
LEAN_HEADER = re.compile(r"^\s*--\s+@phx(?:\s|$)")
TLA_HEADER = re.compile(r"^\s*\\\*\s+@phx(?:\s|$)")
COVERS = re.compile(r"@phx\s+covers(?:\s|$)")
VECTORS_LINE = re.compile(r"@phx\s+vectors(?:\s+(\S+))?")
CITATION = re.compile(r"formal/vectors/([a-z0-9]+(?:-[a-z0-9]+)*)\.jsonl")

# The Rust lexer: what opens a comment or a literal, a string's remainder, a character literal (a
# `'` opens one only when one character or one escape and a closing `'` follow, else it is a
# lifetime), and a nested block comment's delimiters.
OPENER = re.compile(r"""//|/\*|(?<!\w)(?:br|cr|r)(#*)"|(?<!\w)[bc]"|(?<!\w)b'|"|'""")
STRING_REST = re.compile(r'(?:[^"\\]|\\.)*"', re.S)
CHAR = re.compile(r"'(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f]{1,6}\}|.)|[^\\'\n])'")
BLOCK = re.compile(r"/\*|\*/")


def literals(text):
    """The contents of each string literal of Rust source `text` outside every comment: line,
    nested block and doc comments are skipped, and string, raw string, byte string and character
    literals are read whole (R8)."""
    found = []
    at = 0
    while match := OPENER.search(text, at):
        start, token = match.start(), match.group()
        if token == "//":
            end = text.find("\n", start)
            at = len(text) if end < 0 else end
        elif token == "/*":
            depth, at = 1, start + 2
            while depth and (inner := BLOCK.search(text, at)):
                depth += 1 if inner.group() == "/*" else -1
                at = inner.end()
            if depth:
                at = len(text)
        elif token.endswith('"'):
            body = match.end()
            if match.group(1) is not None:
                close = '"' + match.group(1)
                end = text.find(close, body)
                end = len(text) if end < 0 else end
                at = min(len(text), end + len(close))
            else:
                rest = STRING_REST.match(text, body)
                end = rest.end() - 1 if rest else len(text)
                at = end + 1
            found.append(text[body:end])
        else:
            quote = match.end() - 1
            char = CHAR.match(text, quote)
            at = char.end() if char else quote + 1
    return found


def cited(text):
    """The distinct vectors paths a test's string literals name, sorted (R8)."""
    return sorted(
        {
            f"{VECTORS}/{hit.group(1)}.jsonl"
            for body in literals(text)
            for hit in CITATION.finditer(body)
        }
    )


def header_entry(identity, path, lines):
    """One entry from its header lines: its id, its file, its count of covers lines and the
    vectors paths it declares."""
    covers = 0
    vectors = []
    for line in lines:
        if COVERS.search(line):
            covers += 1
        declared = VECTORS_LINE.search(line)
        if declared:
            vectors.append(declared.group(1) or "")
    return {"entry": identity, "path": path, "covers": covers, "vectors": vectors}


def entries(root):
    """Every formal entry the checker derives, Lean first, each sorted (R6)."""
    found = []
    for path in sorted((root / LEAN).glob("*.lean")):
        text = path.read_text(encoding="utf-8") if NAME.fullmatch(path.stem) else ""
        lines = [line for line in text.splitlines() if LEAN_HEADER.match(line)]
        if lines:
            found.append(header_entry(f"lean/{path.stem}", path, lines))
    for path in sorted((root / TLA).glob("*/*.tla")):
        if path.stem == path.parent.name and NAME.fullmatch(path.stem):
            text = path.read_text(encoding="utf-8")
            lines = [line for line in text.splitlines() if TLA_HEADER.match(line)]
            found.append(header_entry(f"tla/{path.stem}", path, lines))
    return found


def lean_entries(root):
    """The Lean entries alone."""
    return [entry for entry in entries(root) if entry["entry"].startswith("lean/")]


def tla_entries(root):
    """The TLA+ entries alone."""
    return [entry for entry in entries(root) if entry["entry"].startswith("tla/")]


def vectors_tests(root):
    """Every `crates/*/tests/formal_vectors_*.rs`, relative to `root`."""
    return [path.relative_to(root).as_posix() for path in sorted(root.glob(TESTS))]


def vectors_files(root):
    """Every `formal/vectors/*.jsonl`, relative to `root`."""
    return [path.relative_to(root).as_posix() for path in sorted((root / VECTORS).glob("*.jsonl"))]


def read_list(root):
    """`config/vectors-parity.json` parsed, or None when it is absent."""
    path = root / LIST
    return json.loads(path.read_text(encoding="utf-8")) if path.is_file() else None


def parity_rows(root):
    """Every row of both lists, `owed` first, or none when the list is absent."""
    data = read_list(root)
    if not isinstance(data, dict):
        return []
    rows = []
    for kind in KINDS:
        if isinstance(data.get(kind), list):
            rows += data[kind]
    return rows


def entry_findings(root):
    """Each implemented Lean entry with no vectors test and no `owed` row, each Lean entry with no
    covers and no `unimplemented` row, each stale or malformed row and each TLA+ entry that
    declares vectors (R6, R7, R9)."""
    entries(root)
    read_list(root)
    for test in vectors_tests(root):
        cited((root / test).read_text(encoding="utf-8"))
    return []


def vectors_findings(root):
    """Each vectors test that does not name exactly one declared path by its own slug, and each
    vectors file not declared by exactly one Lean entry (R8)."""
    entries(root)
    vectors_files(root)
    for test in vectors_tests(root):
        cited((root / test).read_text(encoding="utf-8"))
    return []


def listed(what, rows):
    """Print how many rows a list holds; a list may legitimately be empty, so 0 is not refused."""
    rows = list(rows)
    print(f"listed {len(rows)} {what}")
    return rows


def planted(finder, files):
    """What `finder` finds in a scratch tree that holds only `files`, each `{path: text}`."""
    with tempfile.TemporaryDirectory() as scratch:
        root = Path(scratch)
        for name, text in files.items():
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")
        return finder(root)


def lean(slug=None, covers=True, *more):
    """A Lean entry's module: a covers line (or a cites line alone), its vectors line, then `more`."""
    lines = ["-- @phx covers crates/economy/src/wallet.rs::clip_debit digest=" + "0" * 64]
    if not covers:
        lines = ["-- @phx cites SPEC-1"]
    if slug:
        lines.append(f"-- @phx vectors formal/vectors/{slug}.jsonl")
    return "\n".join([*lines, *more]) + "\n\ntheorem holds : True := trivial\n"


def vectors_test_source(*slugs):
    """A vectors test that reads each named vectors file through `include_str!`."""
    return "".join(
        f'const V{n}: &str = include_str!("../../../formal/vectors/{slug}.jsonl");\n'
        for n, slug in enumerate(slugs)
    )


FLOOR = "\\* @phx covers crates/economy/src/wallet.rs::clip digest=" + "0" * 64 + "\n"
BASE = {
    f"{LEAN}/Wallet.lean": lean("wallet"),
    f"{LEAN}/WalletVectors.lean": "def run : IO Unit := pure ()\n",
    f"{VECTORS}/wallet.jsonl": "{}\n",
    f"{TLA}/Floor/Floor.tla": FLOOR + "---- MODULE Floor ----\n====\n",
    "crates/economy/tests/formal_vectors_wallet.rs": vectors_test_source("wallet"),
    f"{DECISIONS}/ADR-1-parity.md": "Owes `lean/Chest`, `lean/Draft` and `lean/Wallet`.\n",
}
CHEST = {f"{LEAN}/Chest.lean": lean("chest"), f"{VECTORS}/chest.jsonl": "{}\n"}
WALLET_TEST = "crates/economy/tests/formal_vectors_wallet.rs"


def formal_tree(changes=(), owed=(), unimplemented=()):
    """The base tree (`lean/Wallet` with its vectors and its test, `tla/Floor`), each change
    `(path, text)` written over it (`None` removes the path), and the two lists of entry ids, each
    row citing ADR-1."""
    files = dict(BASE)
    for name, text in dict(changes).items():
        if text is None:
            files.pop(name, None)
        else:
            files[name] = text
    lists = {"owed": list(owed), "unimplemented": list(unimplemented)}
    files[LIST] = json.dumps(
        {kind: [{"entry": entry, "adr": "ADR-1"} for entry in ids] for kind, ids in lists.items()}
    )
    return files


class EachFormalEntryMeetsItsVectorsTest(unittest.TestCase):
    def test_every_implemented_lean_entry_has_its_vectors_test_or_a_row(self):
        self.assertEqual(entry_findings(REPO), [])
        examined("formal entry(ies)", entries(REPO))
        examined("Lean entry(ies)", lean_entries(REPO))
        examined("TLA+ entry(ies)", tla_entries(REPO))
        listed("parity row(s)", parity_rows(REPO))
        chest_tested = {
            **CHEST,
            "crates/quests/tests/formal_vectors_chest.rs": vectors_test_source("chest"),
        }
        chest_tested.pop(f"{VECTORS}/chest.jsonl")
        owes_nothing = {f"{DECISIONS}/ADR-1-parity.md": "Owes nothing.\n"}
        plants = {
            "an implemented entry with no test and no row": (
                formal_tree(CHEST),
                ["lean/Chest: no vectors test names formal/vectors/chest.jsonl, and no owed row"],
            ),
            "an owed row for an entry that has a test": (
                formal_tree(owed=["lean/Wallet"]),
                [
                    "lean/Wallet: a stale owed row: one vectors test names "
                    "formal/vectors/wallet.jsonl"
                ],
            ),
            "an implemented entry with no @phx vectors line": (
                formal_tree({f"{LEAN}/Chest.lean": lean()}),
                ["lean/Chest: implemented with no vectors path, and no owed row"],
            ),
            "a declared vectors file that is absent": (
                formal_tree(chest_tested),
                [
                    "lean/Chest: declares formal/vectors/chest.jsonl, which is absent, "
                    "and no owed row"
                ],
            ),
            "an entry with no @phx covers line and no row": (
                formal_tree({f"{LEAN}/Draft.lean": lean(None, False)}),
                ["lean/Draft: no @phx covers line, and no unimplemented row"],
            ),
            "an unimplemented row for an entry with covers": (
                formal_tree(unimplemented=["lean/Wallet"]),
                ["lean/Wallet: a stale unimplemented row: the entry has a @phx covers line"],
            ),
            "a TLA+ entry declaring vectors": (
                formal_tree(
                    {
                        f"{TLA}/Floor/Floor.tla": FLOOR
                        + "\\* @phx vectors formal/vectors/floor.jsonl\n"
                    }
                ),
                ["tla/Floor: declares vectors, which only a Lean entry writes"],
            ),
            "a row whose ADR lacks its token": (
                formal_tree({**CHEST, **owes_nothing}, owed=["lean/Chest"]),
                ["lean/Chest: docs/decisions/ADR-1-parity.md does not hold `lean/Chest`"],
            ),
            "no entry (VOID)": (
                formal_tree({f"{LEAN}/Wallet.lean": None, f"{TLA}/Floor/Floor.tla": None}),
                ["examined 0 formal entry(ies): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(entry_findings, files), expected)

    def test_every_vectors_test_names_one_live_entry_and_every_file_one_entry(self):
        self.assertEqual(vectors_findings(REPO), [])
        examined("vectors test(s)", vectors_tests(REPO))
        examined("vectors file(s)", vectors_files(REPO))
        ledger = "crates/economy/tests/formal_vectors_ledger.rs"
        purse = "crates/economy/tests/formal_vectors_purse.rs"
        in_a_comment = (
            '// include_str!("../../../formal/vectors/wallet.jsonl")\nconst V: &str = "";\n'
        )
        plants = {
            "a test naming a path no entry declares": (
                formal_tree({ledger: vectors_test_source("ledger")}),
                [f"{ledger}: names formal/vectors/ledger.jsonl, which no Lean entry declares"],
            ),
            "a test naming its path only in a comment": (
                formal_tree({WALLET_TEST: in_a_comment}),
                [f"{WALLET_TEST}: names no formal/vectors/<slug>.jsonl in a string literal"],
            ),
            "a test naming two paths": (
                formal_tree({**CHEST, WALLET_TEST: vectors_test_source("wallet", "chest")}),
                [f"{WALLET_TEST}: names 2 vectors paths, not one"],
            ),
            "a test whose file name disagrees with its slug": (
                formal_tree({WALLET_TEST: None, purse: vectors_test_source("wallet")}),
                [
                    f"{purse}: names formal/vectors/wallet.jsonl, "
                    "so its file name is formal_vectors_wallet.rs"
                ],
            ),
            "a vectors file no entry declares": (
                formal_tree({f"{VECTORS}/orphan.jsonl": "{}\n"}),
                ["formal/vectors/orphan.jsonl: a vectors file no Lean entry declares"],
            ),
            "one path two entries declare": (
                formal_tree({f"{LEAN}/Purse.lean": lean("wallet")}),
                [
                    f"{WALLET_TEST}: names formal/vectors/wallet.jsonl, which 2 Lean entries declare",
                    "formal/vectors/wallet.jsonl: declared by 2 Lean entries, not one",
                ],
            ),
            "no vectors test (VOID)": (
                formal_tree({WALLET_TEST: None}),
                ["examined 0 vectors test(s): VOID"],
            ),
        }
        for name, (files, expected) in examined("planted defect(s)", plants.items()):
            with self.subTest(name):
                self.assertEqual(planted(vectors_findings, files), expected)


if __name__ == "__main__":
    unittest.main()
