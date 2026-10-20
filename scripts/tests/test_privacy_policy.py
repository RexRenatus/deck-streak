"""PRIVACY.md gives every category of privacy.json its lawful basis and its retention, and discloses
every copy an erase cannot reach, each with its window: the backup replica, the service journal,
the private copy of the collection, and the cron-fire ledger (SPEC-021 A9, R7; CHARTER 13).

The policy is judged against privacy.json and against the ledger's retention in the code, never
against a number written here, and each census is paired with a planted policy it must refuse.
"""

import json
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined
from test_one_static_library import tracked_paths
from test_sync_server_runbook import OFFSITE_PERIOD

POLICY = REPO / "PRIVACY.md"
INVENTORY = REPO / "privacy.json"
README = REPO / "README.md"
ABOUT = REPO / "web" / "app" / "src" / "routes" / "about" / "+page.svelte"
MAINTENANCE = REPO / "crates" / "coordination" / "src" / "maintenance.rs"
# The section of the policy that lists what an erase leaves.
UNREACHED = "## What an erase does not reach"
# The words an Article 6(1) basis and a retention event are written in, in the policy.
BASES = {
    "consent": "consent",
    "contract": "contract",
    "legal-obligation": "legal obligation",
    "vital-interests": "vital interests",
    "public-task": "public task",
    "legitimate-interests": "legitimate interests",
}
EVENTS = {
    "account-deletion": "account deletion",
    "consent-withdrawal": "consent withdrawal",
    "purpose-end": "purpose end",
    "last-activity": "last activity",
    "collection": "collection",
}
UNITS = {"Y": "year", "M": "month", "W": "week", "D": "day"}


def inventory():
    return json.loads(INVENTORY.read_text(encoding="utf-8"))


def in_words(period):
    """An ISO 8601 period of one unit, `P3D`, as the policy writes it: `3 days`."""
    match = re.fullmatch(r"P(\d+)([YMWD])", period)
    if match is None:
        raise AssertionError(f"{period!r} is not a period of one unit")
    count = int(match.group(1))
    return f"{count} {UNITS[match.group(2)]}{'' if count == 1 else 's'}"


def ledger_days():
    """How many days the cron-fire ledger keeps a row: the maintenance job's own constant."""
    source = MAINTENANCE.read_text(encoding="utf-8")
    match = re.search(r"pub const CRON_FIRES_RETENTION_DAYS: i64 = (\d+);", source)
    if match is None:
        raise AssertionError("the maintenance job names no CRON_FIRES_RETENTION_DAYS")
    return int(match.group(1))


def flat(text):
    """Text with its whitespace runs made single spaces, casefolded."""
    return " ".join(text.split()).casefold()


def undisclosed_categories(policy, categories):
    """Every category no line of the policy names with its basis and its retention."""
    lines = [flat(line) for line in policy.splitlines()]
    refused = []
    for category in categories:
        token = re.compile(rf"(?<![\w-]){re.escape(category['id'])}(?![\w-])")
        basis = BASES[category["lawful_basis"]]
        retention = category["retention"]
        kept = EVENTS[retention["until"]] if "until" in retention else in_words(retention["period"])
        if not any(token.search(line) and basis in line and kept in line for line in lines):
            refused.append(f"{category['id']} ({basis}, {kept})")
    return refused


def copies(data):
    """Each copy an erase cannot reach, with the words its disclosure must carry."""
    return {
        "the backup replica": ("replica", in_words(data["backups"]["retention"])),
        "the service journal": ("journal", in_words(data["logs"]["retention"])),
        "the private copy of the collection": ("collection", "sync", "credential"),
        "the cron-fire ledger": ("cron-fire ledger", f"{ledger_days()} days"),
        # The sync server's store is copied in its window and archived (SPEC-337 R5): no table of
        # the inventory holds it, so the policy names it here, with where its copies are kept.
        "the sync server's snapshots": ("snapshot", "sync server", "three", "offsite"),
    }


