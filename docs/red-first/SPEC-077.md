# Red-first record: SPEC-077

Each criterion's test was committed before the rule it tests, red on a compiling stub, in four
steps: the curriculum goldens of A1 to A6 and A11 at a26ed3ad, made green by 58c946eb; A8 and A10,
with migration 007701 and the store's stubs, at 9a3da777, made green by 782a2c1a; A7 at f015827a,
made green by 683d1a89; and the law block's A12 to A15 and the milestone's A21 at 6fd5308f, made
green by 4b713129. For this record every line was replayed from an export of its commit's tree,
running the criterion's own fence command. Each red line quotes where the replay's run panicked
and what it printed; each green line's run printed `1 passed`.

```red-first
A1: red at a26ed3ad: panicked at crates/ingest/tests/progress_memory_state.rs:49:23: the state of Some("{\"s\": 4.5, \"d\": NaN, \"decay\": Infinity, \"lrt\": -Infinity}"): ours None, theirs Some(MemoryState { stability: 4.5, difficulty: 0.0, decay: 0.2, desired_retention: None, last_review_sec: None })
A1: green at 58c946eb
A2: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:77:9: the mastery of {"card":{"ctype":2,"ivl":21,"queue":-1},"mature_ivl":21,"now_sec":1700000000}: ours -1, theirs 0
A2: green at 58c946eb
A3: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:91:9: assertion `left == right` failed: the unit of "Alpha"; left: Some(4294967295), right: None
A3: green at 58c946eb
A4: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:156:9: assertion `left == right` failed: the courses of the first case; left: 0, right: 1
A4: green at 58c946eb
A5: red at a26ed3ad: panicked at crates/curriculum/tests/progress_goldens.rs:229:9: constants.XP_BONUS_BAND_UP: ours 0, theirs 500
A5: green at 58c946eb
A6: red at a26ed3ad: panicked at crates/curriculum/tests/progress_unit_bands.rs:41:5: assertion `left == right` failed; left: Some("C2"), right: Some("A1")
A6: green at 58c946eb
A7: red at f015827a: panicked at crates/coordination/tests/progress_band_up.rs:429:9: assertion `left == right` failed: case 3 (band_up): each band-up is offered to the router; left: [], right: [("band_up", "bandup:al:a2")]
A7: green at 683d1a89
A8: red at 9a3da777: panicked at crates/coordination/tests/progress_band_up.rs:101:5: assertion `left == right` failed: the first sighting records the current band as a silent baseline, owing nothing; left: [], right: [("be", "B1", 20000, 1, true)]
A8: green at 782a2c1a
A10: red at 9a3da777: panicked at crates/curriculum/tests/progress_store.rs:82:5: assertion `left == right` failed: the export carries both courses' progress; left: [], right: [["al", "A2"], ["be", "A1"]]
A10: green at 782a2c1a
A11: red at a26ed3ad: panicked at crates/curriculum/tests/law_goldens.rs:19:9: the pillar at 0: ours -1, theirs 100
A11: green at 58c946eb
A12: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:175:9: assertion `left == right` failed: case 0: the level of the lifetime law XP; left: Some(0), right: Some(1)
A12: green at 4b713129
A13: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:239:5: assertion `left == right` failed: no law streak, law XP or leech: the block is omitted, dues and language XP or not; left: Some(LawLines { lines: [], level_shown: false }), right: None
A13: green at 4b713129
A14: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:281:5: assertion `left == right` failed: before the first recompute the dues are pending; left: Some(0), right: None
A14: green at 4b713129
A15: red at 6fd5308f: panicked at crates/coordination/tests/law_block.rs:310:5: assertion `left == right` failed: the leeches are pending, never 0; left: Some(0), right: None
A15: green at 4b713129
A21: red at 6fd5308f: panicked at crates/coordination/tests/progress_milestone.rs:67:5: assertion `left == right` failed: the configured courses' stored mature cards, summed; an unconfigured course is not; left: None, right: Some(115)
A21: green at 4b713129
```

## Disclosures

- **How A1, A2, A5 and A11 fail.** Their tests compare each case with its golden and panic with
  the case, ours and theirs on the first that differs, rather than through `assert_eq!`. A1 fails
  at its mismatch arm (`(a, b) => panic!(...)` at progress_memory_state.rs:49): the stub parses
  no memory state where the golden has one. A2 fails at its `assert!` that the two masteries are
  near, A5 at its `assert!` that the constant is the same, and A11 at its `assert!` that the two
  pillars agree. Each red is the behaviour's mismatch on a compiling stub, quoted as the run
  printed it.
- **A4's quoted case is shortened.** The run prints the whole first case, its cards and its
  courses, before `left: 0, right: 1`; the line above names it "the first case".
- **A changed criterion, A3 and A4 (the seat's ruling 12).** 782a2c1a changed what A3 and A4
  decide: a unit beyond 32 bits reads as no unit, as the predecessor counts it, pinned by one new
  golden case, and changed `parse_unit` to match in the same commit. Its red is recorded here and
  not as a second fence line. Replayed on 782a2c1a's tree with 9a3da777's `parse_unit` body put
  back, both tests fail, each with exit 101. A3 panicked at progress_goldens.rs:96:5,
  `assertion left == right failed: a unit beyond u32 is no unit`, left `Some(0)`, right `None`.
  A4 panicked at progress_goldens.rs:168:13, the comparison of a course's total cards, left `3`,
  right `1`: the case's card with a unit beyond 32 bits was counted. On 782a2c1a's own tree both
  pass. 734a85a0 edits only `crates/coordination/tests/relight_order.rs`'s census of statics and
  changes no criterion.
- **The mutation-coverage tests are not criteria.**
  `progress_band_up::the_progress_step_runs_for_the_current_day_only` and
  `progress_store::a_band_up_is_marked_once` were green when committed; each pins a rule that
  already held, and each is the killer of a row the SPEC's T18 names (S07715 and S07722), proved
  by its mutant and not recorded here as red-first evidence.
