"""The native macOS destination of the one app target, read as committed text (SPEC-679).

ADR-342 carries one appended amendment that adds a native macOS destination to its one SwiftUI
app target, rejects the other ways to reach the Mac, names the minimum macOS version and states
the Mac idiom (R1 to R4), SPEC-679 names the same minimum (R3), and the iPhone and iPad
destination in `ios/app.yml` stands (R5). Each test reads the committed text as text, asserts the
literals it writes, and prints how much it examined.
"""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ADR = (
    "docs/decisions/ADR-342-one-universal-iphone-and-ipad-app-driven-by-touch-keys-or-an-"
    "8bitdo-remote.md"
)
SPEC = "docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md"
APP = "ios/app.yml"

# The opening of the last line ADR-342 held before its macOS amendment was appended.
LAST_LINE = "- Amending this record only when the native client changes too: rejected because"
REJECTED = (
    "Mac Catalyst: rejected because",
    "A second app target or project for macOS: rejected because",
)
# The two phrasings that name a minimum macOS version: the amendment's and the SPEC's.
MINIMUM = (
    r"minimum version is macOS (\d+(?:\.\d+)+)",
    r"macOS (\d+(?:\.\d+)+) as the destination's minimum version",
)
IDIOM = ("body text at 13 pt", "controls 28 pt high", "menus", "windows")


def examined(what, items):
    """Print how many items a check examined, and refuse an empty population."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def lines_of(case, relative):
    """Assert that a committed file is present, then return its lines."""
    path = REPO / relative
    case.assertTrue(path.is_file(), f"{relative} is not a file")
    return path.read_text(encoding="utf-8").splitlines()


def joined(lines):
    """The lines as one text, each run of whitespace one space, so a phrase reads across a line
    break."""
    return " ".join(" ".join(lines).split())


def macos_headings(lines):
    """The numbers, from 1, of the heading lines that name macOS and SPEC-679."""
    return [
        number
        for number, line in enumerate(lines, start=1)
        if line.startswith("#") and "macOS" in line and "SPEC-679" in line
    ]


def amendment(lines):
    """The first macOS amendment's section, from its heading to the next heading of its level or
    above, whitespace joined; empty when ADR-342 holds no such heading."""
    numbers = macos_headings(lines)
    if not numbers:
        return ""
    body = []
    for line in lines[numbers[0] :]:
        if re.match(r"#{1,2} ", line):
            break
        body.append(line)
    return joined(body)


def minimums(text):
    """Each distinct macOS version the text names as a minimum, in either phrasing."""
    return sorted({version for pattern in MINIMUM for version in re.findall(pattern, text)})


def indent(line):
    """How many spaces open a line."""
    return re.match(r" *", line).end()


def entries(lines):
    """The lines of ios/app.yml that hold a key, a value or an item: no blank or comment line."""
    return [line for line in lines if line.strip() and not line.lstrip().startswith("#")]


def keys_under(lines, top):
    """The keys indented two spaces under one top-level key of ios/app.yml."""
    keys, inside = [], False
    for line in entries(lines):
        if indent(line) == 0:
            inside = line.rstrip() == f"{top}:"
        elif inside and indent(line) == 2 and line.rstrip().endswith(":"):
            keys.append(line.strip()[:-1])
    return keys


def target(lines, name):
    """The lines of one target under `targets:` in ios/app.yml, read by their indentation."""
    block, inside, within = [], False, False
    for line in entries(lines):
        if indent(line) == 0:
            inside, within = line.rstrip() == "targets:", False
        elif inside and indent(line) == 2:
            within = line.strip() == f"{name}:"
        elif within:
            block.append(line)
    return block


def unquoted(value):
    """A value with its spaces and its quotes taken off."""
    return value.strip().strip("\"'")


def values(block, key):
    """The values one key holds in a target's lines."""
    found = []
    for line in block:
        name, _, value = line.strip().partition(":")
        if name == key:
            found.append(unquoted(value))
    return found


