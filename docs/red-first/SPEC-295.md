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

## Amendment addendum, 2026-09-30 (issue #488)

The test of A4 was committed at 654f96f690037bfdeec1daafba8f6d80eee6634a against the development branch's reader, which already
refuses every planted value, so A4 and A5 were green at their first commit and neither has a red
line. What makes them evidence is the mutant table: of 17 mutants of the reader's integer and map
arms, nine survived the development branch's tests (an integral float admitted as an integer or as a
map value, a null, a float, a string, an array and an object admitted as map values, and a map judged
by its first value alone and by its last alone), and every one of the nine is red under the new
population. The other eight were red before and stay red. The nine rows S29500 to S29508 are each
killed on a clean committed detached head.

```red-first
A4: not red: the reader already refused every planted value; nine mutants of it that the earlier population let survive are each red under this test, and the rows S29500 to S29508 prove it
A5: not red: the reader already refused every planted value; the rows S29500 to S29508 prove the population kills each mutant
```

The green run reads `Ran 5 tests ... OK` and prints `examined 144 planted value types`,
`examined 21 planted values of the kind object at 3 fields`, `examined 7 planted values of the kind
path at 1 fields`, `examined 54 planted values of the kind posint at 6 fields`, `examined 34 planted
values of the kind posint-map at 1 fields`, `examined 28 planted values of the kind strings at 1
fields`, `examined 7 admitted documents` and `examined 158 planted faults`, under each of four hash
seeds.

A round that closed the class the first population left open (the object kind, the elements of a
list, and the field named in a refusal) planted four faults on the reader, each in a scratch copy
and never committed. The earlier test file reads `OK` on every one, and the test of this round reads
`FAILED` by assertion on every one. The record is quoted as prose, since a criterion is recorded
once above.

```text
plant: the object arm raises another arm    earlier test OK (Ran 4)   this test FAILED (A4)
plant: the list-element test made vacuous   earlier test OK (Ran 4)   this test FAILED (A4)
plant: a refusal naming another field       earlier test OK (Ran 4)   this test FAILED (A4)
plant: a map judged by its keys             earlier test OK (Ran 4)   this test FAILED (the admission test)
```
