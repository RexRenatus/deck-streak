# Red-first record: SPEC-361

Every criterion's test is committed before the code it judges, against stubs that compile. A6 runs
locally and in CI's `hygiene` job; A1 to A4 run on the macOS host, in the `card-isolation` job's
`swift test`; A5 and A7 to A15 run only on the simulators, in the `harness` job's `CardProbe` step,
which tests the iPhone simulator and then the iPad simulator in one invocation. A simulator
criterion is red or green only when it is so on both destinations of one run.

```red-first
```
