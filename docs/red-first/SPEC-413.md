# Red-first record: SPEC-413

SPEC-413 (R1 to R8, A1 to A4). The SPEC, ADR-427, the schematic's section 9 and the changelog fragment
were committed first (`9c50653d5732d142408e5c8d281f61e516633785`), then the Lean proof of the floor, its
witness and the writer's stability lines, with `lean/ReplayHistory`'s distance ported as `abs_diff`
(`4677e7d91faf19daa32f7f96ff4bd642349438d0`). The tests of A1 to A3 were committed next with a seam
that compiles (`76a5eb17495d6ca0b9ef29213660c76ff09fae3c`): `stock::STABILITY_MIN` is `0.0001`, and
`stock::stock_stability` returns its interval unfloored, so `project`, which calls it, writes the
interval as the merge base does; `convert::histories` still subtracts two `i64` ids. A1, A2 and A3 each
read red there for its own criterion. Every red below is quoted from the run over that commit, locally,
one test selected each.

```text
A1: cargo test -p deck-streak-fsrs7 --test stock -- --exact a_first_review_rated_again_keeps_a_stock_stability_the_store_holds
A2: cargo test -p deck-streak-fsrs7 --test convert -- --exact a_gap_past_the_signed_range_is_read_whole
A3: cargo test -p deck-streak-fsrs7 --test formal_vectors_fsrs7_stock -- --exact the_stock_projection_and_the_curve_hold_the_lean_claims_on_every_vector
A4: cargo test -p deck-streak-fsrs7 --test formal_vectors_fsrs7_memory_state -- --exact the_replay_holds_the_lean_claims_on_every_vector
```

```red-first
A1: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/tests/stock.rs:112:5: assertion `left == right` failed: Again: the stock stability is 0.000038275626, stored as 0, not the floor 0.0001; left: 941656612 right: 953267991
A2: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/src/convert.rs:68:66: attempt to subtract with overflow
A3: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/tests/formal_vectors_fsrs7_stock.rs:154:5: {"interval":[0],"stability":[3,13743895,137438953472]}: the stock stability is NaN, not 0.0001
A4: not red: it replays the formal pass's memory-state vectors over `crates/fsrs7/src/replay.rs`, which this delivery does not change (SPEC-413 section 3); it passed over the seam commit (1 test), and over the green it examined 2222 vectors, 18495 states in bounds
A1: green at 1bd6bba972a49c6ff885463f80225e0677c4dab5
A2: green at 1bd6bba972a49c6ff885463f80225e0677c4dab5
A3: green at 1bd6bba972a49c6ff885463f80225e0677c4dab5
```

Over the seam commit, SPEC-386 A7 and A8 (`the_stock_stability_is_the_ninety_percent_interval` and
`the_difficulty_is_clamped_and_the_interval_meets_the_retention`) also read red, on their Again row only,
which the seam commit moved to the floor (SPEC-413 R2, ADR-427 D1): `stock.rs:46:9` left 941656612 right
953267991, and `stock.rs:73:9` left `stability: 3.8275626e-5`, right `stability: 0.0001`. Their Hard, Good
and Easy rows and their other assertions are unchanged. The whole suite over the seam commit: 22 tests, 17
passed and 5 failed (A1, A2, A3, and SPEC-386 A7 and A8). The formatter and the lints of the crate, every
target, were green.

The green is the commit `1bd6bba9`, which changes `crates/fsrs7/src/stock.rs` (the floor's body,
`interval.max(STABILITY_MIN)`), `crates/fsrs7/src/convert.rs` (`abs_diff`, and `days` reads a `u64`) and
one comment in `crates/engine-core/src/replay.rs`, over the tests the seam commit added and left unchanged
since. A1, A2 and A3 each passed there, one test selected each; A3 examined 38 vectors, 12 stability lines
and 560 curve readings, and A4 still passes. The whole suite over that tree: 22 tests passed. The
formatter and the lints of the crate were green, and the formatter of `deck-streak-engine-core`. The
vectors' header digests were written again after the stamp (`072bca51`, `77c49d51`); their vector lines
did not change.

The mutation rows S41300 to S41303 were each proved by hand over `d4b30543`: the mutant applied, its
killer run alone and red, the file restored with `git checkout --`, and the killer run again green.
