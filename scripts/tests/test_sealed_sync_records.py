"""SPEC-583 A1 to A11 (R1 to R13; ADR-602 D2 to D5): the sealed sync records name their keys, the
clear head, the daily record, the census and the staged privacy text, each read from the tree, and
each refusal is planted over a fixture this module spells beside a control that reads green.

RED-FIRST: A1, A3, A5, A8 and A10 find no record at the base. MUTATION COVERAGE, not red first: A2,
A4, A6, A9 and A11 plant their refusals, and rows S58301 to S58314 are their reds.
"""

import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

PARKED = re.compile(r"drill|readings_tree|_standin_checks|test_deploy_standin", re.IGNORECASE)
KEYS = ("account key", "device key", "passkey PRF slot", "recovery key slot")
SERVER_FORMS = ("a sealed", "a public half", "nothing")
CLEAR_WORDS = ("plaintext", "in the clear", "unwrapped", "readable")
HEAD_FIELDS = (
    "version",
    "key id",
    "origin",
    "sequence",
    "clock",
    "class",
    "register",
    "base revision",
    "nonce",
)
FACT_FIELDS = ("date", "reviews", "minutes", "xp granted")
VERDICTS = ("stays", "moves", "split", "held", "retires", "unchanged")
SEALED_CLASSES = ("notes", "media", "deck names", "reviews", "palace", "tags")
KEPT_WORDS = ("op heads", "sizes", "wrapped keys")
CLAIMS = ("end-to-end", "end to end", "sealed on your device", "can never read")
# The census names one context by its description, so the context's own name is never spelt here.
ALIAS = {"the reading context": "pre-study"}

KEY_HEADER = "| key | made | kept | wraps | the server stores |"
HEAD_HEADER = "| head field | bytes | why the server needs it |"
FACT_HEADER = "| daily-facts field | type | server check |"
CONTEXT_HEADER = (
    "| context | verdict | src files | src bytes | test files | test bytes | parked files"
    " | proposed item |"
)
TABLE_HEADER = "| table | owning context | verdict | fed by | proposed item |"
MAP_TABLE_HEADER = "| table | owning context | created by | export and erase |"
PARKED_LINE = re.compile(r"^Parked register rows counted, not listed: (\d+)\.$", re.MULTILINE)
FENCE_LINE = re.compile(r"^deck-streak-([a-z0-9-]+)\s+\((.*)\)\s+depends on: (.*)$")
NEVER = "## What the server can never read"
HOLDS = "## What the server holds"

MAP = Path("docs") / "CONTEXT-MAP.md"
CENSUS = Path("docs") / "sealed-sync" / "device-census.md"
STAGED = Path("docs") / "sealed-sync" / "privacy-at-the-cut-over.md"
STOCK_PATH = Path("crates") / "ingest" / "src" / "lib.rs"


def read(path):
    """The file's text, or None when it is absent."""
    return path.read_text(encoding="utf-8") if path.is_file() else None


def spec_text(root):
    """SPEC-583's text, or None unless exactly one file holds it."""
    found = sorted((root / "docs" / "specs").glob("SPEC-583-*.md"))
    return found[0].read_text(encoding="utf-8") if len(found) == 1 else None


def table(text, header):
    """The rows of the markdown table under `header`, as lists of cells, or None without one."""
    lines = (text or "").split("\n")
    if header not in lines:
        return None
    rows = []
    for line in lines[lines.index(header) + 2 :]:
        if not line.startswith("|"):
            break
        rows.append([cell.strip() for cell in line.strip().strip("|").split("|")])
    return rows


def key_findings(text):
    """Each way the key table differs from R1 and R2."""
    rows = table(text, KEY_HEADER)
    if rows is None:
        return ["no-key-table: SPEC-583 has no key table"]
    found = [f"key-row-shape: {row[0]}" for row in rows if len(row) != 5]
    rows = [row for row in rows if len(row) == 5]
    names = tuple(row[0] for row in rows)
    if names != KEYS:
        found.append(f"keys-differ: {names}")
    for row in rows:
        held = row[4].lower()
        if not held.startswith(SERVER_FORMS):
            found.append(f"server-form: {row[0]}")
        if any(word in held for word in CLEAR_WORDS):
            found.append(f"server-clear: {row[0]}")
    return found