def unreached_bullets(policy):
    """Each bullet of the policy's unreached section, flattened."""
    _, found, rest = policy.partition(UNREACHED)
    section = rest.split("\n## ", 1)[0] if found else ""
    return [flat(bullet) for bullet in re.split(r"\n(?=- )", section) if bullet.startswith("- ")]


def undisclosed_copies(policy, data):
    """Every copy no bullet of the policy's unreached section discloses with its words."""
    bullets = unreached_bullets(policy)
    return [
        copy
        for copy, words in copies(data).items()
        if not any(all(word in bullet for word in words) for bullet in bullets)
    ]


class ThePolicyDisclosesWhatAnEraseLeaves(unittest.TestCase):
    def test_the_policy_discloses_every_copy_an_erase_cannot_reach(self):
        data = inventory()
        policy = POLICY.read_text(encoding="utf-8")
        categories = examined("categories of privacy.json", data["categories"])
        self.assertEqual(undisclosed_categories(policy, categories), [])
        disclosed = examined("copies an erase cannot reach", list(copies(data)))
        self.assertEqual(undisclosed_copies(policy, data), [])
        self.assertEqual(len(disclosed), 5)
        # A policy that leaves a copy out, or a category's retention, is refused by name.
        without_replica = "\n".join(
            line for line in policy.splitlines() if "replica" not in line.casefold()
        )
        self.assertEqual(undisclosed_copies(without_replica, data), ["the backup replica"])
        first = categories[0]
        without_first = "\n".join(
            line for line in policy.splitlines() if f"`{first['id']}`" not in line
        )
        self.assertEqual(
            [entry.split(" ")[0] for entry in undisclosed_categories(without_first, categories)],
            [first["id"]],
        )

    def test_the_offsite_snapshot_has_a_stated_period(self):
        """SPEC-340 A14 (R11; ADR-351 D9): the snapshots' bullet states the offsite archives'
        period, the one the runbook's bucket rule checks, and the policy discloses the edge log of
        the sync route, with the journal's window, and the ban list, with the ban's."""
        data = inventory()
        bullets = examined("unreached bullet(s)", unreached_bullets(POLICY.read_text("utf-8")))
        snapshots = [b for b in bullets if "snapshot" in b and "sync server" in b]
        self.assertEqual(len(snapshots), 1, "one bullet discloses the sync server's snapshots")
        self.assertIn(f"`{OFFSITE_PERIOD}`".casefold(), snapshots[0])
        self.assertIn(in_words(OFFSITE_PERIOD), snapshots[0])
        self.assertNotIn("as long as the bucket's own retention", snapshots[0])
        logged = ("edge", "sync route", "address", "method", "path", "status")
        window = in_words(data["logs"]["retention"])
        edge = [b for b in bullets if all(word in b for word in logged)]
        self.assertEqual(len(edge), 1, "one bullet discloses the edge log of the sync route")
        self.assertIn(window, edge[0])
        self.assertIn(f"`{data['logs']['retention']}`".casefold(), edge[0])
        self.assertTrue(
            any("ban list" in b and "address" in b and "up to one day" in b for b in bullets),
            "no bullet discloses the ban list with the ban's period",
        )

    def test_the_readme_and_the_about_page_link_the_policy(self):
        data = inventory()
        entries = examined("policy entry points", data["policy"]["entry"])
        self.assertEqual(data["policy"]["file"], "PRIVACY.md")
        for entry in entries:
            self.assertIn(entry["text"], (REPO / entry["file"]).read_text(encoding="utf-8"))
        named = {entry["file"] for entry in entries}
        # SPEC-026 R14: the bot's command table, whose /privacy links the policy, is an entry point.
        self.assertEqual(
            named,
            {"README.md", "web/app/src/routes/about/+page.svelte", "crates/bot/src/commands.rs"},
        )
        readme = README.read_text(encoding="utf-8")
        self.assertRegex(readme, r"(?m)^## Privacy$")
        self.assertIn("[PRIVACY.md](PRIVACY.md)", readme)
        self.assertIn("PRIVACY.md", ABOUT.read_text(encoding="utf-8"))


