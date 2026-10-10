# Red-first record: SPEC-386

The SPEC, ADR-400, the schematic's sections 6 to 8 and ADR-356's amendment were committed first
(8a6a691), and 56af18a named the history golden in the SPEC. The census of the one seam came next,
alone (23272d2), with A9's and A10's tests red against the tree. Then the fsrs7 crate's tests over
stubs that keep every input but the behaviour (4ea1043): `RevlogRow` gains `factor`, which nothing
reads, a history's `last_id` is 0, and `stock::project` answers the raw memory state. The engine
core's `Dispatcher::replay` stub, which returns no card, came with A11 to A21's tests (bcd70ca), and
the Lean entry, its vectors and A23's test after it (3319489). 461ea72 turns the fsrs7 crate green,
3f70cf1 adds a test of R8's refusal, red, 70d25b8 re-installs the SPEC and the schematic with the
replay taking its caller's engine day, and 7fded39 turns the engine core green. Each red below is
quoted from a run whose tests and code under test equal the red commit's, at the panic site the
red commit's own lines give.

```red-first
A1: not red: SPEC-342's pin test already holds the pin, the crate's one dependency and the two scheduler packages; it guards the new edge
A2: red at 4ea1043: tests/convert.rs:162: assertion `left == right` failed: card 5 keeps the two reviews after its last reset, card 2 keeps both, card 9 none (left: every last_id 0, and card 5 holds its reviews from before the reset)
A2: green at 461ea72
A3: red at 4ea1043: tests/convert.rs:189: assertion `left == right` failed: left: [(4, 0), (6, 0)], right: [(4, 5086400000), (6, 5021600000)]
A3: green at 461ea72
A4: red at 4ea1043: tests/convert.rs:233: assertion `left == right` failed: the rows arrived as [RevlogRow { cid: 4, id: 5000000000, ease: 3, kind: 1, factor: 2500 }, RevlogRow { cid: 4, id: 5043200000, ease: 0, kind: 4, factor: 0 }, ...] (left: card 4 keeps [(3, 0.0), (4, 1.0), (3, 1.0)] with last_id 0, right: [(4, 0.0), (3, 1.0)] with last_id 5172800000)
A4: green at 461ea72
A5: not red: the pinned revision's model already computes every asserted vector, by both methods, over the stubs; a planted mutant of the replayed stability in either method's arm reads it red
A6: not red: the pinned revision's model already computes each first rating's initial stability, by both methods, over the stubs; a planted mutant of the replayed stability in either method's arm reads it red
A7: red at 4ea1043: tests/stock.rs:35: assertion `left == right` failed: Again: the stock stability is 0.1104, not the revision's interval at 0.9, 0.000038275626 (left: 1038227813, right: 941656612)
A7: green at 461ea72
A8: red at 4ea1043: tests/stock.rs:49: assertion `left == right` failed: difficulty 11 projects to 11, not 10 (left: 1093664768, right: 1092616192)
A8: green at 461ea72
A9: red at 23272d2: tests/graph.rs:540: assertion `left == right` failed: the core's sources that name the FSRS-7 crate (left: [], right: ["src/replay.rs"])
A9: green at 7fded39
A10: red at 23272d2: tests/graph.rs:453: assertion `left == right` failed: docs/CONTEXT-MAP.md's fence (left: deck-streak-engine-core depends on Some("nothing"), right: Some("fsrs7"))
A10: green at 7fded39
A11: red at bcd70ca: tests/replay.rs:290: assertion `left == right` failed: the default deck's replay holds its own card and the card whose home it is (left: {})
A11: green at 7fded39
A12: red at bcd70ca: tests/replay.rs:332: assertion `left == right` failed: only the card that still exists is replayed (left: {})
A12: green at 7fded39
A13: red at bcd70ca: tests/replay.rs:357: assertion `left == right` failed: only the card with a kept review has an entry (left: {})
A13: green at 7fded39
A14: red at bcd70ca: tests/replay.rs:387: assertion `left == right` failed: both cards are replayed (left: {})
A14: green at 7fded39
A15: red at bcd70ca: tests/replay.rs:437: assertion `left == right` failed: the card is replayed (left: {})
A15: green at 7fded39
A16: red at bcd70ca: tests/replay.rs:511: assertion `left == right` failed: the reviewed card is replayed (left: {})
A16: green at 7fded39
A17: red at bcd70ca: tests/replay.rs:689: assertion `left == right` failed: the card is replayed (left: {})
A17: green at 7fded39
A18: red at bcd70ca: tests/replay.rs:774: assertion `left == right` failed: every card is replayed (left: {})
A18: green at 7fded39
A19: red at bcd70ca: tests/replay.rs:815: assertion `left == right` failed: the card is replayed (left: {})
A19: green at 7fded39
A20: red at bcd70ca: tests/replay.rs:903: assertion `left == right` failed: the reviewed card is replayed before the answer (left: {})
A20: green at 7fded39
A21: red at bcd70ca: tests/replay.rs:935: assertion `left == right` failed: both cards are replayed (left: {})
A21: green at 7fded39
A22: not red: no client adapter names the replay today; the census guards it, and refuses its planted control by name
A23: red at 3319489: tests/formal_vectors_replay_history.rs:166: assertion `left == right` failed: the rows of {"rows":[[200,86400000,3,0,0]],"histories":[[200,86400000,[[3,0]]]]} (left: [(200, 0, [(3, 0)])], right: [(200, 86400000, [(3, 0)])])
A23: green at 461ea72
```

