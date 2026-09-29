# Red-first record: SPEC-047

The order of work was: the census that the reading use cases touch no streak (50452a9); the
studied rule's tests, the parity golden and inert readings stubs (063e3b0); the tap and settle
tests beside stubs in coordination (56c49f7); the owner's route test beside a stub route that
answered 501 (831ff6e); the implementation, the migration and the refreshed query cache (c27e169);
the mutation rows (ce9a96d); and the tests that pin the XP constants, the grant sources and the
track (the last commit before the SPEC's promotion).

The parity golden was generated from the predecessor's own function, `preread_tracking.py:is_studied`,
at `27ee2bc`, over synthetic counts. The generator wrote to a scratch registry and output, with
bytecode writing off, and the predecessor's checkout read the same before and after.

Each criterion was run at its red commit with the SPEC's own fenced command, selecting one test,
and the whole of each test file was run there as well: all four tests of `readings_read_tap` and
all three of `readings_settle` failed there, each by assertion. The stubs answered nothing: the
tap answered a reading that was not found, the settle step measured no reading, the studied rule
answered false and the window counted no review. The census (A10) was red until the tap's source
file existed in the examined population; it was green as soon as the stub file joined it, because a
stub touches no streak, and it stays green with the implementation.

The read route's test (A11) was red against a stub that answered 501, and the two assertions the
review asked for sit in the tap and route tests: `a_second_read_tap_changes_nothing` asserts
covered 5, studied 0 and verdict open, and `the_read_route_answers_only_the_owner` asserts covered 5,
studied 4 and verdict studied.

```red-first
A1: red at 56c49f7: the tap answered: Err(NotFound)
A1: green at c27e169
A2: red at 56c49f7: assertion `left == right` failed: one caller of the read tick: []
A2: green at c27e169
A3: red at 56c49f7: the tap answered: Err(NotFound)
A3: green at c27e169
A4: red at 063e3b0: the stub answered false where the golden says a reading is studied
A4: green at c27e169
A5: red at 063e3b0: assertion failed: window.counts(rule, generated)
A5: green at c27e169
A6: red at 56c49f7: assertion `left == right` failed: the open reading is measured
A6: green at c27e169
A7: red at 56c49f7: the reading has a row
A7: green at c27e169
A8: red at 56c49f7: the tap answered: Err(NotFound)
A8: green at c27e169
A9: red at 56c49f7: the reading has a row
A9: green at c27e169
A10: red at 50452a9: read_tap.rs is not in the examined population
A10: green at 56c49f7
A11: red at 831ff6e: assertion `left == right` failed: left: 501, right: 200
A11: green at c27e169
A12: not red: the studied window already read the configured offset, so the test passes at the code it was written against (c265d2d3); the plant that turns it red is P06 below
A13: not red: the settle already leaves a reading whose stamp failed open and uncounted, so the test passes at the code it was written against (c265d2d3); the plants that turn it red are P10 and P11 below
```

## Round 1 additions

Two tests were added after the first verification round and made criteria A12 and A13. The code
predates them, so each is not red at the head; the record is the plant that turns each red, run
on the tree with the tests committed at `c265d2d3` and reverted afterwards.

```text
P06 (the window computed in the default rule instead of the configured one; A12)
-            && (0..=1).contains(&(rule.study_day(at).epoch_day() - self.study_day.epoch_day()))
+            && (0..=1).contains(&(StudyDayRule::default().study_day(at).epoch_day() - self.study_day.epoch_day()))
-        rule.study_day(now).epoch_day() >= self.study_day.epoch_day() + 2
+        StudyDayRule::default().study_day(now).epoch_day() >= self.study_day.epoch_day() + 2
red:   thread 'the_window_follows_the_configured_offset' panicked at crates/readings/tests/studied.rs:119:5:
       the rollover that starts d + 2, in the configured offset
       test result: FAILED. 2 passed; 1 failed
P10 (a reading whose stamp failed is counted studied; A13)
-                        .record_measure(&reading.id, count, reading.verdict, None)
+                        .record_measure(&reading.id, count, reading.verdict, { report.studied += 1; None })
red:   thread 'a_failed_stamp_leaves_the_reading_open_for_the_next_pass' panicked at crates/coordination/tests/readings_settle.rs:383:5:
       assertion `left == right` failed: a reading whose stamp failed is not counted studied
       test result: FAILED. 4 passed; 1 failed
P11 (the verdict and the studied instant are recorded on a failed stamp; A13)
-                        .record_measure(&reading.id, count, reading.verdict, None)
+                        .record_measure(&reading.id, count, Verdict::Studied, Some(now))
red:   thread 'a_failed_stamp_leaves_the_reading_open_for_the_next_pass' panicked at crates/coordination/tests/readings_settle.rs:388:5:
       assertion `left == right` failed: the verdict waits for the stamp
       test result: FAILED. 4 passed; 1 failed
```

The data-rights seeds gave the new reading columns their defaults, so an export that swapped
`read_at` and `studied_at` passed the symmetry probe. `2af0059e` gives the seeded rows distinct
read, studied and verdict values; the test is green at the head with it and not red at the code
it was written against, so it is recorded here and not as a criterion. Under the plant below
(`crates/readings/src/data_rights.rs`, the two columns swapped in the export) it is red:

```text
-                                "read_at": row.read_at,
+                                "read_at": row.studied_at,
-                                "studied_at": row.studied_at,
+                                "studied_at": row.read_at,
red:   thread 'the_exported_tables_equal_the_erased_tables_over_every_port' panicked at crates/coordination/tests/data_rights_symmetry.rs:365:5:
       assertion `left == right` failed: the export carries whole exactly the tables the erase clears or resets
       (the readings table is missing from the exported set)
       test result: FAILED. 3 passed; 1 failed
green: the same test at the head with the plant reverted: test result: ok. 4 passed; 0 failed
```

Two red-to-green test edits in `c27e169b` were not disclosed above: `Duration::from_hours(1)`
replaces `Duration::from_secs(3_600)` in `readings_read_tap.rs`, and `.copied()` replaces `.cloned()`
in `readings_settle.rs`. Both are form changes that clippy asked for; no assertion changed.
