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

## Amendment, 2026-10-31: the band is 325 to 420

A1's criterion changed, so its new red is recorded here in prose and the fence above is kept as it
was. `eb207661` changes only `scripts/tests/test_ci_workflows.py`: `WEB_MUTATION_TIMEOUT_MINUTES`
becomes 325 to 420, and A2 also plants a bound one under the floor. Both workflow bounds are still
100 there. A local run of `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py`
at `eb207661` read one failure in 70 tests, A1 alone, by its own assertion, whose two problems name
both legs:

```
- ["mutation-weekly.yml: the web job's timeout is 100, not 325 to 420 minutes",
-  "ci.yml: the mutation-web job's timeout is 100, not 325 to 420 minutes"]
```

The commit that adds this section sets both bounds to 325 and changes no other workflow line; the
same local run read 70 tests OK there, and CI reads A1 green on the pull request's head. A2 passed
at both commits, 8 planted bounds examined: it plants its own bounds, so it is still not red.
