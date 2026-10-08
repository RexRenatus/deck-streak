# Red-first record: SPEC-364

The SPEC (3633f830), ADR-375 (5b3b641f) and the schematic (f09d8029) were committed first, then the
core's test support (8feca65a), before any code the criteria judge. Each criterion's test was then
committed before the code that turns it green, and each red below is quoted from the run at the red
commit, on a clean tree. This delivery is part b1 of SPEC-364: the browser transport and the
browser's one-way write are parts b2 and b3 (section 7).

## The fence, line by line

Each of the 23 lines of SPEC-364 section 3's fence resolves to a test this delivery adds, or to a
test that stands on `dev` and is named as it is.

| # | criterion | test | added or named |
|---|---|---|---|
| 1 | A1 | `crates/engine-core/tests/table.rs` `the_web_column_admits_the_sync_login_and_the_normal_sync` | added (step 3b) |
| 2 | A1 | `crates/web-engine/tests/study.rs` `the_sync_calls_are_the_login_and_the_normal_sync` | added (step 3b) |
| 3 | A2 | `crates/engine-core/tests/login_guard.rs` `every_sync_call_reaches_only_a_guarded_endpoint_and_never_media` | added (step 3b) |
| 4 | A3 | `crates/engine-core/tests/exempt.rs` `run_exempt_refuses_the_one_way_sync` | added (step 3b) |
| 5 | A3 | `crates/engine-core/tests/one_way.rs` `a_write_takes_only_a_one_way_gesture` | added (step 3b) |
| 6 | A4 | `crates/engine-core/tests/one_way.rs` `the_write_sends_the_direction_the_owner_confirmed_and_no_media` | added (step 3b) |
| 7 | A5 | `crates/engine-core/tests/one_way.rs` `a_server_copy_is_fetched_only_into_an_empty_file` | added (step 3b) |
| 8 | A6 | `crates/engine-core/tests/one_way.rs` `a_downloads_backup_holds_every_device_id_before_the_write` | added (step 3a) |
| 9 | A7 | `crates/engine-core/tests/one_way.rs` `an_uploads_backup_is_the_counted_server_copy` | added (step 3b) |
| 10 | A8 | `crates/engine-core/tests/one_way.rs` `a_server_changed_after_the_count_returns_to_the_counts` | added (step 3b) |
| 11 | A9 | `crates/engine-core/tests/one_way.rs` `a_device_review_after_the_backup_refuses_the_download` | added (step 3b) |
| 12 | A10 | `crates/engine-core/tests/one_way.rs` `the_backup_and_the_server_copies_outlive_the_write` | added (step 3b) |
| 13 | A11 | `crates/engine-core/tests/one_way.rs` `an_evicted_device_is_offered_the_download_alone_and_restores_every_review` | added (step 3b) |
| 14 | A12 | `crates/engine-core/tests/containment.rs` `no_non_ui_caller_reaches_an_exempt_function` | named: dev's census, its planted callers grown by four and its refusals by four, insert-only (step 3b) |
| 15 | A13 | `crates/engine-core/tests/graph.rs`, whole | named: unchanged |
| 16 | A14 | `crates/engine-core/tests/parity.rs` `each_adapter_table_equals_its_transport_column` | named: dev's guard, its web side read as `STUDY_CALLS` with `SYNC_CALLS` (step 4) |
| 17 | A15 | `crates/engine-core/tests/one_way.rs` `an_edit_that_adds_no_id_returns_the_upload_to_the_counts` | added (the stamp) |
| 18 | A16 | `crates/engine-core/tests/one_way.rs` `a_tag_or_config_change_alone_returns_the_upload_to_the_counts` | added (the stamp) |
| 19 | A17 | `crates/engine-core/tests/one_way.rs` `a_deletion_returns_the_upload_to_the_counts` | added (the stamp) |
| 20 | A18 | `crates/engine-core/tests/one_way.rs` `an_edit_dated_at_or_below_the_greatest_returns_the_upload_to_the_counts` | added (the stamp) |
| 21 | A19 | `crates/engine-core/tests/one_way.rs` `a_full_upload_after_the_count_returns_the_upload_to_the_counts` | added (the stamp) |
| 22 | A20 | `crates/engine-core/tests/one_way.rs` `an_unchanged_server_rechecks_equal` | added (the stamp) |
| 23 | A21 | `crates/engine-core/tests/one_way.rs` `the_stamp_reads_every_synced_tables_greatest_usn_and_the_schema` | added (the stamp) |

