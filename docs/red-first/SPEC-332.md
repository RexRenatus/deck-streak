# Red-first record: SPEC-332

The SPEC, ADR-333 and the test module were committed first (1e9dacdf) with the three workflows
untouched. At that commit the two enumerating tests fail by assertion, naming each of the three
jobs, and the seven planted-shape controls pass (`FAILED (failures=2)`, 9 tests, `examined 3 jobs
that run mutation_rows.py prove`).

```red-first
A1: red at 1e9dacdf: AssertionError: Lists differ: [] != ['ci.yml:mutation-rows: no step runs `cargo fetch --locked`', 'mutation-weekly.yml:rows: no step runs `cargo fetch --locked`', 'mutation-weekly.yml:rehearsal: no step runs `cargo fetch --locked`']
A2: red at 1e9dacdf: AssertionError: Lists differ: [] != ['ci.yml:mutation-rows: no fetch step to judge', 'mutation-weekly.yml:rows: no fetch step to judge', 'mutation-weekly.yml:rehearsal: no fetch step to judge']
A1: green at a3bbef38
A2: green at a3bbef38
```

The mutation rows S33200 to S33202 prove the tests can fail on the wrong workflow: all three read
KILLED (`rows: examined 3: killed 3, survived 0, void 0`).
