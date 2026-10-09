---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-393: Each Apple CI run records and uploads its own timing

Decides SPEC-382 R1 to R3, R6 and R8, with ADR-394, ADR-395 and ADR-396.

## Context and Problem Statement

The Apple job body times some of its steps and not others. The planted-suite step has no timer,
its result bundle's tests are not in the report, no `xcodebuild` prints its build timing summary,
the static-library step times both targets as one, and per-test durations exist only inside result
bundles. A run therefore cannot say where its own time went, and two runs cannot be compared step
by step without downloading every bundle. How does each run record its own timing without changing
what it tests?

## Decision Drivers

- The planted suite's command produces the load-wait evidence; it must not change by a byte.
- No test, destination, configuration, pass, wait, timeout or bound changes.
- A run's timing must be readable without its result bundles.
- Every new line is held by the census, with a planted negative for each refusal.

## Considered Options (the alternatives it was chosen against)

- Chosen: each run writes a versioned timing record and uploads it as a small artifact whatever
  the outcome, with the timing summary on every `xcodebuild` except the planted suite's, because it
  records every timer, the toolchain and each test's duration per destination in one place.
- The timing summary on every `xcodebuild`, the planted suite's included: lost, because that
  command produces the load-wait evidence, so it stays byte-identical, and its build share is
  negligible.
- Reading timing from the job logs alone, with no record in the run: lost, because per-test
  durations and build categories are not in the step list, and log formats drift without a schema.
- Keeping a timing history or budget in the repository: lost, because it is decided separately
  (ADR-394, no timing history in the repository).

## Decision Outcome

Chosen option: "each run writes a versioned timing record and uploads it", because it records every
timer, the Xcode and OS versions and each test's duration per destination in one small artifact
per job, while the planted suite's command stays byte-identical and nothing the run tests changes.

- The planted-suite step is wrapped by a timer; the report reads its result bundle's tests.
- Every other `xcodebuild` build, test or archive passes `-showBuildTimingSummary`.
- The static-library step writes each target's seconds and the total; its cargo command changes
  only by an optional trailing `--timings`.
- `timing.json`, schema `ci-timing/1`, is uploaded as `timing-xcframework` and `timing-harness`
  with `if: ${{ always() }}`, after each job's existing upload, using the same pinned upload action.

### Consequences

- Good, because a run's own artifacts say where its time went, step by step and test by test.
- Good, because the planted suite's command, and so the evidence it produces, is untouched.
- Bad, because each job gains a small tail of report and upload work.
- Bad, because the timing summary on the Release build adds output to a measured step; its size and
  imports are compared before and after the change.

### Confirmation

The census class of SPEC-382 (A1 to A7): an `xcodebuild` other than the planted suite's without the
summary, a test step without its timer and an uploaded bundle the report does not read are each
refused by a planted negative, and the planted suite's command is held equal to its base text. Each
has a mutation row in `S38200-S38299`.

## What would make this wrong

- A run whose record misses a seconds file its own steps wrote, or whose per-test durations
  disagree with the test log, would mean the record is not the run's own timing; the record's
  reader is then fixed before any comparison is drawn from it.
- A change in the Release size or imports between the runs before and after this delivery, at one
  engine tree, would mean the timing summary changed what the measured step builds.

## More Information

SPEC-382 R1 to R8. It amends the freeze words of SPEC-361 R14 and R15 to admit exactly these
lines (SPEC-382 R9), and keeps SPEC-361 R15's bound and R16's load wait.
