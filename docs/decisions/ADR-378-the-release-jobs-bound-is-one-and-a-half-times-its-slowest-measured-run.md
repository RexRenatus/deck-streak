---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-378: the release job's bound is one and a half times its slowest measured run, rounded up to the ten, held by a band

Decides SPEC-367.

## Context and Problem Statement

The `release` job runs the workspace's tests in the release profile, and the aggregate `ci` job
needs it (SPEC-330 R3). ADR-330 set its `timeout-minutes` to 60 and named a slice matrix as the
answer if CI's own timing showed the job near that. It does: over the 91 release jobs of the last
100 completed `ci` runs that ran to an end, the median took 51:54 and the slowest 55:45, and 14
reached 54:00 or more. The test step is 97.3% of the median job. Inside it, the release build takes
a median of 27:02 and the tests 23:27, of which one test, the progression census, takes 22:11.

Run 37660032517's first attempt, a pull request of one ruling file and one fragment, was cancelled
at the bound after 60:26. Its release build alone took 41:12, 1.48 times the same crates' 27:46 in
dev's run 37659069742, and the census was still running; projected at that ratio, the attempt needed
about 81 minutes. Its second attempt passed in 54:42. A job cancelled at its bound fails `ci`, so a
change that touches no code cannot land when it draws a slow runner.

## Decision Drivers

- The release job is a need of `ci`; a cancellation at its bound blocks every pull request,
  documentation-only ones too.
- No test leaves the step: SPEC-330 R2 requires every test the `test` stage runs.
- The bound is derived from measured runs by a stated rule, and a test holds it, as the engine and
  harness jobs' bounds are held.
- A hung test still ends in bounded time.

## Considered Options (the alternatives it was chosen against)

- **Chosen: `timeout-minutes` of 100, one and a half times the slowest measured job (60:26), rounded
  up to the ten, held inside 91 to 120 by a band test.**
- Re-running a cancelled job at 60: rejected because the same tree was cancelled at 60:26 and then
  passed in 54:42, so a re-run draws on the runner's speed, and every later pull request inherits
  the draw.
- A slice matrix of the test step, ADR-330's named answer: rejected because each slice pays the
  whole release build (median 27:02, 41:12 on the cancelled attempt's runner), and the census is one
  test that no slice can split: on that runner the slice holding it projects to about 79 minutes
  (1:57 of set-up, 41:12 of build, and the census's 24:01 at 1.48 times), past 60, while every slice
  adds a build.
- Taking the census out of the release step: rejected because SPEC-330 R2 requires the step to run
  every test the `test` stage runs, so it removes a check; making the census faster without skipping
  it is #692.
- Caching the release build: rejected as the fix because the bound must still hold a cold build: a
  cache key miss, at every lock-file change, returns the job to the measured build of 27:02 at the
  median and 41:12 at the slowest, which is the case the bound exists for. It may shorten the
  typical run later; it cannot replace the bound.
- Running the release job on pushes to `dev` only: rejected because it takes a check off every pull
  request, and a pull request would then land on a release-profile run it never had.
- A different runner for the job: rejected because ADR-017 holds the repository to hosted runners
  and which runner serves a job is not this delivery's to choose; the bound must hold the slowest
  runner the job is given.
- A bound of 90: rejected because it is one and a half times the slowest completed run (55:45, so
  83.6, rounded up), while the attempt cancelled at 60:26 is the slowest run measured, and its
  projected 81 minutes would leave 90 nine minutes.
- A bound of 130, one and a half times the projected 81 minutes: rejected because the projection is
  a scaling, not a measurement; 100 already holds it with 19 minutes to spare, and a looser bound
  only lets a hung test hold a runner longer.

## Decision Outcome

Chosen: a bound of 100 minutes, because it is the only option that keeps every test in the step,
holds the slowest run measured and its projection, and changes one line:

- the rule is one and a half times the slowest measured release job, rounded up to the ten; the
  factor is SPEC-327 R2's margin, and 60:26 times 1.5 is 90:39, so the bound is 100;
- `RELEASE_TIMEOUT_MINUTES` in `scripts/tests/test_ci_workflows.py` holds it inside 91 to 120: 91 is
  the rule's product rounded up to the minute, and 120 is twice the slowest measured run rounded
  down to the ten, so a hung test ends within two hours and a raise past it needs a new measurement;
- one mutation row sets the bound back to 60, and the band test kills it.

### Consequences

- Good: the attempt that was cancelled at 60:26 would have finished, by its projection of about 81
  minutes, with 19 minutes to spare, so a pull request that draws the slowest runner measured still
  lands.
- Good: the step and its tests are unchanged, so the release-profile check is exactly SPEC-330's.
- Bad: a hung test now holds a runner for up to 100 minutes before the job is cancelled, not 60.
- Neutral: the job's typical duration is unchanged; only its limit moves.

### Confirmation

SPEC-367's A1: the band test reads the job's `timeout-minutes` as a digit string inside 91 to
120, red at the base where it is 60, and its row `S36701` is killed in CI's `mutation-rows` job.
On every later run, the release job's duration stays under two thirds of the bound.

## More Information

- SPEC-330 R1 to R3 and its risk on the bound, ADR-330's bound and slice matrix, each amended by an
  amendment line naming this record; SPEC-327 R2 (the 1.5 margin); ADR-017 (hosted runners only).
- What would make this wrong: a completed release job past two thirds of the bound (66:40), which
  re-derives the bound by the same rule; the census made faster by #692, after which the bound may
  be re-derived downwards; or a cancellation at 100, which is a measurement this rule did not hold.
