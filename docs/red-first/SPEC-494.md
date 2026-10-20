# Red-first record: SPEC-494

The SPEC, ADR-512 and the schematic were installed first (f728dd4). The port's tests came next,
whole, over a stub that keeps every input but the behaviour (82296c5): `check_event` refuses
nothing, `day_queue` keeps every card due on or before the day in input order and moves and
counts none, and the FSRS-7 port answers a zero identifier, appends every event unchecked behind
one planted read of the system clock, gives a constant recall, 0 whole days, zero stock values and
an empty checkpoint, and restores any bytes to a fresh state; the replay kept its direct path.
47c67c4 turns them green, with the planted read removed. Each red below is quoted from a run of
its fence line alone on the tree of 82296c5, and each printed `running 1 test`. Every new test is
RED-FIRST: none read green at 82296c5.

```red-first
A1: red at 82296c5: tests/scheduler_seam.rs:415: assertion `left == right` failed: the port's whole days and stock bits equal today's direct path for every card (left: each of the two cards (0, (0, 0)); right: the Easy card (54, (1113029186, 1065353216)) and the Again card (0, (941656612, 1086678316)))
A1: green at 47c67c4
A2: red at 82296c5: tests/scheduler_seam.rs:616: assertion `left == right` failed: the port's whole days equal today's rounding at every cell (left: 0 days at every cell)
A2: green at 47c67c4
A3: red at 82296c5: tests/scheduler_seam.rs:654: assertion `left == right` failed: the overdue sibling stays, the later two move, another note's card stays and a card due tomorrow is not queued (left: DayQueue { day: 10, cards: [1, 2, 3, 4], moved: [], unlinked: 0 }, right: DayQueue { day: 10, cards: [4, 3], moved: [1, 2], unlinked: 0 }); and the population, tests/scheduler_seam.rs:751: assertion `left == right` failed: the property check finds no queue wrong (left: 5724, right: 0)
A3: green at 47c67c4
A4: red at 82296c5: tests/scheduler_seam.rs:767: assertion `left == right` failed: every card with no note id is queued and counted, and the linked card stays (left: DayQueue { day: 5, cards: [1, 2, 3, 4], moved: [], unlinked: 0 }, right: DayQueue { day: 5, cards: [2, 1, 3, 4], moved: [], unlinked: 3 })
A4: green at 47c67c4
A5: red at 82296c5: tests/scheduler_census.rs:225: assertion `left == right` failed: the route census over the tree (left: ["crates/engine-core/src/replay.rs:252 names scheduler::replay outside the FSRS-7 crate and the port", "crates/engine-core/src/replay.rs:258 names stock::project outside the FSRS-7 crate and the port", "crates/engine-core/src/replay.rs: the replay names no .interval_days("], right: [])
A5: green at 47c67c4
A6: red at 82296c5: tests/scheduler_census.rs:304: assertion `left == right` failed: the purity census over the tree (left: ["crates/engine-core/src/replay.rs:178 reads SystemTime inside the port"], right: [])
A6: green at 47c67c4
A7: red at 82296c5: tests/scheduler_seam.rs:829: assertion `left == right` failed: ReviewEvent { card: 1, note: 10, deck: 1, preset: 1, grade: 3, answer_ms: 1000, key: EventKey(1760000000000), study_day: 20370 } is refused (left: Ok(()), right: Err(KeyNotAfter { last: EventKey(1760000000000), key: EventKey(1760000000000) }))
A7: green at 47c67c4
A8: red at 82296c5: tests/scheduler_seam.rs:902: assertion `left == right` failed: the magic, the format, the identifier and 45 bytes plus 49 per event (left: (None, None, None, 0))
A8: green at 47c67c4
A9: red at 82296c5: tests/scheduler_seam.rs:1058: assertion `left == right` failed: on the card's last review day the curve reads its top, held to the bound (left: Ok(1056964608), right: Ok(1065351168))
A9: green at 47c67c4
```

## What each pair disclosed

- **No test changed between the reds and the greens.** 47c67c4 changes only
  `crates/engine-core/src/scheduler.rs` and `crates/engine-core/src/replay.rs`; both test files
  are byte-identical at 82296c5 and 47c67c4, and so is every line of `crates/` at the pushed head,
  where the band file and these documents are the only later paths.
- **A1's cards are the run's own.** The fixtures are built at the clock's now, as the replay
  suite's are, so the card ids differ run to run; the record names each card by its first
  review's rating.
- **A3 holds two tests,** the named cases and the population of 6561 queues; both were red at
  82296c5, and both are green at 47c67c4.
- **The replay and graph suites stayed green at every step,** through the stub's direct path and
  through the port: at 82296c5 every test of the package suite passed but the ten new ones, and at
  47c67c4 every test passed.
