# Red-first record: SPEC-515

The SPEC, ADR-533, the schematic and the changelog fragment were installed first (0db1397b). The tests came next,
whole, over the base's code (458e2c06), with the adapter's two undo entries as stubs that offer nothing and undo
nothing and its bury taking the shown states unread, so every test compiles and fails by its name: the native column
and the allow-list still refuse the undo status, and the adapter mints no gesture on the exempt Undo. 7183b921 turns
them green, and 26d410d3 stamps UndoOwnAnswer's two new covers. Each red below is quoted from a run of the named
tests on the tree of 458e2c06, each green from a run on the tree of 26d410d3.

```red-first
A1: red at 458e2c06: crates/engine-core/tests/table.rs:114: assertion `left == right` failed: the pairs Native admits (left: the eleven pairs without (3, 7), right: the twelve with it); table.rs:268: CollectionService.GetUndoStatus (3, 7) on the native transport (left: NotAllowed, right: Admit); crates/ffi/tests/review_pairs.rs:225: the allow-list (left: without (3, 7, "CollectionService.GetUndoStatus"), right: with it)
A1: green at 26d410d3
A2: red at 458e2c06: crates/ffi/tests/undo.rs:274: the offer names the card as text, the answer and the state it goes back to (left: None); undo.rs:326: the offer names the card as text, the state it goes back to and the bury's line (left: None); undo.rs:367: the flag is offered (the stub offers none)
A2: green at 26d410d3
A3: red at 458e2c06: crates/ffi/tests/undo.rs:426: the answer is offered (the stub offers none); undo.rs:464: with nothing kept, nothing is offered or undone, and each says why (left: (None, "", false, ""), the sentences empty); crates/ffi/tests/sync.rs:619: the unsynced answer is offered (the stub offers none)
A3: green at 26d410d3
A4: red at 458e2c06: crates/engine-core/tests/containment.rs:972: assertion `left == right` failed: the gesture on the exempt Undo is minted once in each review door, and nowhere else (left: [("crates/web-engine/src/wasm.rs", 1)], right: with ("crates/ffi/src/engine.rs", 1)); scripts/tests/test_ios_undo_reach.py's census over the Swift files 458e2c06 tracks: 11 problems, the ten undo call sites at 0 and the dialog's Keep and confirmation missing
A4: green at 26d410d3
A5: not red: the XCTest classes run on the macOS lane only (section 3); the thin-Swift census reads OK at 26d410d3, the three new files registered at their counts
A6: not red: the model is re-read and its two covers stamped; at 26d410d3 the covers read 12 listed, 0 stale, and the gate check of every TLA+ entry exits 0 with UndoOwnAnswer's four properties clean
```

## What each pair disclosed

- **The red run is the brief's prediction, measured.** At 458e2c06 engine-core's `table`, `containment` and `parity`
  read 11 run, 3 failed (2, 1 and 0); the adapter's `review_pairs`, `review_actions` and `undo` read 10 run, 6 failed
  (1, 0 and 5); its `sync`, filtered to the new test, 1 run, 1 failed. Ten tests failed by name, each for its
  criterion's reason, and no target failed to build.
- **The green run.** At 26d410d3 engine-core's `table`, `containment`, `parity`, `dispatch`, `face`, `undo_answer`
  and `undo_change` read 7 binaries, 61 passed, 0 failed; the adapter's whole package reads 17 test targets and its
  doc tests, 69 passed, 0 failed, 0 ignored. `cargo fmt --all --check` and
  `cargo clippy -p deck-streak-engine-core -p deck-streak-ffi --all-targets --locked --offline -- -D warnings`
  exit 0.
- **The formal check.** `phxd formal check --root . --base 92755ef0 --kind tla --gate --format json` at 26d410d3
  examined 28 entries, 106 properties and 141 witnesses and exits 0. UndoOwnAnswer's four properties read clean, and
  its four witnesses still violate. The only findings are HabitXpFollowsItsLog's five report-mode STALE rows on
  `crates/coordination/src/level_up.rs`, which this delivery does not touch.
- **The control is enabled by the judged offer, not the raw label.** The design read the undo status's label as the
  control's gate. Choosing the review's deck is itself an engine step with a label (`set_current_deck` runs in an
  undoable transaction when the deck changes), so the label is set before the review answers anything. The app reads
  the adapter's offer after every step instead, as the web's card view reads its judged undo (R6; ADR-533).
- **The sync test's support.** The new test reads the queue's head with the wire helpers, so the sync tests leave no
  support unread and the file's `dead_code` expectation goes.
- **S35019 moves with the column.** It holds the web column's mark on the undo status; its anchor named the native
  column's old `false`, so its find and replacement now read `native: true`. Its mutant and its killer are unchanged.
- **The last commit changes no tested line.** It adds this record and moves one awaited read out of a call's argument
  in the review model's flag arm; no Swift decision count moves.

## Mutation rows

- S51500 to S51504 read KILLED at 26d410d3 by the registry's prover, each killer passing without its mutant and
  failing with it: 5 examined, 5 killed, 0 survived, 0 void.
- S35019, re-anchored on `native: true`, reads KILLED at 26d410d3: 1 examined, 1 killed.
