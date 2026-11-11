# Red-first record: SPEC-632

SPEC-632 (R1 to R7, A1 to A10). The SPEC, ADR-655, the schematic, the fragment and the two
amendment sections were committed first, then the census tests of A1 and A2 alone, over the base's
`scripts/check.sh` and `.github/workflows/ci.yml`: no census set is defined there and `stage_test`
filters `not ($ENGINE_TESTS)`, so A1 and A2 read red by their assertions. The stage and the job
came next, over the gate's unchanged pins, which read red for the new stage and job; the pins were
then amended. The ceiling's pins were restated at 184 over the unchanged 185, and read red, before
the ceiling itself moved. Each red below is quoted from the run that read it.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_census_once.py -k test_the_census_set_runs_once_in_its_own_job
A2: python3 -m unittest discover -s scripts/tests -p test_census_once.py -k test_census_set_spawns_its_own_cargo
A3: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_the_gate_has_no_toolchain_stage_and_splits_the_audit
A4: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_both_test_stages_take_the_engine_set_from_its_one_definition
A5: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k test_the_engine_stage_builds_only_the_targets_the_set_names
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_gate_runs_in_four_parallel_jobs -k test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile -k test_every_job_that_compiles_rust_installs_the_pinned_protoc_first
A7: python3 -m unittest discover -s scripts/tests -p test_dispatch_shards.py -k test_the_ceiling_is_the_runs_job_budget
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k test_a_listing_that_needs_exactly_the_shard_cap_is_sized_at_the_cap -k test_the_sizing_constants_are_the_measured_ones -k test_the_whole_tree_is_sized_at_the_batterys_ceiling
A9: python3 -m unittest discover -s scripts/tests -p test_release_rust_ceiling.py
A10: python3 -m unittest discover -s scripts/tests -p test_release_python_ceiling.py -k test_the_placeholder_ceiling_deals_the_release_run_as_a_pull_request_run
```

```red-first
A1: red at 312c9828: scripts/tests/test_census_once.py:36: AssertionError: check.sh defines the census set 0 time(s), not once (job 127930408601)
A1: green at fd64a502
A2: red at 312c9828: scripts/tests/test_census_once.py:36: AssertionError: check.sh defines the census set 0 time(s), not once (job 127930408601)
A2: green at fd64a502
A3: red at fd64a502: scripts/tests/test_check_gate.py:146: AssertionError: Lists differ: ['aud[78 chars]', 'test', 'test-census', 'test-engine', 'test-release', 'web'] != ['aud[78 chars]', 'test', 'test-engine', 'test-rel (job 128115597850)
A3: green at 265c13f5
A4: red at fd64a502: scripts/tests/test_check_gate.py:588: AssertionError: Lists differ: ["test (as written) runs the filterset ['n[406 chars])']"] != [] (job 128115597850)
A4: green at 265c13f5
A5: not red: test_check_gate.py at fd64a502 ran 11 tests and failed A3 and A4 only (box run); the target test reads only the engine stage, which that commit leaves unchanged
A6: red at fd64a502: scripts/tests/test_ci_workflows.py:3204: AssertionError: {'rus[52 chars]'], 'census': ['test-census'], 'engine': ['tes[107 chars]ts']} != {'rus[52 chars]'], 'engine': ['test-engine'], 'release': ['te[ (job 128115597850)
A6: green at 265c13f5
A7: red at fd64a502: scripts/tests/test_dispatch_shards.py:338: AssertionError: {'ci': 185, 'battery': 234} != {'ci': 184, 'battery': 234} (job 128115597850)
A7: green at 2cc727b4
A8: red at 2af568e0: scripts/tests/test_mutation_verdict.py:870: AssertionError: 185 is not None : ci (box run)
A8: green at 2cc727b4
A9: red at 2af568e0: scripts/tests/test_release_rust_ceiling.py:245: AssertionError: None != 185 : a true mark one past the placeholder (box run)
A9: green at 2cc727b4
A10: red at 2af568e0: scripts/tests/test_release_python_ceiling.py:345: AssertionError: Tuples differ: (0, '[366 chars]ng 185\nmutation: shards: run legs 4 of 217: p[68 chars]0\n') != (0, '[366 chars]ng 184\nmutation: shar (box run)
A10: green at 2cc727b4
```

The census tests of A1 and A2 were committed over the base's `scripts/check.sh` and
`.github/workflows/ci.yml`, and the stage and the job that followed changed no test file. A3,
A4, A6 and A7 read red over the stage and the job, by name in the gate python stage of the pull
request run. The gate's census pins were then amended in 265c13f5, in
`scripts/tests/test_check_gate.py` and `scripts/tests/test_ci_workflows.py`, with the census set
beside the engine set and no assertion removed, and A3, A4 and A6 read green there. The ceiling's
pins were restated at 184 in 2af568e0, over the unchanged 185, where A8 to A10 read red; the
ceiling and its row then moved with every test unchanged, and A7 to A10 read green. The commits
after that change one workflow line and this record.
