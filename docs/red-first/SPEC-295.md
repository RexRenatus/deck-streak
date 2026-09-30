# Red-first record: SPEC-295

Recorded 2026-09-30. The three tests of A1 to A3 were committed alone (29df06f), with
`config/formal.json` absent, so each failed by assertion: the tests' loader raises an
`AssertionError` that names the missing file, never a file-not-found error. The file
(4b0a6f1) turned them green. The presence control of A3 is red at the first commit for the same
reason: it reads the committed file and there is none.

```red-first
A1: red at 29df06f: AssertionError: config/formal.json is absent: the formal checker reads none
A1: green at 4b0a6f1
A2: red at 29df06f: AssertionError: config/formal.json is absent: the formal checker reads none
A2: green at 4b0a6f1
A3: red at 29df06f: AssertionError: config/formal.json is absent: the formal checker reads none
A3: green at 4b0a6f1
```

The green run reads `Ran 3 tests ... OK` and prints `examined 9 declared fields`,
`examined 30 planted faults` and `examined 4 bad paths`.

The body of A3's test changed after its red commit. At b9da495 it gained a fault for every refusal
arm of the reader, and at 2af486b a value of every JSON type each kind does not admit, so it now
prints `examined 101 planted faults` and `examined 8 refusal arms of the reader`. The red line of A3
at 29df06f stands as the earlier body's failure, the loader's `AssertionError` that names the
missing file. Each later body reads the same file through the same loader first, so at 29df06f it
fails at the same line: the current body, run with `config/formal.json` absent, reads
`FAILED (failures=3)`, each at that `AssertionError`.
