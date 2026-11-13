"""SPEC-679 section 9a: the app target `DeckStreak` in `ios/app.yml` declares the iOS and macOS
destinations, a macOS deployment target equal to the SPEC's minimum macOS version, and builds
neither Mac Catalyst nor the iPad app run on a Mac. Every file is read as committed text."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
APP = "ios/app.yml"
XCCONFIG = "ios/Config/App.xcconfig"
SPEC = "docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md"
TARGET = "DeckStreak"
MINIMUM = re.compile(r"macOS (\d+(?:\.\d+)+) as the destination's minimum version")
# Each build setting that, set to NO, keeps a Mac build of the iPad app off the target: the Mac
# Catalyst build, and the iPad app run unchanged on a Mac.
MAC_OFF = ("SUPPORTS_MACCATALYST", "SUPPORTS_MAC_DESIGNED_FOR_IPHONE_IPAD")


def examined(what, items):
    """Print how many items a check examined, and refuse an empty population."""
    items = list(items)
    print(f"examined {len(items)} {what}")
    if not items:
        raise AssertionError(f"examined 0 {what}: the population is empty, so nothing was judged")
    return items


def text_of(case, relative):
    """Assert that a committed file is present, then return its text."""
    path = REPO / relative
    case.assertTrue(path.is_file(), f"{relative} is not a file")
    return path.read_text(encoding="utf-8")


def spec_minimum(case):
    """The one minimum macOS version SPEC-679 names, read across line breaks."""
    named = set(MINIMUM.findall(" ".join(text_of(case, SPEC).split())))
    case.assertEqual(len(named), 1, f"SPEC-679 names {sorted(named)} as its minimum macOS version")
    return named.pop()


def depth(line):
    return len(line) - len(line.lstrip(" "))


def target_block(case, lines):
    """The lines under the target's key in `targets:`, up to the next key at its indent."""
    case.assertIn("targets:", lines, f"{APP} holds no targets key")
    head = lines.index("targets:")
    case.assertIn(f"  {TARGET}:", lines[head:], f"{APP} holds no {TARGET} target")
    block = []
    for line in lines[lines.index(f"  {TARGET}:", head) + 1 :]:
        if line.strip() and depth(line) <= 2:
            break
        block.append(line)
    return block


def unquoted(value):
    return value.split(" #")[0].strip().strip("\"'")


def values(block, key):
    """Each value a `key:` line in the block holds."""
    found = []
    for line in block:
        name, colon, value = line.strip().partition(":")
        if colon and name == key and value.strip():
            found.append(unquoted(value))
    return found


def entries_under(block, key):
    """The `name: value` lines indented under a block-form `key:` line."""
    for at, line in enumerate(block):
        if line.strip() == f"{key}:":
            entries = {}
            for inner in block[at + 1 :]:
                if inner.strip() and depth(inner) <= depth(line):
                    break
                name, colon, value = inner.strip().partition(":")
                if colon and value.strip():
                    entries[name] = unquoted(value)
            return entries
    return {}


def xcconfig_values(case, key):
    """Each value the target's xcconfig gives the key."""
    found = []
    for line in text_of(case, XCCONFIG).splitlines():
        name, equals, value = line.split("//")[0].partition("=")
        if equals and name.strip() == key:
            found.append(value.strip())
    return found


class TheAppTargetDeclaresTheMacDestination(unittest.TestCase):
    def test_the_app_target_declares_ios_and_macos_at_the_minimum(self):
        """SPEC-679 section 9a: the target declares the iOS and macOS destinations and leaves its
        platform to them, its macOS deployment target is the SPEC's minimum beside its iOS one,
        and it sets each Mac build of the iPad app off."""
        minimum = spec_minimum(self)
        block = examined(
            f"lines of the {TARGET} target", target_block(self, text_of(self, APP).splitlines())
        )
        self.assertEqual(
            values(block, "supportedDestinations"),
            ["[iOS, macOS]"],
            f"the {TARGET} target declares the iOS and macOS destinations alone",
        )
        self.assertEqual(
            values(block, "platform"), ["auto"], f"the {TARGET} target's platform is auto"
        )
        deployment = entries_under(block, "deploymentTarget")
        self.assertEqual(
            deployment.get("macOS"), minimum, "the macOS deployment target is SPEC-679's minimum"
        )
        self.assertIn("iOS", deployment, "the iOS deployment target stands beside the macOS one")
        for key in examined("Mac builds of the iPad app", MAC_OFF):
            self.assertEqual(
                values(block, key) + xcconfig_values(self, key),
                ["NO"],
                f"the {TARGET} target sets {key} to NO once",
            )


if __name__ == "__main__":
    unittest.main()
