"""ADR-301 amends CHARTER constraint 4, ADR-089 and ADR-037: DeckStreak writes to the collection
only through declared write classes, each with its own ADR in ADR-089's form (SPEC-301, #514).

These tests read documents only. They pin that the ADR states the rule and its parts (a) to (g),
that each never-list entry names what it protects, that mass-reschedule is defined apart from a
declared class's batch, that each rejected alternative says why it lost,
and that the three amended documents keep their earlier text byte for byte and carry one dated note
naming ADR-301. Each reader is also run over a planted document it must refuse, so a reader that
sees nothing cannot pass.
"""

import re
import unittest

from _support import REPO, examined

DECISIONS = REPO / "docs" / "decisions"
CHARTER = REPO / "CHARTER.md"
RULE = (
    "DeckStreak writes to the collection only through declared write classes, each with its own "
    "ADR in ADR-089's form."
)
NOTE_OPENING = "Amendment (2026-10-01):"

# The never-list, entry by entry, as issue #514 states it.
NEVER = (
    "edit review history",
    "forget or reset a card",
    "delete a preset",
    "force a full sync",
    "mass-reschedule",
    "set a per-card due-date policy",
    "change a reviewed note's type",
    "delete a reviewed card or note",
)

# What part (a)'s definition of entry 5 states, so a declared class's batch is told apart from it.
MASS_RESCHEDULE = "- **Mass-reschedule** means"
MASS_RESCHEDULE_TERMS = (
    "names every card",
    "preview",
    "prior state",
    "undo",
    "change budget",
    "skip day",
    "ADR-089 (iv)",
    "ADR-089 (i)",
)

# What each lettered part of the decision states, as issue #514 binds it.
PART_TERMS = {
    "a": ("never-list", "what it protects"),
    "b": (
        "whole-collection backup",
        "restore drill",
        "all-or-nothing",
        "counts before and after",
        "undo",
        "change point",
    ),
    "c": (
        "advisory",
        "approval",
        "autonomous",
        "pre-registered",
        "n-of-1 trial",
        "guard metric",
        "kill switch",
    ),
    "d": ("only automated writer", "skip day", "retires", "package the owner imports"),
    "e": (
        "cards remembered per minute studied",
        "observed",
        "frozen at trial start",
        "a suspended or deleted card counts as a fail",
        "no exam-date target",
        "XP",
        "game score",
    ),
    "f": (
        "AI route",
        "embeddings",
        "similarity edges",
        "data-rights entry",
        "retention limit",
        "public templates",
        "private roster",
    ),
    "g": ("advisory-only", "approval-only"),
}

# The option bullets the ADR must carry, each opening with these words, with its verdict.
OPTIONS = {
    "Declared write classes": "chosen because",
    "Advisory-only": "rejected because",
    "Approval-only": "rejected because",
    "A second scheduled sync": "rejected because",
    "Supersede condition (a) for every path": "rejected because",
}

# The earlier text each amended document keeps, byte for byte as it stood before ADR-301.
CHARTER_KEPT = (
    "4. **Pull, then read.** DeckStreak syncs a private copy of the collection and reads it "
    "read-only.\n"
    "   The skip day is the ONLY write back to Anki.\n"
)
ADR_089_KEPT = (
    "  - (i) \"The only writes ever made are the skip day's reschedule of that day's due review "
    "cards,\n"
    "    and its exact inverse. Every other path records ZERO uploads against the recording fake "
    "sync\n"
    '    server."\n'
    '  - (ii) "Incremental sync only. If the server asks for a full or one-way sync, ABORT, write\n'
    '    nothing, and tell the owner (a forced full upload could overwrite the collection)."\n'
    "  - (iii) \"The write runs only on the owner's explicit skip declaration, inside ADR-037's\n"
    '    owner-trigger rule. No new scheduled syncs."\n'
    '  - (iv) "A preview of the cards to be rescheduled is shown before the write, and their '
    "prior due\n"
    '    dates are recorded so the skip can be undone."\n'
)
ADR_037_KEPT = (
    "- (a) no upload path, proven against a fake sync server that records every request;\n"
    "- (b) at most one sync per day, plus the owner's explicit triggers, so DeckStreak never "
    "contends\n"
    "  with another client's sync and never loads the sync server beyond one scheduled sync per "
    "study\n"
    "  day;\n"
)

# What each document's dated note must name.
CHARTER_NOTE = ("ADR-301", "constraint 4", RULE)
ADR_089_NOTE = ("ADR-301", "(i)", "(ii)", "(iii)", "(iv)", RULE)
ADR_037_NOTE = ("ADR-301", "condition (a)", "condition (b)", RULE)

PART = re.compile(r"(?m)^### \(([a-z])\) ")
ROW = re.compile(r"(?m)^\|\s*(\d+)\s*\|([^|\n]*)\|([^|\n]*)\|\s*$")


def normal(text):
    """Collapse every run of whitespace to one space, so a rewrapped line reads the same."""
    return " ".join(text.split())


