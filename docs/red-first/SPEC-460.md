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
