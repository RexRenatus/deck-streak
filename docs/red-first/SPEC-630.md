# SPEC-630: the red-first record

```red-first
A6: not red: its two tests are the controls beside the refusal arms, a derived_at one release behind and the report-only exit, which the stub reads green at both pushes
A1: red at 3a44a986: test_perf_census.py:291: AssertionError: 0 != 1 : perf-census: derive wrote 0 members at 90%, 0 benches, 0 files (job 127887413253)
A1: green at cfbedcf7
A2: red at 3a44a986: test_perf_census.py:370: AssertionError: 'perf-census: check examined 0 members, 0[52 chars]d)\n' != "perf-census: shape: hot-set.json has the[114 chars]d)\n" (job 127887413253)
A2: green at cfbedcf7
A3: red at 3a44a986: test_perf_census.py:370: AssertionError: 'perf-census: check examined 0 members, 0 defined bench[38 chars]d)\n' != 'perf-census: shape: OK\nperf-census: benches: the tree[514 chars]d)\n' (job 127887413253)
A3: green at cfbedcf7
A4: red at 3a44a986: test_perf_census.py:370: AssertionError: 'perf-census: check examined 0 members, 0 defined bench[38 chars]d)\n' != 'perf-census: shape: OK\nperf-census: benches: the tree[436 chars]d)\n' (job 127887413253)
A4: green at cfbedcf7
A5: red at 3a44a986: test_perf_census.py:370: AssertionError: 'perf-census: check examined 0 members, 0 defined bench[38 chars]d)\n' != 'perf-census: shape: OK\nperf-census: benches: the tree[459 chars]d)\n' (job 127887413253)
A5: green at cfbedcf7
```

A1 to A5 were committed with a stub script that keeps every argument and lacks each
behaviour (3a44a986); each read red by name in the gate python stage of the pull request run.
The script replaced the stub in cfbedcf7, with every test unchanged, and the run at this
delivery head reads them green. The commits after cfbedcf7 change only documents.
