# Red-first record: SPEC-608

Each criterion's test is committed before the change it judges. They run in CI's `hygiene` job, which
runs `scripts/tests`; each was read red there with the report unchanged, and green once the report
reads the planted suite bundle.

```red-first
A1: red at d6d59e22: AssertionError: Lists differ: ['harness: the report lacks cases_in("card[311 chars]ss"'] != [] (job 126735655393)
A1: green at f7db2e10
A2: red at d6d59e22: AssertionError: Lists differ: [] != ['| Card probe `CardProbeTests/test_a_plan[82 chars]d |'] (job 126735655393)
A2: green at f7db2e10
```
