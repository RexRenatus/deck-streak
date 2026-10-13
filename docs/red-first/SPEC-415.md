# Red-first record: SPEC-415

SPEC-415 (R1 to R9, A1 to A4). The SPEC, ADR-429, the amendments to SPEC-087, SPEC-362, ADR-373
and the Python schematic, and the tests of A1 to A4 with the two changed expectations, were
committed first and alone (`f1ac7422dfa9cd8d80ea3e414a57a5ab3318ed97`), over a
`scripts/mutation-verdict.py` that still clamped the Python matrix to 32 legs and a `RELEASING.md`
that named no Python ceiling.
The test modules run only in CI, so every red below is quoted from the pull request's checks over
that commit, run 38171789092, job `hygiene`, the job that runs `scripts/tests`.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_listing_past_thirty_two_legs_is_refused_whole_and_never_capped
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_a_python_listing_past_the_ceiling_refuses_the_plan_whole_by_name
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_verdict.py -k test_the_plan_prints_its_python_legs_against_their_ceiling
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_runbook_names_the_python_ceiling_and_its_whole_refusal
```

```red-first
A1: red at f1ac7422dfa9cd8d80ea3e414a57a5ab3318ed97: AssertionError: 32 is not None : 641 listed: never capped
A2: red at f1ac7422dfa9cd8d80ea3e414a57a5ab3318ed97: shard_the_plan's `assert done.returncode == expect` failed, the plan exiting 0 where 1 was expected and printing `mutation: shards: 32 python shard(s) for 641 listed mutant(s)`
A3: red at f1ac7422dfa9cd8d80ea3e414a57a5ab3318ed97: AssertionError: 'mutation: shards: 4 python shard(s) for 68 listed mutant(s); python legs 4 of ceiling 32\n' not found in 'mutation: shards: 4 python shard(s) for 68 listed mutant(s)\n...'
A4: red at f1ac7422dfa9cd8d80ea3e414a57a5ab3318ed97: AssertionError: '`python legs N of ceiling 32`' not found in '## 2. The release pull request Open the release pull request from `dev` into `main`, ...'
A1: green at 191f24e00288247d316d8dc13f97fd4a544476dd
A2: green at 191f24e00288247d316d8dc13f97fd4a544476dd
A3: green at 191f24e00288247d316d8dc13f97fd4a544476dd
A4: green at 191f24e00288247d316d8dc13f97fd4a544476dd
```

Each red is an assertion on the missing behaviour: A1 on the 32-leg matrix the base dealt for 641
listed mutants where the refusal is due, A2 on the base plan's exit 0 over the same listing, A3 on
the sized line that ends at "listed mutant(s)", and A4 on the runbook's section 2, which named no
Python ceiling. The whole `scripts/tests` suite over that commit ran 1130 tests and failed 6: A1 to
A4 and the two changed expectations below, and nothing else.

Two existing tests keep their names and change one expectation each, both stricter, and are
recorded here in prose rather than as criteria:

- `test_the_matrix_takes_twenty_listed_mutants_a_shard_clamped_to_thirty_two`
  (`test_mutation_python_shard_binding.py`, SPEC-362 A12): its last case, 32 x 20 + 1 listed, moves
  from 32 legs to refused. It was red over `f1ac7422` on `assertIsNone(sized, ...)`, the base
  dealing `{'count': 32, ...}` for 641 listed, and it is green at `191f24e0`.
- `test_the_plan_sizes_the_python_matrix_from_its_listing` (`test_mutation_python_verdict.py`):
  641 and 2000 listed move from 32 legs to refused. It was red over `f1ac7422` on the helper's
  returncode assertion for 641, the base plan exiting 0 and printing
  `mutation: shards: 32 python shard(s) for 641 listed mutant(s)`, and it is green at `191f24e0`.

The green is the commit `191f24e00288247d316d8dc13f97fd4a544476dd`, which changes
`scripts/mutation-verdict.py` and `RELEASING.md` only, over the tests `f1ac7422` added and left
unchanged since. The commit that carries this record adds no code and no test: it holds this record,
the band of rows S41500 to S41507, the model's abstraction comment and digest, and the changelog
fragment. The greens are read in the pull request's checks at the push that carries this record, in
the same job by name.
