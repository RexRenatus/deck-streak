# Red-first record: SPEC-367

A1's test is committed before the line it judges. It runs in CI's `hygiene` job, which runs
`scripts/tests`; it was read red there at the base bound of 60, and green once the bound is 100.

```red-first
A1: red at fa158ccd: AssertionError: False is not true : the release job's timeout is 60, not 91 to 120 minutes
A1: green at 7bd26052
```
