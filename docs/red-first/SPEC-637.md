# Red-first record: SPEC-637

The criterion is decided by the guard test of `scripts/tests/test_perf_benches.py`, which the
`hygiene` job of CI runs with every module of `scripts/tests`; a script run of the module filtered
to the bound criterion's test is read beside it. The first commit carries SPEC-637, ADR-653 and
the changelog fragment. The second adds the guard test at the end of the module, below the guard,
and opens this record; it leaves the guard where the development branch holds it, so the red is
read against the old place of the guard, by an assertion and not by an error, and the filtered
script run finds no test there.
