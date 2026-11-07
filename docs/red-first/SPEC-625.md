# Red-first record: SPEC-625

SPEC-625 (R1 to R5, A1 to A5). The SPEC, ADR-647, the schematic and the fragment were committed
first, then the tests of A1 to A5 alone, over the base's production code unchanged: `judge_rows`
credits a KILLED row only to the class of its changed line's path and names no row that no class
credits, so the scripts class prints no `left to the leg report` line for any input. A1 and A2
are therefore red by their assertion on those lines, never by a missing name; A3, A4 and A5 assert
what the base already holds and are green there.
The test module runs in CI's `hygiene` job, the job that runs `scripts/tests`, so every red below
is quoted from that job's log over the pull request's first push.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_killed_row_on_a_changed_map_line_is_named_left_to_the_leg_report
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_only_the_scripts_class_names_and_only_rows_no_class_credits
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_no_left_line_prints_when_every_selected_row_is_credited
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_row_the_leg_did_not_kill_fails_and_is_never_named_left
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_left_row_adds_nothing_to_the_examined_count
```

```red-first
A1: red at 139828b780d948c10b5bc3499cc1a3c6e674532b: AssertionError: Lists differ: [] != ['mutation: scripts: rows left to the leg [134 chars]son']
A1: green at 635ab5da6e2d8432496679e91ee1cd2d40766bcb
A2: red at 139828b780d948c10b5bc3499cc1a3c6e674532b: AssertionError: Lists differ: [] != ['mutation: scripts: rows left to the leg [228 chars]ine']
A2: green at 635ab5da6e2d8432496679e91ee1cd2d40766bcb
A3: not red: the base prints no left line when every selected row is credited to its class; its proof is S62504 read KILLED
A4: not red: the base fails a row the leg did not kill by name and names it on no other line; its proof is S62503 read KILLED
A5: not red: the base reads the zero-examined VOID when the only selected row is credited to no class; its proof is S62506 read KILLED
```

The rows S62500 to S62506 were read KILLED by id in the second push's `mutation-rows` job.
