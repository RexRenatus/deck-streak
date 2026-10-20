# SPEC-492 red-first record

A1 to A4 are red at the red commit, before the cure, and green after it. A5 to A9 pin behaviour the base already had, so each is
`not red`. The test modules run in CI, so every red and green line is quoted from the pull request's checks, from the job that runs
`scripts/tests`, by the round that reads them, and joins the `red-first` fence below.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_survivor_kills.py -k test_the_map_names_each_kill_module_once_for_its_file
A2: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_the_flags_default_is_excused_by_one_whole_record
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_survivor_kills.py -k test_the_verdict_keeps_none_of_its_equivalent_shapes
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_rows_survivor_kills.py -k test_the_rows_runner_keeps_none_of_its_equivalent_shapes
```

```red-first
A5: not red: resolve_test is new and answers as resolve_killer does over the same cells, so its red before the cure would be a missing name, not this criterion's reason
A6: not red: the verdict script's kill tests pin behaviour the base already had; each one's kill is its mutant's, read by the mutation legs
A7: not red: the rows runner's kill tests pin behaviour the base already had; each one's kill is its mutant's, read by the mutation legs
A8: not red: the configuration and exclusion checks' tests pin behaviour the base already had
A9: not red: the scrub's tests pin behaviour the base already had
```
