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
A1: red at a56eb518: scripts/tests/test_release_cache.py:93: AssertionError: "rust-${{ runner.os }}-${{ hashFiles('rust-tool[42 chars]) }}" != "rust-release-${{ runner.os }}-${{ hashFiles('r[50 chars]) }}" (job 127918635594)
A1: green at 13383500
A2: red at 13383500: scripts/tests/test_rust_cache_workflow.py:154: AssertionError: "rust-${{ runner.os }}-${{ hashFiles('rust-tool[42 chars]) }}" != "rust-release-${{ runner.os }}-${{ hashFiles('r[50 chars]) }}" (job 127945666753)
A2: green at 80b02cde
A3: red at 13383500: scripts/tests/test_ci_workflows.py:2964: AssertionError: 2 != 1 : the Rust cache is saved by 2 steps (job 127945666753)
A3: green at 80b02cde
```

A1 was committed over the base's `ci.yml` (a56eb518), and A2 and A3 over the changed `ci.yml` with
their tests as they were (13383500); each read red by name in the `hygiene` job's Python stage of
the pull request run. The change to `ci.yml` in 13383500 turned A1 green. The two census tests were
amended in 80b02cde: A2 now compares every restore but the release job's with the lookup key and
holds the release job's own key apart, and A3 now counts one save per profile key, each by the job
that restores it, after the clean of its profile. Every old assertion stays or is restated per
key, and both read green; the run at this delivery head reads all three green. The commits after
80b02cde change only documents.
