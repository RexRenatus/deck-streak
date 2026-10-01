"""The wording of the 2026-09-30 log-capture amendment says what the code does (#511)."""

import re
import unittest

from _support import REPO

SPEC = REPO / "docs" / "specs" / "SPEC-024-identity-initdata-and-owner-pin.md"
RECORD = REPO / "docs" / "red-first" / "SPEC-024.md"
HELPER = REPO / "tools" / "log-capture" / "capture.rs"
KILLER = REPO / "crates" / "kernel" / "tests" / "log_capture_class.rs"
NESTED = "a_capture_nested_inside_a_capture_on_one_thread_is_refused"
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


class TheRecordNamesTheNestedCaptureTest(unittest.TestCase):
    def test_the_killer_defines_the_test_the_record_names(self):
        self.assertIn(f"fn {NESTED}()", KILLER.read_text(encoding="utf-8"))

    def test_the_record_names_the_nested_capture_test_beside_its_red(self):
        record = RECORD.read_text(encoding="utf-8")
        self.assertIn(NESTED, record)

    def test_the_record_carries_the_unfiltered_passed_test_count(self):
        record = RECORD.read_text(encoding="utf-8")
        self.assertIn("816 tests run: 816 passed", record)


class TheHelperDocSaysWhenTheFloorIsTheDefault(unittest.TestCase):
    def test_the_nested_refusal_doc_limits_its_claim_to_outside_a_dispatchers_own_call(self):
        text = HELPER.read_text(encoding="utf-8")
        at = text.index("fn refuse_nested_capture")
        block = text[:at].rsplit("\n\n", 1)[-1]
        self.assertIn("any other default means a capture is held", block)
        self.assertIn("outside a dispatcher's own call", " ".join(block.replace("///", "").split()))


if __name__ == "__main__":
    unittest.main()
