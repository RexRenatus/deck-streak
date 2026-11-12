# SPEC-678: the red-first record

```red-first
A1: red at d88d15e: AssertionError: Lists differ: ['S42802-THE-ENGINE-SITE-PINS-NEXTEST', 'S[188 chars]LAN'] != []
A1: green at 7c3d70a
A2: not red: the check it drives is added in the same commit; it is mutation coverage, held by the five rows of S67800-S67899
A3: not red: each of the five finds occurs once at the base too; it guards the re-anchor and every other row on the touched files
```

The first test was committed with the five rows still anchored on their pins, every other input
kept; the pull request's `hygiene` job (130902716842) read it red by its one assertion, the list of
rows whose find holds a pin or a version comment naming S42802, S42803, S59109, S61834 and S61835.

The re-anchor in 7c3d70a changes only those five rows' finds and replacements and adds the three
workflow comment lines, with every test unchanged, and the first test read green on the `hygiene`
job (130923765248). No commit between d88d15e and 7c3d70a changes a test file, and the commit after
7c3d70a changes only this record.

The green commit `7c3d70a` carries the re-anchor, and `git diff --quiet 7c3d70a HEAD -- scripts/tests/test_mutation_rows.py scripts/mutation-rows.d .github/workflows/ci.yml .github/workflows/xcframework.yml` reads rc 0.
