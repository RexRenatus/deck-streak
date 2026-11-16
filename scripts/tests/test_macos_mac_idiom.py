"""SPEC-679 section 9a: on macOS the app takes the Mac idiom. Body text is 13 pt, so the card's
zoom is 1 there; controls are 28 pt high through one platform height, which keeps 44 pt on iOS;
the review's actions sit in a menu with their keys; and the window is resizable with a default
size. The forms are read in the app's sources as committed text, by the platform each line
compiles for."""

import re
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
APP = "ios/App/Sources/"
FACTORY = "ios/CardIsolation/Sources/CardIsolation/CardWebViewFactory.swift"
HELPER = "ControlHeight.least"
# Each file whose control frames take the platform height, with its count of frames.
FRAMES = {
    "AnswerBar.swift": 3,
    "ReviewChrome.swift": 6,
    "UndoControl.swift": 1,
    "UndoDialogView.swift": 2,
}
STRING = re.compile(r'"(?:[^"\\\n]|\\.)*"')


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


def compiled_for(lines, code):
    return {compiled for compiled, line in lines if line == code}


def menu_of(text, head):
    """The text of the block that opens on the line holding the head, through its closing brace."""
    start = text.find(head)
    if start < 0:
        return ""
    depth = 0
    for at in range(text.index("{", start), len(text)):
        depth += {"{": 1, "}": -1}.get(text[at], 0)
        if depth == 0:
            return text[start : at + 1]
    return ""


class TheMacIdiom(unittest.TestCase):
    def test_body_text_and_controls_take_the_mac_sizes(self):
        """SPEC-679 section 9a: the card's zoom is 1 on macOS and the body text scale on iOS, the
        one control height is 28 on macOS and 44 on iOS, and every control frame reads it."""
        zoom = code_lines(text_of(self, FACTORY))
        self.assertTrue(
            any("static var bodyTextZoom" in line for _, line in zoom),
            "the factory states the zoom",
        )
        self.assertEqual(compiled_for(zoom, "return 1"), {"macOS"}, "the card's zoom is 1 on macOS")
        height = code_lines(text_of(self, APP + "CardHealthView.swift"))
        self.assertTrue(
            any("enum ControlHeight" in line for _, line in height), "one control height is stated"
        )
        self.assertEqual(
            compiled_for(height, "return 28"), {"macOS"}, "a control is 28 pt high on macOS"
        )
        self.assertEqual(
            compiled_for(height, "return 44"), {"iOS"}, "a control stays 44 pt high on iOS"
        )
        for name, count in examined("files with control frames", FRAMES.items()):
            code = " ".join(line for _, line in code_lines(text_of(self, APP + name)))
            self.assertEqual(
                code.count(f"minHeight: {HELPER}"), count, f"{name}'s frames read the height"
            )
            self.assertEqual(code.count("minHeight: 44"), 0, f"{name} keeps no bare 44 pt frame")

    def test_the_review_actions_sit_in_a_menu_with_their_keys(self):
        """SPEC-679 section 9a: the scene's commands hold a Review menu whose every button, Again
        and Good among them, carries its keyboard shortcut."""
        text = text_of(self, APP + "DeckStreakApp.swift")
        self.assertIn(".commands {", text, "the scene holds commands")
        menu = menu_of(text, 'CommandMenu("Review")')
        self.assertNotEqual(menu, "", "the commands hold a Review menu")
        buttons = examined("buttons in the Review menu", re.findall(r"\bButton\(", menu))
        self.assertEqual(
            menu.count(".keyboardShortcut("), len(buttons), "each menu button carries its key"
        )
        for grade in examined("grades in the menu", ("Again", "Good")):
            self.assertIn(f'"{grade}"', menu, f"the Review menu holds {grade}")

    def test_the_window_is_resizable_with_a_default_size(self):
        """SPEC-679 section 9a: the scene opens at a default size in a resizable window, on both
        platforms with no conditional."""
        lines = examined("scene lines", code_lines(text_of(self, APP + "DeckStreakApp.swift")))
        sizes = {compiled for compiled, line in lines if line.startswith(".defaultSize(width:")}
        self.assertEqual(sizes, {"both"}, "the scene has one default size on both platforms")
        resizable = {
            compiled for compiled, line in lines if line.startswith(".windowResizability(")
        }
        self.assertEqual(resizable, {"both"}, "the scene's window is resizable on both platforms")


if __name__ == "__main__":
    unittest.main()