def head_findings(text):
    """Each field the head table adds to or drops from R5's."""
    rows = table(text, HEAD_HEADER)
    if rows is None:
        return ["no-head-table: SPEC-583 has no head table"]
    fields = tuple(row[0] for row in rows)
    extra = [field for field in fields if field not in HEAD_FIELDS]
    missing = [field for field in HEAD_FIELDS if field not in fields]
    found = [f"head-extra: {field}" for field in extra]
    return found + [f"head-missing: {field}" for field in missing]


def fact_findings(text):
    """The daily table's fields, unless they are R7's four in order."""
    rows = table(text, FACT_HEADER)
    if rows is None:
        return ["no-daily-table: SPEC-583 has no daily-facts table"]
    fields = tuple(row[0] for row in rows)
    if fields == FACT_FIELDS:
        return []
    return [f"daily-fields: {fields}"]


def map_lines(map_text):
    """Each context of the map's fence, as (name, description, dependencies)."""
    out = []
    for line in map_text.split("\n"):
        match = FENCE_LINE.match(line)
        if match:
            deps = [part.strip() for part in match.group(3).split(",")]
            out.append((match.group(1), match.group(2), deps))
    return out


def resolve(cell, lines):
    """A census cell as the map's context name: backticks dropped, an alias found by description."""
    if cell in ALIAS:
        hits = [name for name, about, _ in lines if ALIAS[cell] in about]
        return hits[0] if len(hits) == 1 else cell
    return cell.strip("`")


def register(map_text):
    """The map's own tables as (table, owner), and how many rows name the parked surface."""
    rows = table(map_text, MAP_TABLE_HEADER) or []
    tables = [(row[0].strip("`"), row[1].strip("`")) for row in rows]
    listed = [entry for entry in tables if not PARKED.search(entry[0])]
    return listed, len(tables) - len(listed)


def census_findings(census_text, map_text):
    """Each way the census differs from the map's ingest dependants and its register (R9)."""
    if census_text is None:
        return ["no-census: docs/sealed-sync/device-census.md is absent"]
    lines = map_lines(map_text)
    ctx_rows = table(census_text, CONTEXT_HEADER)
    tbl_rows = table(census_text, TABLE_HEADER)
    if ctx_rows is None or tbl_rows is None:
        return ["census-table-missing: the census lacks its context table or its table census"]
    wanted = [name for name, _, deps in lines if "ingest" in deps]
    named = [resolve(row[0], lines) for row in ctx_rows]
    absent = [name for name in wanted if name not in named]
    found = [f"context-missing: {name}" for name in absent]
    found += [f"context-extra: {name}" for name in named if name not in wanted]
    found += [f"context-verdict: {row[0]}" for row in ctx_rows if row[1] not in VERDICTS]
    listed, parked = register(map_text)
    owners = dict(listed)
    held = [(row[0].strip("`"), resolve(row[1], lines), row[2]) for row in tbl_rows]
    names = [name for name, _, _ in held]
    missing = [name for name in owners if name not in names]
    misowned = [name for name, owner, _ in held if owners.get(name, owner) != owner]
    found += [f"table-missing: {name}" for name in missing]
    found += [f"table-extra: {name}" for name in names if name not in owners]
    found += [f"table-twice: {name}" for name in sorted(set(names)) if names.count(name) > 1]
    found += [f"table-owner: {name}" for name in misowned]
    found += [f"table-verdict: {name}" for name, _, verdict in held if verdict not in VERDICTS]
    stated = PARKED_LINE.findall(census_text)
    if stated != [str(parked)]:
        found.append(f"parked-count: stated {stated}, the map holds {parked}")
    return found


def section(text, heading):
    """The lower-cased text under `heading`, up to the next level-2 heading."""
    _, found, rest = text.partition(heading + "\n")
    return rest.split("\n## ", 1)[0].lower() if found else ""


def staged_findings(text):
    """Each sealed class or kept item the staged privacy text does not name (R11)."""
    if text is None:
        return ["no-staged-text: docs/sealed-sync/privacy-at-the-cut-over.md is absent"]
    never, holds = section(text, NEVER), section(text, HOLDS)
    unsealed = [name for name in SEALED_CLASSES if name not in never]
    unkept = [name for name in KEPT_WORDS + FACT_FIELDS if name not in holds]
    found = [f"sealed-class-missing: {name}" for name in unsealed]
    return found + [f"kept-missing: {name}" for name in unkept]


