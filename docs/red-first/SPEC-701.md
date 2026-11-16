# SPEC-701: the red-first record

```red-first
A1: red at b1a7876: AssertionError: Lists differ: [] != [28]
A1: green at c83f3fb
```

The census was committed before either index stated a count, every other file kept, and its one
test read red at b1a7876 in both subtests: each failed its equality assertion with an empty list
against the held count of signed rulings.

The next commit, c83f3fb, adds the two decision records, the superseded status line and the
two count lines, with the test unchanged, and the test read green there in both subtests. The
commit after it adds only this record, the changelog fragment and the band file.

The green commit `c83f3fb` carries the two count lines, and `git diff --quiet c83f3fb HEAD` over the test, docs/README.md, AGENTS.md and docs/rulings reads rc 0.
