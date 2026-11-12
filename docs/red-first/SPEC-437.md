# Red-first record: SPEC-437

SPEC-437 (R1 to R10, A1 to A17). The SPEC, ADR-451, the amendments to SPEC-418 and the Python
schematic, the Lean entry `formal/lean/Formal/RecordClassBinding.lean` with its writer and
vectors, and the tests of A1 to A17 were committed first and alone, over the base's production
code unchanged: `python_by_cost` cuts each file's runs as contiguous slices, `python_excuses`
binds every record against the judged class's population and `judge_python` keeps a class's
records only after binding, and the map names no module for `scripts/write-back-issues.py`. Each
red is therefore its criterion's assertion and never a missing name.
The test modules run only in CI, so every red below is quoted from the pull request's checks over
that commit, in the job `hygiene`, the job that runs `scripts/tests` and the parity oracle's tests.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_a_files_runs_are_interleaved_across_its_legs
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_the_interleave_keeps_every_count_and_projection_of_the_contiguous_cut
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_python_shard_binding.py -k test_every_listed_mutant_is_dealt_exactly_once_when_runs_are_interleaved
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_another_class_is_neither_bound_nor_stale
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_a_record_of_the_judged_class_that_binds_nothing_stays_stale
A6: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_a_known_key_is_written_as_its_issue_number
A7: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_an_unknown_key_is_named_and_nothing_is_written
A8: python3 -m unittest discover -s scripts/tests -p test_write_back_issues.py -k test_the_map_runs_this_module_for_the_write_back_script
A9: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_default_registry_names_every_committed_golden
A10: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_registration_is_refused_for_each_operand_alone
A11: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_dependency_the_predecessor_lacks_is_raised_by_its_name
A12: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_date_inside_a_dict_or_a_list_is_written_as_its_epoch_day
A13: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_whose_class_is_the_empty_name_is_refused
A14: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_a_case_input_holding_nan_is_refused_before_the_predecessor_runs
A15: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_main_requires_its_checkout_and_package_and_states_each_help_text
A16: python3 -m unittest discover -s tools/parity-oracle -p test_generate.py -k test_the_checkout_is_imported_before_any_other_path_and_gets_no_bytecode
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict_python_kills.py -k test_the_record_binding_agrees_with_every_vector_of_its_proof
```

The reds were read in job 117221337991, which ran 17 minutes 35 seconds, `hygiene` over the
red commit: `scripts/tests` read 21 failures, one each for the tests the fence above names for
A1, A4 and A8 and eighteen for A17's, one per vector it fails, and no other test failed. The
other thirteen criteria's tests are not among the failures, and the parity oracle's tests all
passed. Each red quotes its test's first `AssertionError` line as CI printed it, unittest's own
`[N chars]` cuts included. The greens were read in job 117324195447, which ran 1198 s, `hygiene`
over the pull request's second push, with no failure; A1's green names a later commit, below.

```red-first
A1: red at 51407e015a4abade65ba2df4ee4161bd1b5a4ddb: AssertionError: {'cou[80 chars]a.py:1:5: replace + with - in f', 'a.py:2:5: r[1066 chars]00}]} != {'cou[80 chars]a.py:2:5: replace + with - in f', 'a.py:4:5: r[1066 chars]00}]}
A1: green at 4eb123242a35f6f093518645c665ddd69ed51998
A2: not red: pins R2's figures, the leg count and each projection the base's contiguous deal already gives; it kills no row of section 9, so its proof is those figures and the second push's Python legs over the changed lines
A3: not red: pins the partition the base already has, every listed mutant dealt to one leg; its proof is S43701 read KILLED
A4: red at 51407e015a4abade65ba2df4ee4161bd1b5a4ddb: AssertionError: Tuples differ: (['STALE python.json record 1 (scripts/guar[61 chars]], 5) != ([], [], 5)
A4: green at 29c861c0ad9a4f0683b7394ba98b9c1669c722dc
A5: not red: pins the base's STALE reading of a judged-class record that binds nothing; its proof is S43702 read KILLED
A6: not red: pins the base's write of a known key as its issue number; its proof is S43706 and S43707 read KILLED
A7: not red: pins the base's refusal of an unknown key with no file written; its proof is S43704 and S43705 read KILLED
A8: red at 51407e015a4abade65ba2df4ee4161bd1b5a4ddb: AssertionError: {'dir': 'scripts/tests', 'modules': []} != {'dir': 'scripts/tests', 'modules': ['test_write_back_issues']}
A8: green at 29c861c0ad9a4f0683b7394ba98b9c1669c722dc
A9: not red: pins the base's default registry over every committed golden; its proof is S43708 read KILLED
A10: not red: pins the base's refusal of each registration operand alone; its proof is S43709 to S43715 read KILLED
A11: not red: pins the base's raise of a missing dependency by its name; its proof is S43716 read KILLED
A12: not red: pins the base's epoch day for a date inside a dict or a list; its proof is S43717 and S43718 read KILLED
A13: not red: pins the base's refusal of a case whose class is the empty name; its proof is S43719 read KILLED
A14: not red: pins the base's refusal of a NaN input before the predecessor's function is called; its proof is S43720 read KILLED
A15: not red: pins the base's required checkout and package flags and each help text; its proof is S43721 to S43725 read KILLED
A16: not red: pins the base's checkout first on the path with no bytecode written into it; its proof is S43726 and S43727 read KILLED
A17: red at 51407e015a4abade65ba2df4ee4161bd1b5a4ddb: AssertionError: Tuples differ: ({1: 'stale', 2: 'stale'}, []) != ({1: 'stale'}, [])
A17: green at 29c861c0ad9a4f0683b7394ba98b9c1669c722dc
```

Each row a `not red` line names was read KILLED by its id in the second push's `mutation-rows`
job, which runs each row's killer against its mutant.

Commit 4b0bbd5, between the red commit and the green, edits a test file,
`scripts/tests/test_mutation_python_shard_binding.py`, and changes no test of A1 to A17. It moves
eight pins of a contiguous membership, thirteen lines, to the interleaved membership by R2's rule,
every count and projection literal kept as written. Six are the pins R2 names. Two, in
`test_the_plan_deals_by_cost_and_refuses_whole_by_name` and
`test_the_plan_prices_scoped_exactly_when_the_legs_trace`, are outside them, and are moved under
section 6's risk for a contiguous pin outside the census.

At the second push one row outside this SPEC's band, S42101, read SURVIVED: its killer,
`test_a_triple_deals_by_its_setup_and_its_mutant_figure` in
`scripts/tests/test_mutation_python_shard_binding.py`, passed with a leg's size divided by a
file's control figure in place of its mutant figure, since the interleave deals that mutant's
eight runs of at most six onto the same two legs as the right two runs of twenty-two. Commit
4eb1232, after the second push, is a test-changing commit: it changes that test file alone, and
no production file, by adding a case whose setup leaves the right divisor nine mutants a leg and
the control figure none, so the mutant refuses. It changes no test of A1 to A17.

Each green line names the commit CI reads green for its criterion. At this record's commit,
`git diff --quiet 29c861c0 HEAD` reads rc 0 over A4's paths
(`scripts/tests/test_mutation_verdict_python_kills.py`, `scripts/mutation-verdict.py`), rc 0 over
A8's (`scripts/tests/test_write_back_issues.py`, `scripts/mutation-python.json`,
`scripts/write-back-issues.py`) and rc 0 over A17's
(`scripts/tests/test_mutation_verdict_python_kills.py`, `scripts/mutation-verdict.py`,
`formal/vectors/record-class-binding.jsonl`), so their greens name the second push's tip. It
reads rc 1 over A1's (`scripts/tests/test_mutation_python_shard_binding.py`,
`scripts/mutation-verdict.py`), whose test file commit 4eb1232 changes, so A1's green names that
commit, which the third push carries to its tip.

Section 3 says each of the thirteen `not red` criteria is the killer of its rows in section 9. A2
kills no row there, so its line names R2's figures and the second push's Python legs over the
changed lines as its proof: the second push's `mutation-python` leg over the two changed
production files, `scripts/mutation-verdict.py` and `scripts/write-back-issues.py`, examined 6
mutants and killed 6, its controls killed and 0 survived, and the `mutation-verdict` scripts
class read `examined 12: generated 6, rows 6` and `survived 0: equivalent 0, unexplained 0`, its
one finding the row S42101 named above.

The run over the red commit, push 1, concluded every job but its `ci` aggregator, whose
conclusion the expected `hygiene` red fixes: that aggregator is not concluded, superseded by push 2.