def claim_findings(root):
    """Each sealed-sync claim of the privacy page while the stock-app path is in the tree (R12)."""
    policy = (root / "PRIVACY.md").read_text(encoding="utf-8").lower()
    if not (root / STOCK_PATH).is_file():
        return []
    return [f"claim-before-cut-over: {claim}" for claim in CLAIMS if claim in policy]


PLANTED_MAP = "\n".join(
    [
        "deck-streak-kernel   (shared kernel)  depends on: study-day",
        "deck-streak-ingest   (the anti-corruption layer)  depends on: kernel",
        "deck-streak-analytics   (daily rollup)  depends on: kernel, ingest",
        "deck-streak-texts   (the daily pre-study texts)  depends on: kernel, ingest",
        "",
        MAP_TABLE_HEADER,
        "|---|---|---|---|",
        "| `daily_rollup` | `analytics` | `migrations/1.sql` | exported and erased |",
        "| `text_runs` | `texts` | `migrations/2.sql` | exported and erased |",
        "",
    ]
)
PLANTED_CENSUS = "\n".join(
    [
        CONTEXT_HEADER,
        "|---|---|---|---|---|---|---|---|",
        "| `analytics` | moves | 1 | 1 | 1 | 1 | 0 | #1 |",
        "| the reading context | held | 1 | 1 | 1 | 1 | 0 | #1 |",
        "",
        TABLE_HEADER,
        "|---|---|---|---|---|",
        "| `daily_rollup` | `analytics` | moves | f | #1 |",
        "| `text_runs` | the reading context | held | f | #1 |",
        "",
        "Parked register rows counted, not listed: 0.",
    ]
)
TEXT_ROW = "| `text_runs` | the reading context | held | f | #1 |"
GHOST_ROW = "| `ghost` | `analytics` | moves | f | #1 |"
ANALYTICS_ROW = "| `analytics` | moves | 1 | 1 | 1 | 1 | 0 | #1 |"


class TheKeyHierarchy(unittest.TestCase):
    def test_the_key_table_names_four_keys_and_the_server_holds_none_in_the_clear(self):
        text = spec_text(REPO)
        self.assertEqual(key_findings(text), [])
        self.assertEqual(len(examined("keys of the key table", table(text, KEY_HEADER))), 4)

    def test_a_key_held_in_the_clear_by_the_server_is_refused(self):
        rows = [f"| {key} | m | k | w | a public half only |" for key in KEYS]
        good = "\n".join([KEY_HEADER, "|---|---|---|---|---|", *rows])
        self.assertEqual(key_findings(good), [])
        clear = good.replace(rows[0], "| account key | m | k | w | the account key itself |")
        self.assertEqual(key_findings(clear), ["server-form: account key"])
        mixed = good.replace(
            rows[1], "| device key | m | k | w | a sealed copy and a plain copy, plaintext |"
        )
        self.assertEqual(key_findings(mixed), ["server-clear: device key"])
        fifth = good + "\n| session key | m | k | w | a public half only |"
        self.assertEqual(key_findings(fifth), [f"keys-differ: {(*KEYS, 'session key')}"])


class TheClearHead(unittest.TestCase):
    def test_the_clear_head_holds_exactly_the_judges_fields(self):
        text = spec_text(REPO)
        self.assertEqual(head_findings(text), [])
        self.assertEqual(len(examined("head fields", table(text, HEAD_HEADER))), 9)

    def test_a_body_field_in_the_head_is_refused(self):
        rows = [f"| {field} | 1 | why |" for field in HEAD_FIELDS]
        good = "\n".join([HEAD_HEADER, "|---|---|---|", *rows])
        self.assertEqual(head_findings(good), [])
        extra = good + "\n| deck name | 32 | why |"
        self.assertEqual(head_findings(extra), ["head-extra: deck name"])
        short = good.replace("\n| nonce | 1 | why |", "")
        self.assertEqual(head_findings(short), ["head-missing: nonce"])


