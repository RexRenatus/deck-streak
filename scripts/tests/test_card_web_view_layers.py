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
# section 4). L4 is an absence the handler rule below holds for the whole tree. L2 shows twice:
# the configuration's default is off, and each navigation's preference follows the switch's
# verdict (SPEC-355 R2, ADR-366 D1).
LAYER_TOKENS = (
    ("L1", "WKWebsiteDataStore.nonPersistent()"),
    ("L2", "allowsContentJavaScript = false"),
    ("L2", "allowsContentJavaScript = verdict == .run"),
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
preferences.allowsContentJavaScript = verdict == .run
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
            plants[f"the factory without {layer} ({token})"] = (
                {"factory": GOOD_FACTORY.replace(token, "")},
                [f"{FACTORY}: lacks {layer} ({token})"],
            )
            plants[f"the factory with {layer} ({token}) only in a comment"] = (
                {"factory": GOOD_FACTORY.replace(token, "") + f"// {token}\n"},
                [f"{FACTORY}: lacks {layer} ({token})"],
            )
        for name, (shape, wanted) in examined("planted trees", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                plant(Path(scratch), **shape)
                self.assertEqual(card_view_problems(Path(scratch))[0], wanted, name)


# The one file that may define the card-script switch (SPEC-355 R1, ADR-366 D1).
SWITCH_FILE = "ios/CardIsolation/Sources/CardIsolation/CardScripts.swift"
SWITCH_DEFINITION = re.compile(r"\bstatic\s+(?:let|var)\s+switchedOn\b")
# The files that hold a control's tokens beside the factory (SPEC-361 R3, R4 and R6). L10 and L11
# build their user scripts in their own files, so the factory's L8 tokens each stay its own.
LINK = "ios/CardIsolation/Sources/CardIsolation/LinkActivationRefusal.swift"
GUARD = "ios/CardIsolation/Sources/CardIsolation/PageGuard.swift"
WINDOW = "ios/CardIsolation/Sources/CardIsolation/WindowRefusal.swift"
# Each control built for scripted cards, with the file and the tokens that show it set (SPEC-355
# R3, SPEC-361 R3 to R6, the schematic's sections 7 and 8). L8's user script and L11's run at
# document start, in every frame, in the page's own world; L10's in a world the app owns. The
# factory installs L10 and L11, hands L12's policy before the card, and turns the link preview
# off; the window refusal answers the context menu (L13).
CONTROL_TOKENS = (
    ("L8", FACTORY, "forMainFrameOnly: false"),
    ("L8", FACTORY, ".atDocumentStart"),
    ("L8", FACTORY, "in: .page"),
    ("L10", LINK, "injectionTime: .atDocumentStart"),
    ("L10", LINK, "forMainFrameOnly: false"),
    ("L10", LINK, "in: world"),
    ("L10", LINK, "static let world = WKContentWorld.world(name:"),
    ("L10", FACTORY, "addUserScript(LinkActivationRefusal.userScript)"),
    ("L11", GUARD, "injectionTime: .atDocumentStart"),
    ("L11", GUARD, "forMainFrameOnly: false"),
    ("L11", GUARD, "in: .page"),
    ("L11", FACTORY, "addUserScript(PageGuard.userScript)"),
    ("L12", FACTORY, "let html = DocumentPolicy.prefixed(html)"),
    ("L13", FACTORY, "allowsLinkPreview = false"),
    ("L13", WINDOW, "contextMenuConfigurationForElement"),
)


def script_switch_problems(root):
    """Every rule of the card-script switch the tree at `root` breaks, each named, and what was
    judged: every Swift file under `ios/`, test targets included, and each file that defines the
    switch, once per definition. A control's file that is missing is named once."""
    root = Path(root)
    problems = []
    judged = {"swift files": [], "switch definitions": []}
    sources = sorted((root / "ios").rglob("*.swift")) if (root / "ios").is_dir() else []
    for path in sources:
        relative = path.relative_to(root).as_posix()
        judged["swift files"].append(relative)
        code = code_of(path.read_bytes().decode("utf-8", errors="replace"))
        for _ in SWITCH_DEFINITION.finditer(code):
            judged["switch definitions"].append(relative)
            if relative != SWITCH_FILE:
                problems.append(f"{relative}: defines switchedOn, which only {SWITCH_FILE} may")
    count = judged["switch definitions"].count(SWITCH_FILE)
    if count != 1:
        problems.append(f"{SWITCH_FILE}: defines switchedOn {count} times, not once")
    codes = {}
    for layer, relative, token in CONTROL_TOKENS:
        if relative not in codes:
            path = root / relative
            codes[relative] = (
                code_of(path.read_bytes().decode("utf-8", errors="replace"))
                if path.is_file()
                else None
            )
            if codes[relative] is None:
                problems.append(f"{relative}: missing")
        if codes[relative] is not None and token not in codes[relative]:
            problems.append(f"{relative}: lacks {layer} ({token})")
    return problems, judged


GOOD_SWITCH = """public enum CardScripts {
    // The planted switch: defined once.
    public static let switchedOn = false
}
"""
SECOND_SWITCH = "extension CardScripts {\n    static var switchedOn: Bool { true }\n}\n"
GOOD_SCRIPTED_FACTORY = (
    GOOD_FACTORY
    + "let removal = WKUserScript(source: s, injectionTime: .atDocumentStart,"
    + " forMainFrameOnly: false, in: .page)\n"
    + "configuration.userContentController.addUserScript(LinkActivationRefusal.userScript)\n"
    + "configuration.userContentController.addUserScript(PageGuard.userScript)\n"
    + "let html = DocumentPolicy.prefixed(html)\n"
    + "view.allowsLinkPreview = false\n"
)
GOOD_LINK = """import WebKit
// The planted link-activation refusal: its world and its user script, every token once.
public enum LinkActivationRefusal {
    static let world = WKContentWorld.world(name: "card-link-activation-refusal")
    static let userScript = WKUserScript(
        source: source, injectionTime: .atDocumentStart, forMainFrameOnly: false, in: world)
}
"""
GOOD_GUARD = """import WebKit
// The planted page guard: its user script, every token once.
public enum PageGuard {
    static let userScript = WKUserScript(
        source: source, injectionTime: .atDocumentStart, forMainFrameOnly: false, in: .page)
}
"""
GOOD_WINDOW = """import WebKit
// The planted window refusal: its context-menu arm.
final class WindowRefusal: NSObject, WKUIDelegate {
    func webView(
        _ webView: WKWebView,
        contextMenuConfigurationForElement elementInfo: WKContextMenuElementInfo,
        completionHandler: @escaping (UIContextMenuConfiguration?) -> Void
    ) {
        completionHandler(nil)
    }
}
"""
GOOD_CONTROLS = {LINK: GOOD_LINK, GUARD: GOOD_GUARD, WINDOW: GOOD_WINDOW}
# How each file that holds a control is named in a plant.
CONTROL_FILE_NAMES = {
    FACTORY: "the factory",
    LINK: "the link-activation refusal",
    GUARD: "the page guard",
    WINDOW: "the window refusal",
}


class TheScriptSwitchLivesInOneFile(unittest.TestCase):
    def test_the_script_switch_lives_in_one_file_and_every_control_is_set(self):
        # The behaviour first: exactly one definition of the switch under ios/, in its own file,
        # and the factory sets every control's token.
        problems, judged = script_switch_problems(REPO)
        self.assertEqual(judged["switch definitions"], [SWITCH_FILE])
        self.assertEqual(problems, [])
        examined("Swift files under ios/", judged["swift files"])

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        switch = {SWITCH_FILE: GOOD_SWITCH}
        controls = {**switch, **GOOD_CONTROLS}
        only = f"which only {SWITCH_FILE} may"
        plants = {
            "the good tree": ({"factory": GOOD_SCRIPTED_FACTORY, "extra": controls}, []),
            "no switch": (
                {"factory": GOOD_SCRIPTED_FACTORY, "extra": GOOD_CONTROLS},
                [f"{SWITCH_FILE}: defines switchedOn 0 times, not once"],
            ),
            "the switch defined twice in its file": (
                {
                    "factory": GOOD_SCRIPTED_FACTORY,
                    "extra": {**controls, SWITCH_FILE: GOOD_SWITCH + SECOND_SWITCH},
                },
                [f"{SWITCH_FILE}: defines switchedOn 2 times, not once"],
            ),
            "a second switch in the harness": (
                {
                    "factory": GOOD_SCRIPTED_FACTORY,
                    "extra": {**controls, "ios/Harness/Sources/Scripts.swift": SECOND_SWITCH},
                },
                [f"ios/Harness/Sources/Scripts.swift: defines switchedOn, {only}"],
            ),
            "a second switch in a test target": (
                {
                    "factory": GOOD_SCRIPTED_FACTORY,
                    "extra": {**controls, "ios/CardProbeTests/Switch.swift": SECOND_SWITCH},
                },
                [f"ios/CardProbeTests/Switch.swift: defines switchedOn, {only}"],
            ),
            "the switch only in a comment": (
                {
                    "factory": GOOD_SCRIPTED_FACTORY,
                    "extra": {
                        **controls,
                        SWITCH_FILE: "// public static let switchedOn = true\n",
                    },
                },
                [f"{SWITCH_FILE}: defines switchedOn 0 times, not once"],
            ),
            "no factory": ({"factory": None, "extra": controls}, [f"{FACTORY}: missing"]),
        }
        for relative in GOOD_CONTROLS:
            plants[f"no {CONTROL_FILE_NAMES[relative]}"] = (
                {
                    "factory": GOOD_SCRIPTED_FACTORY,
                    "extra": {name: text for name, text in controls.items() if name != relative},
                },
                [f"{relative}: missing"],
            )
        for layer, relative, token in CONTROL_TOKENS:
            good = GOOD_SCRIPTED_FACTORY if relative == FACTORY else GOOD_CONTROLS[relative]
            named = CONTROL_FILE_NAMES[relative]
            for name, text in (
                (f"{named} without {layer}'s {token}", good.replace(token, "")),
                (
                    f"{named} with {layer}'s {token} only in a comment",
                    good.replace(token, "") + f"// {token}\n",
                ),
            ):
                shape = (
                    {"factory": text, "extra": controls}
                    if relative == FACTORY
                    else {"factory": GOOD_SCRIPTED_FACTORY, "extra": {**controls, relative: text}}
                )
                plants[name] = (shape, [f"{relative}: lacks {layer} ({token})"])
        for name, (shape, wanted) in examined("planted trees", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                plant(Path(scratch), **shape)
                self.assertEqual(script_switch_problems(Path(scratch))[0], wanted, name)


# The value each definition of the switch is given: after `=`, or as a computed property's body.
SWITCH_VALUE = re.compile(r"\bstatic\s+(?:let|var)\s+switchedOn\b[^=\n{]*(?:=|\{)\s*(\w+)")


def script_switch_values(root):
    """The value each definition of the switch in `SWITCH_FILE` under `root` is given, in order,
    with whole-line comments set aside; none when the file is missing or defines no switch."""
    path = Path(root) / SWITCH_FILE
    if not path.is_file():
        return []
    code = code_of(path.read_bytes().decode("utf-8", errors="replace"))
    return [found.group(1) for found in SWITCH_VALUE.finditer(code)]


class TheScriptSwitchDefaultsOff(unittest.TestCase):
    def test_the_script_switch_defaults_off(self):
        # The behaviour first: the switch defaults off on iOS pending a measured containment
        # layer (SPEC-355 section 7), so its one definition gives it false.
        values = examined("switch definitions", script_switch_values(REPO))
        self.assertEqual(values, ["false"], f"{SWITCH_FILE}: the value the switch is given")

        # The controls: a planted switch off reads false, and each planted switch on reads true.
        on = GOOD_SWITCH.replace("switchedOn = false", "switchedOn = true")
        plants = {
            "the switch off": (GOOD_SWITCH, ["false"]),
            "the switch on": (on, ["true"]),
            "the switch on, with its type": (
                "public enum CardScripts {\n    public static let switchedOn: Bool = true\n}\n",
                ["true"],
            ),
            "the switch on, computed": (SECOND_SWITCH, ["true"]),
            "the switch on, off only in a comment": (
                "// public static let switchedOn = false\n" + on,
                ["true"],
            ),
            "no switch": ("public enum CardScripts {}\n", []),
        }
        for name, (source, wanted) in examined("planted switches", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                path = Path(scratch) / SWITCH_FILE
                path.parent.mkdir(parents=True)
                path.write_text(source, encoding="utf-8")
                self.assertEqual(script_switch_values(Path(scratch)), wanted, name)


# L14, the link strip (SPEC-392 R1 to R3, ADR-406 D1): its file, the one function the factory
# calls, the factory's one card load handed through it, and the case `CardLayer` names it by.
LINK_STRIP = "ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift"
STRIP_FUNCTION = "public static func stripped(_ html: String) -> String"
STRIPPED_LOAD = "load(LinkStrip.stripped(html), into: view)"
# Every call of the factory's card load; the definition, `func load(`, is not a call.
CARD_LOAD = re.compile(r"(?<!func )\bload\(")
CARD_LAYER = re.compile(r"\benum\s+CardLayer\b[^{]*\{([^}]*)\}")


class TheFactoryStripsEveryCard(unittest.TestCase):
    @staticmethod
    def problems(root):
        """Every rule of the link strip's place the tree at `root` breaks, each by its name: every
        card load in the factory goes through `LinkStrip.stripped(_:)` (R1), the strip's file
        defines that function (R2), and `CardLayer` names L14 (R3). Whole-line comments are set
        aside, so a call or a function only a comment holds is not read as code."""
        root = Path(root)
        found = []
        factory = root / FACTORY
        if factory.is_file():
            code = code_of(factory.read_bytes().decode("utf-8", errors="replace"))
            loads = [match.start() for match in CARD_LOAD.finditer(code)]
            if not loads or any(not code.startswith(STRIPPED_LOAD, at) for at in loads):
                found.append(f"{FACTORY}: the factory hands a card past the link strip")
            layers = CARD_LAYER.search(code)
            if layers is None or not re.search(r"\bL14\b", layers.group(1)):
                found.append(f"{FACTORY}: CardLayer does not name L14")
        else:
            found.append(f"{FACTORY}: missing")
        strip = root / LINK_STRIP
        code = ""
        if strip.is_file():
            code = code_of(strip.read_bytes().decode("utf-8", errors="replace"))
        else:
            found.append(f"{LINK_STRIP}: the link strip's file is missing")
        if STRIP_FUNCTION not in code:
            found.append(f"{LINK_STRIP}: the link strip defines no stripped(_:)")
        return found

    def test_the_factory_hands_every_card_through_the_link_strip(self):
        # The behaviour first: the factory hands every card through the link strip, the strip's
        # file defines `stripped(_:)`, and `CardLayer` names L14.
        self.assertEqual(self.problems(REPO), [], "the factory's link strip (SPEC-392 R1 to R3)")

        # The controls: planted trees built from literal text, each refused by its rule's name,
        # and the good tree read clean.
        factory = "ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift"
        strip = "ios/CardIsolation/Sources/CardIsolation/LinkStrip.swift"
        good_factory = (
            "public enum CardLayer: String, CaseIterable, Sendable {\n"
            "    case L1, L2, L3, L4, L5, L6, L7, L8, L10, L11, L12, L13, L14\n"
            "}\n"
            "public enum CardWebViewFactory {\n"
            "    static func build(html: String) -> WKWebView {\n"
            "        let view = WKWebView()\n"
            "        load(LinkStrip.stripped(html), into: view)\n"
            "        return view\n"
            "    }\n"
            "    static func load(_ html: String, into view: WKWebView) {\n"
            "        view.loadHTMLString(html, baseURL: nil)\n"
            "    }\n"
            "}\n"
        )
        good_strip = (
            "public enum LinkStrip {\n"
            "    public static func stripped(_ html: String) -> String {\n"
            "        html\n"
            "    }\n"
            "}\n"
        )
        past = f"{factory}: the factory hands a card past the link strip"
        plants = {
            "the good tree": ({factory: good_factory, strip: good_strip}, []),
            "a factory that loads load(html, into: view)": (
                {
                    factory: good_factory.replace(
                        "load(LinkStrip.stripped(html), into: view)", "load(html, into: view)"
                    ),
                    strip: good_strip,
                },
                [past],
            ),
            "the strip call only in a comment": (
                {
                    factory: good_factory.replace(
                        "        load(LinkStrip.stripped(html), into: view)\n",
                        "        // load(LinkStrip.stripped(html), into: view)\n",
                    ),
                    strip: good_strip,
                },
                [past],
            ),
            "no LinkStrip.swift": (
                {factory: good_factory},
                [
                    f"{strip}: the link strip's file is missing",
                    f"{strip}: the link strip defines no stripped(_:)",
                ],
            ),
            "a LinkStrip.swift with no stripped(_:)": (
                {factory: good_factory, strip: "public enum LinkStrip {}\n"},
                [f"{strip}: the link strip defines no stripped(_:)"],
            ),
            "a CardLayer without L14": (
                {factory: good_factory.replace(", L13, L14\n", ", L13\n"), strip: good_strip},
                [f"{factory}: CardLayer does not name L14"],
            ),
        }
        for name, (files, wanted) in examined("planted trees", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                for relative, text in files.items():
                    path = Path(scratch) / relative
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_text(text, encoding="utf-8")
                self.assertEqual(self.problems(Path(scratch)), wanted, name)


# The iPhone's inline playback (SPEC-393 R10 and R12, ADR-407 D4): the configuration the factory
# builds plays media inline, set inside the iPhone-and-iPad compile guard, because the property
# exists only there and the package also builds for its tests' host.
CONFIGURATION_DEFINITION = re.compile(r"\bfunc\s+configuration\(\s*layers:")
INLINE_LINE = re.compile(r"\bconfiguration\.allowsInlineMediaPlayback\s*=\s*(\w+)")
IOS_GUARD = "#if os(iOS)"
INLINE_RULE = f"{FACTORY}: configuration(layers:)"
NO_INLINE = f"{INLINE_RULE} does not set allowsInlineMediaPlayback"
INLINE_OFF = f"{INLINE_RULE} sets allowsInlineMediaPlayback to false"
INLINE_UNGUARDED = f"{INLINE_RULE} sets allowsInlineMediaPlayback outside {IOS_GUARD}"


def configuration_body(code):
    """The lines of the factory's `configuration(layers:...)` body, between its opening brace and
    the one that closes it; none when the function is absent."""
    found = CONFIGURATION_DEFINITION.search(code)
    start = code.find("{", found.end()) if found else -1
    if start < 0:
        return []
    depth = 0
    for index in range(start, len(code)):
        depth += {"{": 1, "}": -1}.get(code[index], 0)
        if depth == 0:
            return code[start + 1 : index].splitlines()
    return code[start + 1 :].splitlines()


def inline_playback_problems(root):
    """Every rule of the factory's inline playback the tree at `root` breaks, each named, and the
    factories judged: the one whose `configuration(layers:...)` was read."""
    path = Path(root) / FACTORY
    if not path.is_file():
        return [f"{FACTORY}: missing"], []
    body = configuration_body(code_of(path.read_bytes().decode("utf-8", errors="replace")))
    if not body:
        return [f"{FACTORY}: defines no configuration(layers:)"], []
    guards, settings = [], []
    for line in body:
        stripped = line.strip()
        if stripped.startswith("#if"):
            guards.append(stripped == IOS_GUARD)
        elif stripped.startswith(("#else", "#elseif")) and guards:
            guards[-1] = False
        elif stripped.startswith("#endif") and guards:
            guards.pop()
        elif found := INLINE_LINE.search(line):
            settings.append((found.group(1), any(guards)))
    problems = [] if settings else [NO_INLINE]
    for value, guarded in settings:
        if value != "true":
            problems.append(INLINE_OFF)
        if not guarded:
            problems.append(INLINE_UNGUARDED)
    return problems, [FACTORY]


GOOD_INLINE_FACTORY = """import WebKit
public enum CardWebViewFactory {
    private static func configuration(
        layers: Set<CardLayer>, ruleList: WKContentRuleList?, built: Built
    ) -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()
        #if os(iOS)
            configuration.allowsInlineMediaPlayback = true
        #endif
        return configuration
    }
}
"""
INLINE_SETTING = "            configuration.allowsInlineMediaPlayback = true\n"


class TheFactoryPlaysMediaInline(unittest.TestCase):
    def test_the_factory_plays_media_inline_and_every_plant_is_refused(self):
        # The behaviour first: the real factory's configuration sets inline playback on, inside
        # the iPhone-and-iPad guard (SPEC-393 R10).
        problems, judged = inline_playback_problems(REPO)
        self.assertEqual(problems, [], f"{FACTORY}: the factory's inline playback")
        examined("factories", judged)

        # The controls: the planted good factory passes, and each plant breaks one rule and is
        # refused by that rule's name (SPEC-393 R12).
        unguarded = GOOD_INLINE_FACTORY.replace("        #if os(iOS)\n", "")
        plants = {
            "the good factory": (GOOD_INLINE_FACTORY, []),
            "no line": (GOOD_INLINE_FACTORY.replace(INLINE_SETTING, ""), [NO_INLINE]),
            "the line set false": (
                GOOD_INLINE_FACTORY.replace("Playback = true", "Playback = false"),
                [INLINE_OFF],
            ),
            "the line outside the guard": (
                unguarded.replace("        #endif\n", ""),
                [INLINE_UNGUARDED],
            ),
        }
        for name, (source, wanted) in examined("planted factories", list(plants.items())):
            with self.subTest(plant=name), tempfile.TemporaryDirectory() as scratch:
                path = Path(scratch) / FACTORY
                path.parent.mkdir(parents=True)
                path.write_text(source, encoding="utf-8")
                self.assertEqual(inline_playback_problems(Path(scratch))[0], wanted, name)


if __name__ == "__main__":
    unittest.main()
