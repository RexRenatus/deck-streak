"""The review screen shows a card only through the factory, and its answer bar is named, placed
and felt (SPEC-348 R11, R12 and R18; A13 and A14; ADR-359 D9).

A13 reads every Swift file git tracks under `ios/`, outside the card view's own package
(`ios/CardIsolation/`) and outside a test target, by the rule `test_card_web_view_layers.py`
holds, imported so the two never drift: the factory's guard and the card probe's tests name R18's
words by design, and each path skipped is printed with their count. In that population no file
constructs a web view or its configuration, and none names a script message handler, a scheme
handler, a file load or file access; the one file the register gives the role `card` calls the
factory and names no configuration.

A14 reads the bar and the screen that holds it: the bar names Again, Hard, Good and Easy as
string literals in that order and overrides no accessible name with `.accessibilityLabel(`, so
each title is its button's name; the screen holds the bar in a bottom safe-area inset, and
declares one impact and one success `.sensoryFeedback`, and no other.

Each census is a function of the data it is handed, so every planted control is a dict: the test
hands it the live tree, then planted trees that each break one rule and must be refused by that
rule's name.
"""

import re
import unittest

from _support import examined
from test_card_web_view_layers import CONSTRUCTS, is_test_target
from test_ios_thin_swift import REGISTER, live_tree, register_entries

ISOLATION = "ios/CardIsolation/"
# R18's names: a bridge from a frame's script into Swift, a scheme the app would serve, or a file
# a frame could load. Each is read as a prefix over the whole text, so a longer spelling holds too.
NAMES = {
    name: re.compile(rf"\b{name}")
    for name in (
        "WKScriptMessageHandler",
        "addScriptMessageHandler",
        "userContentController",
        "WKURLSchemeHandler",
        "setURLSchemeHandler",
        "loadFileURL",
        "allowFileAccess",
    )
}
FACTORY_CALL = "makeCardWebView(html:"
CONFIGURATION = re.compile(r"\bconfiguration\b", re.IGNORECASE)

BAR = "ios/App/Sources/AnswerBar.swift"
SCREEN = "ios/App/Sources/ReviewView.swift"
TITLES = ("Again", "Hard", "Good", "Easy")
LABEL_OVERRIDE = ".accessibilityLabel("
# The bar is the content of a bottom inset: the inset's closure opens with the bar.
BOTTOM_INSET = re.compile(r"\.safeAreaInset\(\s*edge:\s*\.bottom\b[^{]*\{\s*AnswerBar\(")
FEEDBACK = re.compile(r"\.sensoryFeedback\(\s*\.(\w+)")


def population(sources):
    """(judged, skipped): the Swift files A13 reads, and those it leaves to the factory's own
    censuses, the card view's package and every test target (ADR-359 D9)."""
    judged, skipped = [], []
    for path in sorted(sources):
        if path.startswith(ISOLATION) or is_test_target(path):
            skipped.append(path)
        else:
            judged.append(path)
    return judged, skipped


def card_frame_problems(sources, register_text):
    """Every way the tree could build a card frame other than through the factory, named
    (SPEC-348 R18), and what was examined. `sources` maps each tracked Swift file under `ios/` to
    its text; `register_text` is the register's."""
    problems = []
    judged, skipped = population(sources)
    seen = {"files": judged, "skipped": skipped, "names": []}
    for path in judged:
        text = sources[path]
        for token in sorted(set(CONSTRUCTS.findall(text))):
            problems.append(f"{path}: the frame: constructs {token!r}, which only the factory may")
        for name, pattern in NAMES.items():
            seen["names"].append(f"{path}: {name}")
            if pattern.search(text):
                problems.append(f"{path}: the frame: names {name}")
    entries = register_entries(register_text, problems)
    cards = sorted(
        path
        for path, entry in entries.items()
        if isinstance(entry, dict) and entry.get("role") == "card"
    )
    if not cards:
        problems.append(
            f"{REGISTER}: the card role: no file holds it, so none calls {FACTORY_CALL}"
        )
    for path in cards:
        text = sources.get(path, "")
        if FACTORY_CALL not in text:
            problems.append(f"{path}: the card role: does not call {FACTORY_CALL}")
        if CONFIGURATION.search(text):
            problems.append(
                f"{path}: the card role: names a configuration, which the factory alone sets"
            )
    return problems, seen


