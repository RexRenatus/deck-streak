"""One file builds the card web view on iPhone and iPad, and it carries every layer (SPEC-349 R1,
R2 and R9, A4; ADR-360 D2 and D5).

The tree under `ios/`, test targets aside, constructs a `WKWebView` or a `WKWebViewConfiguration`
in exactly one file, the factory in the `CardIsolation` package. That file sets each layer a
token can show; nothing outside test targets adds a script message handler, loads a file URL or
calls the probe's `make(layers:` door; and only the test-only probe host declares local
networking, with nothing else under its transport key. The checker reads one root, so it judges
the real tree and planted trees alike: each planted tree breaks one rule and must be refused by
that rule's name, the positive control the absence census needs.
"""

import plistlib
import re
import tempfile
import unittest
from pathlib import Path

from _support import REPO, examined

FACTORY = "ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift"
HARNESS_PLIST = "ios/Harness/Info.plist"
PROBE_PLIST = "ios/CardProbeHost/Info.plist"
# The one transport answer the probe host may give: local networking, and nothing else.
PROBE_TRANSPORT = {"NSAllowsLocalNetworking": True}
# Each layer the factory's source can show, with the token that shows it (the schematic's
# section 4). L4 is an absence the handler rule below holds for the whole tree.
LAYER_TOKENS = (
    ("L1", "WKWebsiteDataStore.nonPersistent()"),
    ("L2", "allowsContentJavaScript = false"),
    ("L3", "userContentController.add(ruleList)"),
    ("L5", "navigationDelegate = gate"),
    ("L6", "uiDelegate = refusal"),
    ("L7", "loadHTMLString(html, baseURL: nil)"),
)
CONSTRUCTS = re.compile(r"\bWKWebView(?:Configuration)?\(")
HANDLER = re.compile(r"addScriptMessageHandler|userContentController\.add\([^)\n]*\bname:")
FILE_LOAD = re.compile(r"\bloadFileURL\(")
PROBE_DOOR = re.compile(r"\bmake\(layers:")


def is_test_target(relative):
    """True for a file inside a test target: a directory below `ios/` named `Tests` or ending in
    `Tests` (`ios/CardProbeTests/`, `ios/HarnessUITests/`, `ios/CardIsolation/Tests/`)."""
    return any(part == "Tests" or part.endswith("Tests") for part in Path(relative).parts[1:-1])


def code_of(text):
    """A Swift source without its whole-line comments, so a token only a comment holds is not
    read as code."""
    return "\n".join(line for line in text.splitlines() if not line.lstrip().startswith("//"))


def plist_or_problem(root, relative, problems):
    """The property list at `relative` under `root`, or None with the reason in `problems`."""
    path = root / relative
    if not path.is_file():
        problems.append(f"{relative}: missing")
        return None
    try:
        value = plistlib.loads(path.read_bytes())
    except (plistlib.InvalidFileException, ValueError) as error:
        problems.append(f"{relative}: not a property list ({error})")
        return None
    if not isinstance(value, dict):
        problems.append(f"{relative}: not a dictionary")
        return None
    return value


