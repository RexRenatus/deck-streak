# Red-first record: SPEC-503

The SPEC, ADR-521, the schematic and the changelog fragment were installed first (7d242554). The tests came next,
whole, over the base's code (daef8669): the native column and the adapter's allow-list still refuse the normal sync,
and neither the core's `native_sync` module nor the facade's sync calls exist. 95120f86 turns them green. a1a0a2d9
meets rustfmt and clippy on the branch's own code, and c023bef4 adds the second tap that kills S50304 (below). Each
red below is quoted from a run of the named tests on the tree of daef8669, each green from a run on the tree of
c023bef4.

```red-first
A1: red at daef8669: crates/engine-core/tests/table.rs:112: assertion `left == right` failed: the pairs Native admits (left: the ten pairs without (1, 5), right: the eleven with it); table.rs:290: (native, web): both admit the login and the normal sync (left: ((1, 5), NotAllowed, Admit), right: ((1, 5), Admit, Admit)); crates/ffi/tests/review_pairs.rs:224: the allow-list (left: without (1, 5, "BackendSyncService.SyncCollection"), right: with it)
A1: green at c023bef4
A2: red at daef8669: crates/engine-core/tests/login_guard.rs:415: assertion `left == right` failed: each native normal sync whose endpoint breaks the rule, or that asks for media, is refused before the engine, naming only the rule (left: every case "not an engine-shaped refusal: Err(NotAllowed { service: 1, method: 5 })"); crates/engine-core/tests/native_sync.rs does not compile: error[E0432]: unresolved import `deck_streak_engine_core::native_sync`; crates/ffi/tests/sync.rs does not compile: 12 errors, no method named `sync`, `unsynced`, `full_sync_count` or `full_sync_confirm` on `Engine`, and no `ChoiceLoss`
A2: green at c023bef4
A3: red at daef8669: crates/engine-core/tests/handshake.rs:267: assertion `left == right` failed: below the minimum, each sync call is refused before the engine with the below sentence (the native normal sync's case is refused as a pair the native column does not admit); crates/ffi/tests/sync.rs does not compile (A2)
A3: green at c023bef4
A4: red at daef8669: crates/ffi/tests/sync.rs does not compile (A2)
A4: green at c023bef4
A5: not red: the XCTest runs on the macOS lane only (section 3), and neither it nor the Swift census ran on this box
A6: not red: the three models are re-read and re-stamped; at a8a95ee0 the covers read 4 listed, 0 stale, and tla/FullSyncChoice checks with 5 properties clean
```

## What each pair disclosed

- **The red run is the brief's prediction, measured.** At daef8669 engine-core's `table`, `login_guard` and
  `handshake` read 12 run, 4 failed (2, 1 and 1); the adapter's `review_pairs` read 2 run, 1 failed; engine-core's
  `native_sync` and the adapter's `sync` did not compile. Five tests failed by name and two targets failed to build.
- **The green run.** At c023bef4 engine-core's `table`, `login_guard`, `handshake`, `native_sync`, `parity`,
  `containment` and `graph` read 7 binaries, 25 passed, 0 failed; the adapter's whole package reads 17 test targets
  (its doc tests among them), 50 passed, 0 failed, 0 ignored. `cargo fmt --all --check` and
  `cargo clippy -p deck-streak-engine-core -p deck-streak-ffi --all-targets --locked --offline -- -D warnings`
  exit 0.
- **a1a0a2d9 changes no behaviour.** At 93c470cd rustfmt read two files unformatted and clippy refused two doubled
  must-use marks in `native_sync.rs`, the support items the sync tests leave unread, and a 106-line test. The marks
  go, the tests declare the unread support, and the first sync's test hands the second client's upload to a helper.
  rustfmt moved one `else` in `Engine::full_sync_confirm`, so a8a95ee0 re-stamps FullSyncChoice's cover on it.
- **The web sync's row moves with the table.** S36402 holds the web column's mark on the normal sync; its anchor
  named the native column's old `false`, so its find and replacement now read `native: true`. Its mutant and its
  killer are unchanged.

## Mutation rows

- S50300, S50301 and S50302 read KILLED at 23276f05, each killer passing alone without its mutant (1 test selected)
  and failing with it.
