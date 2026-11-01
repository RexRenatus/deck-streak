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
A5: cargo test -p deck-streak-engine-core --test formal_vectors_fsrs7_schedule
A6: cargo test -p deck-streak-fsrs7 --test formal_vectors_fsrs7_memory_state -- --exact the_replay_holds_the_lean_claims_on_every_vector
A7: cargo test -p deck-streak-engine-core --test formal_vectors_fsrs7_port_state
```

```red-first
A1: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/tests/stock.rs:112:5: assertion `left == right` failed: Again: the stock stability is 0.000038275626, stored as 0, not the floor 0.0001; left: 941656612 right: 953267991
A2: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/src/convert.rs:68:66: attempt to subtract with overflow
A3: red at 76a5eb17495d6ca0b9ef29213660c76ff09fae3c: panicked at crates/fsrs7/tests/formal_vectors_fsrs7_stock.rs:154:5: {"interval":[0],"stability":[3,13743895,137438953472]}: the stock stability is NaN, not 0.0001
A4: not red: it replays the formal pass's memory-state vectors over `crates/fsrs7/src/replay.rs`, which this delivery does not change (SPEC-413 section 3); it passed over the seam commit (1 test), and over the green it examined 2222 vectors, 18495 states in bounds
A5: not red: it replays `lean/Fsrs7Schedule`'s vectors over dev's code, which already holds L8 to L10: dev's `interval_days` (its rounding now `whole_days`, `interval.round() as u32`) then `scheduled` (`rounded.min(max_ivl).max(1)`) compose to exactly the fork's `scheduled`, and `day_of` and `stock_fields` are text-identical; over 2cf9684b it examined 3516 vectors (3510 schedules, 6 writes), 0 differing, and its literal cases passed (2 tests); its teeth are the band rows its killer kills
A6: not red: the pinned revision's `stability_for_set_scalar` already holds both bounds (L12), so its update lines read green over the code as it is; over 211bbc2d it examined 2762 vectors, 18495 states in bounds, 540 update lines and 10800 next states within the update's bounds (slow and fast stability, 5 models); its claims' teeth are the planted-variant witnesses of ADR-427 D7 (main's Q2), and its test's teeth the band rows its killer kills
A7: not red: dev's port already refuses an event before it writes, orders its keys signed and restores its own checkpoints (L13), so its vectors read green over the code as it is; over 6df94b36 it examined 38 vectors (15 fold sequences, 23 restores), 0 differing, and its literal cases passed (2 tests); its claims' teeth are the planted-variant witnesses of ADR-427 D7, and the formal stage's plant control read FORMAL FAIL WITNESS_VACUOUS, exit 1, with the format witness switched to a positive statement (scratch commit b01ab9b2), and FORMAL VOID MODEL_ERROR, exit 3, with its restore switched to the port's own (scratch commit 0622614c)
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

After the merge of dev the band gained S41304 to S41310 (`19606285`, SPEC-413 section 9): three rows for A5's
test over `day_of` and `stock_fields`, one for the memory-state test over the batch, and three for A7's test over
the fold's order of check and write and the checkpoint's format byte and key encoding. At `19606285` the whole
band, S41300 to S41310, was proved by the repository's own prover, `python3 scripts/mutation_rows.py prove --band
S41300-S41399`: for each row its killer ran alone and passed, one test selected; its one mutant was installed and
built; its killer ran alone and failed, one test selected; and the file was restored byte for byte, its sha256
checked. All eleven read KILLED (`rows: examined 11: killed 11, survived 0, void 0`, exit 0). The sha256 of
`crates/fsrs7/src/stock.rs`, `convert.rs`, `replay.rs` and `crates/engine-core/src/replay.rs` read the same before
and after, and the tree held no tracked change. Over the same tree `cargo test --locked -p deck-streak-fsrs7`
passed 22 tests in 11 binaries and `cargo test --locked -p deck-streak-engine-core` 168 tests in 28 binaries. The
update lines of A6 call the pinned revision and no code in this tree, so no row reaches them.
