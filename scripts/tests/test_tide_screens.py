"""Every web screen draws its colours through the declared roles, and every card carries the border
role (SPEC-593 A6 and A13; R4 to R6; ADR-615 D3).

`colour_findings(root, files)` reads every `.svelte` file under `web/app/src` and returns one
`"<reason>: <path>:<line>: <detail>"` line for each colour class that names no declared `color` role
of the token file, each text colour class whose role no declared text pair draws as text, and each
colour literal: a hex or a colour function in a style context, or a Tailwind arbitrary colour.
`card_findings(files)` returns one line for each class list that holds `bg-card` without a `border`
width class, and for each `bg-card` outside a class list the census reads. The tests run both over
this repository, and over the repository with one planted file added.
The standard library only; nothing is imported from the tree.
"""

import json
import re
import unittest
from pathlib import Path

from _support import REPO, examined

SCREENS = Path("web/app/src")
TOKENS = Path("web/app/src/lib/design/tokens.json")
PLANTED = "web/app/src/lib/Planted.svelte"

# SPEC-593 R3: the roles a screen draws as text, each on a pair R3 declares: the ink roles, the
# muted text, and the background drawn on the ink of a filled control.
TEXT_ROLES = ("foreground", "card-foreground", "muted-foreground", "background")

# A Tailwind colour utility: its prefix, then a role name, a palette name or an arbitrary value.
UTILITY = re.compile(
    r"^(border-[xytrblse]|border|ring-offset|ring|bg|text|outline|decoration|divide|fill|stroke"
    r"|accent|caret|placeholder|shadow)-(.+)$"
)
# The suffixes of the same prefixes that set no colour: sizes, sides, widths, styles, layouts.
SIDES = r"(?:\d+|solid|dashed|dotted|double|hidden|none)"
NOT_A_COLOUR = {
    "text": r"(?:xs|sm|base|lg|[2-9]?xl|left|center|right|justify|start|end|wrap|nowrap|balance"
    r"|pretty|ellipsis|clip)",
    "border": r"(?:[xytrblse]|" + SIDES[3:-1] + r"|collapse|separate|spacing-.+)",
    "border-side": SIDES,
    "bg": r"(?:fixed|local|scroll|clip-.+|origin-.+|no-repeat|repeat(?:-.+)?|cover|contain|auto"
    r"|center|top|bottom|left|right|(?:left|right)-(?:top|bottom)|none|linear-.+|radial(?:-.+)?"
    r"|conic(?:-.+)?|gradient-.+|blend-.+)",
    "ring": r"(?:\d+|inset)",
    "ring-offset": r"\d+",
    "outline": r"(?:\d+|none|hidden|solid|dashed|dotted|double|offset-\d+)",
    "decoration": r"(?:\d+|solid|double|dotted|dashed|wavy|auto|from-font|clone|slice)",
    "divide": r"(?:[xy](?:-\d+)?|[xy]-reverse|solid|dashed|dotted|double|none)",
    "fill": r"none",
    "stroke": r"(?:\d+|none)",
    "shadow": r"(?:2xs|xs|sm|md|lg|xl|2xl|none|inner)",
    "accent": r"auto",
    "caret": r"(?!)",
    "placeholder": r"(?!)",
}
# Keywords that paint no colour of their own.
NO_PAINT = ("transparent", "current", "inherit")
SPLIT = re.compile(r"[\s\"'`{}<>,;]+")
VARIANT = re.compile(r":(?![^\[]*\])")
FUNCTION = re.compile(r"\b(?:rgba?|hsla?|hwb|oklch|oklab|lab|lch|color-mix)\(")
HEX_IN_STYLE = re.compile(r"(?:[:(,]|=[\"'])\s*(#[0-9a-fA-F]{3,8})\b")
CLASS_LIST = re.compile(r"\bclass=([\"'])(.*?)\1", re.S)
BORDER_WIDTH = re.compile(r"^border(?:-[xytrblse])?(?:-\d+)?$")


def declared_roles(root):
    """The names of the token file's `color` tier, as it states them."""
    document = json.loads((Path(root) / TOKENS).read_text(encoding="utf-8"))
    return sorted(name for name in document.get("color", {}) if not name.startswith("$"))


def screen_files(root):
    """Every `.svelte` file under `web/app/src`, as (repository path, text)."""
    root = Path(root)
    return [
        (path.relative_to(root).as_posix(), path.read_text(encoding="utf-8"))
        for path in sorted((root / SCREENS).rglob("*.svelte"))
    ]


def utility_of(token):
    """The utility a class token sets, its variants and `!` stripped, or None."""
    last = VARIANT.split(token.lstrip("!"))[-1].lstrip("!").rstrip("=)")
    return re.sub(r"/[\w.%\[\]]+$", "", last)


