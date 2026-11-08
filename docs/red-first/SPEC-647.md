# Red-first record: SPEC-647

SPEC-647, ADR-670, the schematic, the changelog fragment and the SPEC-025 amendment were
committed first and alone (089b9a7f). The commit that adds this record carries the six
criteria's tests with the runner as it was: A1, A2, A4 and A5 read red there by name, each
by assertion, and A3 and A6 are not red, as ADR-670 D4 says.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_while_the_killer_starts_is_held_until_its_group_can_end
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_second_signal_during_the_cleanup_is_ignored_whichever_came_first
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_runner_started_with_sigint_ignored_keeps_it_ignored
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_main_takes_sigint_and_restores_its_handler_on_every_way_out
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_held_while_a_killer_fails_to_start_still_ends_the_run
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigint_of_the_runner_still_ends_it_by_sigint_through_its_cleanup
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_group.py -k test_sigint_to_the_runner_ends_the_killers_group
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sighup_of_the_runner_ends_the_killers_group
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_sigterm_while_the_killer_starts_is_held_until_its_group_can_end
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_second_sigterm_during_the_cleanup_is_ignored_and_the_group_ends
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k test_a_runner_started_with_sighup_ignored_keeps_it_ignored
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_rows.py -k MainLeavesTheProcessAsItFoundIt
```

```red-first
A3: not red: a runner started with SIGINT ignored already kept it ignored at the base, where no handler of the run took SIGINT; this test guards that reading against the new installation
A6: not red: a SIGINT of the runner already ended the killer group and the run by SIGINT at the base, and the other signal edges were already held; these tests guard both against the cure
```
