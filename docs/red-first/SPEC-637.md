# Red-first record: SPEC-637

The criterion is decided by the guard test of `scripts/tests/test_perf_benches.py`, which the
`hygiene` job of CI runs with every module of `scripts/tests`; a script run of the module filtered
to the bound criterion's test is read beside it. The first commit carries SPEC-637, ADR-653 and
the changelog fragment. The second adds the guard test at the end of the module, below the guard,
and opens this record; it leaves the guard where the development branch holds it, so the red is
read against the old place of the guard, by an assertion and not by an error, and the filtered
script run finds no test there.

```red-first
A1: red at d3702d72: AssertionError: Lists differ: ['EachWebBenchPassesItsOwnBound', 'TheScriptRunReachesEveryClass'] != []
A1: green at 6e914ca4
```

The third commit (6e914ca4) moves the guard's two lines from above the bound table to the end of the
module and adds the mutation row; it changes the test module, but no assertion, test or name in
it, so it is disclosed here. At the second commit the script run filtered to the bound criterion's
test printed `NO TESTS RAN` and exited with status 5; at 6e914ca4 it names that test, reads `ok` and
exits 0. The green line names 6e914ca4, the last commit that changes `scripts/`. The commits after it
change only documents, and the pull request's run at this delivery's head reads A1 green.
