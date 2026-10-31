# Red-first record: SPEC-502

The SPEC, ADR-520 and the schematic were installed first (3b3c865). The tests came next, whole,
over a stub of the script (a62a23d). The stub keeps every name the tests read and none of the
behaviour: `read_record` reads any path as an empty object, `read_gates` gives every gate as not on
file, the four helpers, `refused` and `verdict` refuse nothing, and `main` prints nothing and exits
0, so every release is published. At a62a23d `release.yml` had no gate step, `formal/obligations.json`
held O17 as planned with no `lean/ReleaseGates` row, and neither the Lean entry nor its vectors file
existed. b78e1dc changes one test after the red run and before any code (below). 27bcb57 adds the
script, the workflow step, the Lean entry and its vectors writer, and the ledger's rows; 9f78ae5
stamps the entry's seven covers with the digests the formal checker read; 972cd4e writes the vectors
with the formal checker's `--write-vectors`.

Each red below is quoted from one run of the test file alone on the tree of a62a23d
(`test_release_gates.py`: `Ran 12 tests`, `FAILED (failures=53)`; `test_formal_obligations.py`:
`Ran 9 tests`, `FAILED (failures=2)`; every failure an assertion, no error), and each green from a
run of its fence line alone on the tree of 972cd4e (each `OK`). All 12 tests of
`test_release_gates.py` read red at a62a23d; the failures count each planted record apart.

```red-first
A1: red at a62a23d: AssertionError: examined 0 documented gates judged: the population is empty, so nothing was judged (test_the_documented_record_is_read_with_every_gate_on_file); None unexpectedly found in [None, None, None, None, None, None, None, None] : a gate of the passing record is not on file (test_a_passing_record_admits_and_names_its_weights); and Lists differ: [] != [['model', 'missing']] for the upper-case digest, with each of the other 23 values of another type and the record whose dev is a list (test_a_value_of_another_type_reads_as_its_gate_missing)
A1: green at 972cd4e
A2: red at a62a23d: AssertionError: Lists differ: [] != [['model', 'missing']], with each of the other 14 gates missing or failed alone (test_each_gate_missing_or_failed_alone_is_refused_by_its_name); Lists differ: [] != [['model', 'missing'], ['dev.logloss', 'mi[191 chars]ng']] (test_every_refusing_gate_is_named_in_the_records_order); and Lists differ: [] != [['dev.logloss', 'failed']], with each of the other three boundary records (test_a_tie_or_a_negative_count_fails)
A2: green at 972cd4e
A3: red at a62a23d: AssertionError: Lists differ: [] != [['record', 'missing']] (test_a_record_that_is_not_on_file_refuses_as_the_record); and 0 != 1 (test_a_folder_in_the_records_place_is_no_record)
A3: green at 972cd4e
A4: red at a62a23d: AssertionError: False is not true : formal/vectors/release-gates.jsonl is absent (test_every_vector_is_the_scripts_verdict); and in test_formal_obligations.py, 'O17': 'planned' where 'O17': 'tied' is expected (test_the_rows_hold_o1_to_o17_and_the_planned_entries) and Items in the second set but not the first: 'lean/ReleaseGates' : the tree's entries are not the 44 designed (test_every_existing_entry_has_its_disposition_row)
A4: green at 972cd4e
A5: red at a62a23d: AssertionError: 0 release steps run `python3 scripts/release_gates.py` (test_the_publish_step_runs_only_after_a_passing_record)
A5: green at 972cd4e
A6: red at a62a23d: AssertionError: 0 release steps run `python3 scripts/release_gates.py` (test_the_gate_is_one_step_with_a_name_and_a_one_line_run); and Lists differ: ['sys', 'typing'] != ['argparse', 'json', 'math', 'pathlib', 're', 'sys', 'typing'] (test_the_script_sets_nothing_another_step_reads)
A6: green at 972cd4e
```

## What each pair disclosed

- **b78e1dc edits the vectors test after the red run.** The formal checker's vectors header names
  a cover's digest as `sha256:<hex>`, as `formal/vectors/release-carry.jsonl` does, and the test at
  a62a23d read it as bare hex. b78e1dc reads the header's digest whole in that form, refuses the
  unstamped zeros in it, and compares it with `sha256:` and the span digest of `verdict`. It
  changes only `test_every_vector_is_the_scripts_verdict`'s header assertions; the A4 red above
  stands at b78e1dc too, since the vectors file is absent there. No other test line changed after
  a62a23d: `git diff b78e1dc 972cd4e -- scripts/tests/ scripts/mutation-python.json` is empty.
- **The test's span rule is the checker's.** The span digest the test computes for each of the
  seven covered functions of `scripts/release_gates.py` equals the digest the formal checker
  reported for it at 27bcb57, so A4's stamp assertion compares like with like.
- **The Lean entry's red is its two witnesses.** Each proves its theorem's claim false of the
  base's verdict, `publishesAlways`, which publishes every tag with no record read. The formal
  checker's gate run at 972cd4e (`--kind lean --gate`) reads `FORMAL OK exit=0` over 17 entries,
  47 properties and 49 witnesses, both theorems of `lean/ReleaseGates` clean at `ramp=report`;
  the base read 16 entries and 45 properties.
- **A6's second fence line holds the release's existing tests.** `test_release_workflow.py` is
  unchanged by this delivery and guards every other release step; it is not a red-first test. It
  read `Ran 15 tests`, `OK` at 972cd4e, and `git diff --numstat` of `release.yml` against the base
  reads `5 0`: added lines only, the step and the comment before it.
- **The green populations** at 972cd4e: `examined 6 documented gates judged`, `examined 24 values
  of another type`, `examined 8 gates`, `examined 3 records with several refusing gates`,
  `examined 4 boundary records`, `examined 7 records not on file`, `examined 1 folders in the
  record's place`, `examined 22 Lean vectors`, `examined 22 refusing records`, `examined 22
  release steps` and `examined 5 names a step could pass on`.
