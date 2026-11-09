# Red-first record: SPEC-477

This record is part A's, the first of SPEC-477's two pull requests. It records A1, the one
criterion this pull request delivers; part B appends A2 to A4 when it moves them into the
acceptance fence (SPEC-477 section 3c).

The order of work: the SPEC, ADR-494, the ADR-303 amendment and the schematic's section 5
(45cf683a); the six rows' mutants run against the tree before the new test existed (P1, below);
the window-edge members and A1's test, with this record; the rows S47700 to S47705, each proved by
hand, and the five planted copies of `hours.rs`; the cover on the award port in the award-once
model, with its re-read; and the changelog fragment.

```red-first
A1: not red: pins the hour windows' edges the base already holds; its reds are hand rows S47700 to S47705 and five planted copies of hours.rs, each killed by A1
```

## MUTATION COVERAGE

`every_window_edge_day_awards_exactly_the_badges_its_local_hours_earn_once` in
`crates/coordination/tests/badges_steps.rs` is MUTATION COVERAGE: it is green at the base
`0756b82ffb59e06bd515dbfe6c4a74e3372a3136` with the test added, it is not red-first evidence, and
this pull request changes no production line. Its reds are the six rows on
`crates/coordination/src/progression/badge_context.rs` and five planted copies of
`crates/progression/src/badges/hours.rs`: each is installed by script, read red by A1's
awarded-set assertion, and restored byte for byte, its sha256 compared, before the next is built.
The class rule's own test, `every_evaluated_day_awards_exactly_the_badges_its_context_earns_once`,
still reads `examined 368 evaluated day(s)`.

P1, the gap on the tree before the test: at 45cf683a, with no window-edge test in the tree, each
row's mutant was installed alone and `cargo test -p deck-streak-coordination --test badges_steps
--test badges_context --no-fail-fast` was run, then the file was restored (sha256
`e88397876377bd6972816c346a4152a6601a7c9008144eab8722882bd1575eda` each time):

- S47700 (the night window from hour 1): `badges_context` 1 passed, 0 failed; `badges_steps` 13
  passed, 0 failed. No test at the base sees it.
- S47701 (the night window's end included): `badges_context` 0 passed, 1 failed, by
  `the_badge_context_matches_the_parity_golden` (`early: night owl`, left 20, right 0);
  `badges_steps` 13 passed, 0 failed.
- S47702 (the early window from the rollover plus one): `badges_context` 0 passed, 1 failed, by
  `the_badge_context_matches_the_parity_golden` (`early: early bird`, left 40, right 60);
  `badges_steps` 13 passed, 0 failed.
- S47703 (the early window's end included): `badges_context` 0 passed, 1 failed, by
  `the_badge_context_matches_the_parity_golden` (`late-rollover: early bird`, left 13, right 8);
  `badges_steps` 13 passed, 0 failed.
- S47704 (the night window reads the default rule): `badges_context` 0 passed, 1 failed, by
  `the_badge_context_matches_the_parity_golden` (`late-rollover: night owl`, left 13, right 0);
  `badges_steps` 13 passed, 0 failed.
- S47705 (the early window reads the default rule): `badges_context` 0 passed, 1 failed, by
  `the_badge_context_matches_the_parity_golden` (`gaps: early bird`, left 2, right 1);
  `badges_steps` 13 passed, 0 failed.

The class rule's test kills none of the six. The parity golden of the badge context already kills
S47701 to S47705; each stays a row with A1 as its killer, since A1 is the test that holds the
window's edges under three study-day rules.
