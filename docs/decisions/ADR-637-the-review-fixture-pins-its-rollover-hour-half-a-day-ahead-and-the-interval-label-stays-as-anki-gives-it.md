---
status: "accepted"
---

# ADR-637: the review fixture pins its rollover hour half a day ahead, and the interval label stays as Anki gives it

Decides SPEC-615. It keeps SPEC-348's review screen, its label and its Swift tests as they are, and
adds one step to the review fixture's writer (SPEC-348 R8).

## Context and Problem Statement

#779 asks which side is wrong when a learning step that crosses the day boundary is labelled `1d`:
the label, or tests that read the real clock. It asks for that side to be fixed, and for the
expected intervals never to be re-pinned to `1d`.

The engine labels a learning interval that reaches the scheduler's next day cutoff in days, as
Anki does (`IntervalKind::maybe_as_days`, `rslib/src/scheduler/states/interval_kind.rs:12` at the
rev `Cargo.lock` pins, applied to each choice by `describe_next_states`). The review fixture the
iPhone and iPad tests read is a version-2 collection: `CollectionBuilder` writes `"schedVer": 2`,
`"sched2021": true` and a `creationOffset` on create (`rslib/src/config/schema11.rs` lines 25 to
27). Its timing is `sched_timing_today_v2_new` (`rslib/src/scheduler/timing.rs` lines 159 to 177),
whose next day cutoff is the next rollover hour on the reading host's own clock (lines 39 to 45).
The fixture sets no rollover hour, so the engine writes its default on the first reading
(`rslib/src/scheduler/mod.rs` lines 63 to 71), and the fixture's day ends at that hour on every
reading host. A test run in the last ten minutes before it reads the ten-minute Good step as `1d`.

The fixture is written by the `review-fixture` example in the `xcframework` job and read by the
`release-and-app` job, which runs `ReviewModelTests`, `ReviewSessionTests` and `ReviewFlowTests`
over it (`.github/workflows/xcframework.yml` lines 133, 749 to 751 and 1020 to 1030).

## Decision Drivers

- Anki's label is the product's: a learning step that ends after the next day starts reads in days.
- No assertion is loosened, retried or re-pinned to a day.
- No change to the adapter, its allow-list, the app's source or the workflows.
- The writer and the reader are different jobs, so the cure must hold across the gap between them
  and across a difference in their clocks' offsets.

## D1, the label is right: what it was chosen against

- Chosen: the label stays the engine's, because it is Anki's rule, and the tests met it only because the fixture's day could end during their run
- Re-pinning the expected intervals to a day: rejected because the tests would then fail on every run outside the ten minutes, and #779 refuses it (its third item)
- Changing the label to minutes across the cutoff: rejected because it breaks parity with Anki, whose review screen shows the same words
- An app-side clock that the label reads: rejected because the engine reads its own clock for the cutoff, so an app clock changes nothing the engine computes

## D2, the fixture is wrong: what it was chosen against

- Chosen: the fixture is a version-2 collection with no rollover hour, so its day ends at the engine's default hour on the reading host's clock; that is the side to fix, because the engine's rule and the app's label are both right
- Reading the fixture as a version-1 collection: rejected because `CollectionBuilder` writes `"schedVer": 2` on create and the schema 14 upgrade copies it into the config table (`rslib/src/storage/upgrades/mod.rs` line 22), so `scheduler_version()` reads version 2 (`rslib/src/config/mod.rs` lines 213 to 216)

## D3, the cure is the rollover hour, measured and set half a day ahead: what it was chosen against

After `review::build`, the writer's `day_cutoff::pin` opens the review collection once for each
hour of the day, each open closed before the next because the engine caches its day per open
collection (`rslib/src/scheduler/mod.rs` lines 35 to 48). In each it sets `rollover` to that hour
by `set_config_json`, as `review_pairs.rs`'s `lead_with_rollover` does, and reads the hour's lead,
`next_day_at - now`, from the engine's own `timing_today()`. It keeps the hour whose lead lies
nearest twelve hours; the leads are one hour apart, so the kept one lies within half an hour of
twelve. It sets that hour, reopens the collection and reads the day back: a stored `rollover`
equal to the hour and a lead of more than eleven and at most twelve and a half hours. Any other
reading is an error. The pin writes the `rollover` key alone, and only the review collection is
pinned.

- Chosen: half a day ahead, because the hour is read on the reading host's clock in another job; a reader that keeps the writer's offset meets the cutoff about twelve hours after the write, and a reader whose offset differs by some hours meets it that many hours earlier or later, so any difference under eleven hours still leaves more than half an hour after the write
- The creation stamp: rejected because version-2 timing reads it for the days elapsed only, never for the cutoff (`rslib/src/scheduler/timing.rs` line 48)
- The version-1 scheduler, whose day has no rollover hour: rejected because the engine's bury refuses version 1 (`rslib/src/scheduler/bury_and_suspend.rs` lines 92 to 94), and the review screen buries
- The hour farthest from now, as `review_pairs.rs` keeps for its own collection (lines 125 to 143): rejected because there the writer and the reader are one process, while a reader one hour behind this writer would meet that cutoff within the hour
- The reading job's time zone set by the workflow: rejected because it is a workflow change, and the lead of half a day already bounds the offset a reader may differ by
- A config call through the adapter at read time: rejected because it widens the adapter's allow-list for a test's sake
- A clock seam in the engine: rejected because the engine is a pinned fork, and the fixture's own config already carries the hour the timing reads

## Decision Outcome

D3 is built by SPEC-615 in one pull request; D1 and D2 bound it. No Swift file, expected interval,
workflow, adapter source or existing Rust test changes.

### Consequences

- Good, because the fixture's day ends about twelve hours after it is written, so a whole run of
  the workflow never meets it, whatever the time of the run.
- Good, because the pin reads the engine's own timing, so it fails at write time if the engine
  stops honouring the hour.
- Bad, because the `release-and-app` job re-run alone, about twelve hours after the writing job
  (sooner on a reader whose clock is ahead of the writer's), meets the fixture's cutoff again; a
  run of the whole workflow writes a fresh fixture.
- Bad, because a reader whose offset differs from the writer's by eleven hours or more can meet the
  cutoff during its run; both jobs run on one runner label, so their offsets agree.

### Confirmation

SPEC-615's acceptance criteria A1 to A4, its red-first record and its row S61500.

## What would make this wrong

- A reader on a clock whose offset differs from the writer's by eleven hours or more.
- An engine rev whose version-2 timing stops reading `rollover` for the cutoff, which A1 reads.
- A review test that reads the fixture long after the writing job, as a lone re-run does.

## More Information

Built by SPEC-615; drawn in `docs/schematics/review-fixture-day.md`. SPEC-348 R8 (the review
fixture), its A1 (the four intervals) and its A15, A17 and A19 (the app's tests that read them).
