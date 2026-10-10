# Red-first record: SPEC-399

SPEC-399 (R1 to R7, A1 to A3). The SPEC, ADR-413, the schematic's new last section and the three
tests of A1 to A3 were committed first, over helpers that hold a planted copy of the parse they
replace: the engines kept by hand, the cells that hold `UNOBSERVABLE:` alone, and a one-way
difference that computes the missing side only. The six existing tests of
`web/app/src/lib/card/planted-coverage.test.ts` passed at that commit, and each of the three new
tests failed by an assertion, for its own criterion's reason. Each red below is read from a local run
of the whole file at that commit, so no red needed a CI run to be seen. The next commit replaces the
three helpers' bodies and nothing else, and all nine tests of the file read green. No shipped file
was edited to make a red, and no assertion of the six existing tests was removed or loosened.

Disclosure for the fix commit 8b14ab8: the three helpers live in the test file, so the fix edits
`web/app/src/lib/card/planted-coverage.test.ts` to replace the planted copy's bodies with the real
ones. It changes no assertion of any test; the six existing tests and the three new ones are the
same text at both commits.

```red-first
A1: red at 7e0d8707f14857611e02b4e1a29694695f7260c0: AssertionError: expected [ 'chromium dns-prefetch', ...(5) ] to deeply equal [ 'chromium dns-prefetch', ...(12) ] (the old parse names six pairs: the seven cells' named-engine pairs are never read)
A1: green at 8b14ab83ccd06c30da5ca4335f172695414b519e
A2: red at 7e0d8707f14857611e02b4e1a29694695f7260c0: AssertionError: expected { missing: [], extra: [] } to deeply equal { Object (missing, extra) } (plant 1: the old parse never reads the one-engine cell, so the pair missing from the planted table goes unseen)
A2: green at 8b14ab83ccd06c30da5ca4335f172695414b519e
A3: red at 7e0d8707f14857611e02b4e1a29694695f7260c0: AssertionError: expected [ 'chromium', 'webkit', 'firefox' ] to deeply equal [ 'chromium', 'third', 'webkit' ] (the engines are a list kept by hand, so an altered configuration's projects do not follow)
A3: green at 8b14ab83ccd06c30da5ca4335f172695414b519e
```
