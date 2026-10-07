# Red-first record: SPEC-364

The SPEC (3633f830), ADR-375 (5b3b641f) and the schematic (f09d8029) were committed first, then the
core's test support (8feca65a), before any code the criteria judge. Each criterion's test was then
committed before the code that turns it green, and each red below is quoted from the run at the red
commit, on a clean tree. This delivery is part b1 of SPEC-364: the browser transport and the
browser's one-way write are parts b2 and b3 (section 7).

## The fence, line by line

Each of the 16 lines of SPEC-364 section 3's fence resolves to a test this delivery adds, or to a
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
```

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