# SPEC-368: the section that says what DeckStreak trains or fits on a learner's data; its
# paragraph as SPEC-387 R10 replaced it.
MODELS = "## Models and your data"
PARAGRAPH = (
    "DeckStreak does not train or fine-tune a neural network or a language model on your data, "
    "and does not build a dataset from it. The scheduling parameters in your collection are your "
    "own scheduler's: either a fit of that scheduler to your own reviews, made by your Anki app, "
    "or the scheduler's defaults. DeckStreak reads them, and the memory state Anki stores for "
    "each card, to schedule the cards you study, and does not fit them itself. When you ask, "
    "DeckStreak proposes the scheduler's defaults for one preset and keeps a record of the "
    "proposal and of the parameters it would replace, so the change can be undone; your Anki app "
    "applies it, not DeckStreak. An AI duty runs only when the host's AI route is configured, "
    "which it is not by default. Each run sends the cards that duty covers and a summary of your "
    "leeches, lapses and graded practice, not your journal, as the context of that one run, and "
    "DeckStreak's database keeps neither the prompt nor the reply. A reply that passes its checks "
    "is written to your own vault. What the model's provider keeps is set by that provider's "
    "terms."
)
OLD_SENTENCE = "DeckStreak never uses your data to train a model."
# SPEC-387 R9: the proposal record's category, and the fields of its declaration the test pins.
PRESET_CATEGORY = "preset-proposals"
PRESET_DECLARED = ("source", "stores", "lawful_basis", "retention", "export", "erase")
# What a delivery that trips a guard below owes: the page, and its changelog entry (SPEC-368 R7).
OWED = (
    f"the section `{MODELS}` of PRIVACY.md says what DeckStreak does with a learner's data, so "
    "amend that section and its changelog entry in the same delivery"
)
TABLE = REPO / "crates" / "engine-core" / "src" / "table.rs"
AI_SAFETY = REPO / "ai-safety.json"
# The words a fitting method of the engine allow-list would hold.
FITTING_WORDS = ("Params", "Fsrs", "Retention", "Simulate", "Benchmark", "Dataset")
FITTING_CALLS = re.compile(
    r"compute_parameters|ComputeParameters|ComputeFsrsParams|compute_params|optimal_retention"
    r"|evaluate_params|ExportDataset|export_dataset"
)
# The crates that depend on the Anki engine or on a review-memory crate name their dependency at
# the start of a manifest line.
ENGINE_DEPENDENCY = re.compile(r"(?m)^(?:anki|fsrs7|fsrs6)\b")
EMBEDDING_WORDS = re.compile(r"embedding|similarity|cosine", re.IGNORECASE)
EMBEDDING_SUFFIXES = (".rs", ".js", ".ts", ".svelte", ".swift")
EMBEDDING_ROOTS = ("crates/", "web/", "ios/")
DECLARED_CLASSES = "scripts/tests/test_declared_write_classes.py"
# Frame embedding in two comments, exempt by exact path and exact line text; the declared list of
# write classes, exempt by its path (SPEC-368 R7).
FRAME_COMMENTS = (
    (
        "web/app/src/lib/card/policy.js",
        " * itself included, is checked against the embedding page's `frame-src`, "
        "so 'none' holds every frame",
    ),
    (
        "web/app/src/lib/csp.test.ts",
        "    // included, is checked against the embedding page's frame-src, "
        "so 'none' holds every frame to",
    ),
)
# The one parked path that is not drill-named: it belongs to the parked drill surface (#158).
PARKED = ("crates/vault/src/readings_tree.rs",)


def models_section(policy):
    """The text under the models heading, up to the next heading, or empty when it is absent."""
    _, found, rest = policy.partition(MODELS)
    return rest.split("\n## ", 1)[0] if found else ""


