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
A4: not red: it pins what a well-formed population reads and the two refusals the reader already made, which the base already did; it guards them against the new checks
A5: not red: it pins that every committed band file reads as plain JSON reads it, which the base already did; it guards the committed rows against the new checks
```