## The first red: the backup statement

A6 was written alone, over a stub `back_up` that passes the device's ids and writes no file, and
committed at fbee2a7f before any other test of this part. Before that commit, with the tree
uncommitted, the one fixed statement `VACUUM INTO ?` was sent once through the engine's database
door with the backup's path bound, and A6 passed: the door runs the statement, so ADR-375 D4's
fallback is not needed. The stub was put back before the commit.

## The reds and greens

Each line's command is the criterion's line in SPEC-364 section 3's fence, run at the commit named.

```red-first
A1: red at c98b4a79: panicked at crates/engine-core/tests/table.rs:235:5: (native, web): both admit the login, the normal sync is the web's alone, and the one-way sync needs a gesture on both; left: [((1, 3), Admit, NotAllowed), ((1, 5), NotAllowed, NotAllowed), ((1, 6), NotAllowed, NotAllowed)]; and panicked at crates/web-engine/tests/study.rs:283:5: the web side names the sync login and the normal sync, and no other sync pair; left: []
A2: red at c98b4a79: panicked at crates/engine-core/tests/login_guard.rs:296:5: each one-way write whose endpoint breaks the rule is refused before the engine, naming only the rule; left: [("an empty endpoint", Ok((InvalidInput, "Invalid sync server specified. Please check the preferences."))), ("an endpoint that is not a URL", Ok((InvalidInput, "Invalid sync server specified. Please check the preferences."))), ...]
A3: red at 943ac045: panicked at crates/engine-core/tests/exempt.rs:462:5: run_exempt refuses the one-way gesture before it decodes its request; left: [("no message", Err(WrongKind { write: OneWaySync, target: Collection })), ...]; and panicked at crates/engine-core/tests/one_way.rs:346:5: the write refuses a gesture of any other write, naming the write and the target it named; left: Ok(())
A4: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:415:5: (direction, upload, server_usn, auth carried): each write sends its own direction, the checked auth, and no media; left: [(Download, true, None, true), (Upload, true, None, true)]
A5: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:495:5: a server copy is never fetched into the open collection nor into a file that holds rows; left: [("the open collection's path", Err(Engine(Engine { error: [10, 46, 65, 110, 107, 105, ...] }))), ...]
A6: red at fbee2a7f: panicked at crates/engine-core/tests/one_way.rs:121: the backup, opened by a fresh engine, holds every review, card and note id of the device; left: IdSets { reviews: {}, cards: {}, notes: {}, modified: 0 }
A7: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:558:5: the counted copy, read again from its file, backs the upload; emptied after the count, it backs nothing; left: (Ok(()), Ok(()))
A8: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:599:5: a server changed since the count returns the upload to new counts, which lose the second client's review; left: Ok(Err(Some(Losses { reviews: 0, cards: 0, notes: 0 })))
A9: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:200:10: the one-way sync takes the open collection as its target: WrongKind { write: OneWaySync, target: Collection }
A10: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:200:10: the one-way sync takes the open collection as its target: WrongKind { write: OneWaySync, target: Collection }
A11: red at 943ac045: panicked at crates/engine-core/tests/one_way.rs:200:10: the one-way sync takes the open collection as its target: WrongKind { write: OneWaySync, target: Collection }
A12: red at c98b4a79: panicked at crates/engine-core/tests/containment.rs:738:5: each planted caller, include, extended line, second copy and stale line is refused by name, and no comment is; the four planted `run_one_way` callers are absent from the left
A13: not red: the core's dependency census stands on dev unchanged and this part adds a dev-dependency only, so it was green at the base (2 passed at 943ac045) and must stay green
A14: not red: the parity guard stands on dev and was green at the base (1 passed at 943ac045); it reddens only in a tree that flips the web column without `SYNC_CALLS`
A15: not red: the header stamp refuses every re-check, so a changed server already returned the upload to the counts at the tests commit (1 passed at 59f5d9c8); the stamp's rows in SPEC-364 section 10 are its red evidence
A16: not red: the header stamp refuses every re-check, so a changed server already returned the upload to the counts at the tests commit (1 passed at 59f5d9c8); the stamp's rows in SPEC-364 section 10 are its red evidence
A17: not red: the header stamp refuses every re-check, so a changed server already returned the upload to the counts at the tests commit (1 passed at 59f5d9c8); the stamp's rows in SPEC-364 section 10 are its red evidence
A18: not red: the header stamp refuses every re-check, so a changed server already returned the upload to the counts at the tests commit (1 passed at 59f5d9c8); the stamp's rows in SPEC-364 section 10 are its red evidence
A19: not red: the header stamp refuses every re-check, so a changed server already returned the upload to the counts at the tests commit (1 passed at 59f5d9c8); the stamp's rows in SPEC-364 section 10 are its red evidence
A20: red at 59f5d9c8: panicked at crates/engine-core/tests/one_way.rs:1015:5: an unchanged server, downloaded by a second client and fetched again, carries the counted copy's stamp, and the upload is ready; left: (Ok(Err(Some(Losses { reviews: 0, cards: 0, notes: 0 }))), false)
A21: red at 59f5d9c8: panicked at crates/engine-core/tests/one_way.rs:1075:5: (write, the core's stamp moved, it equals the engine's hash read apart from the core): each synced table's greatest usn, and the schema stamp, moves the stamp; left: [("update cards set usn = 100", false, false), ("update notes set usn = 101", false, false), ("update revlog set usn = 102", false, false), ...
A1: green at 01c42f7f
A2: green at 01c42f7f
A3: green at 01c42f7f
A4: green at 01c42f7f
A5: green at 01c42f7f
A6: green at 01c42f7f
A7: green at 01c42f7f
A8: green at 01c42f7f
A9: green at 01c42f7f
A10: green at 01c42f7f
A11: green at 01c42f7f
A12: green at 01c42f7f
A20: green at 01c42f7f
A21: green at 01c42f7f
```

