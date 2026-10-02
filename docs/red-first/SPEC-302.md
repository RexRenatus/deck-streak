# Red-first record: SPEC-302

The SPEC, its schematic and the two pointer edits in SPEC-090 and SPEC-095 were committed first.
The goldens and their registry followed, then the four tests against stubs of `pynum` that compile
and return a wrong value, so each test fails by assertion. The implementation turns them green.

```red-first
A1: red at d3c75b7: assertion `left == right` failed: sum of {"op":"sum","values":[1e+16,1.0,-1e+16]}: got 0e0, CPython 1e0
A1: green at 1fb30c4
A2: red at d3c75b7: assertion `left == right` failed: the percentile 0.9 of [5]: left Some(0), right Some(5)
A2: green at 1fb30c4
A3: red at d3c75b7: assertion `left == right` failed: draw 0 of seed 0: got 0e0, CPython 8.444218515250481e-1
A3: green at 1fb30c4
A4: red at d3c75b7: panicked: lgamma of 1 is a number (the stub returns None)
A4: green at 1fb30c4
```

The red run reads `test result: FAILED. 1 passed; 3 failed` for each of the two test files; the
passing test of each is the one that holds the empty and the undefined cases, which the stubs
happen to return. The green run reads `test result: ok. 4 passed` for each. The two rows of §7
were proved by the mutation-row verb after the green commit, each killed by its test.
