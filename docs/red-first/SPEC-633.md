# Red-first record: SPEC-633

The SPEC, ADR-656, the schematic and the changelog fragment were committed alone. Then came the
tests of A1 and A2 with a module runner that keeps every input (the suites, `PYTHON_STAGE_JOBS`, the
serial tail, the output order and the verdict line) but runs every module one after another, so the
A1 test ran and failed at its barrier.

```red-first
A2: not red: the serial runner already keeps the verdict line, the output order and the tail lines, and the criterion pins them
A1: red at 6ac0863d: scripts/tests/test_python_stage.py:82: AssertionError: Lists differ: ['AssertionError: barrier timeout: module b did not arrive within 10 s'] != [] (job 128147372614)
A1: green at f4500f50
A3: red at 866fe1b5: scripts/tests/test_check_gate.py:236: AssertionError: 'Ran 2 tests' not found in "python3: can't open file (box run)
A3: green at 0f78e662
```

A1 was committed with a module runner that keeps every input and runs each module one after
another (6ac0863d); it read red by name in the python stage of the pull request run. The pool replaced
the serial runner in f4500f50, with every test unchanged, and the A1 test read green on that tree. A3,
SPEC-054 A13's test, read red on a local run once the stage called the runner in 866fe1b5, because the
tree it plants had no runner. 0f78e662, the one commit that changes a test file between a red and its
green, plants the runner and keeps the text of every assertion, and A3 read green. A2 pins rules
the serial runner already keeps, so it has no red line. The commit after 0f78e662 changes only this
record.

A4 was committed alone with its test, against the runner whose module processes inherit the stage's
environment as it is, so a planted module that imports a planted sibling leaves bytecode under its
suite and the A4 test fails on the bytecode it finds.
