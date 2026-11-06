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
```
