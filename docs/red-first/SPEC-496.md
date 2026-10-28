# Red-first record: SPEC-496

The SPEC, ADR-514 and the schematic were installed first (d4697f8). The tests came next, whole,
over a stub of the checker (c80b4d2). The stub keeps every name the tests read and the census list,
and none of the behaviour: `check` refuses nothing, `declared` reads no entry, and `census` counts
the words and finds no name. `formal/obligations.json` was absent. f31d2a8 replaces the stub with
the checker and adds the ledger. The test file is byte-identical at c80b4d2 and f31d2a8, and so is
the census list in `scripts/tests/_formal_ledger.py`.

Each red below is quoted from one run of the test file alone on the tree of c80b4d2 (`Ran 9 tests`,
`FAILED (failures=59)`, every failure an assertion, no error), and each green from one run of it
alone on the tree of f31d2a8 (`Ran 9 tests`, `OK`). The failures count each planted fault apart:
every planted fault of every test read red, and each test that reads the committed ledger also
failed at its loader.

```red-first
A1: red at c80b4d2: AssertionError: formal/obligations.json is absent: the ledger names no obligation (test_the_ledger_exists_in_schema_v1, test_the_rows_hold_o1_to_o17_and_the_planned_entries and test_the_phase_sets_list_o1a_and_o1_to_o11, each at its loader)
A1: green at f31d2a8
A2: red at c80b4d2: AssertionError: Items in the second set but not the first: 'no-artifact' : a planned obligation with no artifact: [] (test_an_obligation_with_no_artifact_is_refused); and 'no-gap-line' : no gap line: [], with each of the other three planted gap faults (test_an_obligation_with_no_gap_line_is_refused)
A2: green at f31d2a8
A3: red at c80b4d2: AssertionError: Items in the second set but not the first: 'undeclared-property' : a property another entry declares: [], with each of the other eight planted faults
A3: green at f31d2a8
A4: red at c80b4d2: AssertionError: Items in the second set but not the first: 'untied-artifact' : an obligation tied only by a covers stamp: [], with each of the other 22 planted faults
A4: green at f31d2a8
A5: red at c80b4d2: AssertionError: Items in the second set but not the first: 'entry-without-row' : an entry of the tree with no row: [], with each of the other five planted faults
A5: green at f31d2a8
A6: red at c80b4d2: AssertionError: Items in the second set but not the first: 'private-brand' : the brand in a gap line: [], and 'private-name' : a listed name in a gap line: [], with each of the other five planted names
A6: green at f31d2a8
```

## What each run printed

- **Red, per test:** the census test 8 failures (7 planted names and its loader), the tie test 24
  (23 planted faults and its loader), the artifact test 10 (9 and its loader), the rows test 7 (6
  and its loader), the no-artifact test 2, the gap test 5, and the three committed-ledger tests 1
  each: 59.
- **Green:** `examined 18 obligations in the committed ledger`, `examined 27 planned entries`,
  `examined 29 planned harnesses`, `examined 63 present artifacts`, `examined 14 phase-set
  members`, `examined 41 committed gap lines`, `examined 1 committed ties`, `examined 43 rows`,
  `examined 3022 tokens of the committed ledger` and `examined 50 listed private-name digests`;
  the planted faults read 20 (schema), 2 (phase sets), 1 (no artifact), 4 (no gap line), 9
  (undeclared), 23 (ties), 6 (rows) and 7 (census), each refused by its one arm alone.
- **Beside the fence, at f31d2a8:** `test_stand_in_census.py` (`Ran 29 tests`, `OK`, 118 test
  files read), `test_planned_specs.py` (`Ran 3 tests`, `OK`, 224 SPEC files) and the public scrub
  over the six new files of f31d2a8 (`examined 6 file(s) against public shapes only; 0
  finding(s)`).
