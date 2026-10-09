# Red-first record: SPEC-379

A1's test is committed before the two lines it judges. It runs in CI's `hygiene` job, which runs
`scripts/tests`. At `70bcd637`, where both bounds are 60, CI job 113866368200 read it red: one
failure in 1092 tests, A1 alone, by its own assertion, whose two problems name both legs (CI):

```
- ["mutation-weekly.yml: the web job's timeout is 60, not 100 to 120 minutes",
-  "ci.yml: the mutation-web job's timeout is 60, not 100 to 120 minutes"]
```

`35a058db` raises both bounds to 100 and changes no other line; A1's green there is read by CI on
the pull request's head. A2 plants its own bounds into copies of both workflow files, so it pins
the checker and cannot be red at the base; rows S37900 to S37905 read A1 red against each planted
defect.

```red-first
A1: red at 70bcd637: AssertionError: Lists differ: ["mutation-weekly.yml: the web job's timeo[101 chars]tes"] != []
A1: green at 35a058db
A2: not red: it plants its own bounds into copies of both workflow files, so it pins the checker and not the tree
```
