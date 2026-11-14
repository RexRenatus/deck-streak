---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-692: A suite's time is read from the stage logs and held to a listed bound, at warn

Decides SPEC-669. Follows ADR-450 D6 for its level and its promotion.

## Context

CI's stage logs already hold a time per suite for four runners: each Rust test's own duration on
its result line, each Python module's `Ran N tests in Xs` line before its `module:` line, each
web unit file's duration on its file line, and each crate's doc-test result lines after its
`Doc-tests` line (run 42582016940: jobs 129090178681, 129090178646 and
129090236405). No check reads them. The figures move between runs: over eight runs, two runs show
every large Rust binary at about two thirds of its figure in the other six (runs 42552326063 and
42541868033), and two suites move by more than twice their lowest figure.

## Decision

- D1. The record is the stage logs CI uploads, read by each runner's own form (SPEC-669 R2). A Rust
  binary's figure is the sum of its tests' durations, so the order the runner schedules them in
  does not move it.
- D2. The bound of a suite is the figure one CI job recorded, listed in `scripts/suite-times.json`
  with that job's id. Its margin is the larger of 100 % of the bound and 30 s, unless its entry
  states its own margin with its reason.
- D3. A site is reported at warn. Promotion to fail follows ADR-450 D6, by its own delivery.
- D4. A faster figure is reported; the bound is lowered only by a delivery that lists it.
- D5. A browser suite or a Swift test target is in the population and is reported as having no
  time recorded in `ci`, never passed.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| a step that fails a run over a time limit | the figures move between runs on the same tree, so a hard limit fails runs the change did not slow |
| a median over several runs as the bound | it needs a job that reads other runs' records, and the listing would no longer name the one job its bound came from |
| each suite's share of the run's total | it tells a slower suite from a slower runner, but misses a slowdown of every suite at once |
| the runner's slow-test setting | it marks or ends one test past a fixed period; it bounds no suite against a recorded figure, and it changes the runner's configuration |
| a runner report file for every suite | the stage logs already hold the figures, so a report adds a configuration change and an upload for no new figure |

## Consequences

- Good, because no runner configuration or gate stage changes, and no run fails on a suite's time
  until a delivery promotes the census.
- Good, because each bound names the job it came from, so a reader can re-derive it.
- Bad, because a runner-wide slowdown reports many sites at once; the margin keeps the runs read
  under it, and the report names each site.
- Bad, because the browser suites and the Swift test targets carry no time in `ci`.

## What would make this wrong

- A site reported on a change that slowed no suite, in more than one rule review out of five, would
  show the margin is too narrow for the runs it reads.
- A slowdown that passes under the margin and is found only by reading the logs by hand would show
  the margin is too wide.
