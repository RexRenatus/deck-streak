# Red-first record: SPEC-605

The criteria tests run in the `hygiene` job of CI, which runs `scripts/tests`. The red commit adds
the Kani run budget to the test table and the reader field table and leaves the settings file
unchanged, so A1 reads red by name there. A2 is generated from the field table the same commit
extends and is recorded not red, with the mutation row that proves it can fail.

```red-first
A1: red at 0626b6d: AssertionError: False is not true : ('budgets', 'kani_seconds') (job 126485063480)
A1: green at 498bfb8
A2: not red: generated from the field table the red commit extends; row S60502 proves it can fail
```

After 498bfb8 the delivery's own commit changes only documents. Its three merges of the
development branch bring in other deliveries' changes: one adds SPEC-611's tests to
`scripts/tests/test_formal_config.py` (126 lines added, none removed), and the A1 and A2 tests
read the same, byte for byte, from 498bfb8 to the delivery's head.
