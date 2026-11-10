"""The counts that living documents state are read, and each bound count is derived again
(SPEC-676, ADR-699).

A living document is every tracked Markdown file that no record rule leaves out (R1, R2). Its
prose lines are read by the forms of R3 to R6: a number, in digits or as a word from two up, then
at most three words, then a noun that names something the tree holds (a job, a stage, a row, a
crate, a file, a test, a locale and the like). A count inside a fenced or indented code block, an
inline code span, a block quote, front matter, an HTML comment or a table row is not read, and nor
is a line that names its run, job or attempt id, a figure measured once (R5, R6).

A claim BOUND to a derivation has its count derived again from the tree, and a count that differs
is a site (R7, R8). Every other claim is a site unless the listing names it with its reason (R9).
At `warn` each unlisted site prints a `WARN` line and passes; at `fail` it fails; a listing or a
binding entry that names no current claim fails at both levels (R10). Moving `LEVEL` to `fail` is
its own delivery, under ADR-450 D6 (R11).
"""

import collections
import fnmatch
import re
import subprocess
import unittest

from _support import REPO, examined

FIXTURES = REPO / "scripts" / "tests" / "fixtures" / "doc-counts"
LEVEL = "warn"

# R2: the records whose counts are history by their own rule, each left out by path; the rule
# that makes each one a record is named in SPEC-676 section 1.
RECORDS = re.compile(
    r"^(?:docs/specs/SPEC-[^/]+\.md|docs/decisions/ADR-[^/]+\.md|docs/red-first/[^/]+\.md"
    r"|changelog\.d/(?!README\.md$)[^/]+\.md)$"
)
# R2: a ruling whose authority is the signed commit that adds it, and a released changelog section.
SIGNED = re.compile(r"sign(?:ing|s) the commit that adds this file")
RELEASED = re.compile(r"^## \[\d+\.\d+\.\d+\]")

WORDS = {
    "two": 2, "three": 3, "four": 4, "five": 5, "six": 6, "seven": 7, "eight": 8, "nine": 9,
    "ten": 10, "eleven": 11, "twelve": 12, "thirteen": 13, "fourteen": 14, "fifteen": 15,
    "sixteen": 16, "seventeen": 17, "eighteen": 18, "nineteen": 19, "twenty": 20, "thirty": 30,
    "forty": 40, "fifty": 50, "sixty": 60, "seventy": 70, "eighty": 80, "ninety": 90,
    "hundred": 100,
}  # fmt: skip
NOUNS = (
    "jobs?|stages?|steps?|rows?|crates?|members?|packages?|workflows?|tests?|modules?|files?|"
    "rules?|locales?|catalogs?|migrations?|tables?|routes?|checks?|census(?:es)?|shards?|"
    "lints?|specs?|adrs?|schematics?|personas?|duty|duties|packs?|screens?|fixtures?|keys?|"
    "entry|entries|criterion|criteria|requirements?|sections?|columns?|targets?|binary|"
    "binaries|scripts?|legs?|mutants?|records?|lanes?|suites?|commands?|sources?|projects?|"
    "contexts?|endpoints?|events?|templates?|languages?|apps?|services?|units?|timers?|"
    "triggers?|fields?|platforms?|simulators?|devices?|benches|benchmarks?|probes?|guards?|"
    "tools?|actions?|plants?|cases?|properties|property|obligations?|models?|proofs?|"
    "theorems?|counts?|readers?|registers?|pins?|ids?|bands?"
)
STOP = (
    "a|an|the|of|in|on|at|to|by|for|from|with|and|or|nor|but|is|are|was|were|be|been|when|"
    "then|than|that|which|who|its|it|their|his|her|your|our|my|as|if|per|each|every|all|any|"
    "no|not|into|onto|over|under|apart|more|less|before|after|so|do|does|did|has|have|had|"
    "can|may|must|will|would|should|could|this|these|those|there|here|out|up|down|off|about|"
    "between|through|via|while|where|until|since|times"
)
# R3: a number no identifier, version, date, path or line citation holds, then at most three
# words that are not function words, then the noun. "\x01" stands for an inline code span.
CLAIM = re.compile(
    r"(?<![\w.\-#/:@$`])(?P<count>\d{1,6}|" + "|".join(WORDS) + r")(?![\w.\-/:%]|\.\d)"
    r"(?:\s+(?:(?!(?:" + STOP + r")\b)[A-Za-z][\w'\-]*|\x01)){0,3}?\s+(?P<noun>" + NOUNS + r")\b",
    re.I,
)
FENCE = re.compile(r"^(`{3,}|~{3,})(.*)$")
DELIMITER = re.compile(r"^\s*\|?\s*:?-+:?\s*(?:\|\s*:?-+:?\s*)+\|?\s*$")
ITEM = re.compile(r"^\s*(?:[-*+]|\d{1,9}[.)])\s+")
ONCE = re.compile(r"\b(?:run|job|attempt)s?\s+`?\d{6,}", re.I)
SPAN = re.compile(r"(`+)(.+?)\1")
TARGET = re.compile(r"\]\([^)]*\)")
TAG = re.compile(r"<[^>]+>")