## What each pair disclosed

- **461ea72 edits two of SPEC-342's convert tests and the generator's golden, between the fsrs7
  reds and their greens.** A history now names its last kept review, so
  `a_cards_reviews_become_fractional_day_intervals_from_zero` and
  `manual_rescheduled_and_unrated_entries_are_dropped` expect each card's real last id through
  `ended(..)` (card 3 at T plus 60 hours, card 7 at T plus 48 hours, card 5 at T plus 24 hours)
  where they read the stub's 0. Their rows, their kept reviews and their messages are unchanged, so
  each is strengthened, not narrowed. The golden row of `tests/history.rs` logs the generator's
  factor, `FACTOR` = 2500, where 4ea1043 wrote 0: the generator writes no reset, so no generated
  row is cut.
- **3f70cf1 adds a test beyond the criteria.** `a_parameter_vector_of_another_length_is_refused_whole`
  holds R8's refusal of a vector neither empty nor 34 values long, over six lengths. It is red at
  3f70cf1, tests/replay.rs:1001: assertion `left == right` failed: the card is replayed under the
  empty vector (left: {}). It is green at 7fded39, where its assertion reads at tests/replay.rs:1040.
  It has no criterion of its own, so it adds no fence line.
- **7fded39 gives the replay its caller's engine day.** The engine's own day read unburies cards on
  a new day and writes a stamp, which R10 forbids, so `Dispatcher::replay` takes the day its caller
  read and makes no engine call (R8). In the tests, the `replayed` helper reads `engine_day()` on its
  own dispatcher before the replay; A16's two replay calls take a literal day, `UNREAD_DAY`, so A16's
  collection sees no engine call, and its fixture is unchanged; the refusal test reads one day before
  its calls. No assertion, fixture row or message changed.
- **The engine core's red lines moved.** 3f70cf1 added two import lines and 7fded39 the day's
  helpers, so each red of A11 to A21 is kept at its line at bcd70ca and re-recorded here at its line
  at 7fded39, the commit that moved it, in tests/replay.rs: A11 :290 to :321, A12 :332 to :363,
  A13 :357 to :388, A14 :387 to :418, A15 :437 to :468, A16 :511 to :549, A17 :689 to :727,
  A18 :774 to :812, A19 :815 to :853, A20 :903 to :941 and A21 :935 to :973. At 3f70cf1 each read two
  lines below its line at bcd70ca. The fsrs7 and census red lines did not move.
- **A5 and A6 have no red.** Over 4ea1043's stubs the pinned revision's model computed every value
  they assert, by both methods, so each read green there: mutation coverage, not red first. Each was
  shown able to fail. A stability scaled by 1.001 in the single method's arm of
  `crates/fsrs7/src/replay.rs` reads A5 red at tests/replay.rs:124 ("single: the four first reviews,
  Again to Easy", a stability of 0.1105104 against 0.1104) and A6 red at :148 ("single: the four first
  ratings' stabilities, Again to Easy"); the same mutant in the batch arm reads both red under
  "batch:". The file was restored by copy, its digest equal to the one before.
- **A1 and A22 have no red.** A1's pin test is SPEC-342's, and it held before this delivery; A22's
  census finds no client adapter naming the replay, and a planted control that names it is refused
  by name.
- **A10's expectation for the engine core changed** at 23272d2, from `nothing` to `fsrs7`: R7's one
  edge (ADR-400 D1). It read red until 7fded39 drew the edge in the context map.
- **56af18a and 70d25b8 change only the SPEC and the schematic**; no test.