def card_view_problems(root):
    """Every rule of the card view's construction the tree at `root` breaks, each named, and what
    was judged: the Swift files outside test targets, the files that construct a view, and the
    property lists read."""
    root = Path(root)
    problems = []
    judged = {"swift files": [], "constructors": [], "property lists": []}
    sources = sorted((root / "ios").rglob("*.swift")) if (root / "ios").is_dir() else []
    for path in sources:
        relative = path.relative_to(root).as_posix()
        if is_test_target(relative) or ".build" in path.parts:
            continue
        judged["swift files"].append(relative)
        code = code_of(path.read_bytes().decode("utf-8", errors="replace"))
        if CONSTRUCTS.search(code):
            judged["constructors"].append(relative)
            if relative != FACTORY:
                problems.append(f"{relative}: constructs a card web view outside the factory")
        if HANDLER.search(code):
            problems.append(f"{relative}: adds a script message handler")
        if FILE_LOAD.search(code):
            problems.append(f"{relative}: loads a file URL")
        if relative != FACTORY and PROBE_DOOR.search(code):
            problems.append(f"{relative}: calls make(layers:), the probe's door")
    factory = root / FACTORY
    if not factory.is_file():
        problems.append(f"{FACTORY}: missing")
    else:
        code = code_of(factory.read_bytes().decode("utf-8", errors="replace"))
        for layer, token in LAYER_TOKENS:
            if token not in code:
                problems.append(f"{FACTORY}: lacks {layer} ({token})")
    plists = sorted((root / "ios").rglob("Info.plist")) if (root / "ios").is_dir() else []
    for path in plists:
        relative = path.relative_to(root).as_posix()
        if relative == PROBE_PLIST:
            continue
        judged["property lists"].append(relative)
        value = plist_or_problem(root, relative, problems)
        if value is not None and "NSAppTransportSecurity" in value:
            problems.append(
                f"{relative}: declares NSAppTransportSecurity, which only the probe host may"
            )
    if HARNESS_PLIST not in judged["property lists"]:
        problems.append(f"{HARNESS_PLIST}: missing")
    probe = plist_or_problem(root, PROBE_PLIST, problems)
    if probe is not None:
        judged["property lists"].append(PROBE_PLIST)
        transport = probe.get("NSAppTransportSecurity", "absent")
        exact = isinstance(transport, dict) and transport.keys() == PROBE_TRANSPORT.keys()
        # `True == 1` in Python, so the answer is compared by identity: the boolean true.
        if not (exact and transport["NSAllowsLocalNetworking"] is True):
            problems.append(
                f"{PROBE_PLIST}: NSAppTransportSecurity is {transport!r}, not {PROBE_TRANSPORT!r}"
            )
    return problems, judged


GOOD_FACTORY = """import WebKit
// The planted factory: every token, once.
let configuration = WKWebViewConfiguration()
configuration.websiteDataStore = WKWebsiteDataStore.nonPersistent()
configuration.defaultWebpagePreferences.allowsContentJavaScript = false
configuration.userContentController.add(ruleList)
let view = WKWebView(frame: .zero, configuration: configuration)
view.navigationDelegate = gate
view.uiDelegate = refusal
view.loadHTMLString(html, baseURL: nil)
func make(layers: Set<CardLayer>) {}
"""
GOOD_HARNESS = "struct CardWebView { let built = CardWebViewFactory.makeCardWebView(html: html) }\n"
# A test target may build any view it needs: the reference views are built there.
GOOD_PROBE_TEST = (
    "let reference = WKWebView(frame: .zero, configuration: configuration)\n"
    'reference.configuration.userContentController.add(counter, name: "bridge")\n'
    "reference.loadFileURL(card, allowingReadAccessTo: folder)\n"
    "let variant = CardWebViewFactory.make(layers: [])\n"
)
GOOD_INFO = {"CFBundlePackageType": "APPL"}
GOOD_PROBE_INFO = {"CFBundlePackageType": "APPL", "NSAppTransportSecurity": dict(PROBE_TRANSPORT)}


def plant(root, factory=GOOD_FACTORY, info=None, probe_info=None, extra=None):
    """Writes a card-view tree under `root`: the good tree, with `factory` (None for none),
    `info`, `probe_info` and `extra` (relative path to text) in place of or beside its files."""
    files = {
        "ios/Harness/Sources/CardWebView.swift": GOOD_HARNESS,
        "ios/CardProbeTests/PlantedCardTests.swift": GOOD_PROBE_TEST,
        "ios/CardIsolation/Tests/CardIsolationTests/GateTests.swift": GOOD_PROBE_TEST,
        **({FACTORY: factory} if factory is not None else {}),
        **(extra or {}),
    }
    for relative, text in files.items():
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    for relative, value in ((HARNESS_PLIST, info or GOOD_INFO), (PROBE_PLIST, probe_info)):
        path = root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(plistlib.dumps(GOOD_PROBE_INFO if value is None else value))


