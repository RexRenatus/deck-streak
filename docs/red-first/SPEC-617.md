# Red-first record: SPEC-617

SPEC-617 (R1 to R5, A1 and A2). The SPEC, ADR-639, the schematic and the changelog fragment were
committed first. The two criteria's tests were committed next with the resolver unchanged: the
census takes the root manifest's table but finds no rename in it, so each test read red for its
own criterion by assertion. The resolver changed alone after them, and both tests read green. Each
red below is quoted from the run at its commit.

```red-first
A1: red at c60c9131e303bc3db3a9ac94c3cc1e4f0ec5c9b3: AssertionError: Items in the first set but not the second: (the inherited rename alias named as no edge, and deck-streak-old as an edge to old)
A2: red at c60c9131e303bc3db3a9ac94c3cc1e4f0ec5c9b3: AssertionError: 'progression --> xp: an arrow no manifest names' not found in []
```