def answer_bar_problems(sources):
    """Every way the answer bar could lose its names, its place or its feel, named (SPEC-348 R11
    and R12), and what was examined."""
    problems = []
    seen = {"files": [], "titles": []}
    for path in (BAR, SCREEN):
        if path in sources:
            seen["files"].append(path)
        else:
            problems.append(f"{path}: the answer bar: absent")
    bar, screen = sources.get(BAR, ""), sources.get(SCREEN, "")
    at = []
    for title in TITLES:
        seen["titles"].append(title)
        where = bar.find(f'"{title}"')
        if where < 0:
            problems.append(f"{BAR}: the titles: {title!r} is absent")
        else:
            at.append(where)
    if len(at) == len(TITLES) and at != sorted(at):
        problems.append(f"{BAR}: the titles: not in the order {', '.join(TITLES)}")
    if LABEL_OVERRIDE in bar:
        problems.append(f"{BAR}: the titles: a title is overridden by {LABEL_OVERRIDE}")
    if not BOTTOM_INSET.search(screen):
        problems.append(
            f"{SCREEN}: the place: the bar is not the content of a bottom safe-area inset"
        )
    kinds = FEEDBACK.findall(screen)
    for kind in ("impact", "success"):
        if kinds.count(kind) != 1:
            problems.append(f"{SCREEN}: the haptics: {kinds.count(kind)} {kind} feedback, not one")
    others = sorted(kind for kind in kinds if kind not in ("impact", "success"))
    if others:
        problems.append(f"{SCREEN}: the haptics: {', '.join(others)} feedback, which R12 names not")
    return problems, seen


APP = "ios/App/Sources/"
CARD = APP + "CardFaceView.swift"
CHROME = APP + "ReviewChrome.swift"
MODEL = APP + "ReviewModel.swift"
HARNESS_VIEW = "ios/Harness/Sources/CardWebView.swift"
# A tree that keeps R18: the card file calls the factory; the factory and the tests hold R18's
# words, and are skipped; the harness's view calls the factory and is judged.
FRAME_GOOD = {
    CARD: (
        "import CardIsolation\nimport SwiftUI\nimport WebKit\n\nstruct CardFaceView: View {\n"
        "    let document: String\n    @State private var view: WKWebView?\n"
        "    var body: some View {\n        Color.clear.task(id: document) {\n"
        "            view = try? await CardWebViewFactory.makeCardWebView(html: document)\n"
        "        }\n    }\n}\n"
    ),
    CHROME: 'import SwiftUI\n\nstruct ReviewChrome: View {\n    var body: some View { Text("") }\n}\n',
    MODEL: "import Observation\n\n@Observable\nfinal class ReviewModel {\n    var shown = false\n}\n",
    HARNESS_VIEW: "let view = try await CardWebViewFactory.makeCardWebView(html: html)\n",
    "ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift": (
        "let view = WKWebView(frame: .zero, configuration: WKWebViewConfiguration())\n"
        "func add(_ handler: WKScriptMessageHandler, name: String) {}\n"
    ),
    "ios/CardProbeTests/PlantedCardTests.swift": (
        'holder.userContentController.addScriptMessageHandler(probe, name: "p")\n'
        "view.loadFileURL(file, allowingReadAccessTo: file)\n"
    ),
    "ios/HarnessWire/Tests/HarnessWireTests/FrameTests.swift": "let view = WKWebView(frame: .zero)\n",
}
FRAME_REGISTER = '{\n  "ios/App/Sources/CardFaceView.swift": {"role": "card", "decisions": 0}\n}\n'


def frame_tree(changes=None, register=None):
    """The good frame tree, with `changes` (path to text; None drops one) in place of or beside its
    own, and `register` in place of its register's text."""
    tree = dict(FRAME_GOOD)
    for path, text in (changes or {}).items():
        if text is None:
            tree.pop(path)
        else:
            tree[path] = text
    return tree, FRAME_REGISTER if register is None else register


def in_chrome(line):
    """The good chrome file with one line added to its body."""
    return {CHROME: FRAME_GOOD[CHROME] + line}


