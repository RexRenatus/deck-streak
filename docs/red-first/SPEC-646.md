# Red-first record: SPEC-646

SPEC-646, ADR-669, the schematic and the changelog fragment were committed first and alone
(b3c1162a). The commit that adds this record carries the tests of A1 to A4 with the reader as it
was: there A1 to A3 read red by assertion, and A4 and A5 are not red, as ADR-669 D4 says.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_header_table_that_is_not_an_array_is_refused_naming_it
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_that_is_not_an_array_is_refused_naming_its_place
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_holding_a_cell_that_is_not_a_string_is_refused_naming_its_place
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_well_formed_population_reads_as_before_beside_the_refusals_it_kept
A5: python3 -m unittest discover -s scripts/tests -p test_band_repeated_key.py -k test_every_committed_band_file_reads_as_plain_json_reads_it
```

```red-first
A1: red at 367fdb37: AssertionError: 1 != 2 : Traceback (most recent call last):
A1: green at 746e7e36
A2: red at 367fdb37: AssertionError: 1 != 2 : Traceback (most recent call last):
A2: green at 746e7e36
A3: red at 367fdb37: AssertionError: 0 != 2 : 2
A3: green at 746e7e36
A4: not red: it pins what a well-formed population reads and the two refusals the reader already made, which the base already did; it guards them against the new checks
A5: not red: it pins that every committed band file reads as plain JSON reads it, which the base already did; it guards the committed rows against the new checks
```

## What changed between the red and its green

The green commit (746e7e36) changed `scripts/mutation_rows.py` only, and no test file. The
commit after it (78c67e47) added `scripts/mutation-rows.d/S64600-S64699.json` only. No test
file changed after the commit that carries the tests: `git log 367fdb37..HEAD --
scripts/tests` prints nothing.

## The rows

Rows S64600 to S64602 pin each of the three checks to the test of its criterion. At
78c67e47, `python3 scripts/mutation_rows.py prove --band S64600-S64699` proved each KILLED by
its full id and exited 0, and the reader read restored after it:

- `S64600-A-HEADER-TABLE-THAT-IS-NOT-AN-ARRAY-IS-REFUSED`: KILLED
- `S64601-A-ROW-THAT-IS-NOT-AN-ARRAY-IS-REFUSED`: KILLED
- `S64602-A-CELL-THAT-IS-NOT-A-STRING-IS-REFUSED`: KILLED

## The green at the head

The reader and the test files at the pushed head equal those of the green commit and of the
commit that carries the tests: `git diff --quiet 746e7e36 HEAD -- scripts/mutation_rows.py
scripts/tests` exits 0, since the commits after the green change no file under either path.