class TheDailyRecord(unittest.TestCase):
    def test_the_daily_record_holds_exactly_four_fields(self):
        text = spec_text(REPO)
        self.assertEqual(fact_findings(text), [])
        self.assertEqual(len(examined("daily-facts fields", table(text, FACT_HEADER))), 4)

    def test_a_fifth_or_reordered_daily_field_is_refused(self):
        rows = [f"| {field} | t | c |" for field in FACT_FIELDS]
        good = "\n".join([FACT_HEADER, "|---|---|---|", *rows])
        self.assertEqual(fact_findings(good), [])
        fifth = good + "\n| due count | t | c |"
        self.assertEqual(fact_findings(fifth), [f"daily-fields: {(*FACT_FIELDS, 'due count')}"])
        swapped = good.replace(rows[1] + "\n" + rows[2], rows[2] + "\n" + rows[1])
        reordered = ("date", "minutes", "reviews", "xp granted")
        self.assertEqual(fact_findings(swapped), [f"daily-fields: {reordered}"])


class TheCensus(unittest.TestCase):
    def test_the_census_covers_every_ingest_context_and_every_table(self):
        map_text = (REPO / MAP).read_text(encoding="utf-8")
        self.assertEqual(census_findings(read(REPO / CENSUS), map_text), [])
        contexts = [name for name, _, deps in map_lines(map_text) if "ingest" in deps]
        self.assertEqual(len(examined("contexts that depend on ingest", contexts)), 9)
        examined("register tables", register(map_text)[0])

    def test_a_census_that_drops_adds_or_misjudges_a_table_is_refused(self):
        self.assertEqual(census_findings(PLANTED_CENSUS, PLANTED_MAP), [])
        dropped = PLANTED_CENSUS.replace(TEXT_ROW + "\n", "")
        self.assertEqual(census_findings(dropped, PLANTED_MAP), ["table-missing: text_runs"])
        added = PLANTED_CENSUS.replace(TEXT_ROW, TEXT_ROW + "\n" + GHOST_ROW)
        self.assertEqual(census_findings(added, PLANTED_MAP), ["table-extra: ghost"])
        owner = PLANTED_CENSUS.replace(
            "| `daily_rollup` | `analytics` |", "| `daily_rollup` | `quests` |"
        )
        self.assertEqual(census_findings(owner, PLANTED_MAP), ["table-owner: daily_rollup"])
        verdict = PLANTED_CENSUS.replace(
            "| `analytics` | moves | f |", "| `analytics` | maybe | f |"
        )
        self.assertEqual(census_findings(verdict, PLANTED_MAP), ["table-verdict: daily_rollup"])
        no_context = PLANTED_CENSUS.replace(ANALYTICS_ROW + "\n", "")
        self.assertEqual(census_findings(no_context, PLANTED_MAP), ["context-missing: analytics"])
        parked = PLANTED_CENSUS.replace("not listed: 0.", "not listed: 1.")
        stated = "parked-count: stated ['1'], the map holds 0"
        self.assertEqual(census_findings(parked, PLANTED_MAP), [stated])


class ThePrivacyText(unittest.TestCase):
    def test_the_staged_privacy_text_names_what_is_sealed_and_what_is_kept(self):
        self.assertEqual(staged_findings(read(REPO / STAGED)), [])
        examined("sealed classes", SEALED_CLASSES)
        never = ", ".join(SEALED_CLASSES)
        holds = ", ".join(KEPT_WORDS + FACT_FIELDS)
        good = "\n".join([NEVER, never, "", HOLDS, holds, ""])
        self.assertEqual(staged_findings(good), [])
        no_reviews = good.replace(", reviews, palace", ", palace")
        self.assertEqual(staged_findings(no_reviews), ["sealed-class-missing: reviews"])
        no_xp = good.replace(", xp granted", "")
        self.assertEqual(staged_findings(no_xp), ["kept-missing: xp granted"])

    def test_privacy_md_claims_no_sealed_sync_while_the_stock_path_is_in_the_tree(self):
        self.assertEqual(claim_findings(REPO), [])
        examined("sealed-sync claims", CLAIMS)
        with tempfile.TemporaryDirectory() as scratch:
            root = Path(scratch)
            (root / "PRIVACY.md").write_text("Your notes are sealed on your device.\n", "utf-8")
            self.assertEqual(claim_findings(root), [])
            (root / STOCK_PATH).parent.mkdir(parents=True)
            (root / STOCK_PATH).write_text("//! a stand-in for the stock-app path\n", "utf-8")
            refused = ["claim-before-cut-over: sealed on your device"]
            self.assertEqual(claim_findings(root), refused)


if __name__ == "__main__":
    unittest.main()