FRAME_PLANTS = {
    "the good tree": (frame_tree(), []),
    "a web view constructed in a view": (
        frame_tree(in_chrome("let view = WKWebView(frame: .zero)\n")),
        [f"{CHROME}: the frame: constructs 'WKWebView(', which only the factory may"],
    ),
    "a configuration constructed in the model": (
        frame_tree({MODEL: FRAME_GOOD[MODEL] + "let made = WKWebViewConfiguration()\n"}),
        [f"{MODEL}: the frame: constructs 'WKWebViewConfiguration(', which only the factory may"],
    ),
    "a web view constructed in the harness": (
        frame_tree({HARNESS_VIEW: "let view = WKWebView(frame: .zero)\n"}),
        [f"{HARNESS_VIEW}: the frame: constructs 'WKWebView(', which only the factory may"],
    ),
    "a script message handler": (
        frame_tree(in_chrome("final class Bridge: NSObject, WKScriptMessageHandler {}\n")),
        [f"{CHROME}: the frame: names WKScriptMessageHandler"],
    ),
    "a script message handler added": (
        frame_tree(in_chrome('holder.addScriptMessageHandler(bridge, name: "b")\n')),
        [f"{CHROME}: the frame: names addScriptMessageHandler"],
    ),
    "the content controller reached": (
        frame_tree(in_chrome("let reached = holder.userContentController\n")),
        [f"{CHROME}: the frame: names userContentController"],
    ),
    "a scheme handler": (
        frame_tree(in_chrome("final class Scheme: NSObject, WKURLSchemeHandler {}\n")),
        [f"{CHROME}: the frame: names WKURLSchemeHandler"],
    ),
    "a scheme handler set": (
        frame_tree(in_chrome('holder.setURLSchemeHandler(scheme, forURLScheme: "card")\n')),
        [f"{CHROME}: the frame: names setURLSchemeHandler"],
    ),
    "a file loaded": (
        frame_tree(in_chrome("view.loadFileURL(file, allowingReadAccessTo: file)\n")),
        [f"{CHROME}: the frame: names loadFileURL"],
    ),
    "file access allowed": (
        frame_tree(in_chrome('holder.setValue(true, forKey: "allowFileAccessFromFileURLs")\n')),
        [f"{CHROME}: the frame: names allowFileAccess"],
    ),
    "the card file without the factory's call": (
        frame_tree({CARD: FRAME_GOOD[CARD].replace(FACTORY_CALL, "make(html:")}),
        [f"{CARD}: the card role: does not call {FACTORY_CALL}"],
    ),
    "the card file naming a configuration": (
        frame_tree(
            {CARD: FRAME_GOOD[CARD] + "let preferences = view?.configuration.preferences\n"}
        ),
        [f"{CARD}: the card role: names a configuration, which the factory alone sets"],
    ),
    "no card file registered": (
        frame_tree(register="{}\n"),
        [f"{REGISTER}: the card role: no file holds it, so none calls {FACTORY_CALL}"],
    ),
}

# A bar and a screen that keep R11 and R12.
BAR_GOOD = {
    BAR: (
        "import SwiftUI\n\nstruct AnswerBar: View {\n    let intervals: [String]\n"
        "    var body: some View {\n        HStack {\n"
        '            RatingButton(title: "Again", interval: intervals[0])\n'
        '            RatingButton(title: "Hard", interval: intervals[1])\n'
        '            RatingButton(title: "Good", interval: intervals[2])\n'
        '            RatingButton(title: "Easy", interval: intervals[3])\n'
        "        }\n    }\n}\n"
    ),
    SCREEN: (
        "import SwiftUI\n\nstruct ReviewView: View {\n    let model: ReviewModel\n"
        "    var body: some View {\n        CardFaceView(document: model.document)\n"
        "            .safeAreaInset(edge: .bottom, spacing: 0) {\n"
        "                AnswerBar(intervals: model.intervals)\n            }\n"
        "            .sensoryFeedback(.impact, trigger: model.answered)\n"
        "            .sensoryFeedback(.success, trigger: model.finished)\n    }\n}\n"
    ),
}


def bar_tree(path=None, old=None, new=None, drop=False):
    """The good bar tree, with `old` replaced once by `new` in `path`, or `path` dropped."""
    tree = dict(BAR_GOOD)
    if drop:
        tree.pop(path)
    elif path is not None:
        assert tree[path].count(old) == 1, f"{path}: {old!r} is not in the planted file once"
        tree[path] = tree[path].replace(old, new)
    return tree