Claim = collections.namedtuple("Claim", "document line count noun words ordinal")

# R7: each bound claim, as (document, the claim's words, derivation, argument, argument).
BOUND = (
    (
        "docs/TESTING.md",
        "five jobs",
        "jobs-running",
        ".github/workflows/ci.yml",
        "bash scripts/check.sh",
    ),
    (
        "docs/schematics/the-web-apps-seven-message-catalogs-and-the-census-that-holds-them-whole.md",
        "seven message catalogs",
        "tracked",
        "web/app/messages/*.json",
        "",
    ),
)
# R9: today's sites, each as (key, reason); measured by the hygiene job of the first push.
LISTED = ()


def line_kinds(text):
    """Each line's kind (R4, R5): prose, table, code, front, quote or comment."""
    return ["prose"] * len(text.split("\n"))


def living_text(name, text):
    """The text of `name` the census reads, or None for a record (R1, R2)."""
    return text


def claims(name, text):
    """Every count claim on a prose line of `name` (R3 to R6), with its ordinal among equal words."""
    return []


def key(claim):
    return f"{claim.document} {claim.count} {claim.noun} {claim.words!r} #{claim.ordinal}"


def jobs_running(tree, workflow, command):
    """How many jobs of `workflow` hold a line that runs `command`."""
    return 0


def tracked(tree, pattern, _unused=""):
    """How many tracked paths match `pattern`."""
    return sum(0 for name in tree if fnmatch.fnmatchcase(name, pattern))


DERIVATIONS = {"jobs-running": jobs_running, "tracked": tracked}


def sites(tree, documents, bound, listed):
    """(sites, stale, bound claims seen): each site as (key, form) (R7 to R10)."""
    return [], [], 0


def verdict(found, stale, listed, level):
    """(printed lines, failures): an unlisted site warns at `warn` and fails at `fail`; a stale
    entry fails at both (R10)."""
    return [], []


def planted_tree():
    """The planted documents and the planted workflow, each read as text, never run."""
    tree = {}
    for path in examined("planted file(s)", sorted(FIXTURES.glob("*.fixture"))):
        tree[path.name.removesuffix(".fixture")] = path.read_text(encoding="utf-8")
    return tree


PLANTED_BOUND = (
    ("bound-match.md", "three jobs", "jobs-running", "planted-ci.yml", "planted-check"),
    (
        "bound-differs.md",
        "four jobs",
        "jobs-running",
        "planted-ci.yml",
        "planted-check",
    ),
)


