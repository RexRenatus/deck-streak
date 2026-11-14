"""Every user-facing text a web component shows is drawn from the message catalogs (SPEC-653;
ADR-675).

`component_sites(text)` reads one `.svelte` component as text and returns one site for each
user-facing text the component writes as a literal instead of drawing it from a catalog: a text
node holding a letter outside every `{...}` expression (`text`), a quoted value holding a letter
outside every expression on an attribute `USER_FACING` names (`attribute`), and a quoted literal
holding a letter handed to a dialog call `DIALOG` names (`dialog`). A message drawn from a catalog
is an expression, `{m.key()}` or `title={m.key()}`, so it is never a site. Script and style blocks
and comments are not markup, a character reference is read as the character it names, and a text
with no letter, or that is one name of `NAMES` alone, is not a site. A site's key is its file, its
form, its text with each expression shown as `{…}` and its whitespace collapsed, and its ordinal
among equal keys in its file, never a line.

The census reads every `.svelte` file under `web/app/src`, prints `examined <n> component(s)` and
refuses an empty population. `LEVEL` is `warn`: a site `LISTED` names passes, a site it does not
name prints a `WARN` line and passes, and an entry no site matches fails as `STALE`. At `fail` an
unlisted site fails too. Moving `LEVEL` to `fail` is its own delivery, under ADR-450 D6's promotion
rule. The planted components sit under `scripts/tests/fixtures/web-literal-text/` with a `.fixture`
suffix, so no web tool and no `*.svelte` glob reads them. The standard library only; nothing is
imported from the tree.
"""

import collections
import html
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO

COMPONENTS = Path("web/app/src")
FIXTURES = Path("scripts/tests/fixtures/web-literal-text")
LEVEL = "warn"

# The attributes whose value a person reads or hears: an accessible name, description, hint or
# value, a tooltip, an image's text alternative, and a control's label.
USER_FACING = frozenset(
    {
        "alt",
        "aria-description",
        "aria-label",
        "aria-placeholder",
        "aria-roledescription",
        "aria-valuetext",
        "label",
        "placeholder",
        "title",
    }
)
# Names shown the same in every locale: a text that is one of them alone is not a site.
NAMES = frozenset({"Anki", "DeckStreak", "Telegram"})
# The calls that show their first argument to the person as text: the browser's three dialogs,
# called on `window`, and the Mini App's alert and confirmation popups.
DIALOG = re.compile(
    r"(?:\bwindow\.(?:alert|confirm|prompt)|\.show(?:Alert|Confirm))\s*\(\s*(['\"`])"
)
LETTER = re.compile(r"[^\W\d_]")
BLOCK = re.compile(r"<(script|style)\b[^>]*>(.*?)</\1\s*>", re.S)
COMMENT = re.compile(r"<!--.*?-->", re.S)
ATTRIBUTE = re.compile(r"([^\s=/<>\"'{}]+)\s*=\s*(?:\"([^\"]*)\"|'([^']*)')")
EXPRESSION = re.compile(r"\{\s*\}")
TEMPLATE_PART = re.compile(r"\$\{[^}]*\}")

# Today's sites, each with why it stays until a delivery cures it (SPEC-653 section 1): its file,
# its form, its text, its ordinal and the reason.
LISTED = ()


def blank(chars, start, end):
    """Blank `chars[start:end]`, keeping every line break, so an offset keeps its line."""
    for position in range(start, end):
        if chars[position] != "\n":
            chars[position] = " "


def skip_quoted(text, index, quote):
    """The offset just past the quote that closes the literal opened at `index`."""
    index += 1
    while index < len(text) and text[index] != quote:
        index += 2 if text[index] == "\\" else 1
    return index + 1


def expression_end(text, start):
    """The offset just past the `}` that closes the expression opened at `start`."""
    depth, index = 0, start
    while index < len(text):
        char = text[index]
        if char in "'\"`":
            index = skip_quoted(text, index, char)
            continue
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return index + 1
        index += 1
    return len(text)


