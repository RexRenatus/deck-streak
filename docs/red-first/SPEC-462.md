# Red-first record: SPEC-462

SPEC-462 (R1 to R8, A1 to A5). The first commit holds the documents and every test and changes no
production file: the SPEC, ADR-479, SPEC-407's insert-only amendment, the schematic's section 13,
and the tests of A1 to A5. A1 to A4 each read red for their own criterion, by their first assertion,
over a host that leaves a carriage return raw. The second commit is the one production change, the
host's attribute escaping, and every criterion reads green there. No commit between a red and its
green changes a test file. A1 to A3 are Vitest tests that fail by an assertion on every run; A4
enumerates the first 256 code points and reports `examined 256 of 256 code points` before it
fails. A5 is SPEC-407's A2 test with its two carriage-return plants restated as a NUL in the card
document and a NUL in the title, under its own title, with its policy plant and its kept control
unchanged: the restatement is in the first commit and removes no refusal the read-back makes, since
A4 refuses a NUL in both places too. Each red below is quoted from the run at its commit.

```red-first
A1: red at 53ffe0c883b245b9fd627afcfbd926a413d128d5: AssertionError: "<p>a\rb</p>": expected 'escaped' to be undefined (the host refused a card holding a carriage return)
A1: green at 34274161acb09aa353c28679e2f51e65b25e076e
A2: red at 53ffe0c883b245b9fd627afcfbd926a413d128d5: AssertionError: <p>a&#13;b</p>: expected 'escaped' to be undefined (the host refused a frame document holding a carriage return)
A2: green at 34274161acb09aa353c28679e2f51e65b25e076e
A3: red at 53ffe0c883b245b9fd627afcfbd926a413d128d5: AssertionError: expected 'escaped' to be null (the card frame was marked refused)
A3: green at 34274161acb09aa353c28679e2f51e65b25e076e
A4: red at 53ffe0c883b245b9fd627afcfbd926a413d128d5: AssertionError: expected [ +0, 13 ] to deeply equal [ +0 ] (the host refused U+000D as well as U+0000, after examined 256 of 256 code points)
A4: green at 34274161acb09aa353c28679e2f51e65b25e076e
A5: not red: restates SPEC-407's A2 plants as NULs, which the base already refuses; it guards the read-back
```

The rebuild onto current dev merged the original branch unchanged. Current dev had since built four
host refusal plants from a carriage return, in the card frame, frame document and review tests; the
commit after that merge plants a NUL in the frame title in each instead, which the host still
refuses with the same reason, keeps the review's former carriage-return face by a test of its own,
and has the field test also read each field through the one composition the frame renders. No
refusal is removed.

That merge (6118295d) read red by name in the pull request run 42050736979, and the restating commit
(fcb2de81) reads green in the pull request run at this delivery head. After it, a merge of current
dev (7140a485) met this change only in the schematic, where dev had appended its own section 13
first, so the schematic section this record's opening names as section 13 is now section 14; the
commit after that merge changes only documents.
