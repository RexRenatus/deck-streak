# Red-first record: SPEC-379

A1's test is committed before the two lines it judges. It runs in CI's `hygiene` job, which runs
`scripts/tests`; it is read red there at the base bounds of 60, and green once both are 100. A2
plants its own bounds into copies of both workflow files, so it pins the checker and cannot be red
at the base; rows S37900 to S37905 read A1 red against each planted defect.

```red-first
A1: red at <C1 sha>: <the failure line, quoted from CI>
A2: not red: it plants its own bounds into copies of both workflow files, so it pins the checker and not the tree
```
