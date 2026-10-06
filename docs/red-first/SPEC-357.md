# Red-first record: SPEC-357

The SPEC, its schematic and ADR-368 were committed first (d62caae6), then the formal model, before
any of the code it covers. Each criterion's test was then committed before the code that turns it
green, and each red below is quoted from the run at the red commit. This delivery is part a of
SPEC-357: its criteria are A1 to A12; the screens, the snapshot and the acceptance session are the
next pull requests' (section 7).

## The fence, line by line

Each of the 12 lines of SPEC-357 section 3's fence resolves to a test this delivery adds, or to a
test that stands on `dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `scripts/tests/test_formal_config.py` `the_committed_file_holds_exactly_the_declared_fields` | named: dev's test, its `EXPECTED` entries grown by one line, insert-only (step 2) |
| 2 | A2 | `crates/engine-core/tests/full_sync.rs` `the_offer_follows_the_engines_answer` | added (step 4) |
| 3 | A3 | `crates/engine-core/tests/full_sync.rs` `each_direction_counts_the_ids_the_replaced_side_alone_holds` | added (step 4) |
| 4 | A4 | `crates/engine-core/tests/full_sync.rs` `a_direction_not_offered_is_refused_and_the_choice_kept` | added (step 4) |
| 5 | A5 | `crates/engine-core/tests/full_sync.rs` `a_backup_missing_an_id_of_the_replaced_side_is_refused` | added (step 4) |
| 6 | A6 | `crates/engine-core/tests/full_sync.rs` `an_upload_reaches_ready_only_after_the_snapshot_and_the_recheck` | added (step 4) |
| 7 | A7 | `crates/engine-core/tests/full_sync.rs` `a_download_needs_no_snapshot_and_refuses_one` | added (step 4) |
| 8 | A8 | `crates/engine-core/tests/full_sync.rs` `a_write_refuses_a_device_row_its_backup_lacks` | added (step 4) |
| 9 | A9 | `crates/engine-core/tests/full_sync.rs` `the_unsynced_read_counts_reviews_and_changes_since_the_last_sync` | added (step 3) |
| 10 | A10 | `crates/engine-core/tests/full_sync.rs` `the_id_reads_take_every_row` | added (step 3) |
| 11 | A11 | `crates/engine-core/tests/full_sync.rs` `only_the_choice_makes_a_write` | added (step 4) |
| 12 | A12 | `crates/engine-core/tests/graph.rs`, whole | named: unchanged |

## The model first

`formal/tla/FullSyncChoice` was committed at 3689a4a3, with its configuration and its five
witnesses, before `crates/engine-core/src/full_sync.rs` existed. Its seven `covers` lines carried a
digest of 64 zeros. The paired checker's `--gate --entry tla/FullSyncChoice` check at that commit
read `FORMAL EXAMINED entries=1 properties=5 witnesses=5 configs=1 landings=20 rulings=0
deferred=0`, 35 `FORMAL REPORT STALE ... does not resolve` (seven covers on each of five
properties: the items they name did not exist yet), and `FORMAL OK exit=0`. It reported no
`WITNESS_SURVIVED`, `VIOLATION`, `FLOOR`, `TIMEOUT` or `MODEL_ERROR`, so each witness was caught:

| property | witness |
|---|---|
| `BackupBeforeReplace` | `a-download-before-its-backup` |
| `SnapshotBeforeUpload` | `an-upload-with-no-snapshot-found` |
| `CountsCoverTheUpload` | `an-upload-with-no-re-check` |
| `AWindowSyncIsNotSilent` | `an-upload-that-keeps-the-servers-schema` |
| `NoReviewLost` | `a-write-that-does-not-re-read-the-device` |

The state floor is one hand run of the model's configuration, a measurement only: "59747 states
generated, 14882 distinct states found, 0 states left on queue." and "The depth of the complete
state graph search is 13.", so the configuration reads `states >= 14882` at `MaxReviews=3`. The
budget is two timed runs of the entry's check, 18 s and 19 s, so ceil(19 x 1.5 / 60) x 60 = 60
seconds, which A1 pins.

The covers were stamped at 9ecd03a3, after the last edit to `full_sync.rs`: each digest moved from
the zeros to its item's span. At that commit the entry's check read `FORMAL OK exit=0`, with no
`STALE`, `WITNESS_SURVIVED`, `FLOOR`, `VIOLATION`, `TIMEOUT` or `MODEL_ERROR`, and each of the five
properties clean.

## The reds and greens

Each line's command is the criterion's line in SPEC-357 section 3's fence, run at the commit named.

```red-first
A1: red at 34c7966e: AssertionError: False is not true : budgets.entries: {'tla/HabitXpFollowsItsLog': 300, ...} != {'tla/FullSyncChoice': 60, 'tla/HabitXpFollowsItsLog': 300, ...}
A1: green at afae52b2
A2: red at ed7d99c7: assertion `left == right` failed: a full sync offers both directions, a full upload the upload only, a full download the download only, and no change or a normal sync neither; left: [(true, true), (true, true), (true, true), (true, true), (true, true)], right: [(true, true), (true, false), (false, true), (false, false), (false, false)]
A2: green at 29219a7d
A3: red at ed7d99c7: assertion `left == right` failed: left: Counts { upload: Some(Losses { reviews: 0, cards: 0, notes: 0 }), download: Some(Losses { reviews: 0, cards: 0, notes: 0 }) }, right: Counts { upload: Some(Losses { reviews: 1, cards: 2, notes: 0 }), download: Some(Losses { reviews: 2, cards: 1, notes: 1 }) }
A3: green at 29219a7d
A4: red at ed7d99c7: assertion `left == right` failed: a direction the offer does not hold is refused, and the counted choice comes back unchanged; left: [Ok(Confirmed { .. direction: Download }), Ok(Confirmed { .. direction: Upload })], right: [Err(Counted { .. }), Err(Counted { .. })]
A4: green at 29219a7d
A5: red at ed7d99c7: assertion `left == right` failed: a download's backup that lacks one of the device's review, card or note ids is refused, and the confirmed choice is kept; left: [None, None, None], right: [Some(Confirmed { .. }), Some(Confirmed { .. }), Some(Confirmed { .. })]
A5: green at 29219a7d
A6: red at ed7d99c7: assertion `left == right` failed: an upload is not ready from its backup alone; left: None, right: Some(BackedUp { .. })
A6: green at 29219a7d
A7: red at ed7d99c7: assertion `left == right` failed: a download is ready from its backup; left: Err(BackedUp { .. }), right: Ok(())
A7: green at 29219a7d
A8: red at 1d1236ca: assertion `left == right` failed: a download whose device gained a row its backup lacks is refused, with new counts over the device read now; left: [Ok(Download), Ok(Download), Ok(Download)], right: [Err(Counts { .. }), Err(Counts { .. }), Err(Counts { .. })]
A8: green at 17b41840
A9: red at 6eaf5d2f: assertion `left == right` failed: three reviews, two of them synced, then one review since the sync; a schema changed after its sync; a collection untouched since its sync; left: [Ok(Unsynced { reviews: 3, changed: true, schema: false }), ..], right: [Ok(Unsynced { reviews: 1, changed: true, schema: false }), ..]
A9: green at 3181a5ad
A10: red at 6eaf5d2f: assertion `left == right` failed: the core reads every review, card and note id and the modified stamp; left: one id of each kind, right: two of each
A10: green at 3181a5ad
A11: red at ed7d99c7: assertion `left == right` failed: only Ready::at_write constructs a Write; left: ["made"], right: []
A11: green at 29219a7d
A12: not red: the dependency census of the core stands on dev unchanged, and this delivery adds no dependency, so it was green at the base and stays green at 17b41840
```

## What the record discloses

- **A3's red cell was amended in its red commit.** SPEC-357 is new in this pull request. Its A3
  cell first read "counts taken from the unsynced rows", a stub the id sets cannot express, since
  they carry no sync number. The red commit ed7d99c7 amended it to "counts of zero", and the stub
  it measured is a `between` that counts zero.
- **The step-3 draft compiled as written.** The reads' draft (`dispatch.rs`, `lib.rs` and the first
  two tests) was carried into this delivery uncompiled. It built with no error and no warning, and
  needed no cure before its red commit 6eaf5d2f.
- **A test-only commit sits between two greens.** A8's test was committed at 1d1236ca, after the
  green of A2 to A7 and A11 (29219a7d) and before A8's own green (17b41840). It changes no earlier
  assertion; its one removed line is the test file's import list, grown by `Ready`.
- **The red commits' removed lines are imports.** ed7d99c7 replaces the import of `IdSets` and
  `Unsynced` with the grown list, and changes the A3 cell above.

## What the mutation pass added

- **Mutation coverage, green when written.** The diff's own mutation pass listed 59 mutants and
  found 4 that no test told apart: two in `Unsynced::warns` (`>` as `<`, and the first `||` as
  `&&`), each still warning on A9's fixtures, where every unsynced review also changes the
  collection; and two in the reads' refusal (its message or its kind dropped), which no fixture
  reached. Two tests were added after the last green, each green when written:
  `each_unsynced_change_warns_on_its_own` holds each of the three conditions warning with neither
  of the others, and `a_reply_that_is_not_integers_is_the_engines_database_error` gives the core a
  modified stamp the engine answers as a fraction and reads the refusal's kind and whole message.
  Neither changes an earlier assertion, and neither changes `full_sync.rs`, so the stamp stands.