## The upload re-check stamp

The owner's ruling, `docs/rulings/OWNER-RULING-2026-10-08-upload-recheck-stamp.md`, set the stamp
the re-check compares (R6, ADR-375 D13). Its tests, A15 to A21, were committed at 59f5d9c8 over the
header stamp `select mod from col` and run there on a clean tree. The stamp's statement followed at
01c42f7f, where every line of the fence, A1 to A21, and the whole test sets of the core, the web engine
and the native adapter ran again on a clean tree and passed, with clippy clean.

- **A10 is the red the stamp turns green.** At 86e8d0e6, and again at 59f5d9c8, A10 failed at
  `crates/engine-core/tests/one_way.rs:691:10`: `an unchanged server is ready for the upload:
  Counted { offer: Offer { upload: true, download: true }, ...`. The fetch's download moves
  `col.mod`, so the counted and the fresh copy never shared the header stamp. Its red line above
  stays the one measured at 943ac045.
- **A15 to A19 are not red.** The header stamp refuses every re-check, so each already returned the
  upload to the counts at 59f5d9c8. What each would lose under a narrower stamp is held by SPEC-364
  section 10's rows that drop a table or the schema stamp, or read the greatest modified time, each
  killed by a named test.
- **Three existing tests changed with R6, and each read red at 59f5d9c8 for its oracle's or its
  fixture's reason; each is green at 01c42f7f.** A6 (`one_way.rs:309:5`) and SPEC-357's
  `full_sync::the_id_reads_take_every_row` (`full_sync.rs:137:5`) compare the core's read with the
  `held` oracle, which now computes the stamp apart from the core, while the core still read
  `col.mod` (`modified: 1791443231449` against `-8548778104718257131`, and `1791443231190` against
  `-5811412170459196523`). `full_sync::a_reply_that_is_not_integers_is_the_engines_database_error`
  (`full_sync.rs:241:5`) now plants `scm = 1.5`, which the header stamp never read (`left:
  Ok(IdSets { ...`). Its fixture moved from `mod = 1.5`, and its expected message from the core's
  refusal of a reply that is not integers to the engine's own database error, because the stamp's
  statement answers an integer or the engine's error, never a fraction. That message was measured
  before the tests commit with the stamp's statement in the tree, uncommitted, and the tree was put
  back first. The core's own refusal of such a reply stays in `dispatch.rs`; no fixture reaches it
  now.
