# Red-first record: SPEC-342

The SPEC, ADR-353, the schematic and the two scheduler pin tests were committed first (1fb8c22),
with A1's and A2's tests red against the tree. `deny.toml` came next (359f0fa), then the crate's
shape with stubs that compile and answer wrongly (f7ffd4e), then A4 to A13's tests over those
stubs (664cae5), and then the code that turns them green (6aa2c2c). Each red below is quoted from
the run at the red commit.

```red-first
A1: red at 1fb8c22: the lockfile holds 1 scheduler package(s), 1 from the registry and 0 from fsrs-rs, not one of each
A1: green at e53bb40
A2: red at 1fb8c22: allow-git is ['https://github.com/RexRenatus/anki.git', 'https://github.com/ankitects/rust-url.git'], not exactly the fork, rust-url and fsrs-rs
A2: green at 359f0fa
A3: red at 359f0fa: https://github.com/open-spaced-repetition/fsrs-rs.git: allow-git names it, and no crate in the graph comes from it
A3: green at f7ffd4e
A4: not red: proves the two upstream packages link into one binary; no code of the crate makes it pass
A5: red at 664cae5: assertion `left == right` failed: card 3 [(4, 0.0), (1, 0.0)] and card 7 [(3, 0.0), (3, 0.0), (2, 0.0)], not [(4, 0.0), (1, 2.0)] and [(3, 0.0), (2, 1.5), (3, 0.5)]
A5: green at 6aa2c2c
A6: red at 664cae5: assertion `left == right` failed: card 5 kept five rows, ease 0 among them, and card 9 one, not card 5's [(3, 0.0), (2, 1.0)] alone
A6: green at 6aa2c2c
A7: red at 664cae5: assertion `left == right` failed: left: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1], right: [1, 2, 3, 1, 2, 1]
A7: green at 6aa2c2c
A8: red at 664cae5: single: one Good review replayed to stability 0, not 3.9221
A8: green at 6aa2c2c
A9: red at 664cae5: batch: one state per card (left: 0, right: 200)
A9: green at 6aa2c2c
A10: red at 664cae5: assertion `left == right` failed: left: "", right: "fsrs7-replay target=native method=single reviews=10000 cards=1250 mean=8 runs=5 median_ms=3.000 min_ms=1.000 max_ms=5.000 per_review_us=0.3000 checksum=12345.679"
A10: green at 6aa2c2c
A11: red at 664cae5: the line "" is not the cell single reviews=10 mean=2, timed five times
A11: green at 6aa2c2c
A12: red at 664cae5: assertion `left == right` failed: left: Ok(""), right: Err(["wasm32-wasip1: cell batch reviews=1000000 mean=32 is missing"])
A12: green at 6aa2c2c
A13: red at 664cae5: assertion `left == right` failed: left: Ok(""), right: Err(["cell single reviews=10000 mean=8: checksums disagree: native 10000.000 and wasm32-wasip1 10002.000"])
A13: green at 6aa2c2c
A14: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A15: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A16: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A17: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A18: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A19: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A20: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A21: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
A22: not red: measures the engine at the workspace's pin; no DeckStreak code makes it pass
```

## What each pair disclosed

- **A1 failed once for a defect of its own resolver, between its red and its green.** At f7ffd4e
  the crate's scheduler package entered the lockfile, and A1 still failed, but not for the
  criterion's reason: the test compared a dependency's `(source)` with the package's whole source
  string, and cargo leaves the `#<commit>` fragment out of a dependency reference. e53bb40 fixed the
  resolver, and the planted spelling its negative control uses, and A1 reads green there. The red at
  1fb8c22 is the criterion's own reason, a lockfile with no package from fsrs-rs; the fix changed
  how the test reads a dependency reference, not what the lockfile must hold.
- **A2 and A3 are SPEC-055's own tests.** A2's expected `allow-git` set gained fsrs-rs at 1fb8c22
  and read red until `deny.toml` named it (359f0fa). A3 then read red because no crate of the graph
  came from the new entry, and read green once the crate depended on the pinned revision (f7ffd4e).
  Run by name at bbe8ff0, A1 to A3 read `Ran 3 tests` and `OK`.
- **A4 has no red.** It links the registry package and the pinned one into one test binary and asks
  each for its own model's answer, so it holds as soon as both packages resolve; no code of the
  crate is what makes it pass.
- **A5 to A13 were red over stubs.** f7ffd4e committed `convert.rs`, `replay.rs` and the `measure`
  module as stubs that type-check and answer wrongly (every entry kept at a zero interval, zero
  memory states and none from the batch method, one review per card, an empty line, a report that
  accepts anything), so each red at 664cae5 is an assertion on the missing behaviour, not a build
  error. 6aa2c2c replaced the stubs, and the crate's tests read 10 passed; at bbe8ff0 nextest over
  the crate reads `10 tests run: 10 passed, 0 skipped`.
- **A11 asserts the grid's behaviour before its constants.** It runs the grid over small sizes with
  a fake clock and checks each cell's line first; the assertions on `REVIEWS`, `MEANS`, `RUNS` and
  `WARMUPS` come after, so its red is a cell's line that does not name its cell, not a constant.
- **A14 to A22 are probes of the engine, not of DeckStreak code.** They build a synthetic collection
  and run the engine's own undo and its own sync server at the workspace's pin, and each measured
  the outcome its hypothesis in section 1 states; none contradicted it. A local run read
  `9 tests run: 9 passed`; CI's `rust` job is their verdict, and section 7 records it.
- **The seven hand rows pin the crate's literal constants** (S34200 to S34206: the grid's sizes,
  its two mean lengths, its five runs and one warm-up, a day's milliseconds, the checksum tolerance
  and the dropped review-log kinds). A mutation tool never mutates a literal value in a `const`
  initializer, so each needs a row. Each was proved at bbe8ff0 by `mutation_rows.py prove --band
  S34200-S34299` in a clean clone: `rows: examined 7: killed 7, survived 0, void 0`, with every
  control selecting one test that passed, and every mutant selecting one test that failed.
- **A13's test gained one assertion after green, as mutation coverage, not red first.** The pull
  request's `mutation-verdict` read one missed mutant at 9afb08b: in `web_target`, the match guard
  `!crates.is_empty()` replaced with `true`, which would record a failing web check that names no
  crate as `fail ()` instead of refusing it. 2c3ea4d asserts that `fail crates=` is refused by
  name. In a git-archive export of 2c3ea4d, the test read `1 passed` without the mutant and
  `1 failed` with it, on that assertion. It was green when written, so it adds no fence line.