def table_methods(source):
    """Each `Service.Method` name of the engine allow-list's two tables."""
    names = []
    for const in ("pub const ORDINARY", "pub const EXEMPT"):
        _, _, rest = source.partition(const)
        names += re.findall(r'name: "([A-Za-z]+\.[A-Za-z]+)"', rest.split("\n];", 1)[0])
    return names


def refuse_fitting_methods(methods):
    """Refuse a method of the allow-list that holds a fitting word, by its name."""
    fitting = [m for m in methods if any(word in m for word in FITTING_WORDS)]
    if fitting:
        raise AssertionError(f"the engine allow-list holds {fitting}: {OWED}")


def engine_crates(tracked):
    """The crates whose manifest depends on the Anki engine or a review-memory crate."""
    crates = []
    for path in tracked:
        parts = path.split("/")
        if len(parts) == 3 and parts[0] == "crates" and parts[2] == "Cargo.toml":
            if ENGINE_DEPENDENCY.search((REPO / path).read_text(encoding="utf-8")):
                crates.append(parts[1])
    return sorted(crates)


def refuse_fitting_calls(root, paths):
    """Refuse a line of the given files that names a fitting call, by file and line."""
    hits = []
    for path in paths:
        text = (root / path).read_text(encoding="utf-8", errors="replace")
        for number, line in enumerate(text.splitlines(), 1):
            if FITTING_CALLS.search(line):
                hits.append(f"{path}:{number}")
    if hits:
        raise AssertionError(f"an engine crate names a fitting call at {hits}: {OWED}")


def tool_holders(data):
    """The id of every AI task that holds a tool."""
    return [str(t.get("id") or t.get("name")) for t in data["tasks"] if t.get("tools")]


def refuse_tools(data):
    """Refuse an AI task that holds a tool, by its id."""
    holders = tool_holders(data)
    if holders:
        raise AssertionError(f"the AI task(s) {holders} hold a tool: {OWED}")


def embedding_population(tracked):
    """The files A5 reads, with the count of drill-named and parked paths dropped by name.

    Both are dropped from the listing before any file is opened, so a skipped path is never
    read; the counts are printed by the caller so the blind spot is never silent.
    """
    listed = [
        p for p in tracked if p.startswith(EMBEDDING_ROOTS) and p.endswith(EMBEDDING_SUFFIXES)
    ]
    drill = [p for p in listed if "drill" in p.casefold()]
    parked = [p for p in listed if p in PARKED]
    kept = [p for p in listed if "drill" not in p.casefold() and p not in PARKED]
    return kept, len(drill), len(parked)


def embedding_lines(root, paths):
    """Each line of the given files that holds an embedding word, as (path, line text)."""
    found = []
    for path in paths:
        text = (root / path).read_text(encoding="utf-8", errors="replace")
        found += [(path, line) for line in text.splitlines() if EMBEDDING_WORDS.search(line)]
    return found


def refuse_embeddings(found):
    """Refuse every line that is not exempt, and every exemption that matched nothing.

    Returns how many lines the exemptions matched.
    """
    matched = {key: 0 for key in FRAME_COMMENTS}
    declared = 0
    unexplained = []
    for path, line in found:
        if (path, line) in matched:
            matched[(path, line)] += 1
        elif path == DECLARED_CLASSES:
            declared += 1
        else:
            unexplained.append(f"{path}: {line.strip()}")
    stale = [path for (path, _), count in matched.items() if count == 0]
    if declared == 0:
        stale.append(DECLARED_CLASSES)
    if unexplained or stale:
        raise AssertionError(
            f"an embedding or similarity word at {unexplained}, stale exemption(s) {stale}: {OWED}"
        )
    return sum(matched.values()) + declared