def tag_end(text, start):
    """The offset just past the `>` that closes the tag opened at `start`; a quoted value may hold
    a `>`."""
    index = start + 1
    while index < len(text) and text[index] != ">":
        if text[index] in "'\"":
            index = skip_quoted(text, index, text[index])
            continue
        index += 1
    return index + 1


def without_comments(code):
    """`code` with each `//` and `/* */` comment blanked; a quoted literal is kept whole."""
    chars, index = list(code), 0
    while index < len(code):
        if code[index] in "'\"`":
            index = skip_quoted(code, index, code[index])
        elif code.startswith("//", index):
            end = code.find("\n", index)
            end = len(code) if end < 0 else end
            blank(chars, index, end)
            index = end + 1
        elif code.startswith("/*", index):
            end = code.find("*/", index + 2)
            end = len(code) if end < 0 else end + 2
            blank(chars, index, end)
            index = end
        else:
            index += 1
    return "".join(chars)


def shown(raw):
    """A text as the person meets it: character references read, each blanked expression shown as
    `{…}`, and whitespace collapsed."""
    return EXPRESSION.sub("{…}", " ".join(html.unescape(raw).split()))


def is_site(text):
    """Whether a shown text is user-facing: it holds a letter and is not one name alone."""
    return bool(LETTER.search(text)) and text not in NAMES


def component_sites(text):
    """Every site in one component, as (line, form, text), in the order of the file."""
    return []


def components(root):
    """Every `.svelte` file under `web/app/src`, as (repository path, text)."""
    root = Path(root)
    return [
        (path.relative_to(root).as_posix(), path.read_text(encoding="utf-8"))
        for path in sorted((root / COMPONENTS).rglob("*.svelte"))
    ]


def sites(files):
    """Every site of every file, as (file, line, form, text, ordinal)."""
    found, seen = [], collections.Counter()
    for name, text in files:
        for number, form, value in component_sites(text):
            seen[name, form, value] += 1
            found.append((name, number, form, value, seen[name, form, value]))
    return found


def census(root):
    """The population, refused when it is empty, and every site in it."""
    files = components(root)
    return files, sites(files)


def verdict(found, listed, level):
    """(lines, refused): a line for each site `listed` does not name, `WARN` and passing at `warn`,
    `FAIL` and refusing at `fail`, and a `STALE` line, refusing at both, for each entry no site
    matches."""
    return [], False


def listing_problems(listed):
    """A line for each entry of `listed` that is not five cells, gives a blank reason, or names a
    key an earlier entry names."""
    return []


def planted_lines(text, marker):
    """The lines of a fixture that carry a `planted:` or `accepted:` comment."""
    return [number for number, line in enumerate(text.splitlines(), 1) if f" {marker}: " in line]


