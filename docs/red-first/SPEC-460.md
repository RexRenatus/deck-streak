# Red-first record: SPEC-460

SPEC-460 (R1 to R9, A1 and A2). SPEC-460, ADR-477 and the insert-only amendments of ADR-376 and of
the answer's schematic were committed first and alone (f68d5c57), and the two tests of A1 and A2
were committed with this record in the commit after it. Both tests are MUTATION COVERAGE, not red
first: the refusal they pin is the engine's, a third-party dependency, and it is already on `dev`,
so no edit of the base could make either test red for its criterion's reason without changing
what is pinned (ADR-477 D3). Each criterion's red is a hand-written mutation row of the core's
answer door instead, which its test kills, and each row's proof is recorded below. The fence runs
from the worktree root, as SPEC-460 section 3 writes it.

```text
A1: cargo test -p deck-streak-engine-core --test stale_answer -- --exact a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine
A2: cargo test -p deck-streak-engine-core --test stale_answer -- --exact a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine
```

```red-first
A1: not red: pins the base's stale-state refusal; its red is hand row S46001, killed by A1
A2: not red: pins the recorded refusal; its red is hand row S46002, killed by A2
```

## Each fence line, green

Each line selects its one test and passes on the tree this record is committed with:

- A1: `running 1 test`, then
  `test a_second_press_on_the_states_shown_before_the_first_is_refused_by_the_engine ... ok` and
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out`.
- A2: `running 1 test`, then
  `test a_press_on_the_states_shown_before_a_due_date_was_set_is_refused_by_the_engine ... ok` and
  `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out`.

## The gap, measured before the tests

At f68d5c57, with no `crates/engine-core/tests/stale_answer.rs` in the tree, each row's mutant was
installed alone in `crates/engine-core/src/dispatch.rs` by a script that refuses unless the row's
find occurs once, and the whole engine-core package suite ran under it
(`cargo test -p deck-streak-engine-core --no-fail-fast`, the crate recompiled under the mutant):

| tree at f68d5c57 | `test result:` lines | passed | failed | ignored |
|---|---|---|---|---|
| unmutated | 24 | 152 | 0 | 0 |
| row S46001's mutant | 24 | 152 | 0 | 0 |
| row S46002's mutant | 24 | 152 | 0 | 0 |

No test on `dev` failed under either mutant: no test there held the refusal, the gap #760 names.
After each run the file was restored by writing its saved bytes back, and its sha256 read
`d67f083ea6d0274697d3145423a1dc6a3dc877d7c1bbac216e3e544bf3b63198` before the mutant and after the
restore.

## Each row's proof, by hand

At e7f5905b, the commit that adds the tests, each row's mutant was installed once in
`crates/engine-core/src/dispatch.rs` from the band file `scripts/mutation-rows.d/S46000-S46099.json`
by a script that refuses unless the row's find occurs once. Both tests then ran under it, each by
its fence line's `--exact` selection (`running 1 test`, the crate recompiled under the mutant).
The file was then restored by writing its saved bytes back, its sha256 read
`d67f083ea6d0274697d3145423a1dc6a3dc877d7c1bbac216e3e544bf3b63198` again, it was touched, and the
target was rebuilt and run on the restored tree, where both tests passed
(`test result: ok. 2 passed; 0 failed`), before the next measurement.

`S46001-THE-CORE-SENDS-THE-SHOWN-STATE`, the core reading the card's states from the engine at
answer time (`SchedulerService.GetSchedulingStates`, 13 and 23) and sending the engine's current
state in place of the shown one:
- A1, its killer: `FAILED`, at its one `assert_eq!`, `` assertion `left == right` failed ``. The
  stale press's outcome is `Ok(())` where the pin holds
  `Err(Some((InvalidInput, "card was modified")))`: the press passed the engine's comparison and
  was recorded, so the card's repetitions read 2 after it where the first answer left 1. The
  index 23 is so measured: under it the stale press is no longer refused as stale.
- A2, the other test: `FAILED`, at its one `assert_eq!`. The stale press's outcome is `Ok(())`,
  and the card was answered from the states shown before the due date, so its snapshot no longer
  reads as the gesture left it.

`S46002-THE-ENGINES-REFUSAL-REACHES-THE-PRESS`, the engine's refusal read as an empty reply:
- A2, its killer: `FAILED`, at its one `assert_eq!`. The stale press's outcome is `Ok(())` where
  the pin holds `Err(Some((InvalidInput, "card was modified")))`, while the card's snapshot and the
  newest review read as the gesture left them: the engine refused and wrote nothing, and the core
  reported the refusal as a recorded answer.
- A1, the other test: `FAILED`, at its one `assert_eq!`, the same way: the stale press's outcome is
  `Ok(())`, and every other part of the tuple equals its pin.

Neither mutant failed to compile, and neither build printed a warning. CI's rows job decides both
rows by id after the push.