class ThePolicyStatesWhatDeckStreakFitsOnYourData(unittest.TestCase):
    def test_the_policy_states_what_deckstreak_trains_and_fits(self):
        policy = POLICY.read_text(encoding="utf-8")
        self.assertIn(flat(PARAGRAPH), flat(models_section(policy)))
        self.assertNotIn(flat(OLD_SENTENCE), flat(policy))

    def test_the_preset_proposals_record_is_declared_and_named(self):
        """SPEC-387 R9: the proposal record is a declared category, and the page's table names it."""
        categories = {category["id"]: category for category in inventory()["categories"]}
        self.assertIn(PRESET_CATEGORY, categories)
        category = categories[PRESET_CATEGORY]
        self.assertEqual(
            {key: category[key] for key in PRESET_DECLARED},
            {
                "source": "derived",
                "stores": ["preset_proposals.*"],
                "lawful_basis": "contract",
                "retention": {"until": "account-deletion"},
                "export": True,
                "erase": "delete",
            },
        )
        self.assertTrue(category["purpose"].strip(), "the category states a purpose")
        policy = POLICY.read_text(encoding="utf-8")
        self.assertEqual(undisclosed_categories(policy, [category]), [])

    def test_the_engine_allow_list_holds_no_fitting_method(self):
        methods = examined("engine methods", table_methods(TABLE.read_text(encoding="utf-8")))
        refuse_fitting_methods(methods)
        plant = "SchedulerService.ComputeFsrsParams"
        with self.assertRaises(AssertionError) as refused:
            refuse_fitting_methods([*methods, plant])
        self.assertIn(plant, str(refused.exception))
        self.assertIn(MODELS, str(refused.exception))

    def test_no_engine_crate_names_a_fitting_call(self):
        tracked = tracked_paths(REPO)
        crates = examined("engine crates", engine_crates(tracked))
        roots = tuple(f"crates/{name}/" for name in crates)
        files = [p for p in tracked if p.startswith(roots) and p.endswith(".rs")]
        print(f"examined {len(files)} files in {len(crates)} crates")
        self.assertTrue(files, "examined 0 files: the population is empty, so nothing was judged")
        refuse_fitting_calls(REPO, files)
        with tempfile.TemporaryDirectory() as tmp:
            crate = Path(tmp) / "crates" / "planted" / "src"
            crate.mkdir(parents=True)
            (crate / "lib.rs").write_text("fn fit() { compute_params(); }\n", encoding="utf-8")
            with self.assertRaises(AssertionError) as refused:
                refuse_fitting_calls(Path(tmp), ["crates/planted/src/lib.rs"])
        self.assertIn("crates/planted/src/lib.rs:1", str(refused.exception))
        self.assertIn(MODELS, str(refused.exception))

    def test_no_ai_task_holds_a_tool(self):
        data = json.loads(AI_SAFETY.read_text(encoding="utf-8"))
        examined("AI tasks", data["tasks"])
        refuse_tools(data)
        runs = [flat(line) for line in POLICY.read_text(encoding="utf-8").splitlines()]
        self.assertTrue(
            any("`agent-runs`" in line and "never the prompt or the reply" in line for line in runs)
        )
        with self.assertRaises(AssertionError) as refused:
            refuse_tools({"tasks": [{"id": "planted-task", "tools": ["x"]}]})
        self.assertIn("planted-task", str(refused.exception))
        self.assertIn(MODELS, str(refused.exception))

    def test_no_code_computes_an_embedding_or_similarity(self):
        kept, drill, parked = embedding_population(tracked_paths(REPO))
        self.assertEqual(parked, len(PARKED), "the parked path is not in the tree once")
        files = examined("files", [*kept, DECLARED_CLASSES])
        matched = refuse_embeddings(embedding_lines(REPO, files))
        print(f"examined {matched} exempt lines matched")
        print(f"examined {drill} drill-named paths skipped, {parked} parked path skipped")
        with tempfile.TemporaryDirectory() as tmp:
            planted = Path(tmp) / "crates" / "planted.rs"
            planted.parent.mkdir()
            planted.write_text("let v = embedding(card);\n", encoding="utf-8")
            with self.assertRaises(AssertionError) as refused:
                refuse_embeddings(embedding_lines(Path(tmp), ["crates/planted.rs"]))
        self.assertIn("crates/planted.rs", str(refused.exception))
        self.assertIn(MODELS, str(refused.exception))


if __name__ == "__main__":
    unittest.main()