def says(text, term):
    """True when `text` states `term` as a whole phrase, case and line breaks aside."""
    pattern = r"(?<![\w-])" + re.escape(normal(term)) + r"(?![\w-])"
    return re.search(pattern, normal(text), re.IGNORECASE) is not None


def one_file(pattern):
    found = sorted(DECISIONS.glob(pattern))
    if len(found) != 1:
        raise AssertionError(f"{len(found)} files match docs/decisions/{pattern}, not one")
    return found[0].read_text(encoding="utf-8")


def the_adr():
    """ADR-301's text; SPEC-301 needs exactly one such file."""
    return one_file("ADR-301-*.md")


def section(text, title_word):
    """The one `## ` section whose title holds `title_word`, heading included."""
    blocks = re.split(r"(?m)^(?=## )", text)
    found = [
        block
        for block in blocks
        if block.startswith("## ") and title_word.lower() in block.splitlines()[0].lower()
    ]
    if len(found) != 1:
        raise AssertionError(f"{len(found)} sections are titled {title_word!r}, not one")
    return found[0]


def parts(outcome):
    """Map each lettered part of the decision (`### (a) ...`) to its text, up to the next heading."""
    found = {}
    for mark in PART.finditer(outcome):
        letter = mark.group(1)
        if letter in found:
            raise AssertionError(f"part ({letter}) is stated twice")
        rest = outcome[mark.end() :]
        end = re.search(r"(?m)^#{2,3} ", rest)
        found[letter] = outcome[mark.start() : mark.end() + (end.start() if end else len(rest))]
    return found


def never_list(part):
    """The never-list table's rows as (number, entry, what it protects), normalized."""
    return [(int(n), normal(entry), normal(protects)) for n, entry, protects in ROW.findall(part)]


def never_list_problems(rows):
    """Why `rows` fail to be #514's never-list with what each entry protects; empty when they are."""
    problems = []
    entries = [entry for _, entry, _ in rows]
    for entry in NEVER:
        if entries.count(entry) != 1:
            problems.append(f"{entry!r} is listed {entries.count(entry)} times, not once")
    for entry in entries:
        if entry not in NEVER:
            problems.append(f"{entry!r} is not an entry of the never-list #514 states")
    for _, entry, protects in rows:
        if len(protects.split()) < 3:
            problems.append(f"{entry!r} does not name what it protects")
    return problems


def mass_reschedule_definitions(part):
    """Each bullet of `part` that defines mass-reschedule, normalized."""
    bullets = (normal(block) for block in re.split(r"(?m)^(?=- )", part))
    return [bullet for bullet in bullets if bullet.startswith(MASS_RESCHEDULE)]


def paragraphs(text):
    """Each blank-line-separated block of `text`, with its offset."""
    offset = 0
    for block in text.split("\n\n"):
        yield offset, block
        offset += len(block) + 2


def amendment_problems(text, kept, terms):
    """Why `text` fails to keep `kept` and to carry ONE dated note naming ADR-301 and each term,
    after the kept text and inside the document's last section; empty when it does all of it."""
    problems = []
    at = text.find(kept)
    if at < 0:
        problems.append("the earlier text is not kept byte for byte")
    notes = [
        (offset, block)
        for offset, block in paragraphs(text)
        if block.startswith(NOTE_OPENING) and "ADR-301" in block
    ]
    if len(notes) != 1:
        problems.append(f"{len(notes)} dated notes name ADR-301, not one")
        return problems
    offset, note = notes[0]
    if offset < text.rfind("\n## "):
        problems.append("the note is not in the document's last section")
    if 0 <= at and offset < at:
        problems.append("the note precedes the text it amends")
    for term in terms:
        if normal(term) not in normal(note):
            problems.append(f"the note does not name {term!r}")
    return problems


