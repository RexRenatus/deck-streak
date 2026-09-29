"""The box driver's proxy-client scan admits an expected red that names an open issue (SPEC-123).

SPEC-056 R12 lets a pack's entry name `expected_red: {row: issue}`; this carries it to the scan's
entry, so a row a test substitutes can be deferred to an open issue without hiding any other row.
The driver runs here with the fake runner, fake gh and stand-in scan of `test_box_packs.py`.
"""

import unittest

from test_box_packs import Box, examined, pack_line, void_lines

SCAN = "proxy-client-scan"
ROW = "credential-from-secret-manager"
ISSUE = "#341"
SETTLED = ["green-settings", "green-surface"]


def deferred(case, rows, exit_code=1, expected=None, extra=None):
    """A box whose scan entry defers ROW to ISSUE and whose stand-in scan prints `rows`."""
    box = Box(case)
    entry = {"expected_red": {ROW: ISSUE} if expected is None else expected, **(extra or {})}
    box.set_box(dict(box.box, **{SCAN: entry}))
    box.set_scan(rows, exit_code)
    return box


def refusal(case, box):
    """The one VOID line a refused wiring prints, having run no pack."""
    done, calls = box.run()
    case.assertEqual(done.returncode, 2, done.stdout + done.stderr)
    void = void_lines(done.stdout)
    case.assertEqual(len(void), 1, done.stdout)
    case.assertEqual(calls, [], "no pack ran")
    return void[0]


class TheScanAdmitsAnExpectedRed(unittest.TestCase):
    def test_an_expected_red_row_passes_and_the_line_counts_it(self):
        box = deferred(self, [*SETTLED, "red-secret-manager"])
        done, _ = box.run()
        line = pack_line(done.stdout, SCAN)
        self.assertTrue(line.startswith("ok"), line)
        self.assertIn("unexpected 0, expected 1, stale 0", line)
        self.assertIn("1 settings document(s); blocking 2 green, 1 red, 0 void (1 expected)", line)
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        self.assertIn("BOX PACKS OK", done.stdout)

    def test_an_unexpected_red_row_beside_an_expected_one_still_fails(self):
        box = deferred(self, [*SETTLED, "red-secret-manager", "red-surface"])
        done, _ = box.run()
        line = pack_line(done.stdout, SCAN)
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("unexpected 1, expected 1, stale 0", line)
        self.assertIn("unexpected red: credential-not-on-disk", line)
        self.assertNotIn("unexpected red: credential-from-secret-manager", line)
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_a_red_total_beyond_the_expected_rows_fails(self):
        box = deferred(self, [*SETTLED, "red-secret-manager"])
        # The summary counts two red rows while the lines name one: the extra red is no row the
        # file expects, so the run fails though every printed red row is expected.
        path = box.checkout / "scripts" / "client-scan.py"
        path.write_text(path.read_text().replace("1 red,", "2 red,"), encoding="utf-8")
        box.repin()
        done, _ = box.run()
        line = pack_line(done.stdout, SCAN)
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn("2 red", line)
        self.assertEqual(done.returncode, 1, done.stdout)

    def test_an_expected_row_that_does_not_read_red_is_stale(self):
        for rows in (
            [*SETTLED, "green-secret-manager"],
            [*SETTLED, "void-secret-manager"],
            list(SETTLED),
        ):
            with self.subTest(rows=rows):
                done, _ = deferred(self, rows, exit_code=0).run()
                line = pack_line(done.stdout, SCAN)
                self.assertTrue(line.startswith("FAIL"), line)
                self.assertIn(f"stale: {ROW} ({ISSUE})", line)
                self.assertEqual(done.returncode, 1, done.stdout)

    def test_an_expectation_whose_issue_is_closed_is_stale(self):
        box = deferred(self, [*SETTLED, "red-secret-manager"])
        done, _ = box.run(closed=[ISSUE])
        line = pack_line(done.stdout, SCAN)
        self.assertTrue(line.startswith("FAIL"), line)
        self.assertIn(f"{ROW} ({ISSUE} is closed)", line)
        self.assertIn("unexpected 0, expected 1, stale 1", line)
        self.assertEqual(done.returncode, 1, done.stdout)
        # A closed issue on a row that no longer reads red is one stale expectation, not two.
        box.set_scan([*SETTLED, "green-secret-manager"], 0)
        done, _ = box.run(closed=[ISSUE])
        line = pack_line(done.stdout, SCAN)
        self.assertIn("unexpected 0, expected 0, stale 1", line)
        self.assertIn(f"stale: {ROW} ({ISSUE} is closed)", line)
        box.set_scan([*SETTLED, "red-secret-manager"], 1)
        # Each issue the scan's expectation names is read once, in the run's root.
        done, _ = box.run()
        asked = examined("issue state(s) read", box.gh_calls())
        self.assertEqual(sorted(call["argv"][2] for call in asked), ["29", "341", "59"])
        self.assertIn(f"{ISSUE} OPEN", done.stdout)


class TheScansExpectationIsRefusedUnlessItIsWellFormed(unittest.TestCase):
    def test_an_expected_red_beside_pending_is_refused(self):
        box = deferred(self, [*SETTLED, "red-secret-manager"], extra={"pending": "#29"})
        void = refusal(self, box)
        self.assertIn(f"box.{SCAN} is pending, so it expects no red row", void)

    def test_an_expectation_that_names_no_issue_is_refused(self):
        wrong = {
            "not an issue": {ROW: "later"},
            "a number without its hash": {ROW: "341"},
            "an empty mapping": {},
            "a list": [ROW],
            "a row that is no name": {"": ISSUE},
        }
        for what, expected in wrong.items():
            with self.subTest(what):
                box = deferred(self, [*SETTLED, "red-secret-manager"], expected=expected)
                void = refusal(self, box)
                self.assertIn(f"box.{SCAN}'s expected_red maps a row to an issue", void)

    def test_only_the_scan_takes_an_expected_red_and_no_other_key(self):
        box = deferred(self, [*SETTLED, "red-secret-manager"])
        # The scan's entry takes `note` and `pending` beside it, and nothing else.
        box.set_box(dict(box.box, **{SCAN: {"expected_red": {ROW: ISSUE}, "note": "interim"}}))
        done, _ = box.run()
        self.assertEqual(done.returncode, 0, done.stdout + done.stderr)
        wired = dict(box.box)
        for name, entry, takes in (
            (
                SCAN,
                {"expected_red": {ROW: ISSUE}, "waived": "#341"},
                "['expected_red', 'note', 'pending']",
            ),
            ("no-apikeyhelper", {"expected_red": {ROW: ISSUE}}, "['note', 'pending']"),
        ):
            with self.subTest(name):
                box.set_box(dict(wired, **{name: entry}))
                self.assertIn(f"box.{name} takes only {takes}", refusal(self, box))


if __name__ == "__main__":
    unittest.main()
