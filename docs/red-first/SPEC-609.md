# Red-first record: SPEC-609

Each criterion's test is committed before the change it judges. Both run in the `harness` job of
the Apple body, on the iPhone simulator and the iPad simulator, in the pull request's own run.

```red-first
A1: red at 43e4aab: failed - A1 watchdog: the ask of a silent page with `seconds: 2` had not returned by the end of the watchdog, `seconds: 10`, so the ask has no bound (job 126563391319, both simulators)
A1: green at cf5d03d
A2: red at 43e4aab: failed - A2 watchdog: the load wait over a page that falls silent, with `askSeconds: 2`, had not returned by the end of the watchdog, `seconds: 12`, so its ready ask has no bound (job 126563391319, both simulators)
A2: green at cf5d03d
```

A1 and A2 were committed with the bounded ask as an inert seam that waits with no bound (43e4aab),
and read red by their own watchdogs on both simulators in the pull request's own run. The bound came
in cf5d03d, with both tests unchanged, and the run at this delivery's head reads them green on both
simulators. The commits after cf5d03d change only documents.
