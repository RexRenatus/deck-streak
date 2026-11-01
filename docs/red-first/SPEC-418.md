# Red-first record: SPEC-418

SPEC-418 (R1 to R12, A1 to A11). The SPEC, ADR-432, the amendments to SPEC-087, SPEC-362, ADR-373
and the Python schematic, and the tests of A1 to A11 with the two census tuples that admit A10's
call of `verdict_module()` and its read of `module.PYTHON_LEG_BOUND_SECONDS` (the census lists
both sites for the Rust bound's test beside it, so A10 needs both), were committed first and
alone, over stubs that keep the base
behaviour: `PYTHON_LEG_BOUND_SECONDS` at 3600, an empty `PYTHON_CONTROL_SECONDS`, a
`python_by_cost` that returns None, a `planned_leg` that returns `k::n`, and a `prices` verb that
examines nothing. Each red is therefore its criterion's assertion and never a missing name.
The test modules run only in CI, so every red below is quoted from the pull request's checks over
that commit, in the job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_priced_listing_is_dealt_by_cost_into_the_fewest_legs_within_the_bound
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_the_cost_deal_cannot_fit_is_refused_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_with_no_priced_file_keeps_the_count_rule
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_unpriced_mutants_keep_twenty_a_leg_beside_the_priced_legs
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_plan_deals_by_cost_and_refuses_whole_by_name
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_leg_runs_the_names_its_plan_deals_it
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_a_plan_whose_python_legs_do_not_partition_the_listing_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_python.py -k test_each_files_control_cost_is_recorded_beside_the_report_before_its_mutants
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_prices_verb_derives_each_files_price_from_the_recorded_runs
A10: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_python_bound_is_half_of_the_python_legs_timeout
A11: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_cost_deal
```

The reds were read in job 114685530289, which ran 17 minutes 49 seconds, `hygiene` over the
red commit: ten failures, each the test the fence above names for A1, A2 and A4 to A11, and no
other, while A3 passed there. Each red quotes its test's first `AssertionError` line as CI
printed it, unittest's own `[N chars]` cuts included. The greens are read in the checks over the
green commit.

```red-first
A1: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: None != {'count': 3, 'shards': [{'shard': 0, 'mut[2999 chars]20}]}
A1: green at a423e79df0b3e9464b7a365d117bd711470811b5
A2: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: None != "mutation: shards: REFUSED: d.py's contro[92 chars]ound"
A2: green at a423e79df0b3e9464b7a365d117bd711470811b5
A3: not red: pins the count rule the base already has for a listing with no priced file
A4: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: None != {'count': 5, 'shards': [{'shard': 0, 'mut[2784 chars]00}]}
A4: green at a423e79df0b3e9464b7a365d117bd711470811b5
A5: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: 'muta[99 chars]rds: 1 shard(s) for 0 listed mutant(s), projec[84 chars]93\n' != 'muta[99 chars]rds: python legs dealt by cost: 2 priced leg(s[330 chars]00\n'
A5: green at a423e79df0b3e9464b7a365d117bd711470811b5
A6: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: Lists differ: [['sc[85 chars]y:2:16: replace 2 with 3 in double'], ['script[40 chars]le']] != [['sc[85 chars]y:2:14: replace * with / in double'], ['script[40 chars]le']]
A6: green at a423e79df0b3e9464b7a365d117bd711470811b5
A7: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: 0 != 2 : the count: mutation-python: killed 2, survived 0, uncovered 0, unviable 0, timeout 0, void 0
A7: green at a423e79df0b3e9464b7a365d117bd711470811b5
A8: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: unexpectedly None : no costs.jsonl beside the report
A8: green at a423e79df0b3e9464b7a365d117bd711470811b5
A9: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: '' != '    "scripts/a.py": 36,  # run 4242 attem[185 chars] 1\n'
A9: green at a423e79df0b3e9464b7a365d117bd711470811b5
A10: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: 3600 != 1800
A10: green at a423e79df0b3e9464b7a365d117bd711470811b5
A11: red at 8b545d2ca59c4e3d39666e093d1827e0f9cdce2b: AssertionError: Lists differ: ["the Python legs are dealt by each file's[110 chars]418'] != []
A11: green at a423e79df0b3e9464b7a365d117bd711470811b5
```

A3 pins the count rule the base already has: a listing with no priced file reads None from the
deal, so `shards` keeps SPEC-415's matrix byte for byte, and the shipped price table is empty. It
is recorded `not red`, with that reason, and holds the deal's no-price arm against a change.