- **A tag no note carries is planted by the test's own handle.** The engine registers a tag only
  through a note, so A16's tag half inserts the tag row and moves `col.mod` with the test's SQL on
  the second client, before its normal sync; the server registers the name at its own next usn.
- **Every commit after 01c42f7f leaves `crates/` unchanged**, so each green above holds at the pushed
  head.

## What the record discloses

- **Three reds stop at the gesture, before their own stub.** With the table unchanged (A1's stub),
  `OwnerGesture::from_tap(OneWaySync, Collection)` answers `WrongKind`, so A9, A10 and A11 fail where
  they build the owner's tap, before the stub each was written over is reached. The reds are recorded
  as measured.
- **A3's write half runs over the stub that runs any write.** The stub `run_one_way` ignores its
  gesture, so a `Forget` gesture's write ran and answered `Ok(())`. The stub that refuses every
  write, which A11 was to be red over, cannot stand in the same commit; A11's red is the measured one
  above.
- **A10 has no removal stub.** R8 forbids removal code, even as a stub, so A10's red is measured
  against the absence of the owner's gesture, not against a stub that removes a copy.
- **A8's and A9's stubs are the ones the types allow.** `Checked` and `Ready` keep their ids private
  and the re-check and the write take no copy path, so the stubs are a re-check that reads `fresh`
  without fetching it and a write that passes empty ids to its last check.
- **A5's left is the engine's own refusal.** A private engine on the open collection's path is
  refused by the engine itself (its error decodes to the engine's own refusal of a collection that
  is already open), so the stub's fetch replaced nothing; the core's refusal by name is the green's.
- **Every red here was re-run at its commit on a clean tree**: A1, A2 and A12 at c98b4a79 and again
  at 943ac045, A3 to A11 at 943ac045, A6 at fbee2a7f and again at 943ac045, each exit 101.

## After the stamp: the core's own refusal and the native adapter's anchor

- **The core's own refusal is reached again, by the review ids.** The stamp's statement answers an
  integer or the engine's own error, so the not-integers test above no longer reaches the core's
  refusal of a reply that is not the integers its statement selects; the review ids' statement
  does. `full_sync::a_review_id_that_is_not_an_integer_is_the_cores_own_refusal`, committed at
  f84a0127, plants a review id that is not an integer and asserts the refusal's kind and message
  byte for byte, naming `select id from revlog`. It is mutation coverage, not a red: the refusal
  stood before it, so it passed at its own commit and takes no line in the fence. Two hand mutants
  of the refusal, its message emptied and its kind changed, each failed it at `full_sync.rs:267:5`
  with exit 101, and the unmutated run passed. This corrects the line above that says no fixture
  reaches the refusal now.
- **The native adapter's four earlier sentences are back in one match.** The one-way sync's
  refusal had been told inside the match that an existing mutation row holds whole, closing brace
  included, so that row's anchor occurred 0 times. d2fd428a tells the four earlier refusals from a
  private type's match, verbatim, and the one-way sync's refusal beside it; no sentence changed, and
  the row's anchor occurs once again.
- **Two commits after 01c42f7f change `crates/`**, f84a0127 (a test) and d2fd428a (the native
  adapter), so the line above that says every later commit leaves `crates/` unchanged no longer
  holds. At af575016, whose `crates/` tree is d2fd428a's, every line of the fence,
  A1 to A21, the whole of `full_sync`, and the whole test sets of the core, the web engine and the
  native adapter ran again on a clean tree and passed, with clippy clean. This record's own commit
  changes no path under `crates/`.

## Three test edits between A1's red and its green

Three commits between A1's red at c98b4a79 and its green at 01c42f7f edit a test file. Each is
growth the base brief's step 4 orders, admitted insert-only under ruling 591, and none removes a
name or rewrites an assertion.

- **9073554 grows the containment census by one gesture name** (step 4.7). In
  `crates/engine-core/tests/containment.rs`, `GESTURE_NAMES` holds four names where it held three,
  adding `run_one_way`, and its doc comment names the one-way door. The three earlier names and the
  census's assertions are unchanged.
- **09a1b52 reads the web side's sync calls beside its study calls** (step 4.8). In
  `crates/engine-core/tests/parity.rs`, the web set is `STUDY_CALLS` and `SYNC_CALLS` joined, and
  its examined label names both. The comparison with the core's web column is unchanged.
- **a83483d adds a fifth sentence** (step 4.6). In `crates/ffi/tests/exempt.rs`, the list of exempt
  refusals gains `NeedsTheChoice` and its sentence. The four earlier sentences and the assertion
  that reads them are unchanged.

## Mutation coverage: three tests for the diff's own mutants

The diff's own mutants, run at 48dfcade, left five missed. 96f92e33 adds three tests for them. Each
is mutation coverage, not a red: the code each reaches stood before it, so each passed at its own
commit and takes no line in the fence. Each was run by hand at 8e8aef53 on a clean tree against the
mutant it covers, and the source was put back by its checksum after each run.

- **`one_way::a_server_copy_is_fetched_into_a_file_whose_collection_holds_no_row`** plants an
  existing file whose collection holds no review, card or note, fetches the server's copy into it,
  and reads the server's ids back from the file. A5 fetched only into an absent path, so the check
  that a planted file holds no row was reached by no test. With that check answering true, and
  again with its negation deleted, the test failed at `one_way.rs:1114:5`, `a file whose collection
  holds no row is fetched into: Err(HoldsRows)`, exit 101 each.
- **`one_way::a_copy_path_that_is_not_utf8_is_the_cores_own_refusal`** gives the fetch a copy path
  that is not UTF-8 and asserts the core's refusal, its kind and message byte for byte, and that
  nothing is written at the path. With the refusal's message deleted it failed at
  `one_way.rs:1157:5` (left `Err(Some((InvalidInput, "")))`), exit 101. With its kind deleted it
  passed, exit 0: the kind is the proto3 default, so the bytes are equal. That mutant is recorded
  as equivalent in `scripts/mutation-equivalent.d/deck-streak-engine-core.json` at 25930076.
- **`dispatch::tests::a_close_with_no_collection_open_is_the_engines_refusal`** is a unit test in
  `crates/engine-core/src/dispatch.rs`, because the dispatcher's close is crate-private and both of
  its callers reach it only with a collection open. It closes a dispatcher that opened none and
  asserts the engine's own refusal, its kind and message byte for byte. With close answering
  `Ok(())` it failed at `dispatch.rs:494:9` (left `Ok(())`), exit 101.

The unmutated runs of all three passed, one test selected each. This record's own commit changes no
path under `crates/`.
