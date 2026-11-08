# Red-first record: SPEC-645

SPEC-645, ADR-668, the schematic and the changelog fragment were committed first and alone
(3a8e2a51). The commit that adds this record carries the six criteria's tests with the runner as
it was: A1 to A5 read red there by name, each by assertion, and A6 is not red, as ADR-668 D3
says.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_an_array_is_refused_naming_the_shape
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_a_string_is_refused_naming_the_shape
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_plan_that_is_a_number_a_boolean_or_null_is_refused_naming_the_shape
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_rows_value_that_is_not_an_array_is_refused_naming_the_shape
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_row_id_that_is_not_a_string_is_refused_naming_its_place
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_well_formed_plan_still_selects_its_rows
```

```red-first
A1: red at 61ca42e8: AssertionError: 1 != 2 : prove ended 1: Traceback (most recent call last):
A1: green at 00ae13f2
A2: red at 61ca42e8: AssertionError: 1 != 2 : prove ended 1: Traceback (most recent call last):
A2: green at 00ae13f2
A3: red at 61ca42e8: AssertionError: 1 != 2 : prove ended 1: Traceback (most recent call last):
A3: green at 00ae13f2
A4: red at 61ca42e8: AssertionError: 1 != 2 : prove ended 1: Traceback (most recent call last):
A4: green at 00ae13f2
A5: red at 61ca42e8: AssertionError: 1 != 2 : prove ended 1: Traceback (most recent call last):
A5: green at 00ae13f2
A6: not red: a well-formed plan already selected its rows at the base and an unknown id was already refused by name; this test guards that reading against the new checks
```

## What changed between the red and its green

The cure (00ae13f2) changed `scripts/mutation_rows.py` alone and no test file, so each criterion's
test reads green there in the form it read red at 61ca42e8. The rows commit (289cd87f) added
`scripts/mutation-rows.d/S64500-S64599.json` alone.

## Rows

The nine rows S64500 to S64508 were proved KILLED by id at 289cd87f, by
`python3 scripts/mutation_rows.py prove --band S64500-S64599`, whose summary line read
`rows: examined 9: killed 9, survived 0, void 0`.

## The green at the head

`git diff --quiet 00ae13f2 HEAD -- scripts/mutation_rows.py` exits 0, and
`scripts/tests/test_mutation_rows.py` is unchanged since 61ca42e8, so the greens recorded at 00ae13f2
hold at the head.
