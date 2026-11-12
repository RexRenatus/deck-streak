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
A4: red at 9fc1b59ab6bf207d8ded1185f01ea461911d0728: AssertionError: expected [ 'no projects array' ] to deeply equal [ 'projects array never closes' ] (the old parse gives an unclosed array the missing array's refusal)
A4: green at fcc5fecbe198d2db58d24cd792c3a4ad8a6718ef
```

Amendment for #805 (SPEC-399 section 8, ADR-413 D6): A4's test was committed at 9fc1b59a with
the parse unchanged. Its first unclosed plant read the old refusal, so A4 failed by an
assertion in a local run of the whole file, and every other test of the file passed. The next
commit, fcc5fecb, changes the refusal of the unclosed arm and nothing else; like the fix commit
8b14ab8, it edits the test file and changes no assertion of any test. With that arm deleted from
a local copy at fcc5fecb and the A4 acceptance line run again, A4 failed by `AssertionError: expected [] to deeply equal [ 'projects array never closes' ]`, so the planted case
is what holds the arm.
