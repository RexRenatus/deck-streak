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

## Fix round 1

The review ran 95 hand mutants of the driver, the corpus and the registry at feb253c0, each against its criterion's
test and then the three new test files whole, and 43 survived: the tests could not tell those changed copies from the
driver. 48b20789 adds one case for each of the 43, one case covering the two that loosen the golden-name check, and a
control that the planted registry loads when nothing is planted; the stand-in harness gains three test-only faults
(an answer that is not an object, a hello with another key, stock texts sent as one text). No production file
changed, so no fix commit follows the cases.

On an export of 48b20789, each of the 43 new cases run alone was green on arrival (Ran 1, OK, 43 of 43), and the three new
test files run whole read Ran 70 tests, OK. Then each mutant was applied to the export by the review's own anchor and
replacement (the corpus mutant with the registry's corpus pin moved to the changed corpus, so the pin does not hide
it), the three files ran whole, and the export was restored byte for byte. Every mutant turned its case red, and no
other test: each run's failing and erroring blocks were all its case's. Each line quotes the case's failure; where more
than three subtests read red, the first three were quoted. `<scratch>` stands for the build host's temporary
directory, `<export>` for the export's root, and `...` marks a quote cut at 240 characters.

```text
A1 test_a_harness_source_holding_a_symbolic_link_is_refused: green on arrival at 48b20789; red with the link check removed: AssertionError: Refused not raised
A1 test_a_harness_source_with_no_file_is_refused: green on arrival at 48b20789; red with the empty-tree check removed: AssertionError: Refused not raised
A1 test_the_harness_digest_covers_each_file_name: green on arrival at 48b20789; red with the digest leaving out each file's path: AssertionError: '6afcfe322b9592605c1f0939d7d0c427809ca840e429ce094adb49edafb30e5e' == '6afcfe322b9592605c1f0939d7d0c427809ca840e429ce094adb49edafb30e5e'
A2 test_a_module_with_no_functions_mapping_is_refused: green on arrival at 48b20789; red with the FUNCTIONS check removed (2 subtests): AttributeError: 'NoneType' object has no attribute 'items'; AttributeError: 'list' object has no attribute 'items'
A2 test_a_module_with_no_integer_seed_is_refused: green on arrival at 48b20789; red with the integer SEED check removed (3 subtests): AssertionError: Refused not raised, twice; AssertionError: "exports no integer SEED" does not match "anki.timing.day: its case builder draws from more than random.Random(SEED)"
A2 test_a_module_with_no_corpus_is_refused: green on arrival at 48b20789; red with the CORPUS check removed: IsADirectoryError: [Errno 21] Is a directory: '.'
A2 test_a_golden_registered_twice_is_refused: green on arrival at 48b20789; red with the check for a golden two modules register removed: AssertionError: Refused not raised
A2 test_an_id_taken_twice_is_refused: green on arrival at 48b20789; red with the check for an id taken twice removed: AssertionError: Refused not raised
A2 test_a_registry_that_registers_nothing_is_refused: green on arrival at 48b20789; red with the empty-registry check removed: AssertionError: Refused not raised
A2 test_a_name_that_is_not_a_golden_name_is_refused: green on arrival at 48b20789; red with the golden-name check removed (5 subtests, the first three quoted): AssertionError: Refused not raised, in each
A2 test_a_name_that_is_not_a_golden_name_is_refused: green on arrival at 48b20789; red with the golden-name pattern widened to any text (5 subtests, the first three quoted): AssertionError: Refused not raised, in each
A2 test_a_registration_with_other_keys_is_refused: green on arrival at 48b20789; red with the registration's key check reduced to its type (2 subtests): KeyError: 'note'; AssertionError: Refused not raised
A2 test_a_class_name_outside_the_grammar_is_refused: green on arrival at 48b20789; red with the class-name check removed (3 subtests): TypeError: sequence item 0: expected str instance, int found; AssertionError: "declares classes that are not a list of class names" does not match "anki.timing.day: declared class(es) with no case: Rollover"; AssertionError: "declares classes that are not a list of class names" does not match "anki.timing.day: declared class(es) with no case: roll over"
A2 test_a_class_declared_twice_is_refused: green on arrival at 48b20789; red with the duplicate-class check removed: AssertionError: Refused not raised
A2 test_a_registration_with_no_id_is_refused: green on arrival at 48b20789; red with the id check removed (4 subtests, the first three quoted): AssertionError: Refused not raised, in each
A2 test_a_registration_with_no_note_is_refused: green on arrival at 48b20789; red with the note check removed (3 subtests): AssertionError: Refused not raised, in each
A2 test_a_registration_with_no_case_builder_is_refused: green on arrival at 48b20789; red with the builder check removed (3 subtests): TypeError: 'str' object is not callable; TypeError: 'NoneType' object is not callable; TypeError: 'list' object is not callable
A2 test_a_builder_that_draws_from_more_than_its_seed_is_refused: green on arrival at 48b20789; red with the second draw's comparison removed: AssertionError: Refused not raised
A2 test_a_builder_that_draws_no_case_is_refused: green on arrival at 48b20789; red with the empty-draw check removed: AssertionError: Refused not raised
A2 test_a_case_class_outside_the_grammar_is_refused: green on arrival at 48b20789; red with the case-class check removed (4 subtests, the first three quoted): AssertionError: Refused not raised, in each
A1 test_an_input_that_is_not_an_object_is_refused: green on arrival at 48b20789; red with the input-object check removed (4 subtests, the first three quoted): AssertionError: Refused not raised, in each
A1 test_an_input_that_is_not_strict_json_is_refused: green on arrival at 48b20789; red with NaN and the infinities allowed in an input (3 subtests): AssertionError: Refused not raised, in each
A1 test_a_source_that_is_not_a_directory_is_refused: green on arrival at 48b20789; red with the directory check removed: AssertionError: Lists differ: [] != ['<scratch>/n[99 chars]ory']
A1 test_the_default_probe_asks_for_write_access: green on arrival at 48b20789; red with the default probe asking whether a path exists, not whether it is writable: AssertionError: Items in the first set but not the second:
A5 test_the_census_reads_lines_inside_lists: green on arrival at 48b20789; red with the census no longer walking lists: AssertionError: 0 != 2 : []
A1 test_a_source_commit_that_is_not_hex_is_refused: green on arrival at 48b20789; red with any text taken as a source_commit (3 subtests): AssertionError: Lists differ: [] != ['anki.timing.day: source_commit is not a 40-hex commit'], in each
A1 test_a_digest_that_is_not_hex_is_refused: green on arrival at 48b20789; red with the digest check removed (6 subtests, the first three quoted): AssertionError: Lists differ: [] != ['anki.timing.day: generator_sha256 is not a sha256'], in each
A1 test_a_golden_with_no_case_is_refused: green on arrival at 48b20789; red with an empty case list accepted: AssertionError: Lists differ: [] != ['anki.timing.day: holds no case']
A1 test_a_case_that_is_not_input_output_and_class_is_refused: green on arrival at 48b20789; red with the case's key check reduced to its type (3 subtests): AssertionError: Lists differ: [] != ['anki.timing.day: case 1 is not input, output and an optional class'], in each
A1 test_a_golden_naming_a_file_not_committed_is_refused: green on arrival at 48b20789; red with the committed-file check removed: FileNotFoundError: [Errno 2] No such file or directory: '<export>/tools/parity-oracle/anki/no-such-driver.py'
A1 test_a_registry_digest_that_differs_is_refused: green on arrival at 48b20789; red with the registry digest no longer compared: AssertionError: Lists differ: [] != ['anki.timing.day: registry_sha256 differs[62 chars].py']
A3 test_a_workflow_that_names_the_driver_is_caught: green on arrival at 48b20789; red with the driver's path dropped from the names a workflow may not hold: AssertionError: Lists differ: [] != ['.github/workflows/planted.yml: names tools/parity-oracle/anki/generate.py']
A1 test_a_harness_that_cannot_start_fails_the_run: green on arrival at 48b20789; red with a harness that cannot start no longer caught: FileNotFoundError: [Errno 2] No such file or directory: '<scratch>/no-such-harness'
A1 test_an_answer_that_is_not_an_object_fails_the_run: green on arrival at 48b20789; red with the answer-object check removed: TypeError: unhashable type: 'dict'
A1 test_a_hello_with_other_keys_fails_the_run: green on arrival at 48b20789; red with the hello's key check removed: AssertionError: RunFailed not raised
A1, A5 test_stock_texts_that_are_not_a_list_of_texts_fail_the_run: green on arrival at 48b20789; red with the stock texts' type check removed: AssertionError: RunFailed not raised
A1 test_a_call_missing_a_required_argument_is_refused: green on arrival at 48b20789; red with the required-arguments check removed (4 subtests, the first three quoted): TypeError: argument should be a str or an os.PathLike object where __fspath__ returns a str, not 'NoneType', twice; AssertionError: '--harness, --harness-source, --engine-source and --out are required' not found in 'generate.py: <scratch>/harness is writable: the sandbox mounts every source read-only; ...
A1 test_the_engine_source_is_probed_as_well_as_the_harness_source: green on arrival at 48b20789; red with the engine's source no longer probed: AssertionError: '<scratch>/engine is writable' not found in "generate.py: the harness source's digest e7bad6034eca7812a6895157aad60c89da415dffe17fdcb118d4f0e98db345e6 is not the pinned 0000000000000000000...
A1 test_a_run_that_fails_exits_3_and_writes_nothing: green on arrival at 48b20789; red with a failed run exiting 0: AssertionError: 0 != 3
A1 test_the_written_goldens_record_the_registry_seed: green on arrival at 48b20789; red with main drawing from SEED + 1: AssertionError: 499 != 498 : anki.render.card
A1 test_the_written_goldens_record_the_driver_digest: green on arrival at 48b20789; red with main recording a constant for the driver's digest: AssertionError: 'abababababababababababababababababababababababababababababababab' != '89c54452a047d8adb3648ea57d39269ba3e3eea4f404afa07ed73010eac88887'
A1 test_the_run_writes_no_bytecode_beside_what_it_loads: green on arrival at 48b20789; red with main leaving bytecode writes on: AssertionError: False is not true
A2 test_every_rollover_clock_is_one_second_before_the_local_rollover_hour: green on arrival at 48b20789; red with the rollover clock adding the UTC offset where it subtracts it: AssertionError: 25199 != 61199
A2 test_the_planted_registry_loads_when_nothing_is_planted: green at 48b20789, the CONTROL that each registry fault above is refused for its own reason, not for the planted module's
```