class OneFactoryBuildsTheCardWebView(unittest.TestCase):
    def test_one_file_builds_the_card_web_view_and_it_carries_every_layer(self):
        # The behaviour first: exactly one file under ios/, test targets aside, builds the view.
        problems, judged = card_view_problems(REPO)
        self.assertEqual(judged["constructors"], [FACTORY])
        self.assertEqual(problems, [])
        examined("Swift files outside test targets", judged["swift files"])
        examined("property lists", judged["property lists"])

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        plants = {
            "the good tree": ({}, []),
            "a second constructor": (
                {"extra": {"ios/Harness/Sources/Other.swift": "let v = WKWebView(frame: .zero)\n"}},
                ["ios/Harness/Sources/Other.swift: constructs a card web view outside the factory"],
            ),
            "a second configuration": (
                {
                    "extra": {
                        "ios/Harness/Sources/Other.swift": "let c = WKWebViewConfiguration()\n"
                    }
                },
                ["ios/Harness/Sources/Other.swift: constructs a card web view outside the factory"],
            ),
            "no factory": (
                {"factory": None},
                [f"{FACTORY}: missing"],
            ),
            "a handler added": (
                {
                    "extra": {
                        "ios/Harness/Sources/Bridge.swift": "c.addScriptMessageHandler(h, name: n)\n"
                    }
                },
                ["ios/Harness/Sources/Bridge.swift: adds a script message handler"],
            ),
            "a handler added to the user content controller": (
                {
                    "extra": {
                        "ios/Harness/Sources/Bridge.swift": (
                            'view.configuration.userContentController.add(h, name: "bridge")\n'
                        )
                    }
                },
                ["ios/Harness/Sources/Bridge.swift: adds a script message handler"],
            ),
            "a file URL loaded": (
                {
                    "extra": {
                        "ios/Harness/Sources/Files.swift": "view.loadFileURL(card, allowingReadAccessTo: d)\n"
                    }
                },
                ["ios/Harness/Sources/Files.swift: loads a file URL"],
            ),
            "the probe's door called from the app": (
                {
                    "extra": {
                        "ios/Harness/Sources/Door.swift": "let v = CardWebViewFactory.make(layers: [])\n"
                    }
                },
                ["ios/Harness/Sources/Door.swift: calls make(layers:), the probe's door"],
            ),
            "a transport key in the harness": (
                {"info": {**GOOD_INFO, "NSAppTransportSecurity": dict(PROBE_TRANSPORT)}},
                [
                    f"{HARNESS_PLIST}: declares NSAppTransportSecurity, which only the probe host may"
                ],
            ),
            "the probe host's transport widened": (
                {
                    "probe_info": {
                        **GOOD_PROBE_INFO,
                        "NSAppTransportSecurity": {
                            "NSAllowsLocalNetworking": True,
                            "NSAllowsArbitraryLoads": True,
                        },
                    }
                },
                [
                    f"{PROBE_PLIST}: NSAppTransportSecurity is {{'NSAllowsArbitraryLoads': True, "
                    f"'NSAllowsLocalNetworking': True}}, not {PROBE_TRANSPORT!r}"
                ],
            ),
            "the probe host's answer of one": (
                {
                    "probe_info": {
                        **GOOD_PROBE_INFO,
                        "NSAppTransportSecurity": {"NSAllowsLocalNetworking": 1},
                    }
                },
                [
                    f"{PROBE_PLIST}: NSAppTransportSecurity is {{'NSAllowsLocalNetworking': 1}}, "
                    f"not {PROBE_TRANSPORT!r}"
                ],
            ),
        }
        for layer, token in LAYER_TOKENS:
            plants[f"the factory without {layer}"] = (
                {"factory": GOOD_FACTORY.replace(token, "")},
                [f"{FACTORY}: lacks {layer} ({token})"],
            )
            plants[f"the factory with {layer} only in a comment"] = (
                {"factory": GOOD_FACTORY.replace(token, "") + f"// {token}\n"},
                [f"{FACTORY}: lacks {layer} ({token})"],
            )
        for name, (shape, wanted) in examined("planted trees", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                plant(Path(scratch), **shape)
                self.assertEqual(card_view_problems(Path(scratch))[0], wanted, name)


if __name__ == "__main__":
    unittest.main()
