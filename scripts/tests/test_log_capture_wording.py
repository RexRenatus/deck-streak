"""The wording of the 2026-09-30 log-capture amendment says what the code does (#511)."""

import re
import unittest

from _support import REPO

SPEC = REPO / "docs" / "specs" / "SPEC-024-identity-initdata-and-owner-pin.md"
RECORD = REPO / "docs" / "red-first" / "SPEC-024.md"
KILLER = REPO / "crates" / "kernel" / "tests" / "log_capture_class.rs"
NESTED = "a_capture_nested_inside_a_capture_on_one_thread_is_refused"
STRUCK = re.compile(r"~~.*?~~")
ROUTED_COUNT = re.compile(r"\b\d+ routed\b")
AMENDMENT = re.compile(r"^## 10\. Amendment, 2026-10-01: .+$", re.MULTILINE)


def spec():
    return SPEC.read_text(encoding="utf-8")


def amendment():
    text = spec()
    found = AMENDMENT.search(text)
    assert found, "SPEC-024 has no section 10 amendment dated 2026-10-01"
    return text[found.start() :]


class TheKillersOwnExemptionIsStated(unittest.TestCase):
    def test_the_amendment_names_the_two_calls_the_killer_admits_in_itself(self):
        section = amendment()
        self.assertIn("crates/kernel/tests/log_capture_class.rs", section)
        self.assertIn("`callsite`", section)
        self.assertIn("`set_interest`", section)

    def test_the_amendment_answers_both_census_sentences_by_their_opening_words(self):
        section = amendment()
        for opening in (
            "The census refuses outside the helper every name in its fixed lists",
            "The census refuses any test that creates a dispatcher",
        ):
            self.assertIn(opening, section)

    def test_the_amendment_says_the_straddled_scenarios_register_by_hand_in_a_child(self):
        section = amendment()
        self.assertIn("register a callsite by hand", section)
        self.assertIn("child process", section)

    def test_the_code_admits_exactly_those_two_names_in_the_killer(self):
        code = KILLER.read_text(encoding="utf-8")
        self.assertIn('(matches!(token, "callsite" | "set_interest") && name == KILLER)', code)


def routed_counts(row):
    """Every `<n> routed` the row states once its struck spans are removed."""
    return ROUTED_COUNT.findall(STRUCK.sub("", row))


class TheRoutedCountHasOneHome(unittest.TestCase):
    def test_the_a18_row_states_no_routed_count(self):
        self.assertEqual(len(routed_counts("| A18 | x: 15 routed, none raw | t |")), 1)
        self.assertEqual(routed_counts("| A18 | x: ~~15 routed,~~ none raw | t |"), [])
        rows = [line for line in spec().splitlines() if line.startswith("| A18 |")]
        self.assertEqual(len(rows), 1, "SPEC-024 must hold exactly one A18 row")
        self.assertEqual(
            routed_counts(rows[0]),
            [],
            "the census test's assertion holds the routed count, so the A18 row states none",
        )


class TheRecordNamesTheNestedCaptureTest(unittest.TestCase):
    def test_the_killer_defines_the_test_the_record_names(self):
        self.assertIn(f"fn {NESTED}()", KILLER.read_text(encoding="utf-8"))

    def test_the_record_names_the_nested_capture_test_beside_its_red(self):
        record = RECORD.read_text(encoding="utf-8")
        self.assertIn(NESTED, record)

    def test_the_record_carries_the_unfiltered_passed_test_count(self):
        record = RECORD.read_text(encoding="utf-8")
        self.assertIn("816 tests run: 816 passed", record)


if __name__ == "__main__":
    unittest.main()
