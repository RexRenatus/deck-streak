# Red-first record: SPEC-327

The owner's signed ruling was merged first (332b1df6), and the SPEC, ADR-328 and the schematic
were committed on it (95dbc69b). The red then came in two commits, so that each criterion fails by
assertion and none by a missing name:

- 002818a0 adds A1 and A3 to A6 alone. `BOUNDS` still reads `--timeout 300 --build-timeout 600`
  and the sizer is the base's. A1 fails on its first assertion, the budget against the census's
  need of 788 s with its 1.5 margin; its second, the sizer's census term, sits after it, since at
  the base it could only fail by a missing name. A3 to A6 fail on exact projections computed from
  literals.
- ccbb9aa4 moves `BOUNDS` and the byte pins of every `cargo mutants` command to `--timeout 1200`,
  with each fixture that stands for the bounds spelled from `BOUNDS`, while the workflows still
  carry the base's `--timeout 300`. A2 and A7 are the existing guards, so they fail against the
  base's workflows by assertion.

```red-first
A1: red at 002818a0: AssertionError: 300 not greater than or equal to 1182 : --timeout 300 --build-timeout 600
A2: red at ccbb9aa4: AssertionError: 1 != 6 : each package branch is a command; the finder reads no command with the bounds and refuses each workflow, `a word bash computes before the bounds: "$shard"`
A3: red at 002818a0: AssertionError: 316 != 1892, the serial seconds of two progression mutants and one coordination mutant without the census term
A4: red at 002818a0: AssertionError: 371 != 1159, the plan's baseline_seconds without the census term
A5: red at 002818a0: AssertionError: 3 != 23, the shard count of #592's listing without the census term
A6: red at 002818a0: AssertionError: 'mutation: size: 2 shard(s) for 3 listed mutant(s), projected at 2742 s serially, the slowest at 2987 s of its 3600 s bound' not found in 'mutation: size: 1 shard(s) for 3 listed mutant(s), projected at 378 s serially, the slowest at 749 s of its 3600 s bound'
A7: red at ccbb9aa4: 10 of 27 tests fail, each `AssertionError: 0 != 1`: no pinned command at --timeout 1200 is in the base's workflows
A1: green at ce165b6b
A2: green at ce165b6b
A3: green at ce165b6b
A4: green at ce165b6b
A5: green at ce165b6b
A6: green at ce165b6b
A7: green at ce165b6b
```

A8 is the post-merge CI reading, SPEC-327 s7; it is not a red-first criterion.

## What changed between red and green

ce165b6b changes the two workflows and the sizer only: `--timeout 300` becomes `--timeout 1200` on
the nine `cargo mutants` commands (`.github/workflows/ci.yml` three, `.github/workflows/mutation-weekly.yml`
six), and `scripts/mutation-verdict.py` gains `CENSUS_SECONDS`, the term in `mutant_costs`,
`baseline_seconds`, and the baseline passed to `projected` and `fewest_shards` by `shards` and
`size`. No test changed between the red commits and the green.

Each new test was also replayed on a clean `git archive` export of 002818a0, the base's code with
the new tests, and read the same five failures. The whole of `scripts/tests/test_mutation_verdict.py`
at 002818a0 reads `Ran 34 tests`, `FAILED (failures=4)`: A3 to A6 and no other test.

ccbb9aa4 also turns one existing guard red beside A2 and A7:
`test_mutation_workflows.EveryRunIsBounded.test_every_cargo_mutants_command_bounds_its_builds_and_its_tests`,
by assertion and for A2's cause, since the finder at the new bounds refuses the base's `ci.yml`
(`'--timeout \d+' not found in 'refused: a word bash computes before the bounds: "$out"'`). It is
green at ce165b6b. It decides no criterion, so it has no line in the fence.

A3 to A6 were read green at ce165b6b by their fence commands, each `Ran 1 test`, `OK`, and the
whole of `test_mutation_verdict.py` there reads `Ran 34 tests`, `OK`. A1, A2 and A7 were read
green on the implementation's tree before its commit, whose content is ce165b6b's, and CI's
`hygiene` job runs both of their modules at the pushed head.
