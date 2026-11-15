# Red-first record: SPEC-624

The criterion's test is committed before the change it judges. It runs in the `harness` job of
the Apple body, on the iPhone simulator and the iPad simulator, in the pull request's own run.

```red-first
A1: red at 1d74882: failed - A1 watchdog: the start wait over a listener pair that never gets ready, with `seconds: 2`, had not returned by the end of the watchdog, `seconds: 10`, so the start wait has no bound (job 128006463373, both simulators)
A1: green at 2c6d1e5
```

A1 was committed with the start bound as an inert seam that waits with no bound (95b9591), then
carried onto current dev by a merge (1d74882), where it read red by its own watchdog on both
simulators in the pull request's own run. The bound came in 2c6d1e5, with the test and its seam
unchanged, and the run at this delivery's head reads it green on both simulators. The commits
after 2c6d1e5 change only documents.
