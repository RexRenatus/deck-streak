# Red-first record: SPEC-662

SPEC-662 (R1 to R10, A1 to A9). The tests of A1 to A9 in `scripts/tests/test_ci_job_budget.py`
and the planted trees under `scripts/tests/fixtures/job-budget/` were committed first, over stubs
that keep every input but the missing behaviour: `started` starts every counted event the workflow
names, `combinations` counts every job once and names no source, `run_count` reads no call,
`verdict` prints the RUN lines and no other, and `runs` still examines every file. `BUDGET`,
`UNDECIDED` and `LEVEL` were already there. Push 1 carried that red commit alone, and its
`hygiene` job read each red below by its FAIL header, by test name.

```red-first
A1: red at d0483f6: AssertionError: Tuples differ: (1, []) != (6, [])
A1: green at 0e16148
A2: red at d0483f6: AssertionError: Tuples differ: (2, []) != (7, [])
A2: green at 0e16148
A3: red at d0483f6: AssertionError: Tuples differ: (2, []) != (4, [])
A3: green at 0e16148
A4: red at d0483f6: AssertionError: Tuples differ: (3, []) != (3, [('computed.yml', 'legs', 'leg: ${{ fromJS[122 chars]1')])
A4: green at 0e16148
A5: red at d0483f6: AssertionError: Lists differ: [('ca[51 chars]), ('elsewhere.yml', 'pull_request'), ('elsewh[252 chars]sh')] != [('ca[51 chars]), ('ignored.yml', 'pull_request'), ('matrix.y[165 chars]sh')]
A5: green at 0e16148
A6: red at d0483f6: AssertionError: 'WARN job-budget past.yml pull_request: 4 job(s), past 3' not found in []
A6: green at 0e16148
A7: red at d0483f6: AssertionError: Lists differ: [] != ['WARN job-budget caller.yml push: 7 job(s[572 chars]egs']
A7: green at 0e16148
A8: not red: the empty population is refused by the loader the stub keeps; it guards the refusal
A9: not red: it reads every workflow file, which only CI reads; its arms are red over the planted trees above
```

Push 2 carried the green commit, the rows and this record, and its `hygiene` job read each
criterion green by test name. `git diff --quiet 0e16148 HEAD --
scripts/tests/test_ci_job_budget.py scripts/tests/fixtures/job-budget` exits 0. The repository
criterion's line is written from push 1's reading: the not-red line above if its test passed
there, else a red line quoting the failure observed. Each red line names the red commit
`d0483f6` and quotes the failure observed there, and each green line names the green commit
`0e16148`.
