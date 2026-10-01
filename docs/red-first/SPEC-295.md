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

## Amendment addendum, 2026-09-30 (issue #504, the toolchain identity)

The test of A6 and the field table's new row were committed alone (bd9f84c0) with `config/formal.json`
unchanged, so the module read `FAILED (failures=3)`, each by assertion and none by error: A1 and the
presence control of A3 because the file names no toolchain, and A6 because its identity is absent.
The presence control of A3 was made to fail by assertion (`self.fail`) and not by the reader's
refusal escaping as an error. The field (e4521f8b) turned them green.

```red-first
A6: red at bd9f84c0: AssertionError: False is not true : toolchain.identity is named
A6: green at e4521f8b
```

A1 and A3 are recorded once above, at their own first commits, so this round's replay of them is
quoted as prose and as text.

```text
A1 at bd9f84c0: AssertionError: False is not true : ('toolchain', 'identity')
A3 at bd9f84c0: AssertionError: presence control: the file is refused: missing required field toolchain.identity
A1, A3 and A6 at e4521f8b: Ran 6 tests ... OK
```

The green run prints `examined 158 planted value types` (it was 144), `examined 7 planted values of the
kind hex64 at 1 fields`, `examined 28 planted values of the kind object at 4 fields`,
`examined 10 declared fields`, `examined 180 planted faults` (it was 158), `examined 9 refusal
arms of the reader` (it was 8) and `examined 22 planted digest faults`. The nine mutant rows
S29500 to S29508 keep their anchors, each occurring once in the test file.

## Amendment addendum, 2026-09-30 (issue #504, round 1)

The tests of A7 and A8 were committed alone (bfa66c37) and the module read `FAILED (failures=3)`,
each by assertion and none by error. The reader change and the assertions in the loader and in the
presence controls (00937178) turned them green.

```red-first
A7: red at bfa66c37: AssertionError: the sources each combination names, as the checker reads them (R6): the combination of the field named and a pin file committed lists one source, not two
A7: green at 00937178
A8: red at bfa66c37: AssertionError: ['test_the_toolchain_identity_is_named_and_a_malformed_one_is_refused line 493'] != [] : a call of the reader whose refusal would escape as an error
A8: green at 00937178
```

The green run reads `Ran 9 tests ... OK` and prints `examined 4 toolchain source combinations`,
`examined 182 planted refusals of the committed file`, `examined 5 tests that load the committed
file`, `examined 6 calls of the reader` and `examined 2 presence controls of the committed file`.
At the same commit the second test of A8 reads `AssertionError: a refusal escaped a test as an error, not
a failure` for 159 of its 182 planted refusals of the committed file. The rows S29510 to S29513 are each killed on a clean committed detached head.

## Amendment addendum, 2026-10-01 (issue #504, round 2)

The tests were committed alone (577bca19) and each module was run alone. The reader change and the
loader's refusals (418728f7) turned them green; the tests that hold each plant to its kind and the
identity to its presence were added after (ccae4f21), each killing mutants named in the rows.

```red-first
A7: red at 577bca19: AssertionError: the sources each combination names, as the checker reads them (R6)
A7: green at 418728f7
A8: red at 577bca19: AssertionError: a refusal escaped a test as an error, not a failure
A8: green at 418728f7
```

Module `test_formal_config.py` alone at 577bca19: `Ran 7 tests ... FAILED (failures=1)`, the failing
test `test_the_tree_names_one_toolchain_source_the_identity`, by assertion, no error.

Module `test_formal_config_presence.py` alone at 577bca19: `Ran 2 tests ... FAILED (failures=1)`, the
failing test `test_a_refused_committed_file_fails_every_reader_by_assertion_and_errors_none`, by
assertion, no error.

Module `test_formal_config.py` alone at the green head: `Ran 10 tests ... OK`, and it prints `examined 12
toolchain source combinations`, `examined 6 pin shapes planted as their kind`, `examined 6 identity
states` and `examined 4 places an invalid byte sits`.

Module `test_formal_config_presence.py` alone at the green head: `Ran 3 tests ... OK`, and it prints
`examined 186 planted refusals of the committed file`, `examined 5 tests that load the committed
file`, `examined 186 planted refusals held to the kind they name`, `examined 6 calls of the reader`
and `examined 2 presence controls of the committed file`. The rows S29510 to S29521 are each killed
on a clean committed detached head.