The review's runner, unchanged, then ran over all 95 mutants on an export of 48b20789: each mutant against the earlier
test the review named for it, then the three new test files whole when that test stayed green. It read
`MUT-SUMMARY examined=95 killed=52 killed_by_other_test=43 survived=0 not_applied=0 restore_bad=0`: the 52 the
earlier tests killed are killed again, and each of the 43 is killed by the new case that names it and by no other
test. The Python leg at 48b20789: `test_anki_driver.py` 39 tests, `test_anki_registry.py` 26, `test_anki_workspace.py` 5,
`test_generate.py` 20 and `test_goldens.py` 9, each OK.

## The second delivery: the engine writes the goldens

Red at 036befdc, before the snapshot writer, the pin and the goldens existed.

- Python, DeckStreak's own form from the checkout root, one line per file: `test_goldens.py` ran 18 tests, 8 failed,
  each for its own reason: six read `examined 0 engine golden(s): the population is empty, so nothing was judged`,
  `test_no_engine_golden_holds_a_stock_line` read `tools/parity-oracle/anki/stock-lines.sha256 is not committed`, and
  `test_the_schema_snapshot_carries_its_pin_line` read `tools/parity-oracle/anki/snapshot.py is not committed`. The
  ninth, `test_the_manifest_pins_one_engine_commit`, is the guard that the manifest's pin is the driver's, green at red.
  `test_generate.py` 20, `test_anki_driver.py` 39, `test_anki_registry.py` 26, `test_anki_workspace.py` 5 and
  `test_anki_hand_cases.py` 7 stayed OK.
- Rust: `cargo test --locked --offline --no-fail-fast -p deck-streak-ingest --test engine_goldens --test
  oracle_consumer` exited 101: `engine_goldens` 1 passed and 35 failed, the 33 per-golden tests, the on-disk census and
  the snapshot test, each with `anki.<name>.json: cannot be read: No such file or directory`; the control
  `an_answer_off_its_keys_or_a_worded_refusal_is_a_fault` passed, and `oracle_consumer` passed 3 of 3.

The failing tests by name: `test_engine_golden_provenance`, `test_engine_goldens_are_synthetic_and_seeded`,
`test_every_engine_golden_is_read_by_a_rust_test`, `test_every_registered_engine_function_has_its_golden`,
`test_no_engine_golden_holds_a_date_an_id_or_a_clock`, `test_no_engine_golden_holds_a_stock_line`,
`test_the_schema_snapshot_carries_its_pin_line`, `test_the_two_oracles_split_the_goldens_whole`; and in Rust each of
the 33 tests named for its golden, `every_engine_golden_is_read_here` and
`the_schema_snapshot_is_its_golden_under_one_pin_line`.

Green waits for the one sandboxed run: the pin commit, the export, the run, then the goldens, the census and the
snapshot in one commit.
