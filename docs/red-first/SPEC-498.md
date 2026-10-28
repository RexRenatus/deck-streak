# Red-first record: SPEC-498

The SPEC, ADR-516 and the schematic were installed first (6067ef53). The tests came next, whole,
over stubs that keep every name the tests reach and none of the behaviour (94b1f76e): the driver
loads any registry unchecked, draws each builder once, refuses no source, hashes no stock line,
finds no strictness, provenance or workspace refusal, runs no harness and returns no golden; the
corpus holds its seed and no class; the registry pins no corpus and registers nothing; and the
golden reader's comparison finds no mismatch. 65d5e3b9 turns them green. Each red below is quoted
from the run of the Python leg (`python3 -m unittest discover -s tools/parity-oracle -p <file>`
for the three new files, then `test_generate.py` and `test_goldens.py`) or of
`cargo test --locked --offline --no-fail-fast -p deck-streak-ingest --test oracle_consumer` on the
tree of 94b1f76e, which exited 1 and 101; each green from the same runs on the tree of 65d5e3b9,
which exited 0 and 0.

```red-first
A1: red at 94b1f76e: test_every_golden_is_strict_synthetic_and_seeded: AssertionError: examined 0 goldens: the population is empty, so nothing was judged
A1: red at 94b1f76e: test_golden_provenance: AssertionError: examined 0 goldens: the population is empty, so nothing was judged
A1: red at 94b1f76e: test_a_writable_source_is_refused: AssertionError: 0 != 1
A1: red at 94b1f76e: test_the_run_writes_only_in_its_output_directory: AssertionError: Lists differ: [] != ['out', 'out/goldens', 'out/goldens/anki.r[74 chars]256']
A1: green at 65d5e3b9
A2: red at 94b1f76e: test_every_function_is_registered: AssertionError: examined 0 registrations: the population is empty, so nothing was judged
A2: red at 94b1f76e: test_the_corpus_is_seeded_and_every_class_is_declared: AssertionError: examined 0 corpus classes: the population is empty, so nothing was judged
A2: red at 94b1f76e: test_revlog_to_events_is_registered_with_every_review_type: KeyError: 'anki.history.revlog_events'
A2: red at 94b1f76e: test_edge_classes_covered: AssertionError: examined 0 (function, class) pairs: the population is empty, so nothing was judged
A2: green at 65d5e3b9
A3: red at 94b1f76e: test_generator_outside_workspace: AssertionError: Lists differ: [] != ['.github/workflows/planted.yml: names tools/render-oracle']
A3: green at 65d5e3b9
A5: red at 94b1f76e: test_a_golden_holding_a_stock_line_is_refused: AssertionError: RunFailed not raised
A5: green at 65d5e3b9
A6: red at 94b1f76e: golden_consumer_can_fail: oracle_consumer.rs:39: assertion `left == right` failed: [] (left: 0, right: 1)
A6: red at 94b1f76e: the_consumer_panics_naming_the_wrong_case: oracle_consumer.rs:52: a golden with a wrong output must fail its consumer: Examined { function: "anki.fixture.whole_days", count: 6 }
A6: green at 65d5e3b9
```

## The whole runs

| run | tree | exit | counts |
|---|---|---|---|
| the Python leg, red | 94b1f76e | 1 | `test_anki_driver.py` 15 tests, failures=17 errors=2 (subtests counted); `test_anki_registry.py` 8 tests, failures=6 errors=1; `test_anki_workspace.py` 4 tests, failures=8; `test_generate.py` 20 OK; `test_goldens.py` 9 OK |
| the Rust test, red | 94b1f76e | 101 | 1 passed, 2 failed |
| the Python leg, green | 65d5e3b9 | 0 | 15, 8, 4, 20 and 9 tests, each OK |
| the Rust test, green | 65d5e3b9 | 0 | 3 passed, 0 failed |

## What each pair disclosed

- Every test of the three new Python files read red at 94b1f76e but one:
  `test_no_registry_or_corpus_module_reads_a_file_or_the_network`, a guard over the modules' syntax, read green over
  the stubs, which read nothing either. It is a GUARD, not red-first.
- `the_corrected_fixture_passes_its_consumer` read green at 94b1f76e: it is the CONTROL that the fixture's port is
  right, so the one mismatch the consumer reports is the planted one.
- `test_generate.py` and `test_goldens.py` read green at both trees: the second oracle adds no golden and no module
  to the first oracle's registry.
- The first attempt of the Rust red run ended without an exit when its build host restarted; the second attempt, on
  the same tree, is the one quoted.