def ios_deployment_targets(block):
    """A target's iOS deployment targets: the value of `deploymentTarget`, or the value of the
    `iOS` key in the map it holds."""
    found = []
    for number, line in enumerate(block):
        name, _, value = line.strip().partition(":")
        if name != "deploymentTarget":
            continue
        if value.strip():
            found.append(unquoted(value))
            continue
        for child in block[number + 1 :]:
            if indent(child) <= indent(line):
                break
            key, _, inner = child.strip().partition(":")
            if key == "iOS":
                found.append(unquoted(inner))
    return found


class TheMacosDestinationIsRecorded(unittest.TestCase):
    def test_adr_342_carries_one_macos_amendment(self):
        """R1, R2: ADR-342 holds exactly one heading that names macOS and SPEC-679, after the last
        line the record held before it, and that section rejects Mac Catalyst and a second app
        target or project."""
        lines = lines_of(self, ADR)
        examined("heading(s) of ADR-342", [line for line in lines if line.startswith("#")])
        found = macos_headings(lines)
        self.assertEqual(
            len(found),
            1,
            f"ADR-342 holds {len(found)} heading(s) naming macOS and SPEC-679, not one",
        )
        last = [number for number, line in enumerate(lines, start=1) if line.startswith(LAST_LINE)]
        self.assertEqual(len(last), 1, "ADR-342 does not hold its earlier last line once")
        self.assertGreater(
            found[0], last[0], "the macOS amendment does not follow every line ADR-342 held"
        )
        section = amendment(lines)
        for phrase in REJECTED:
            self.assertIn(phrase, section, f"the macOS amendment of ADR-342 lacks: {phrase}")

    def test_the_amendment_and_the_spec_name_one_minimum(self):
        """R3: the amendment and SPEC-679 each name one minimum macOS version, read across line
        breaks, and both name 14.0."""
        lines = examined("line(s) of ADR-342", lines_of(self, ADR))
        named = minimums(amendment(lines))
        self.assertEqual(
            len(named),
            1,
            f"the macOS amendment of ADR-342 names {len(named)} minimum macOS version(s), not one",
        )
        self.assertEqual(named[0], "14.0", "the macOS amendment of ADR-342 names another minimum")
        spec = examined("line(s) of SPEC-679", lines_of(self, SPEC))
        stated = minimums(joined(spec))
        self.assertEqual(
            len(stated), 1, f"SPEC-679 names {len(stated)} minimum macOS version(s), not one"
        )
        self.assertEqual(stated[0], "14.0", "SPEC-679 names another minimum macOS version")
        self.assertEqual(stated, named, "SPEC-679 and the macOS amendment name two minimums")

    def test_the_amendment_states_the_mac_idiom(self):
        """R4: the amendment names body text at 13 pt, controls 28 pt high, and the Mac's menus
        and windows, read across line breaks."""
        lines = examined("line(s) of ADR-342", lines_of(self, ADR))
        section = amendment(lines)
        for phrase in IDIOM:
            self.assertIn(phrase, section, f"the macOS amendment of ADR-342 lacks: {phrase}")

    def test_the_ios_destination_stands(self):
        """R5: the DeckStreak target in ios/app.yml keeps its iOS deployment target, its device
        family, its scheme and its two test targets."""
        lines = lines_of(self, APP)
        block = examined(
            "line(s) of the DeckStreak target in ios/app.yml", target(lines, "DeckStreak")
        )
        self.assertEqual(
            ios_deployment_targets(block),
            ["17.0"],
            f"{APP}: the DeckStreak target does not keep its iOS deployment target",
        )
        self.assertEqual(
            values(block, "TARGETED_DEVICE_FAMILY"),
            ["1,2"],
            f"{APP}: the DeckStreak target does not keep its device family",
        )
        targets = keys_under(lines, "targets")
        for name in ("DeckStreakTests", "DeckStreakUITests"):
            self.assertIn(name, targets, f"{APP} holds no target named {name}")
        self.assertIn(
            "DeckStreak", keys_under(lines, "schemes"), f"{APP} holds no DeckStreak scheme"
        )


if __name__ == "__main__":
    unittest.main()
