# Red-first record: SPEC-434

SPEC-434 (R1 to R16, A1 to A19). The SPEC, ADR-448, the schematic, the register
`config/fsm-tables.json` with its twelve tables, the tests of A1 to A19 and the helper
`scripts/tests/_fsm_tables.py` were committed first and alone, the helper as stubs that keep every
input but the missing behaviour: the product walk reads only the top level of each crate's `src`;
the enum reader keeps every item under `#[cfg(test)]` and reads no suffix but `State`; the variant
reader takes the first name of each piece between commas outside parentheses, so a struct
variant's fields and a `#[cfg(test)]` variant are read as states; no table is VOID; FSM-REACH
examines only the initial states, and no graph rule reports a hit; FSM-DEAD-END, FSM-TERMINAL and
FSM-DETERMINISTIC print no summary line; no reading is VOID; every listed entry is admitted and
none reads `stale`; `fail` is read as `warn`; a hit line carries the text of the line it names;
and the register reader takes any shape. The stubs also carry the comment and literal masker the
green helper keeps, which every reading of the tree needs to read the product files at all; no
criterion reads it red. Each red below is therefore its criterion's assertion and never a missing
name. The test modules run only in CI, so every red is quoted from the pull request's checks over
that commit, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_the_tree_reading_reads_every_product_file_and_every_table
A2: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_only_a_product_state_enum_with_no_table_is_a_hit
A3: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_every_variant_form_is_read_and_a_test_variant_is_not
A4: python3 -m unittest discover -s scripts/tests -p test_fsm_table.py -k test_a_table_naming_what_is_absent_is_void
A5: python3 -m unittest discover -s scripts/tests -p test_fsm_reach.py -k test_the_tree_reading_reads_every_state_of_every_table
A6: python3 -m unittest discover -s scripts/tests -p test_fsm_reach.py -k test_only_a_state_no_walk_from_the_initial_set_reaches_is_a_hit
A7: python3 -m unittest discover -s scripts/tests -p test_fsm_dead_end.py -k test_the_tree_reading_reads_every_reached_state_outside_the_terminal_set
A8: python3 -m unittest discover -s scripts/tests -p test_fsm_dead_end.py -k test_only_a_reached_sink_outside_the_terminal_set_is_a_hit
A9: python3 -m unittest discover -s scripts/tests -p test_fsm_terminal.py -k test_the_tree_reading_reads_every_reached_terminal
A10: python3 -m unittest discover -s scripts/tests -p test_fsm_terminal.py -k test_only_a_reached_terminal_with_an_outgoing_row_is_a_hit
A11: python3 -m unittest discover -s scripts/tests -p test_fsm_deterministic.py -k test_the_tree_reading_reads_every_state_and_event_pair
A12: python3 -m unittest discover -s scripts/tests -p test_fsm_deterministic.py -k test_only_one_event_moving_one_state_two_ways_is_a_hit
A13: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_every_rule_is_void_with_nothing_to_examine
A14: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_listed_entry_needs_a_ruling_that_exists_and_reads_stale_without_its_hit
A15: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_rule_at_fail_fails_on_new_and_stale_and_at_warn_prints_them
A16: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_a_hit_line_names_its_site_and_never_its_source
A17: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_the_register_names_five_rules_at_warn_and_refuses_the_malformed
A18: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_every_graph_rule_agrees_with_an_independent_oracle_over_every_small_table
A19: python3 -m unittest discover -s scripts/tests -p test_fsm_table_lints.py -k test_no_state_is_a_hit_of_two_graph_rules
```

The reds were read in run 38911156433, job 117007052918 (`hygiene`), a job of 1267 s, over
the red commit, from that job's `check-stage-logs-hygiene` artifact: `Ran 1304 tests` and
`FAILED (failures=19)`, with no error. The nineteen are the tests the fence above names for A1
to A19, every one of them and no other: seven in `test_fsm_table_lints.py`, four in
`test_fsm_table.py`, and two in each of `test_fsm_reach.py`, `test_fsm_dead_end.py`,
`test_fsm_terminal.py` and `test_fsm_deterministic.py`. Each failing line below is its test's
first failing assertion, quoted as CI printed it, unittest's own `[N chars]` cuts included.
A18's red is the helper's reading of a table whose hits are stated by hand, on the line after
the oracle's reading of the same table passed.

Two of the tests read more than SPEC-434 section 3 says, each a strengthening:
- A17 reads the tree's own register for its levels as well as the fixture registers it refuses, so
  the five rows that set a rule's level to `fail` in `config/fsm-tables.json`, S43457 to S43461,
  are killed by it. Section 3 names only A1, A5, A7, A9 and A11 as reading the tree.
- A19 also refuses a population over which any of FSM-REACH, FSM-DEAD-END and FSM-TERMINAL has no
  hit, so it can never pass over rules that report nothing; that is how it reads red over the
  stubs, whose graph rules report no hit: its red names FSM-REACH, the first of the three it
  counts.

Test edits after the red commit: no line of a test module changes. One commit between the red
and the green changes a file under `scripts/tests`: the green commit `d1c95441`,
d1c954412b3ea6d55cdccbd7b41f45aec14da8ff, which changes the helper `scripts/tests/_fsm_tables.py`
alone. The rows commit `47d9741c`, 47d9741c6fefd34fa41f2b6a8e577f32a65c13d4, adds only
`scripts/mutation-rows.d/S43400-S43499.json` and touches no test or code file, and this record's
commit adds only this record and `changelog.d/fsm-tables-434.md`. So the helper, the six test
modules and the register read at the head exactly as at the green commit; at the head this
command exits 0:
`git diff --quiet d1c95441 HEAD -- scripts/tests/_fsm_tables.py scripts/tests/test_fsm_dead_end.py scripts/tests/test_fsm_deterministic.py scripts/tests/test_fsm_reach.py scripts/tests/test_fsm_table.py scripts/tests/test_fsm_table_lints.py scripts/tests/test_fsm_terminal.py config/fsm-tables.json`

No CI run reads the green commit or the rows commit alone: every green below is read by name in
the second push's checks, at its head, which carries the green commit, the rows and this
record, and those checks confirm each green.

```red-first
A1: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: 299 != 340 : the walk read 299 product files and the listing counts 340
A1: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A2: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: [Hit(path='crates/demo/src/lib.rs', line=7[80 chars]te')] != [('crates/demo/src/lib.rs', 7, 'DoorState'[275 chars]de')]
A2: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A3: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Tuples differ: (1, [[39 chars]), ('right', 5), ('Numbered', 6), ('Attributed[33 chars]11)]) != (1, [[39 chars]), ('Numbered', 6), ('Attributed', 8), ('Last', 11)])
A3: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A4: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Void not raised
A4: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A5: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: 41 not greater than or equal to 51 : FSM-REACH examined 41 states of 51 the register names
A5: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A6: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: [] != [('crates/demo/src/lib.rs', 5, 'LostState:[56 chars]re')]
A6: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A7: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Regex didn't match: '(?m)^FSM-LINT FSM-DEAD-END level=(warn|fail) examined=[1-9][0-9]* states hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$' not found in ''
A7: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A8: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: [] != [('crates/demo/src/lib.rs', 1, 'SinkState::Stuck')]
A8: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A9: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Regex didn't match: '(?m)^FSM-LINT FSM-TERMINAL level=(warn|fail) examined=[1-9][0-9]* terminals hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$' not found in ''
A9: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A10: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: [] != [('crates/demo/src/lib.rs', 3, 'HatchState[61 chars]ng')]
A10: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A11: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Regex didn't match: '(?m)^FSM-LINT FSM-DETERMINISTIC level=(warn|fail) examined=[1-9][0-9]* pairs hits=[0-9]+ new=[0-9]+ listed=[0-9]+ stale=[0-9]+$' not found in ''
A11: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A12: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: [] != [('crates/demo/src/lib.rs', 1, 'ForkState::Root on split')]
A12: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A13: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Void not raised
A13: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A14: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Refused not raised
A14: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A15: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Failed not raised
A15: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A16: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Lists differ: ['FSM[105 chars]n new crates/demo/src/lib.rs:2 GateState pub enum GateState {'] != ['FSM[105 chars]n new crates/demo/src/lib.rs:2 GateState']
A16: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A17: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: Refused not raised
A17: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A18: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: {'FSM-REACH': set(), 'FSM-DEAD-END': set(), 'FSM-TERMIN[34 chars]et()} != {'FSM-REACH': {'C'}, 'FSM-DEAD-END': set(), 'FSM-TERMIN[34 chars]et()}
A18: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
A19: red at 6576dc8f3840e8c8bd13a762c66cc21ae43f6d69: AssertionError: examined 0 FSM-REACH hits over the small tables: the population is empty, so nothing was judged
A19: green at d1c954412b3ea6d55cdccbd7b41f45aec14da8ff
```
