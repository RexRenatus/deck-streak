# Red-first record: SPEC-494

The SPEC, ADR-512 and the schematic were installed first (f728dd4). The port's tests came next,
whole, over a stub that keeps every input but the behaviour (82296c5): `check_event` refuses
nothing, `day_queue` keeps every card due on or before the day in input order and moves and
counts none, and the FSRS-7 port answers a zero identifier, appends every event unchecked behind
one planted read of the system clock, gives a constant recall, 0 whole days, zero stock values and
an empty checkpoint, and restores any bytes to a fresh state; the replay kept its direct path.
47c67c4 turns them green, with the planted read removed, and 23fc80b, after the in-diff mutants,
makes the replay's selection check one comparison, makes the censuses refuse a missing source
tree, and adds two tests. Each red below is quoted from a run of its fence line alone on the tree
of 82296c5, each green from a run of it alone on the tree of 23fc80b, and each printed
`running 1 test`. The ten fence tests are RED-FIRST: none read green at 82296c5. The two tests
23fc80b adds are MUTATION COVERAGE, written after the code they cover.

```red-first
A1: red at 82296c5: tests/scheduler_seam.rs:415: assertion `left == right` failed: the port's whole days and stock bits equal today's direct path for every card (left: each of the two cards (0, (0, 0)); right: the Easy card (54, (1113029186, 1065353216)) and the Again card (0, (941656612, 1086678316)))
A1: green at 23fc80b
A2: red at 82296c5: tests/scheduler_seam.rs:616: assertion `left == right` failed: the port's whole days equal today's rounding at every cell (left: 0 days at every cell)
A2: green at 23fc80b
A3: red at 82296c5: tests/scheduler_seam.rs:654: assertion `left == right` failed: the overdue sibling stays, the later two move, another note's card stays and a card due tomorrow is not queued (left: DayQueue { day: 10, cards: [1, 2, 3, 4], moved: [], unlinked: 0 }, right: DayQueue { day: 10, cards: [4, 3], moved: [1, 2], unlinked: 0 }); and the population, tests/scheduler_seam.rs:751: assertion `left == right` failed: the property check finds no queue wrong (left: 5724, right: 0)
A3: green at 23fc80b
A4: red at 82296c5: tests/scheduler_seam.rs:767: assertion `left == right` failed: every card with no note id is queued and counted, and the linked card stays (left: DayQueue { day: 5, cards: [1, 2, 3, 4], moved: [], unlinked: 0 }, right: DayQueue { day: 5, cards: [2, 1, 3, 4], moved: [], unlinked: 3 })
A4: green at 23fc80b
A5: red at 82296c5: tests/scheduler_census.rs:225: assertion `left == right` failed: the route census over the tree (left: ["crates/engine-core/src/replay.rs:252 names scheduler::replay outside the FSRS-7 crate and the port", "crates/engine-core/src/replay.rs:258 names stock::project outside the FSRS-7 crate and the port", "crates/engine-core/src/replay.rs: the replay names no .interval_days("], right: [])
A5: green at 23fc80b
A6: red at 82296c5: tests/scheduler_census.rs:304: assertion `left == right` failed: the purity census over the tree (left: ["crates/engine-core/src/replay.rs:178 reads SystemTime inside the port"], right: [])
A6: green at 23fc80b
A7: red at 82296c5: tests/scheduler_seam.rs:829: assertion `left == right` failed: ReviewEvent { card: 1, note: 10, deck: 1, preset: 1, grade: 3, answer_ms: 1000, key: EventKey(1760000000000), study_day: 20370 } is refused (left: Ok(()), right: Err(KeyNotAfter { last: EventKey(1760000000000), key: EventKey(1760000000000) }))
A7: green at 23fc80b
A8: red at 82296c5: tests/scheduler_seam.rs:902: assertion `left == right` failed: the magic, the format, the identifier and 45 bytes plus 49 per event (left: (None, None, None, 0))
A8: green at 23fc80b
A9: red at 82296c5: tests/scheduler_seam.rs:1058: assertion `left == right` failed: on the card's last review day the curve reads its top, held to the bound (left: Ok(1056964608), right: Ok(1065351168))
A9: green at 23fc80b
```

## What each pair disclosed

- **47c67c4 changes no test.** It changes only `crates/engine-core/src/scheduler.rs` and
  `crates/engine-core/src/replay.rs`, and each fence line read green alone on its tree.
- **23fc80b edits both test files, between the reds and the greens, and the producer.** The
  in-diff mutants at 9d23509 read 7 missed of 93. Two of the replay's selection check (`&&` to
  `||`) could never be killed, since the producer's selection always agrees with the crate's, so
  the check is one comparison of the last id and the ratings, whose mutant every replay test
  kills. The other five are killed by the two new tests:
  `an_empty_state_round_trips_and_a_refused_event_is_its_load_errors_source` restores an empty
  state's checkpoint (the header bound's `<` to `==` and to `<=`), refuses a checkpoint whose
  event the fold refuses, and asserts each load error's source (the source's two mutants); and
  `rows_the_selection_drops_after_the_last_review_leave_the_replay_on_the_direct_path` plants a
  row with no rating and a rated reschedule after a card's review (the kept filter's `&&` to
  `||`). At 23fc80b the in-diff mutants read 0 missed of 88. Neither new test has a criterion of
  its own, so neither adds a fence line. In
  `crates/engine-core/tests/scheduler_census.rs`, the walk no longer skips a census tree that is
  not a directory, so a missing tree fails the walk, and each planted root holds every census
  tree, empty where nothing is planted; no assertion, plant or expected refusal changed, and A5
  and A6 examine the same 60 files. The A1 to A4 and A7 to A9 tests are byte-identical at
  82296c5 and 23fc80b, the two new tests appended after them.
- **Every line of `crates/` at the pushed head equals 23fc80b's;** the later commit changes only
  this record.
- **A1's cards are the run's own.** The fixtures are built at the clock's now, as the replay
  suite's are, so the card ids differ run to run; the record names each card by its first
  review's rating.
- **A3 holds two tests,** the named cases and the population of 6561 queues; both were red at
  82296c5, and both are green at 23fc80b.
- **The replay and graph suites stayed green at every step,** through the stub's direct path and
  through the port: at 82296c5 every test of the package suite passed but the ten new ones, and at
  47c67c4 and at 23fc80b every test passed.