BAR_PLANTS = {
    "the good bar": (bar_tree(), []),
    "the ratings out of order": (
        bar_tree(
            BAR,
            '"Again", interval: intervals[0])\n            RatingButton(title: "Hard"',
            '"Hard", interval: intervals[0])\n            RatingButton(title: "Again"',
        ),
        [f"{BAR}: the titles: not in the order Again, Hard, Good, Easy"],
    ),
    "a title absent": (
        bar_tree(BAR, '"Easy"', '"Simple"'),
        [f"{BAR}: the titles: 'Easy' is absent"],
    ),
    "a title overridden": (
        bar_tree(
            BAR,
            "        }\n    }\n}\n",
            '        }\n        .accessibilityLabel("ratings")\n    }\n}\n',
        ),
        [f"{BAR}: the titles: a title is overridden by {LABEL_OVERRIDE}"],
    ),
    "the bar at the top": (
        bar_tree(SCREEN, "edge: .bottom", "edge: .top"),
        [f"{SCREEN}: the place: the bar is not the content of a bottom safe-area inset"],
    ),
    "the bar outside the inset": (
        bar_tree(
            SCREEN,
            "                AnswerBar(intervals: model.intervals)\n",
            "                Spacer()\n                AnswerBar(intervals: model.intervals)\n",
        ),
        [f"{SCREEN}: the place: the bar is not the content of a bottom safe-area inset"],
    ),
    "no impact on a rating": (
        bar_tree(SCREEN, "            .sensoryFeedback(.impact, trigger: model.answered)\n", ""),
        [f"{SCREEN}: the haptics: 0 impact feedback, not one"],
    ),
    "a second success": (
        bar_tree(
            SCREEN,
            "            .sensoryFeedback(.success, trigger: model.finished)\n",
            "            .sensoryFeedback(.success, trigger: model.finished)\n"
            "            .sensoryFeedback(.success, trigger: model.revealed)\n",
        ),
        [f"{SCREEN}: the haptics: 2 success feedback, not one"],
    ),
    "a haptic on Show Answer": (
        bar_tree(
            SCREEN,
            "            .sensoryFeedback(.success, trigger: model.finished)\n",
            "            .sensoryFeedback(.success, trigger: model.finished)\n"
            "            .sensoryFeedback(.selection, trigger: model.revealed)\n",
        ),
        [f"{SCREEN}: the haptics: selection feedback, which R12 names not"],
    ),
    "no screen": (
        bar_tree(SCREEN, drop=True),
        [
            f"{SCREEN}: the answer bar: absent",
            f"{SCREEN}: the place: the bar is not the content of a bottom safe-area inset",
            f"{SCREEN}: the haptics: 0 impact feedback, not one",
            f"{SCREEN}: the haptics: 0 success feedback, not one",
        ],
    ),
}


class TheCardFrameIsTheFactorysAlone(unittest.TestCase):
    maxDiff = None

    def test_the_card_frame_is_the_factorys_alone(self):
        # The behaviour first: the live tree builds a card frame through the factory alone.
        sources, register = live_tree()
        problems, seen = card_frame_problems(sources, register)
        for path in seen["skipped"]:
            print(f"skipped by ADR-359 D9: {path}")
        print(
            f"skipped {len(seen['skipped'])} file(s): the card view's package and the test targets"
        )
        with self.subTest(tree="the live tree"):
            self.assertEqual(problems, [], f"examined {len(seen['files'])} Swift files")

        # The controls: the good tree is accepted, and each plant is refused by its rule's name.
        for name, ((tree, text), wanted) in examined("planted trees", list(FRAME_PLANTS.items())):
            with self.subTest(plant=name):
                got = card_frame_problems(tree, text)[0]
                print(f"planted {name}: {'; '.join(got) if got else 'accepted'}")
                self.assertEqual(got, wanted, name)

        examined("Swift files judged", seen["files"])
        examined("Swift files skipped", seen["skipped"])
        examined("names read", seen["names"])


class TheAnswerBarIsNamedPlacedAndFelt(unittest.TestCase):
    maxDiff = None

    def test_the_answer_bar_is_named_placed_and_felt(self):
        # The behaviour first: the live bar is named, placed and felt.
        sources, _register = live_tree()
        problems, seen = answer_bar_problems(sources)
        with self.subTest(tree="the live tree"):
            self.assertEqual(problems, [], f"examined {len(seen['files'])} files")

        # The controls: the good bar is accepted, and each plant is refused by its rule's name.
        for name, (tree, wanted) in examined("planted bars", list(BAR_PLANTS.items())):
            with self.subTest(plant=name):
                got = answer_bar_problems(tree)[0]
                print(f"planted {name}: {'; '.join(got) if got else 'accepted'}")
                self.assertEqual(got, wanted, name)

        examined("titles", seen["titles"])
        examined("bar files", seen["files"])


if __name__ == "__main__":
    unittest.main()