- S50304 read SURVIVED at 23276f05: no test tapped the choice again after a write. c023bef4 adds that second tap to
  the first sync's test, and the band reads 4 examined, 4 killed, 0 survived, 0 void at c023bef4.
- S50303 (the full-sync threshold in the app's model) is not registered: its killer is the XCTest (A5), and the
  registry's prover runs cargo and unittest killers only. It is proved on the macOS lane.

## Fix 1: tests for the facade's surviving mutants

A frozen population of 44 hand mutants over the facade, the allow-list, the table and the core's `native_sync` read
23 killed and 21 survived at ee4b05f4: each survivor changed a line no test reached. ccbdc6e1 adds tests that reach
them and changes no production line; 4d50b52a makes section 4's manifest and section 7's S50303 true. On a scratch
tree of 4d50b52a the population reads 44 examined, 42 killed, 2 survived, 0 void, each member applied alone and the
file restored byte for byte. Each red is a member's run failing by the test named; each green is the unmutated run,
before and after the population: engine-core's 7 targets 26 passed, the adapter's 17 targets 61 passed.

- **The sync's own statement read** (deleted, read and never handed to the core, or read after the sync):
  `a_fresh_engine_syncs_on_its_own_statement_read_with_no_login_before_it` syncs a fresh engine with another
  engine's host key, so no login's read admits it first.
- **A statement read at an endpoint the rule refuses**:
  `a_native_normal_sync_reads_no_statement_at_a_name_the_rule_refuses` counts the connections made to
  `http://localhost:<port>/`, a name the rule refuses.
- **The choice's taps**: `a_direction_the_choice_does_not_name_is_refused_and_writes_nothing` (a third direction run
  as the download), `an_upload_waits_for_its_snapshot_and_writes_nothing_without_it` (every snapshot answer read as
  found), `a_cancelled_choice_holds_no_counts_so_a_tap_after_it_writes_nothing` (the cancel keeps the stage),
  `a_refused_tap_keeps_the_open_counts_for_the_next_tap` (every tap takes the stage),
  `a_server_change_after_the_counts_shows_new_counts_and_the_next_tap_writes` (a recount, or a server change, holds
  no new stage), `an_upload_over_a_server_that_holds_rows_rechecks_it_into_a_new_file_and_writes` (the re-check
  reuses the counted copy, or the choice's files drop their stamp) and
  `a_count_at_an_endpoint_the_rule_refuses_is_the_engines_refusal_not_a_sentence` (an engine refusal shown as a
  sentence).
- **The counts and the unsynced read**, from fixtures the engine's own API writes:
  `the_counts_name_each_directions_lost_reviews_cards_and_notes` (lost reviews read as lost cards, lost notes as
  none) and `the_unsynced_count_reads_the_unsynced_reviews_and_each_stamp_alone` (two unsynced reviews, then each
  stamp after the last sync alone).
- **A negative requirement**: `native_sync`'s `a_reply_that_names_a_negative_requirement_reads_as_none`.
- **Two survive.** The device-changed recount's stage is reached only when the device gains a row between the
  download's backup and its write's read of the device, both inside one `full_sync_confirm` call, so only a write from
  another thread reaches it. The facade's refusal of an undecodable reply is reached only when the engine's own
  `SyncCollection` reply does not decode, and the engine's encoder always writes one that decodes, with a requirement
  of 0 to 4. Neither is reached by a test of the public calls alone.

Two behaviour-preserving seams then bring both lines within a test's reach, each line kept as it was: the normal
sync's last line moves into a private function that takes the reply's bytes, and the match on the one-way write's
result into a private method that takes that result. Two tests inside the adapter crate reach them.
`an_undecodable_sync_reply_is_the_engines_refusal_with_its_bytes` feeds the reply reader bytes that do not decode and
reads the engine's refusal carrying those bytes, then a reply that decodes and reads its requirement (an undecodable
reply read as requiring nothing). `a_device_change_at_the_write_holds_the_new_counts_for_the_next_tap` hands the
method a recount the core made at the write, from a device that gained a review after its backup, and reads a step
that wrote nothing, said the device changed and shows the new counts, with the stage holding those counts over the
device as it is now, so the next tap writes (new counts shown over the old stage).
