# Red-first record: SPEC-634

The SPEC, ADR-658, the schematic and the fragment came first (14878375), then the decision module
with a stand-in `decide` (37c01a8e), the Lean entry (4d5b2e19), its vectors writer (b15d0199), and
the five covers' stamps with the first vectors, both from the formal checker (0fa1dbeb). The
stand-in keeps every input and answers `"run"` on each, the base's behaviour; the reader and the
other four functions are final there. At 0fa1dbeb `formal/obligations.json` held no O18 and no
`lean/PathGate` row.

The commit that adds this record adds the tests and nothing else: `scripts/tests/test_path_gate.py`
and the ledger's expectations in `scripts/tests/test_formal_obligations.py`. Each red below is
quoted from one run of its test file alone over the tree of 0fa1dbeb with those two files carried
in, which is that commit's tree but for this record (`test_path_gate.py`: `Ran 10 tests`,
`FAILED (failures=25)`; `test_formal_obligations.py`: `Ran 9 tests`, `FAILED (failures=3)`; every
failure an assertion, no error).

```red-first
A1: red at 0fa1dbe: AssertionError: {'map[240 chars] 'decision': 'run', 'admit': {'refused': ['rust'], 'rest': []}} != {'map[240 chars] 'decision': 'skip', 'admit': {'refused': [], 'rest': []}} (test_every_vector_is_the_gates_decision, case event-pr-into-dev), and so in 15 of the 78 vectors, each a vector whose decision is skip: event-pr-into-dev, rust-outside, rust-outside-many, engine-outside, release-outside, rename-outside, release-exact-beside-toolsx, release-exact-beside-subpath, rust-exact-beside-longer-name, rust-prefix-beside-toolsx, rust-prefix-bare-name, engine-always-prefix-beside-cratesx, engine-always-prefix-bare-name, bare-map-no-reads and witness-map-outside
A1: green at 4e3763e
A2: red at 0fa1dbe: AssertionError: 0 != 1 : the script does not hold 'if changed is None:\n        return "run"' once (test_none_treated_as_an_empty_list), and so for the event test's two plants (test_a_push_treated_as_a_pull_request and test_a_push_treated_as_a_pull_request_with_no_base_test); and AssertionError: Lists differ: ['admit', 'decision'] != [] at the script itself, for each of the seven named vectors of the two prefix plants (test_the_prefix_test_without_its_endswith and test_the_prefix_test_without_its_slash); 10 failures in 5 of the 8 tests
A2: green at 4e3763e
A3: not red: the five covers were stamped at 0fa1dbeb from the formal checker's reading of the five spans, so the test's span digest agrees with each stamp there (examined 5 covers lines, OK at 0fa1dbeb); it is the control that the test reads a span as the checker does, and it guards the cover that the decision moves
A4: red at 0fa1dbe: AssertionError: Lists differ: ['O1a[56 chars] 'O10', 'O11', 'O12', 'O13', 'O14', 'O15', 'O16', 'O17'] != ['O1a[56 chars] 'O10', 'O11', 'O12', 'O13', 'O14', 'O15', 'O16', 'O17', 'O18'] (test_the_rows_hold_o1_to_o17_and_the_planned_entries); Refusal(arm='entry-without-row', where='lean/PathGate', why="the tree's entry has no row") (test_the_ledger_exists_in_schema_v1); and the dispositions differ, the tree's lean/PathGate holding no row (test_every_existing_entry_has_its_disposition_row)
A4: green at cb6dcd7
```

## What each pair disclosed

- **A2's red is in five of its eight tests.** At the stand-in, three plants are caught as SPEC-634
  section 2a says, because their named vectors decide `run` and the plant changes their `reads` or
  their admission: an `always` entry dropped, `admit` without its `gate_ok` check, and `reads`
  false for a job with no entry. The other five fail because the stand-in has no event test and no
  None clause to plant in, or because the named vectors decide `skip` and the stand-in runs.
- **A3 reads the five covers the formal checker stamped.** The span rule is that of
  `scripts/tests/test_release_gates.py`'s `span_digest`; it agreed with all five stamps at
  0fa1dbeb, so the decision's commit must stamp its cover again with the digest the test computes.
- **The three A4 failures** each read the ledger before its O18 row: the expected ids and statuses,
  the ledger checker's own refusal of an entry with no row, and the dispositions.
- **The greens.** 4e3763e gives `decide` its body, as section 2 R4 states it, and stamps its cover
  again with the digest the test computes; the other four covers, the reader and the vectors are
  unchanged. A1, A2 and A3 read `Ran 10 tests`, `OK` there. cb6dcd7 adds O18 and its
  `lean/PathGate` row to the ledger and maps the script in `scripts/mutation-python.json`; A4
  reads `Ran 9 tests`, `OK` there. No test file changed after the commit that adds this record.
  After 4e3763e only the ledger, the mutation map, this record and the SPEC change, and A1 to A3
  read none of them; after cb6dcd7 only this record and the SPEC change.
- **The green populations:** `examined 78 Lean vectors`, `examined 5 map texts the reader must
  refuse`, `examined 5 covers lines`, and the named vectors of the six controls' eight plants, 3, 3,
  4, 5, 1, 1, 2 and 3 in section 2a's order.
