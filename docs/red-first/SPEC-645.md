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
A6: not red: a well-formed plan already selected its rows at the base and an unknown id was already refused by name; this test guards that reading against the new checks
```
