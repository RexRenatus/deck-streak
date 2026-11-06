# Red-first record: SPEC-631

SPEC-631 (R1 to R8, A1 to A3). The SPEC, ADR-654, the schematic and the changelog fragment were
committed first, then the test of A1 alone, over the base's `ci.yml` unchanged: the `release` job's
only cache restore is the debug key `rust-${{ runner.os }}-...`, so A1 reads the restore's key and
finds it is not the `rust-release-` key. A1's red is therefore its criterion's assertion and never a
missing file. The change to `ci.yml` follows, and the two census tests of A2 and A3 are each seen red
against it before their amendment: A2 because the `release` job's restore is not the lookup key of
`rust-cache.yml`, A3 because the workflow then holds two saves where the test counted one. The tests
run in the `hygiene` job's Python stage, and each red below is quoted from that job's log.

```text
A1: python3 -m unittest discover -s scripts/tests -p test_release_cache.py -k test_the_release_job_restores_and_saves_its_own_profile_key
A2: python3 -m unittest discover -s scripts/tests -p test_rust_cache_workflow.py -k test_its_key_and_paths_are_the_ones_ci_and_the_weekly_battery_restore
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_rust_cache_is_keyed_on_the_toolchain_pin_and_the_lockfile
```

```red-first
```
