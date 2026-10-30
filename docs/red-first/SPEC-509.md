# Red-first record: SPEC-509

The crate's tests, the Lean entry for the id allocator with its vectors, and the four stamped sync entries' vectors
tests came first, whole, over a stub crate that keeps every shape and plants one defect per criterion (170d75fa): the
decoder skips the version byte, the clock is `max(pt, last)` with no step, the key compares the origin before the
clock, the queue drops an operation on send, the field merge is last writer by arrival, the slot is added to the id
without reducing mod 64, `Op` carries a planted `ReviewEdited` variant, the body carries the clock, and the clock
reads the system time. 43d84b78 turns them green. 06095041 stamps the four Lean entries' covers on the crate's code and
ties them in the ledger. The SPEC, ADR-527, the schematic, the map and the lexicon came after the code (000097f4), so
A12's red is read at 06095041, the code's head before them.

Each Rust red below is quoted from one run of the crate's eleven test targets at 170d75fa (`cargo test --locked
--offline --no-fail-fast -p deck-streak-sync --test codec --test clock --test crate_shape --test field_merge --test
formal_vectors_field_merge --test formal_vectors_id_slot --test formal_vectors_review_key --test
formal_vectors_sync_clock --test ids --test op_census --test pending`: exit 101, 16 failed, 3 passed). The green is
the same command on the tree of b11e750c: exit 0, 19 passed, 0 failed.

```red-first
A01: red at 170d75fa: codec.rs:155: assertion `left == right` failed: ReviewAnswered { card: 0, ... } (left: Err(UnknownKind { tag: 0 }), right: Ok((OpHead { ... }, ReviewAnswered { ... })))
A01: red at 170d75fa: codec.rs:227: assertion `left == right` failed (left: Err(UnknownKind { tag: 0 }), right: Err(Version { found: 0 }))
A01: red at 170d75fa: codec.rs:275: assertion `left == right` failed: ease (left: Err(UnknownKind { tag: 0 }), right: Err(OutOfRange { field: "ease" }))
A01: red at 170d75fa: codec.rs:329: assertion `left == right` failed (left: 262152, right: 262144)
A01: green at 43d84b78
A02: red at 170d75fa: formal_vectors_sync_clock.rs:41: assertion `left == right` failed: the crate's line for {"last":0,"pt":0,"seen":0,"tick":1} (left: "tick":0, right: "tick":1)
A02: red at 170d75fa: clock.rs:11: assertion `left == right` failed: a wall clock behind the last reading (left: Ok(100), right: Ok(101))
A02: green at 43d84b78
A03: red at 170d75fa: formal_vectors_review_key.rs:67: assertion `left == right` failed: the crate's line for {"arrived":[[2,1,1],[1,2,1]],"sorted":[[1,2,1],[2,1,1]]} (left: "sorted":[[2,1,1],[1,2,1]])
A03: green at 43d84b78
A04: red at 170d75fa: pending.rs:34: assertion `left == right` failed (left: [(4, Queued), (5, Queued)], right: [(1, Sent), (2, Sent), (3, Sent), (4, Queued), (5, Queued)])
A04: green at 43d84b78
A05: red at 170d75fa: field_merge.rs:71: assertion `left == right` failed: one value and one loser set for every order (left: three outcomes, right: one)
A05: red at 170d75fa: field_merge.rs:95: assertion `left == right` failed (left: the stale edit, right: the edit "third")
A05: red at 170d75fa: formal_vectors_field_merge.rs:65: assertion `left == right` failed: the crate's line for {"edits":[[1,2,1,20],[1,1,1,10]],"register":[1,2,1,20]} (left: "register":[1,1,1,10])
A05: green at 43d84b78
A06: red at 170d75fa: codec.rs:199: assertion `left == right` failed: ReviewAnswered { card: 0, ... } (left: a body of 54 bytes, the 46 of the right and 8 more holding the clock, right: 46 bytes)
A06: green at 43d84b78
A07: red at 170d75fa: formal_vectors_id_slot.rs:46: assertion `left == right` failed: the crate's line for {"floor":-9223372036854775808,"id":-9223372036854775744,"now":-9223372036854775808,"slot":0} (left: "id":-9223372036854775807)
A07: red at 170d75fa: ids.rs:63: assertion `left == right` failed: id 1761955176964 of device 0 (left: 4, right: 1)
A07: green at 43d84b78
A08: red at 170d75fa: op_census.rs:132: assertion `left == right` failed: the never-list (left: ["ReviewEdited: a review row updated or deleted"], right: [])
A08: green at 43d84b78
A09: not red: recorded as not run; no Kani install was on the machine this was built on (section 7 of the SPEC)
A10: red at 170d75fa: crate_shape.rs:173: assertion `left == right` failed: the crate (left: ["crates/sync/src/clock.rs:43: names SystemTime", "crates/sync/src/clock.rs:43: names time", "crates/sync/src/clock.rs:44: names time"], right: [])
A10: green at 43d84b78
A11: red at 43d84b78: test_formal_obligations.py of 06095041 over the ledger of 43d84b78: test_the_ledger_exists_in_schema_v1: Refusal(arm='entry-without-row', where='lean/IdSlot', why="the tree's entry has no row"); test_the_rows_hold_o1_to_o17_and_the_planned_entries and test_every_existing_entry_has_its_disposition_row: AssertionError
A11: green at 06095041
A12: red at 06095041: test_context_map_schematic.py: test_every_crate_directory_is_one_node_named_by_it: AssertionError: findings: 1 (sync: no node); test_every_arrow_is_a_manifest_edge_the_fence_names: AssertionError: findings: 1 (sync: no fence row)
A12: green at 000097f4
```

## What each pair disclosed

- **Three tests passed at the stub:** `each_refusal_reads_as_its_sentence`, `a_slot_is_admitted_below_64_and_refused_from_64`
  and `every_terminal_reason_keeps_the_bytes_and_every_pending_operation_keeps_its_frame` read the refusals' sentences,
  the slot's admission and the terminal reasons, which the stub already had. They support A01, A07 and A04; each of
  those criteria has a red test above.
- **Tests changed after the red.** 43d84b78 adds one assertion to `a_value_out_of_its_range_is_refused_by_both_ends`
  (a note of 65,536 fields is refused as out of range, since the 16-bit count cannot say it) and writes three emptiness
  checks in `field_merge.rs` as comparisons with the empty list. At the stub the range test failed at line 275, before
  the new assertion, so that assertion was never seen red. b11e750c writes the hour out in the same test (3,600,000 and
  3,600,001 ms) in place of the crate's constant, so a moved bound fails it; the green above is read on that tree.
- **A11's red was read by an overlay.** The ledger's row and its test landed together in 06095041, so the red is that
  commit's `test_formal_obligations.py` run over the ledger of 43d84b78: 9 tests, 3 failed. At bf79264c, whose ledger
  and test are those of 06095041, it reads 9 tests, OK.
- **A12's red names the crate only.** At 06095041 the census's other two tests and `test_context_map_register.py` (4
  tests) pass: the crate directory and its manifest are the only things missing from the map and its drawing. On the
  tree of 000097f4 and at bf79264c both files read 4 tests, OK.
- **The equal-bodies identity (A06) is held over each of the fourteen kinds:** 8,314 operations, each written by two
  devices. The design's reschedule instance of it is not applicable here: the vocabulary holds
  reschedule out, so that instance is owed by the change that admits the kind (SPEC-509 R8), and nothing above reads
  it as passed.
- **The Lean entries.** The formal check of the Lean kind at bf79264c, against the base `554a223e`, reads `FORMAL OK
  exit=0` over 23 entries, 65 properties and 68 witnesses, each property `clean: true`. The covers this change touches
  read 6 of 6 FRESH (`merge` and `pick`, `next_id`, `sort_keys` and `compare`, `tick`). IdSlot's four planted controls
  (each theorem stated of its defect and given the real proof) are each refused at their own lines, and `#print axioms`
  lists at most `propext`, `Classical.choice` and `Quot.sound` for every theorem and witness of the entry.
- **The crate's dependencies (A10).** `cargo tree --locked --offline -p deck-streak-sync -e normal,build` prints the
  crate alone for `x86_64-unknown-linux-gnu`, `wasm32-unknown-unknown`, `aarch64-apple-ios` and every target; with
  `-e dev` its one dependency is `serde_json`. The same census over `deck-streak-companion-slot` counts its one path
  dependency, so the census can fail. The library builds for the host target and for `wasm32-unknown-unknown`. Its
  build for `aarch64-apple-ios` was not run here, since the pinned toolchain holds no standard library for that target:
  CI's `web-engine` job proves it.
- **The mutation rows.** `python3 scripts/mutation_rows.py prove --band S50900-S50999` at b11e750c reads 6 rows
  examined, 6 killed, 0 survived, 0 void: each killer passes without its mutant and fails with it.
- **Not run here:** the whole workspace's tests, clippy and release profile, the mutation jobs, and CI's builds for
  the clients' targets, which only CI proves (SPEC-509 section 7); and the four Kani harnesses (A09).