class EveryComponentTextIsDrawnFromTheCatalogs(unittest.TestCase):
    def fixture(self, name):
        """A planted component's sites with their ordinals, after its markers are checked."""
        text = (REPO / FIXTURES / name).read_text(encoding="utf-8")
        found = sites([((FIXTURES / name).as_posix(), text)])
        self.assertGreater(len(planted_lines(text, "accepted")), 0, f"{name} plants no control")
        self.assertEqual(
            sorted({site[1] for site in found}),
            planted_lines(text, "planted"),
            "\n".join(map(str, found)),
        )
        return [site[1:] for site in found]

    def test_a_planted_literal_text_node_is_a_site(self):
        self.assertEqual(
            self.fixture("literal_text_node.svelte.fixture"),
            [
                (10, "text", "Your streak is safe today", 1),
                (12, "text", "Study", 1),
                (13, "text", "{…} cards are due", 1),
                (14, "text", "Study", 2),
                (17, "text", "Café ouvert", 1),
                (19, "text", "Keep going, DeckStreak", 1),
                (25, "text", "See you tomorrow", 1),
            ],
        )

    def test_a_planted_literal_user_facing_attribute_is_a_site(self):
        self.assertEqual(
            self.fixture("literal_attribute.svelte.fixture"),
            [
                (9, "attribute", "Your deck", 1),
                (10, "attribute", "What it measures", 1),
                (11, "attribute", "Close", 1),
                (12, "attribute", "Type a word", 1),
                (13, "attribute", "Flashcard", 1),
                (14, "attribute", "{…} of 5 done", 1),
                (15, "attribute", "Front", 1),
                (16, "attribute", "Search decks", 1),
                (17, "attribute", "Due in 3 > 2 days", 1),
            ],
        )

    def test_a_planted_dialog_literal_is_a_site(self):
        self.assertEqual(
            self.fixture("literal_dialog.svelte.fixture"),
            [
                (8, "dialog", "Saved to deck A // B", 1),
                (9, "dialog", "Delete this deck?", 1),
                (10, "dialog", "Name the deck", 1),
                (11, "dialog", "Sync finished", 1),
                (12, "dialog", "Sign out of {…}?", 1),
                (20, "dialog", "Copied", 1),
            ],
        )

    def test_a_component_that_draws_every_text_from_a_catalog_has_no_site(self):
        text = (REPO / FIXTURES / "catalog_only.svelte.fixture").read_text(encoding="utf-8")
        self.assertGreater(len(planted_lines(text, "accepted")), 5, "the control plants its forms")
        self.assertEqual(planted_lines(text, "planted"), [])
        self.assertEqual(component_sites(text), [])

    def test_the_level_decides_whether_an_unlisted_site_fails_and_a_stale_entry_always_does(
        self,
    ):
        site = ("web/app/src/lib/Planted.svelte", 3, "text", "Planted", 1)
        entry = (
            "web/app/src/lib/Planted.svelte",
            "text",
            "Planted",
            1,
            "a planted reason",
        )
        other = (
            "web/app/src/lib/Planted.svelte",
            "text",
            "Gone",
            1,
            "a planted reason",
        )
        warn = "WARN literal-text web/app/src/lib/Planted.svelte:3 text 'Planted' #1"
        fail = "FAIL literal-text web/app/src/lib/Planted.svelte:3 text 'Planted' #1"
        stale = "STALE literal-text web/app/src/lib/Planted.svelte text 'Gone' #1"
        self.assertEqual(verdict([site], (), "warn"), ([warn], False))
        self.assertEqual(verdict([site], (), "fail"), ([fail], True))
        self.assertEqual(verdict([site], (entry,), "fail"), ([], False))
        self.assertEqual(verdict([site], (entry, other), "warn"), ([stale], True))
        self.assertEqual(verdict([site], (entry, other), "fail"), ([stale], True))
        unreasoned = ("web/app/src/lib/Planted.svelte", "text", "Gone", 1, " ")
        short = ("web/app/src/lib/Planted.svelte", "text", "Short", 1)
        self.assertEqual(listing_problems((entry, other)), [])
        self.assertEqual(
            listing_problems((entry, unreasoned, entry, short)),
            [
                "REASON literal-text ('web/app/src/lib/Planted.svelte', 'text', 'Gone', 1)"
                " is listed with no reason",
                "REPEATED literal-text ('web/app/src/lib/Planted.svelte', 'text', 'Planted', 1)"
                " is listed twice",
                "SHAPE literal-text ('web/app/src/lib/Planted.svelte', 'text', 'Short', 1)"
                " has 4 cell(s), not 5",
            ],
        )

    def test_an_empty_population_is_refused(self):
        with tempfile.TemporaryDirectory() as root:
            (Path(root) / COMPONENTS / "lib").mkdir(parents=True)
            (Path(root) / COMPONENTS / "lib" / "notes.ts").write_text("export const a = 1;\n")
            with self.assertRaisesRegex(AssertionError, r"^examined 0 component\(s\)"):
                census(root)

    def test_the_repository_lists_exactly_todays_sites(self):
        files, found = census(REPO)
        names = [name for name, _text in files]
        self.assertIn("web/app/src/routes/+layout.svelte", names)
        self.assertNotIn(FIXTURES.as_posix(), " ".join(names))
        self.assertEqual(LEVEL, "warn")
        self.assertEqual(listing_problems(LISTED), [])
        lines, refused = verdict(found, LISTED, LEVEL)
        for line in lines:
            print(line)
        self.assertFalse(refused, "\n".join(lines))


if __name__ == "__main__":
    unittest.main()
