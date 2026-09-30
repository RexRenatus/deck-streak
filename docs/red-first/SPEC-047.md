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
A14: not red: the settle and the tap already read the configured rule at every point, so the test passes at the code it was written against (09e1dfc2); the plants that turn it red are P22 to P27 below
A15: not red: generation already dates its run and readings in the configured rule, so the test passes at the code it was written against (09e1dfc2); the plants that turn it red are P28 and P29 below
A15 note: the test first lived in readings_generate.rs and moved to readings_generate_rule.rs, because the stacked change (#386) edits readings_generate.rs and the two would conflict; it drives the same entry point, and P28 and P29 were re-measured against the new file
A16: not red: the resolution already reads its pause window and dates its run in the configured rule, so the test passes at the code it was written against (09e1dfc2); the plants that turn it red are P30 to P33 below
A17: not red: the export already writes every reading column from its own column, so the test passes at the code it was written against (09e1dfc2); the plants that turn it red are the 462 swap plants below
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

## Round 2 additions

The second verification round planted the default rule at the two points where the settle reads the
window, and both survived: every settle fixture ran under the default rule. Its finding was a
class, so the fix is a population and not two extra fixtures. Class 1: every point where the
readings settle, the read tap, generation or the resolution derives a study day or reads a window
takes the configured `StudyDayRule`. Class 2: every exported reading column holds, in every seeded
row, a value distinct from every other column of its kind and from its own default.

The A13 asserts of round 1 quoted at `readings_settle.rs:383` and `:388` above are historical: the
settle test file gained the population test and its helpers, and those two asserts now sit at other
lines. Their tests and messages are unchanged.

Read points, derived by grep over `crates/coordination/src/readings` and `crates/readings/src`
for `StudyDayRule|self\.rule|\.rule\b|study_day\(`, excluding docs: `settle.rs` (the day handed to
the grant and the stamp, the count, the close), `read_tap.rs` (the tap's day), `generate.rs` (the
run's and the reading's day), `resolve.rs` (the pause window and the run's day), `studied.rs` (the
count and the close), `gates.rs` (the window's gate) and `day_set.rs` (the studied-before gate).
Twelve sites read a rule. Each is planted with `StudyDayRule::default()` below, and each is red by
assertion. The rules are generated as the product of eight offsets (minus 720, minus 300, minus 210,
0, 330, 345, 540 and 840 minutes) and two rollover hours (0 and 4), sixteen in all, and each test
prints `examined 16 configured rule(s)` and asserts the count. Every begins-instant is written from
the definition of a study day, never derived from the code under test.

Plants P28 to P31 were not red under the settle test alone, so generation and the resolution each
gained a test on the same generated population (A15 and A16). Plants P26 and P27 were already killed at the head by the studied tests of A12 (rows S04710 and
S04711 own them), so they take no new row. Plants P24 and P25 keep the head's
choice of day (the instant the pass runs at); they change only which rule reads it.

```text
P22 (the settle counts reviews in the default rule; A14)
-studied_count(&window, self.rule,
+studied_count(&window, StudyDayRule::default(),
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:518:9:
       assertion `left == right` failed: offset -720, hour 0: the last instant of d + 1 counts
       test result: FAILED. 5 passed; 1 failed
P23 (the settle retires a reading in the default rule; A14)
-window.is_over(self.rule, now)
+window.is_over(StudyDayRule::default(), now)
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:531:9:
       assertion `left == right` failed: offset -720, hour 0: open before the close
       test result: FAILED. 5 passed; 1 failed
P24 (the settle hands the grant and the stamp lookup the default rule's day; A14)
-let today = self.rule.study_day(now);
+let today = StudyDayRule::default().study_day(now);
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:562:13:
       assertion `left == right` failed: offset -720, hour 0, instant 1728100800000: the settle's XP is dated by the configured day
       test result: FAILED. 5 passed; 1 failed
P25 (the tap hands its grant the default rule's day; A14)
-let today = self.rule.study_day(now);
+let today = StudyDayRule::default().study_day(now);
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:574:13:
       assertion `left == right` failed: offset -720, hour 0, instant 1728100800000: the tap's XP is dated by the configured day
       test result: FAILED. 5 passed; 1 failed
P26 (the studied rule counts a review in the default rule; A14)
-(0..=1).contains(&(rule.study_day(at)
+(0..=1).contains(&(StudyDayRule::default().study_day(at)
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:518:9:
       assertion `left == right` failed: offset -720, hour 0: the last instant of d + 1 counts
       test result: FAILED. 5 passed; 1 failed
P27 (the studied rule closes a window in the default rule; A14)
-rule.study_day(now).epoch_day() >= self
+StudyDayRule::default().study_day(now).epoch_day() >= self
red:   thread 'the_settle_and_the_tap_read_the_configured_study_day' panicked at crates/coordination/tests/readings_settle.rs:531:9:
       assertion `left == right` failed: offset -720, hour 0: open before the close
       test result: FAILED. 5 passed; 1 failed
P28 (generation reads the topic's days in the default rule; A15)
-parts.rule.study_day(parts.clock.now())
+StudyDayRule::default().study_day(parts.clock.now())
red:   thread 'the_generation_dates_its_run_and_readings_in_the_configured_study_day' panicked at crates/coordination/tests/readings_generate_rule.rs:430:17:
       assertion `left == right` failed: offset -720, hour 0, instant 1728100800000: the reading is dated by the configured day
       test result: FAILED. 0 passed; 1 failed
P29 (generation dates its run and readings in the default rule; A15)
-let study_day = parts.rule.study_day(now);
+let study_day = StudyDayRule::default().study_day(now);
red:   thread 'the_generation_dates_its_run_and_readings_in_the_configured_study_day' panicked at crates/coordination/tests/readings_generate_rule.rs:442:17:
       assertion `left == right` failed: offset -720, hour 0, instant 1728100800000: the absent-route run is dated by the configured day
       test result: FAILED. 0 passed; 1 failed
P30 (the resolution dates its run in the default rule; A16)
-parts.rule.study_day(started_at)
+StudyDayRule::default().study_day(started_at)
red:   thread 'the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule' panicked at crates/coordination/tests/readings_resolve.rs:325:17:
       assertion `left == right` failed: offset 0, hour 0: the run is dated by the configured day
       test result: FAILED. 4 passed; 1 failed
P31 (the resolution reads its pause window with the default rule; A16)
-rule: parts.rule,
+rule: StudyDayRule::default(),
red:   thread 'the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule' panicked at crates/coordination/tests/readings_resolve.rs:320:17:
       assertion `left == right` failed: offset -720, hour 0, review 1728043199999: the pause window is read in the configured rule
       test result: FAILED. 4 passed; 1 failed
P32 (the pause window gate reads a review in the default rule; A16)
-window.contains(&rule.study_day(
+window.contains(&StudyDayRule::default().study_day(
red:   thread 'the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule' panicked at crates/coordination/tests/readings_resolve.rs:320:17:
       assertion `left == right` failed: offset -720, hour 0, review 1728043199999: the pause window is read in the configured rule
       test result: FAILED. 4 passed; 1 failed
P33 (the day set gates studied-before with the default rule; A16)
-gates::studied_before(&data.reviews, inputs.today, inputs.rule)
+gates::studied_before(&data.reviews, inputs.today, StudyDayRule::default())
red:   thread 'the_resolution_reads_the_pause_window_and_the_day_in_the_configured_rule' panicked at crates/coordination/tests/readings_resolve.rs:320:17:
       assertion `left == right` failed: offset -720, hour 0, review 1728043199999: the pause window is read in the configured rule
       test result: FAILED. 4 passed; 1 failed
```

The seeds gave `studied_count` the value `i % 5`, the same as `carried_nights` in every row and the
column default in a fifth of them, so an export that wrote one from the other passed the probe. The
readings seed now gives every column a value no same-kind column shares in that row and none of its
column defaults, and the test asserts that as an invariant before it compares anything, over every
seeded row and every same-kind pair (`examined 10752 same-kind column pair(s) of the readings
seed`). The export names its columns in one block of 22, and a generated script planted every
ordered pair of them (`"a": row.b` for each a and b): `swap plants: 462, red 462`, none surviving and
none excluded by the compiler. Each is red at the symmetry assert, for instance the pair below:

```text
P34 (the export writes created_at from studied_at; A17)
-                                "created_at": row.created_at,
+                                "created_at": row.studied_at,
red:   thread 'the_exported_tables_equal_the_erased_tables_over_every_port' panicked at crates/coordination/tests/data_rights_symmetry.rs:416:5:
       assertion `left == right` failed: the export carries whole exactly the tables the erase clears or resets
       test result: FAILED. 3 passed; 1 failed
```
