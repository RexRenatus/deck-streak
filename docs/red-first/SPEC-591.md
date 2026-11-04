# Red-first record: SPEC-591

Each criterion's test is committed before the change it judges. They run in CI's `hygiene` job, which runs
`scripts/tests`; each was read red there with the workflow unsplit, and green once the job is split.

```red-first
A1: red at d4656991: AssertionError: 'release-and-app' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A1: green at c7951852
A2: red at d4656991: AssertionError: Lists differ: ['harness: it holds the measurements, Rele[1013 chars]app'] != [] (job 125875260773)
A2: green at c7951852
A3: red at d4656991: AssertionError: 'release-and-app' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A3: green at c7951852
A4: red at d4656991: AssertionError: 'verdict' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A4: green at c7951852
A5: red at d4656991: AssertionError: Lists differ: ['release-and-app: no bound line of its ow[35 chars]own'] != [] (job 125875260773)
A5: green at c7951852
A6: red at d4656991: AssertionError: 'release-and-app' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A6: green at c7951852
A7: red at d4656991: AssertionError: Lists differ: ['release-and-app: 0 report steps, not one[179 chars]pp"'] != [] (job 125875260773)
A7: green at c7951852
A8: red at d4656991: AssertionError: 'release-and-app' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A8: green at c7951852
A9: red at d4656991: AssertionError: 'release-and-app' not found in ['xcframework', 'harness-wire', 'card-isolation', 'harness'] (job 125875260773)
A9: green at c7951852
```

Commit c795185, the green commit, also edits `scripts/tests/test_ci_workflows.py` after the red commit d465699.
It gives the four readers `harness_warm_problems`, `app_steps_problems`, `review_step_problems` and
`review_actions_step_problems` a job argument that defaults to `harness`, and rewrites their four live
assertions to pass the job each now reads. It adds and removes no test method and no planted control.