class TheCountsDocumentsStateAreDerived(unittest.TestCase):
    def test_the_repository_holds_todays_listing(self):
        listed = subprocess.run(
            ["git", "-C", str(REPO), "ls-files", "-z", "--", "*.md", "*.json"],
            capture_output=True,
            check=True,
        ).stdout.decode("utf-8")
        names = [name for name in listed.split("\0") if name]
        markdown = [name for name in names if name.endswith(".md")]
        tree = {name: (REPO / name).read_text(encoding="utf-8") for name in markdown}
        tree.update({name: "" for name in names if name not in tree})
        tree[".github/workflows/ci.yml"] = (REPO / ".github/workflows/ci.yml").read_text("utf-8")
        living = [name for name in markdown if living_text(name, tree[name]) is not None]
        examined("living document(s)", living)
        found, stale, seen = sites(tree, living, BOUND, LISTED)
        examined("bound claim(s)", range(seen))
        printed, failures = verdict(found, stale, LISTED, LEVEL)
        print("\n".join(printed))
        print(f"sites {len(found)}, listed {len(LISTED)}, level {LEVEL}")
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(failures, [])

    def test_a_planted_bound_claim_whose_count_matches_is_clean(self):
        tree = planted_tree()
        found, stale, seen = sites(tree, ["bound-match.md"], PLANTED_BOUND[:1], ())
        self.assertEqual((found, stale, seen), ([], [], 1))
        listing = {"a/x.json": "", "a/y.json": "", "a/z.md": ""}
        self.assertEqual(DERIVATIONS["tracked"](listing, "a/*.json", ""), 2)

    def test_a_planted_bound_claim_whose_count_differs_is_a_site(self):
        tree = planted_tree()
        found, stale, seen = sites(tree, ["bound-differs.md"], PLANTED_BOUND[1:], ())
        self.assertEqual(found, [("bound-differs.md 4 jobs 'four jobs' #1", "mismatch")])
        self.assertEqual((stale, seen), ([], 1))

    def test_a_planted_unbound_claim_is_a_site_at_warn_and_fails_at_fail(self):
        tree = planted_tree()
        found, stale, _seen = sites(tree, ["unbound.md"], (), ())
        site = "unbound.md 5 crates 'five crates' #1"
        self.assertEqual(found, [(site, "unbound")])
        warned = verdict(found, stale, (), "warn")
        self.assertEqual(warned, ([f"WARN doc-count unbound {site}"], []))
        failed = verdict(found, stale, (), "fail")
        self.assertEqual(failed[1], [f"WARN doc-count unbound {site}"])
        self.assertEqual(verdict(found, stale, ((site, "planted"),), "fail"), ([], []))

    def test_a_count_in_code_a_quote_front_matter_a_table_or_a_measured_run_is_not_read(
        self,
    ):
        text = planted_tree()["code-block.md"]
        kinds = collections.Counter(line_kinds(text))
        self.assertEqual(
            (
                kinds["code"],
                kinds["quote"],
                kinds["front"],
                kinds["table"],
                kinds["comment"],
            ),
            (7, 1, 3, 3, 1),
        )
        self.assertEqual(
            [claim.words for claim in claims("code-block.md", text)],
            ["two prose files"],
        )

    def test_a_dated_record_leaves_the_population_by_its_rule(self):
        text = planted_tree()["dated-record.md"]
        signed = "The owner signs the commit that adds this file.\n" + text
        for name, record in (
            ("docs/red-first/SPEC-001.md", text),
            ("docs/specs/SPEC-001-a.md", text),
            ("docs/decisions/ADR-001-a.md", text),
            ("changelog.d/a.md", text),
            ("docs/rulings/a.md", signed),
        ):
            self.assertIsNone(living_text(name, record), name)
        for name, kept in (
            ("docs/specs/planned/SPEC-001-a.md", text),
            ("changelog.d/README.md", text),
            ("docs/rulings/a.md", text),
            ("docs/a.md", signed),
        ):
            self.assertEqual(living_text(name, kept), kept, name)
        released = "# Changelog\n\n## [1.0.0]\n\n" + text
        self.assertEqual(living_text("CHANGELOG.md", released), "# Changelog\n")
        found, _stale, _seen = sites({"docs/a.md": text}, ["docs/a.md"], (), ())
        self.assertEqual(found, [("docs/a.md 3 rows 'three rows' #1", "unbound")])

    def test_an_empty_population_is_void(self):
        with self.assertRaises(AssertionError):
            examined("living document(s)", [])

    def test_a_listing_entry_that_names_no_site_fails_at_both_levels(self):
        tree = planted_tree()
        entry = ("bound-match.md 9 jobs 'nine jobs' #1", "planted")
        found, stale, _seen = sites(tree, ["bound-match.md"], PLANTED_BOUND[:1], (entry,))
        line = f"STALE doc-count listed {entry[0]}"
        for level in ("warn", "fail"):
            self.assertEqual(verdict(found, stale, (entry,), level), ([line], [line]))
        gone = (("bound-match.md", "nine jobs", "tracked", "*.yml", ""),)
        _found, stale, _seen = sites(tree, ["bound-match.md"], gone, ())
        self.assertEqual(stale, ["bound bound-match.md 'nine jobs'"])


if __name__ == "__main__":
    unittest.main()