def colour_findings(files, roles):
    """Each off-role colour class and colour literal of `files`, and how many colour classes it judged."""
    problems, judged = [], 0
    for path, text in files:
        for number, line in enumerate(text.splitlines(), 1):
            for token in SPLIT.split(line):
                utility = utility_of(token)
                match = UTILITY.match(utility)
                if not match:
                    continue
                prefix, rest = match.groups()
                group = "border-side" if prefix.startswith("border-") else prefix
                if re.fullmatch(NOT_A_COLOUR[group], rest) or rest in NO_PAINT:
                    continue
                if re.fullmatch(r"\[[^\]#(]*\]", rest):
                    continue  # an arbitrary length or keyword, such as text-[0.625rem]
                judged += 1
                where = f"{path}:{number}"
                if rest.startswith(("[", "(")) or "#" in rest:
                    problems.append(f"raw-colour: {where}: {utility} holds a colour literal")
                elif rest not in roles:
                    problems.append(
                        f"off-role: {where}: {utility} names {rest}, no declared colour role"
                    )
                elif prefix == "text" and rest not in TEXT_ROLES:
                    problems.append(
                        f"off-role: {where}: {utility} draws {rest} as text, which no declared pair does"
                    )
            for found in FUNCTION.finditer(line):
                problems.append(
                    f"raw-colour: {path}:{number}: {found.group(0)} is a colour function"
                )
            for found in HEX_IN_STYLE.finditer(line):
                problems.append(f"raw-colour: {path}:{number}: {found.group(1)} is a hex colour")
    return problems, judged


def card_findings(files):
    """Each card without the border role in `files`, and how many cards it judged."""
    problems, judged = [], 0
    for path, text in files:
        inside = 0
        for found in CLASS_LIST.finditer(text):
            tokens = found.group(2).split()
            if not any(utility_of(token) == "bg-card" for token in tokens):
                continue
            judged += 1
            inside += sum(utility_of(token) == "bg-card" for token in tokens)
            if not any(BORDER_WIDTH.match(token) for token in tokens):
                number = text.count("\n", 0, found.start()) + 1
                problems.append(f"card-without-border: {path}:{number}: {found.group(2).strip()}")
        anywhere = sum(utility_of(token) == "bg-card" for token in SPLIT.split(text))
        if anywhere != inside:
            problems.append(
                f"card-unread: {path}: {anywhere - inside} bg-card outside a class list the census reads"
            )
    return problems, judged


# One planted line per way a colour can leave the roles, each its own finding.
PLANTED_COLOURS = "\n".join(
    (
        '<div class="rounded-lg bg-muted p-4">a</div>',
        '<a class="text-sm text-link" href="/">a</a>',
        '<div class="border border-primary">a</div>',
        '<p class="text-blue-500">a</p>',
        '<div class="bg-[#ff0000]">a</div>',
        '<p style="color: #ff0000">a</p>',
        '<div style="background: rgb(1 2 3)">a</div>',
    )
)
PLANTED_CARDS = "\n".join(
    (
        '<div class="rounded-lg bg-card p-4 text-card-foreground">a</div>',
        '<div class="rounded-lg border bg-card p-4 text-card-foreground">a</div>',
    )
)


class EveryScreenDrawsThroughTheRoles(unittest.TestCase):
    def test_every_screen_colour_class_is_a_declared_role(self):
        """A6: no screen colour class leaves the declared roles, and no screen holds a literal."""
        roles = examined("declared colour roles", declared_roles(REPO))
        files = examined("screen files", screen_files(REPO))
        planted, _ = colour_findings([(PLANTED, '<a class="text-link" href="/">a</a>')], roles)
        self.assertEqual(len(planted), 1, "a planted member: the census sees an off-role class")
        found, judged = colour_findings(files, roles)
        examined("colour classes judged", range(judged))
        self.assertEqual(found, [])

    def test_a_planted_off_role_class_or_literal_is_refused(self):
        """A6: each planted off-role class and colour literal is refused, and nothing else is."""
        roles = declared_roles(REPO)
        files = examined("screen files", screen_files(REPO))
        found, _ = colour_findings(files + [(PLANTED, PLANTED_COLOURS)], roles)
        self.assertEqual(
            [finding.split(": ")[1] for finding in found],
            [f"{PLANTED}:{number}" for number in range(1, 8)],
            found,
        )
        self.assertEqual(
            [finding.split(": ")[0] for finding in found], ["off-role"] * 4 + ["raw-colour"] * 3
        )

    def test_every_card_carries_the_border_role(self):
        """A13: every class list that holds bg-card holds a border width class."""
        files = examined("screen files", screen_files(REPO))
        planted, _ = card_findings([(PLANTED, PLANTED_CARDS)])
        self.assertEqual(
            len(planted), 1, "a planted member: the census sees a card without a border"
        )
        found, judged = card_findings(files)
        examined("cards judged", range(judged))
        self.assertEqual(found, [])

    def test_a_planted_card_without_the_border_is_refused(self):
        """A13: a planted card without the border is refused, and its bordered twin and the
        repository's cards are not."""
        files = examined("screen files", screen_files(REPO))
        found, judged = card_findings(files + [(PLANTED, PLANTED_CARDS)])
        self.assertGreater(judged, 2)
        self.assertEqual(
            found,
            [
                f"card-without-border: {PLANTED}:1: rounded-lg bg-card p-4 text-card-foreground",
            ],
        )


if __name__ == "__main__":
    unittest.main()
