# Red-first record: SPEC-414

SPEC-414 (R1 to R8, A1 to A8). The SPEC, ADR-428, the schematic, the lexicon's row and the changelog fragment
were committed first (`e64a628d`). A8's census was committed next, alone (`b333717a`), and read red: no file
yet named the input. The input, a reorder that returns the stock order whatever the input holds, the module's
line in the core and the two formal entries' covers were committed next (`0eb1c3af`), and A8 read green there.
The tests of A1 to A7 were then written over that commit (`71b5c388`), and each read red on an assertion about
the missing reorder. The reorder was then written to ADR-339's input as it reads, an identifier and a score
applied whenever the switch is on (`c42180e6`): A1, A2, A3, A5 and A6 passed there, and A4 read red on the
defect the model found, yesterday's scores ordering today's queue; A7 read red on the four vectors that key the
input to another day or snapshot. The fix, the key's comparison and nothing else, is `f6cac38f`, where every
criterion passed. Every red below is quoted from a local run over the commit named.

```text
A1: cargo test -p deck-streak-engine-core --test reorder -- --exact the_reorder_presents_the_same_entries_and_writes_no_field
A2: cargo test -p deck-streak-engine-core --test reorder -- --exact scored_cards_lead_highest_first_and_the_stock_order_breaks_ties
A3: cargo test -p deck-streak-engine-core --test reorder -- --exact an_invalid_score_counts_as_no_score
A4: cargo test -p deck-streak-engine-core --test reorder -- --exact scores_made_for_another_day_or_snapshot_leave_the_stock_order
A5: cargo test -p deck-streak-engine-core --test reorder -- --exact an_unused_input_presents_the_stock_queue_and_on_again_reorders
A6: cargo test -p deck-streak-engine-core --test reorder -- --exact reordering_twice_is_once_and_the_order_reads_only_the_queued_scores
A7: cargo test -p deck-streak-engine-core --test formal_vectors_due_reorder -- --exact the_rust_reorder_gives_every_lean_vectors_output
A8: cargo test -p deck-streak-engine-core --test reorder_shape -- --exact the_input_is_named_by_the_reorder_module_alone_and_it_reaches_no_engine
```

```red-first
A1: red at 71b5c388: the population moved no card, so it judged no reorder (examined 31104 queue and input case(s), moved 0 of 31104 case(s))
A2: red at 71b5c388: assertion `left == right` failed: 13 leads at 1, then 12 and 14 at 0.5 in stock order, then unscored 11; left: [11, 12, 13, 14] right: [13, 12, 14, 11]
A3: red at 71b5c388: assertion `left == right` failed: a score of NaN counts as no score: 11 keeps its stock place among the unscored; left: [11, 12, 13] right: [13, 11, 12]
A4: red at c42180e6: assertion `left == right` failed: yesterday's scores leave the stock order; left: card 12 then card 11, right: card 11 then card 12, each entry's fields unchanged
A5: red at 71b5c388: assertion `left == right` failed: the switch on again reorders the queue by the scores; left: [11, 12, 13] right: [12, 13, 11]
A6: red at 71b5c388: the population moved no card, so it judged no reorder (examined 31104 queue and input case(s), moved 0 of 31104 case(s))
A7: red at 71b5c388: 649 of 1187 vectors differ; the first: line 3: input day 3, snapshot 7, card 13 scored "0", queue 11, 12, 13, output 13, 11, 12: the reorder gave 11, 12, 13
A8: red at b333717a: assertion `left == right` failed: the core's and the client adapters' sources that name DueScores; left: [] right: ["crates/engine-core/src/reorder.rs"]
A1: green at f6cac38f
A2: green at f6cac38f
A3: green at f6cac38f
A4: green at f6cac38f
A5: green at f6cac38f
A6: green at f6cac38f
A7: green at f6cac38f
A8: green at 0eb1c3af
```

A4 also read red at `71b5c388`, over the stock-order reorder, on its last assertion: "the same scores on their
own day and snapshot reorder the queue; left: [11, 12] right: [12, 11]". Its red at `c42180e6` is the one the
criterion exists for: the reorder built to ADR-339's input as it reads applies scores made for another day, and
the model `tla/DueReorderInput` finds the same behaviour with its witness `Keyed = FALSE`. At `c42180e6` the
vectors test read "4 of 1187 vectors differ; the first: line 1179", the vector whose input is keyed to day 2
while today is day 3.

At `f6cac38f`, with `--nocapture`: the vectors test examined 1187 vectors, 0 differing; A1 and A6 each examined
31104 cases and moved 26280; A3 examined 6 invalid scores, A4 4 stale keys, A5 5 unused inputs; A8 examined 31
core and client adapter source files and 4 planted ones.

A3 was strengthened after the green, at `34037255`: its band row S41404 (a score below 0 counted as a score)
survived the first version, because the invalid score sat on the first card of the stock order, where the
unscored order and the mutant's order agree. The invalid score now sits on a card that follows an unscored one,
and the row is killed (`scripts/mutation-rows.d/S41400-S41499.json`, `920e6ec8`). Over the stock-order reorder
the strengthened assertion reads the stock order, so it is red there for the same reason as the first version.

Each of the band's eight rows was proved by hand over the committed tree at `920e6ec8`: its one mutant
installed, its one killer run as `cargo test -p deck-streak-engine-core --test reorder -- --exact <name>`, the
file restored byte for byte. All eight read killed, each on an assertion of its killer; the killers' target
passed its six tests on the unmutated tree first.

The covers were stamped after the last edit to `crates/engine-core/src/reorder.rs` (`8de59f02`), and the
vectors were written at the stamped covers by the Lean writer (`3ae536c8`). At `3ae536c8` the formal check of
`lean/DueReorder` and of `tla/DueReorderInput` each read `FORMAL OK exit=0` with no finding, and the ratchet
against `dev` read `FORMAL OK exit=0` over 41 entries with no finding.
