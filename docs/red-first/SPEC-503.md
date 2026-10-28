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
