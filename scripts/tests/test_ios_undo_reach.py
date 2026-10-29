"""The iPhone and iPad review's undo is reached only from the review, and its dialog's focus is on
Keep (SPEC-515 R6, R9 and A4; ADR-533 D4).

The census reads every Swift file git tracks under `ios/App/Sources`, with its comments and the
text of its strings removed by the thin-Swift census's own reader and every blank dropped, so a
sentence about a call is not a call and a call split over lines is still one. It holds each undo
call to the files and the counts the review's one path gives it:

- `.undoOffer(` and `.undo(`: once each in the session, which calls the adapter
  (`EngineSession.swift`), once each in the review's session, which calls the session
  (`ReviewSession.swift`), and once each in the review's undo model, which calls the review's
  session (`ReviewUndo.swift`), and nowhere else;
- `undo.perform(`: once, in the review model's one entry for its gestures (`ReviewModel.swift`);
- `.perform(.undo)`, `.perform(.keep)` and `.perform(.confirmUndo)`: only in the undo control
  (`UndoControl.swift`), the press once, the confirmation once, and Keep twice, its button's and
  the dialog's dismissal.

In the dialog (`UndoDialogView.swift`), the Keep button carries both the keyboard's and the reader's
focus, and the confirmation carries neither.

Each census is a function of the data it is handed, so every planted control is a dict: the test
hands it the live tree, then planted trees that each break one rule and must be refused by that
rule's name.
"""

import unittest

from _support import examined
from test_ios_thin_swift import code_of, live_tree

APP = "ios/App/Sources/"
DIALOG = APP + "UndoDialogView.swift"
# Each undo call, written without blanks, and the files that may make it with how many times each.
SITES = {
    ".undoOffer(": {
        APP + "EngineSession.swift": 1,
        APP + "ReviewSession.swift": 1,
        APP + "ReviewUndo.swift": 1,
    },
    ".undo(": {
        APP + "EngineSession.swift": 1,
        APP + "ReviewSession.swift": 1,
        APP + "ReviewUndo.swift": 1,
    },
    "undo.perform(": {APP + "ReviewModel.swift": 1},
    ".perform(.undo)": {APP + "UndoControl.swift": 1},
    ".perform(.keep)": {APP + "UndoControl.swift": 2},
    ".perform(.confirmUndo)": {APP + "UndoControl.swift": 1},
}
# The dialog's two buttons, each opened by its action, and the two focus modifiers.
KEEP_BUTTON = "Button(action:keep)"
CONFIRM_BUTTON = "Button(action:confirm)"
FOCUS = (".focused($", ".accessibilityFocused($")


def bare(source):
    """A Swift file's code with comments, string text and every blank removed."""
    return "".join(code_of(source).split())


def undo_reach_problems(sources):
    """Every rule the tree breaks, each named, and what was examined: (problems, judged)."""
    problems = []
    judged = {"files": [], "sites": []}
    codes = {}
    for path in sorted(sources):
        if not path.startswith(APP):
            continue
        judged["files"].append(path)
        codes[path] = bare(sources[path])
    for call, held in SITES.items():
        for path in sorted(set(codes) | set(held)):
            found = codes.get(path, "").count(call)
            if found:
                judged["sites"].append(f"{path}: {call} {found}")
            if found != held.get(path, 0):
                problems.append(
                    f"{path}: `{call}` called {found} time(s), held {held.get(path, 0)}"
                )
    problems.extend(focus_problems(codes.get(DIALOG, "")))
    return problems, judged


def focus_problems(code):
    """Whether the dialog's Keep button carries both focuses and its confirmation neither."""
    keep_at, confirm_at = code.find(KEEP_BUTTON), code.find(CONFIRM_BUTTON)
    if keep_at == -1 or confirm_at == -1:
        return [f"{DIALOG}: the dialog: its Keep or its confirmation button is missing"]
    if keep_at < confirm_at:
        keep, confirm = code[keep_at:confirm_at], code[confirm_at:]
    else:
        confirm, keep = code[confirm_at:keep_at], code[keep_at:]
    problems = [
        f"{DIALOG}: the dialog: Keep does not carry `{focus}`"
        for focus in FOCUS
        if focus not in keep
    ]
    problems += [
        f"{DIALOG}: the dialog: the confirmation carries `{focus}`"
        for focus in FOCUS
        if focus in confirm
    ]
    return problems


class TheUndoIsReachedOnlyFromTheReview(unittest.TestCase):
    def test_the_live_tree_reaches_the_undo_only_from_the_review(self):
        sources, _ = live_tree()
        problems, judged = undo_reach_problems(sources)
        examined("Swift file(s) under ios/App/Sources", judged["files"])
        for site in examined("undo call site(s)", judged["sites"]):
            print(site)
        self.assertEqual(problems, [], "the undo is reached only from the review")

    def test_each_plant_is_refused_by_name(self):
        sources, _ = live_tree()
        plants = {
            "a call outside the path": (
                APP + "DeckListView.swift",
                "\nlet offered = try await engine.undoOffer()\n",
                f"{APP}DeckListView.swift: `.undoOffer(` called 1 time(s), held 0",
            ),
            "a second call on the path": (
                APP + "EngineSession.swift",
                "\nlet again = try engine.undo(card: card, step: step)\n",
                f"{APP}EngineSession.swift: `.undo(` called 2 time(s), held 1",
            ),
            "a confirmation from the bar": (
                APP + "AnswerBar.swift",
                "\nTask { await model.perform(.confirmUndo) }\n",
                f"{APP}AnswerBar.swift: `.perform(.confirmUndo)` called 1 time(s), held 0",
            ),
        }
        refused = []
        for name, (path, text, expected) in plants.items():
            planted = dict(sources)
            planted[path] = planted.get(path, "") + text
            problems, _ = undo_reach_problems(planted)
            self.assertEqual(problems, [expected], f"the plant {name!r} is refused by name")
            refused.append(expected)

        # A sentence or a comment that names a call is not one.
        quiet = dict(sources)
        quiet[APP + "DeckListView.swift"] += (
            '\n// engine.undoOffer() in a comment\nlet line = "model.perform(.confirmUndo)"\n'
        )
        self.assertEqual(undo_reach_problems(quiet)[0], [], "prose and strings are not calls")

        # The focus moved to the confirmation.
        moved = dict(sources)
        moved[DIALOG] = (
            moved.get(DIALOG, "")
            .replace(".focused($keyboardOnKeep)", "")
            .replace(
                '.accessibilityIdentifier("undo-confirm")',
                ('.accessibilityIdentifier("undo-confirm").focused($keyboardOnKeep)'),
            )
        )
        problems, _ = undo_reach_problems(moved)
        expected = [
            f"{DIALOG}: the dialog: Keep does not carry `.focused($`",
            f"{DIALOG}: the dialog: the confirmation carries `.focused($`",
        ]
        self.assertEqual(problems, expected, "a dialog whose focus is not on Keep is refused")
        refused.extend(expected)
        examined("planted refusal(s)", refused)


if __name__ == "__main__":
    unittest.main()
