"""No Swift source under `ios/` answers a card through the native `run`, and none names a third
grade (SPEC-365 R12, R13, A15 and A16; ADR-376 D13, D14).

The native door that records a grade is the adapter's `Engine::answer`: the press names its card,
its grade, Again or Good, and the states the card was shown with, and the adapter picks the next
state and mints the owner's answer. So no Swift source writes AnswerCard's pair `(13, 4)`, which
the allow-list no longer carries, and no Swift source names Hard or Easy: no `case` declares
`hard` or `easy`, and no expression names `.hard` or `.easy`.

Every Swift file git tracks under `ios/` is read with its comments and the text of its string
literals removed (the thin-Swift census's own reader), so a comment or a string that quotes the
pair or a grade is not counted. The census is a function of the sources it is handed: each test
first hands it planted sources it must refuse by name, and the same text in a comment and in a
string it must not count, then the live tree. Each refusal is printed as `path:line`, and the
count examined is printed after the behaviour's assertions.
"""

import re
import unittest

from _support import REPO, examined
from test_ios_thin_swift import code_of
from test_one_static_library import tracked_paths

#: AnswerCard's pair as a Swift tuple, spaces free.
PAIR = re.compile(r"\(\s*13\s*,\s*4\s*\)")

#: A `case` declaration's list, at a line's start or after `{` or `;`: each comma-separated item's
#: leading identifier is a case name (`case again, hard`, `case easy = 3`, `case hard(Int)`). A
#: `switch` arm's `case .hard:` names no identifier first, so the member rule judges it.
CASE_LIST = re.compile(r"(?:^|[{;])\s*(?:indirect\s+)?case\s+([^{}:;\n]*)")
CASE_NAME = re.compile(r"\s*([A-Za-z_]\w*)")

#: The two names no Swift source may give a grade.
THIRD_GRADES = ("hard", "easy")

#: An expression naming a third grade as a member: `.hard` or `.easy`, never `.hardState`.
THIRD_MEMBER = re.compile(r"\.(hard|easy)\b")


def swift_sources():
    """The tracked Swift files under `ios/` with their text. A drill-named path is never opened:
    the drill surface is parked, and the filter runs on the listing before any read."""
    tracked = tracked_paths(REPO)
    swift = [
        path
        for path in tracked
        if path.startswith("ios/") and path.endswith(".swift") and "drill" not in path.lower()
    ]
    return {path: (REPO / path).read_text(encoding="utf-8") for path in swift}


def door_findings(sources):
    """Each `path:line` whose code writes AnswerCard's pair."""
    found = []
    for path in sorted(sources):
        for number, line in enumerate(code_of(sources[path]).split("\n"), start=1):
            if PAIR.search(line):
                found.append(f"{path}:{number}: writes the pair (13, 4)")
    return found


def grade_findings(sources):
    """Each `path:line` whose code declares or names a third grade, with the grade it names."""
    found = []
    for path in sorted(sources):
        for number, line in enumerate(code_of(sources[path]).split("\n"), start=1):
            for declaration in CASE_LIST.finditer(line):
                for item in declaration.group(1).split(","):
                    name = CASE_NAME.match(item)
                    if name and name.group(1) in THIRD_GRADES:
                        found.append(f"{path}:{number}: declares the case {name.group(1)}")
            for member in THIRD_MEMBER.finditer(line):
                found.append(f"{path}:{number}: names .{member.group(1)}")
    return found


PLANTED = "ios/App/Sources/Planted.swift"


class NoSwiftSourceAnswersThroughRun(unittest.TestCase):
    def test_no_swift_source_answers_through_run(self):
        # Planted callers, each refused by name: the tuple with and without spaces.
        for tuple_text in ("(13, 4)", "(13,4)", "( 13 ,  4 )"):
            planted = {PLANTED: f"enum Call {{\n    static let answer = {tuple_text}\n}}\n"}
            self.assertEqual(
                door_findings(planted),
                [f"{PLANTED}:2: writes the pair (13, 4)"],
                f"the planted tuple {tuple_text} is refused by name",
            )
        # The same text in a comment, a block comment and a string is not counted, and another
        # pair is not AnswerCard's.
        quoted = {
            PLANTED: (
                "// answers through (13, 4)\n/* (13,4) */\n"
                'let said = "(13, 4)"\nlet other = (13, 24)\nlet queue = (13, 3)\n'
            )
        }
        self.assertEqual(door_findings(quoted), [], "a comment, a string or another pair")

        sources = swift_sources()
        found = door_findings(sources)
        self.assertEqual(
            found, [], "a Swift source answers through the native run:\n" + "\n".join(found)
        )
        examined("Swift source(s) under ios/ read for the pair (13, 4)", sources)


class NoSwiftSourceNamesAThirdGrade(unittest.TestCase):
    def test_no_swift_source_names_a_third_grade(self):
        # Planted grades, each refused by name: a case list, a case with a raw value, and members.
        planted = {
            PLANTED: (
                "enum Grade: Int32 { case again, hard, good }\n"
                "enum Wire: Int32 {\n    case easy = 3\n}\n"
                "let pressed: Grade = .easy\n"
                "let other = Grade.hard\n"
            )
        }
        self.assertEqual(
            grade_findings(planted),
            [
                f"{PLANTED}:1: declares the case hard",
                f"{PLANTED}:3: declares the case easy",
                f"{PLANTED}:5: names .easy",
                f"{PLANTED}:6: names .hard",
            ],
            "each planted third grade is refused by name",
        )
        # The same words in comments and strings, the engine's states for them, and the two
        # grades are not counted.
        quoted = {
            PLANTED: (
                "// case hard, .easy\n/* case easy = 3 */\n"
                'let titles = ["Hard", ".easy", "case hard"]\n'
                "enum Grade: Int32 { case again = 0, good = 2 }\n"
                "let states = [card.hardState, card.easyState]\n"
                "let pressed: Grade = .good\n"
            )
        }
        self.assertEqual(grade_findings(quoted), [], "comments, strings, states and two grades")

        sources = swift_sources()
        found = grade_findings(sources)
        self.assertEqual(found, [], "a Swift source names a third grade:\n" + "\n".join(found))
        examined("Swift source(s) under ios/ read for a third grade", sources)


if __name__ == "__main__":
    unittest.main()
