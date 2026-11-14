"""SPEC-679 section 9a: every section 1 site that needs a platform conditional or a macOS
counterpart takes its macOS form. The sites are read from SPEC-679's section 1 table, and each
site's iOS-only names are read in the app's sources as committed text, by the platform each line
compiles for."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
SPEC = "docs/specs/SPEC-679-a-native-macos-destination-for-the-one-app-target.md"
APP = "ios/App/Sources/"
FACTORY = "ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift"
NEEDS = ("needs a platform conditional", "needs a macOS counterpart")
STRING = re.compile(r'"(?:[^"\\\n]|\\.)*"')
# Each site, by a phrase of its section 1 site cell, and what its macOS form holds: a name that
# compiles for iOS alone ("ios"), a name no line keeps ("gone"), a name a macOS line holds
# ("mac"), or a name every keychain query holds ("query").
SITES = (
    (
        "UIViewRepresentable",
        (
            (APP + "CardFaceView.swift", "UIViewRepresentable", "ios"),
            (APP + "CardFaceView.swift", "NSViewRepresentable", "mac"),
        ),
    ),
    (
        "UIFontMetrics",
        (
            (APP + "CardFaceView.swift", "UIFontMetrics", "gone"),
            (APP + "CardFaceView.swift", "CardWebViewFactory.bodyTextZoom", "mac"),
            (FACTORY, "UIFontMetrics", "ios"),
            (FACTORY, "bodyTextZoom", "mac"),
        ),
    ),
    (
        "UIAccessibility.post",
        (
            (APP + "CardRefusalWords.swift", "UIAccessibility", "ios"),
            (APP + "CardRefusalWords.swift", "AccessibilityNotification.Announcement", "mac"),
        ),
    ),
    (
        "UIAccessibility.isVoiceOverRunning",
        (
            (APP + "ReviewModel.swift", "UIAccessibility", "gone"),
            (APP + "AppModel.swift", "UIAccessibility.isVoiceOverRunning", "ios"),
            (APP + "AppModel.swift", "NSWorkspace.shared.isVoiceOverEnabled", "mac"),
        ),
    ),
    (
        "import UIKit",
        (
            (APP + "CardFaceView.swift", "import UIKit", "gone"),
            (APP + "ReviewModel.swift", "import UIKit", "gone"),
            (APP + "CardRefusalWords.swift", "import UIKit", "ios"),
        ),
    ),
    (
        "AVAudioSession",
        (
            (APP + "ClipPlayer.swift", "AVAudioSession", "gone"),
            (APP + "InstalledVoices.swift", "AVAudioSession", "ios"),
        ),
    ),
    (
        "navigationBarTitleDisplayMode",
        ((APP + "CardHealthView.swift", "navigationBarTitleDisplayMode", "ios"),),
    ),
    (
        "keychain item",
        ((APP + "SyncCredentialStore.swift", "kSecUseDataProtectionKeychain: true", "query"),),
    ),
    ("platforms and slices", (("ios/EnginePackage/Package.swift", ".macOS(", "mac"),)),
)


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


def platform(stack):
    """The platform a line compiles for under its open conditionals: "iOS", "macOS" or "both"."""
    for condition, in_else in stack:
        if condition in ("os(iOS)", "canImport(UIKit)"):
            return "macOS" if in_else else "iOS"
        if condition == "os(macOS)":
            return "iOS" if in_else else "macOS"
    return "both"


def code_lines(text):
    """Each code line, its string text and comment removed, with the platform it compiles for."""
    stack, lines = [], []
    for raw in text.splitlines():
        code = STRING.sub('""', raw).split("//")[0].strip()
        if code.startswith("#if "):
            stack.append([code[4:].strip(), False])
        elif code == "#else" and stack:
            stack[-1][1] = True
        elif code == "#endif" and stack:
            stack.pop()
        elif code:
            lines.append((platform(stack), code))
    return lines


def section_one_sites(case):
    """The site cell of each section 1 row whose class needs a conditional or a counterpart."""
    sites = []
    for line in text_of(case, SPEC).splitlines():
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if line.startswith("|") and len(cells) == 4 and cells[2].startswith(NEEDS):
            sites.append(cells[0])
    return sites


def judged(case, path, name, rule):
    """The problems one name's rule finds in one file."""
    lines = code_lines(text_of(case, path))
    where = [compiled for compiled, code in lines if name in code]
    if rule == "ios" and set(where) - {"iOS"}:
        return [f"{path}: {name} compiles for {sorted(set(where) - {'iOS'})}, not for iOS alone"]
    if rule == "gone" and where:
        return [f"{path}: {name} is still named {len(where)} times"]
    if rule == "mac" and not set(where) & {"macOS", "both"}:
        return [f"{path}: no line that compiles for macOS names {name}"]
    queries = sum("kSecClass:" in code for _, code in lines)
    if rule == "query" and (not queries or len(where) != queries):
        return [f"{path}: {name} is in {len(where)} of its {queries} keychain queries"]
    return []


class TheSitesTakeTheirMacosForms(unittest.TestCase):
    def test_every_ios_only_site_sits_behind_its_conditional_or_its_counterpart(self):
        """SPEC-679 section 9a: each section 1 site that needs a platform conditional or a macOS
        counterpart is one of this census's sites, and each keeps its iOS-only names for iOS and
        holds its macOS form."""
        sites = examined("section 1 sites that need a macOS form", section_one_sites(self))
        matched = []
        for site in sites:
            phrases = [phrase for phrase, _ in SITES if phrase in site]
            self.assertEqual(len(phrases), 1, f"the section 1 site {site!r} is one census site")
            matched += phrases
        self.assertEqual(
            sorted(matched),
            sorted(phrase for phrase, _ in SITES),
            "each census site is a section 1 site",
        )
        forms = examined("site forms", [form for _, forms in SITES for form in forms])
        problems = [problem for form in forms for problem in judged(self, *form)]
        self.assertEqual(problems, [], "each section 1 site takes its macOS form")


if __name__ == "__main__":
    unittest.main()
