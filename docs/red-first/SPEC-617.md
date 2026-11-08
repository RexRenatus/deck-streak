# Red-first record: SPEC-617

SPEC-617 (R1 to R5, A1 and A2). The SPEC, ADR-639, the schematic and the changelog fragment were
committed first. The two criteria's tests were committed next with the resolver unchanged: the
census passed the root manifest's table on, but the resolver never read it, so it found no rename
and each test read red for its own criterion by assertion. The resolver changed alone after them,
and both tests read green. Each red below is quoted from the run at its commit.

```red-first
A1: red at c60c9131e303bc3db3a9ac94c3cc1e4f0ec5c9b3: AssertionError: Items in the first set but not the second: (the inherited rename alias named as no edge, and deck-streak-old as an edge to old)
A2: red at c60c9131e303bc3db3a9ac94c3cc1e4f0ec5c9b3: AssertionError: 'progression --> xp: an arrow no manifest names' not found in []
A1: green at 6fc4083156d5095cbec7a1a6c4830fd2832e7ba9
A2: green at 6fc4083156d5095cbec7a1a6c4830fd2832e7ba9
```

The tests were committed with the resolver unchanged (c60c9131), and read red by name. The
resolver changed in 6fc40831, which edits only the text and the body of `dependency_name`: no
test, fixture or assertion changed after c60c9131. The commit after 6fc40831 changes only this
record, so the pull request run at its head reads the same tests and the same census.

An amendment (SPEC-617 section 8) grew A1's fixture to hold an inherited rename in every edge
table R1 names, and made every fixture reading pass its workspace dependencies table. The resolver
did not change, so the grown A1 is not red: it guards a reading the resolver already gives. Row
S61702 shows that it can fail: its mutant drops the plain `[build-dependencies]` table, which the
fixture before the amendment let pass, and A1 kills it in the pull request's run. Row S61703 drops
a target's `[dependencies]` table, where the clean fixture A1 checks first holds an edge, and A1
kills it. Apart from the edges the grown fixture expects, no expected result of any test changed.