class TheAdrStatesTheRule(unittest.TestCase):
    def test_the_adr_states_the_rule_and_names_what_it_amends(self):
        text = the_adr()
        front = text.split("---\n", 2)[1]
        self.assertRegex(front, r"(?m)^status:\s*\"?accepted\"?\s*$")
        flat = normal(text)
        self.assertIn(RULE, flat)
        names = examined(
            "named documents and duties",
            (
                "CHARTER constraint 4",
                "ADR-089",
                "ADR-037",
                "its own ADR in ADR-089's form",
                "formal-methods decision",
            ),
        )
        self.assertEqual([name for name in names if name not in flat], [])
        # Planted: the same ADR with its rule reworded no longer states it.
        self.assertNotIn(RULE, flat.replace("only through declared", "through declared"))

    def test_the_adr_states_each_part_a_to_g(self):
        found = parts(section(the_adr(), "Decision Outcome"))
        examined("parts of the decision", found)
        self.assertEqual("".join(sorted(found)), "".join(PART_TERMS))
        terms = examined(
            "terms the parts state",
            [(letter, term) for letter, words in PART_TERMS.items() for term in words],
        )
        missing = [f"({letter}) {term}" for letter, term in terms if not says(found[letter], term)]
        self.assertEqual(missing, [])
        self.assertTrue(says(found["b"], "restore drill"))
        # Planted: a part that drops a term is caught.
        self.assertFalse(
            says(normal(found["b"]).replace("restore drill", "drill"), "restore drill")
        )

    def test_every_batch_backup_is_proven_by_a_drill_before_the_batch_writes(self):
        part_b = parts(section(the_adr(), "Decision Outcome"))["b"]
        proof = (
            "restore drill proves that backup before the batch writes anything",
            "a batch whose backup fails its drill writes nothing",
        )
        terms = examined("per-batch drill terms", proof)
        self.assertEqual([term for term in terms if not says(part_b, term)], [])
        self.assertTrue(says(part_b, terms[0]))
        # The drill is not scoped to a class's first batch or to a change of backup method.
        for scoped in ("first batch", "the way the backup is taken changes"):
            self.assertFalse(says(part_b, scoped), scoped)
        # Planted: the narrowed wording is caught.
        narrowed = normal(part_b).replace(
            "proves that backup before the batch writes anything",
            "proves the backup before the class's first batch",
        )
        self.assertFalse(says(narrowed, proof[0]))

    def test_each_never_list_entry_names_what_it_protects(self):
        outcome = section(the_adr(), "Decision Outcome")
        rows = examined("never-list entries", never_list(parts(outcome)["a"]))
        self.assertEqual(sorted(entry for _, entry, _ in rows), sorted(NEVER))
        self.assertEqual([n for n, _, _ in rows], list(range(1, len(NEVER) + 1)))
        self.assertEqual(never_list_problems(rows), [])
        # Planted: an entry whose protection is blank, and an entry #514 does not state.
        number, entry, _ = rows[-1]
        blank = [*rows[:-1], (number, entry, "")]
        self.assertIn(f"{entry!r} does not name what it protects", never_list_problems(blank))
        foreign = [*rows, (number + 1, "rename a deck", "the deck tree the readings derive")]
        self.assertIn(
            "'rename a deck' is not an entry of the never-list #514 states",
            never_list_problems(foreign),
        )

    def test_mass_reschedule_is_defined_apart_from_a_declared_batch(self):
        part = parts(section(the_adr(), "Decision Outcome"))["a"]
        found = mass_reschedule_definitions(part)
        self.assertEqual(len(found), 1, "part (a) defines mass-reschedule once")
        terms = examined("terms the definition states", MASS_RESCHEDULE_TERMS)
        self.assertEqual([term for term in terms if not says(found[0], term)], [])
        # Planted: part (a) without the definition, and a definition without its change budget.
        kept = [
            block
            for block in re.split(r"(?m)^(?=- )", part)
            if not normal(block).startswith(MASS_RESCHEDULE)
        ]
        self.assertEqual(mass_reschedule_definitions("".join(kept)), [])
        self.assertFalse(says(found[0].replace("change budget", "budget"), "change budget"))


class TheOptionsSayWhyTheyLost(unittest.TestCase):
    def test_the_options_say_why_advisory_only_and_approval_only_lost(self):
        bullets = examined(
            "option bullets",
            [
                line
                for line in section(the_adr(), "Considered Options").splitlines()
                if line.startswith("- ")
            ],
        )
        for opening, verdict in OPTIONS.items():
            hits = [line for line in bullets if line.startswith(f"- {opening}")]
            self.assertEqual(len(hits), 1, opening)
            self.assertIn(verdict, hits[0], opening)
        # The sdd probe's rule: every bullet's reason is on its first line.
        self.assertEqual([line for line in bullets if "because" not in line], [])
        self.assertGreaterEqual(len(bullets), len(OPTIONS))


class TheAmendedDocumentsKeepTheirText(unittest.TestCase):
    def assert_kept_and_noted(self, text, kept, terms):
        self.assertIn(kept, text)
        examined("terms the note names", terms)
        self.assertEqual(amendment_problems(text, kept, terms), [])
        # Planted: the same note over a document that lost its earlier text is refused.
        self.assertIn(
            "the earlier text is not kept byte for byte",
            amendment_problems(text.replace(kept, ""), kept, terms),
        )

    def test_the_charter_keeps_constraint_4_and_its_note_names_adr_301(self):
        self.assert_kept_and_noted(CHARTER.read_text(encoding="utf-8"), CHARTER_KEPT, CHARTER_NOTE)

    def test_adr_089_keeps_guardrails_i_to_iv_and_its_note_names_adr_301(self):
        self.assert_kept_and_noted(one_file("ADR-089-*.md"), ADR_089_KEPT, ADR_089_NOTE)

    def test_adr_037_keeps_conditions_a_and_b_and_its_note_names_adr_301(self):
        self.assert_kept_and_noted(one_file("ADR-037-*.md"), ADR_037_KEPT, ADR_037_NOTE)


if __name__ == "__main__":
    unittest.main()
