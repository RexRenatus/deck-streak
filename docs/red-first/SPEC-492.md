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
A1: red at 7fabf64e9af3cb8b82655bcc4fcb0964046899f1: AssertionError: 0 != 1
A1: green at 1a3e8fe8734486e2737ccb9f8152fa453f056831
A2: red at 7fabf64e9af3cb8b82655bcc4fcb0964046899f1: AssertionError: 0 != 1 : exactly one record names scripts/public-scrub.py
A2: green at 1a3e8fe8734486e2737ccb9f8152fa453f056831
A3: red at 7fabf64e9af3cb8b82655bcc4fcb0964046899f1: AssertionError: Lists differ: ['382: end < 0', '388: end < 0', '526: sto[226 chars]ect"] != []
A3: green at 1a3e8fe8734486e2737ccb9f8152fa453f056831
A4: red at 7fabf64e9af3cb8b82655bcc4fcb0964046899f1: AssertionError: Lists differ: ["declared_spellings: rest.split(',', 1)[0[264 chars]-')"] != []
A4: green at 1a3e8fe8734486e2737ccb9f8152fa453f056831
A5: not red: resolve_test is new and answers as resolve_killer does over the same cells, so its red before the cure would be a missing name, not this criterion's reason
A6: not red: the verdict script's kill tests pin behaviour the base already had; each one's kill is its mutant's, read by the mutation legs
A7: not red: the rows runner's kill tests pin behaviour the base already had; each one's kill is its mutant's, read by the mutation legs
A8: not red: the configuration and exclusion checks' tests pin behaviour the base already had
A9: not red: the scrub's tests pin behaviour the base already had
```

The red lines are quoted from push 1's `hygiene` job (job 118390538458) at the red commit 7fabf64e. Each green line
names the cure commit 1a3e8fe8, which also adds A5's `TheResolveTest` to
`scripts/tests/test_mutation_rows_survivor_kills.py`. CI read A1 to A4 green at push 2's head 1eb2c625 (`hygiene`, job
118414623787), and `git diff --quiet 1a3e8fe8 1eb2c625` exits 0 over `scripts/mutation-python.json`,
`scripts/mutation-verdict.py`, `scripts/mutation_rows.py`, `scripts/mutation-equivalent.d/python.json`, `scripts/public-scrub.py`
and the four test modules these criteria run, so that run reads the cure commit's tests and code. A later commit appends one
class to `scripts/tests/test_mutation_verdict_survivor_kills.py` that reaches the four not-found branches the cure's comparisons
left unread. It pins behaviour the base already had, under A6, and leaves every earlier statement of that module unchanged.
