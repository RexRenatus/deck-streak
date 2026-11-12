# Red-first record: SPEC-673

SPEC-673 (R1 to R12, and its twelve acceptance criteria). The SPEC, ADR-696, the fragment, the
eight planted listings and
the census's tests were committed first, over stubs that keep every name and every input but the
missing behaviour: `judge` returns three empty lists, so no site, stale entry or undecided entry
is found, and `verdict` returns an empty list without counting its population. `BOUND`, `LEVEL`,
`LISTED` and `records` were already there, so each red below is its criterion's assertion and
never a missing name. The planted tests run by name; the census over the repository is read in
the `hygiene` job of CI, which runs `scripts/tests`.

```red-first
A1: red at 001c3a3: AssertionError: Tuples differ: ([], [], []) != (['WARN large-file assets/past.bin 1048577 [40 chars], [])
A1: green at 0d62e8d
A2: not red: the stub finds no site, as a file at the bound must not be one; row S67301 proves it can fail
A3: not red: the stub finds no site and no stale entry, as a listed file at its size must not be one; rows S67302, S67304 and S67305 prove it can fail
A4: red at 001c3a3: AssertionError: Tuples differ: ([], [], []) != (['WARN large-file assets/listed.bin 200000[42 chars], [])
A4: green at 0d62e8d
A5: red at 001c3a3: AssertionError: Tuples differ: ([], [], []) != (['WARN large-file assets/moved/listed.bin [123 chars], [])
A5: green at 0d62e8d
A6: red at 001c3a3: AssertionError: Tuples differ: ([], [], []) != ([], ['STALE large-file assets/listed.bin 2000000 (at or under the bound)'], [])
A6: green at 0d62e8d
A7: red at 001c3a3: AssertionError: Tuples differ: ([], [], []) != ([], [], ['UNDECIDED large-file vendor/engine (comm[34 chars]e)'])
A7: green at 0d62e8d
A8: red at 001c3a3: AssertionError: Lists differ: [] != ['STALE large-file assets/listed.bin 2000000 (no tracked file at this path)']
A8: green at 0d62e8d
A9: red at 001c3a3: AssertionError: AssertionError not raised
A9: green at 0d62e8d
A10: not red: the planted listings were committed with the tests; the test reads their names as the tree holds them
A11: not red: the level and the listing were committed with the tests; row S67312 proves it can fail
A12: not red: the stub fails nothing over the repository, as the census at warn must not; its presence check names the census's own file
```

The census's arms were committed in 0d62e8d with every test, planted listing and expected
line unchanged, and the commits after it change only the rows, this record and `LISTED`'s entries
for the sites the first CI run printed (job 130593140054).
This record's own commit, a commit added inside the second push, also writes the one line of
SPEC-673 section 1 that names that job; the merge commit that brings dev into the branch after
the green commit changes neither the module nor its planted files.

The green commit `0d62e8d` carries the census, and `git diff --quiet 0d62e8d HEAD` over the module
and its planted files reads rc 0.
