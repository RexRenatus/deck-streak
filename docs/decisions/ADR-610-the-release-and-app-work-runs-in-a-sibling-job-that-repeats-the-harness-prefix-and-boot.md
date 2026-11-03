---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-610: the Release, app, review, bury and flag work and the archive run in a sibling job, `release-and-app`, that repeats the prefix and boot of `harness`

Decides SPEC-591 (R1 to R5 and R7).

## Context and Problem Statement

`harness` runs 22 steps in series under `timeout-minutes: 150`. The worst time of each step over three measured jobs
sums to 166.53 minutes (jobs 125484649336, 125641992874 and 125710788160), and one of them was cancelled at its
bound in the bury and flag step (job 125641992874). The bound is not to move. How does the body run all of its steps,
each inside a bound, with no step dropped and no test weakened?

## Decision Drivers

- No step is dropped, skipped, filtered or made conditional, and no test, wait or retry changes.
- `harness` keeps its bound of 150 and the planted suite, whose evidence and bound pin are read from that job.
- The Release boot step runs the warm script that the Debug boot step writes, on simulators the Debug boot booted.
- Every copied byte is held equal by a test, so a copy cannot drift.

## Considered Options (the alternatives it was chosen against)

- **Chosen: a sibling job `release-and-app` that needs `xcframework`, repeats the environment and the first eight
  steps of `harness` byte for byte (through the boot and launch before the Debug tests), then runs the moved steps
  in their order at the base, with a bound of 130 held inside 123 to 149.**
- A bound of 180 for the one job: rejected because the worst sum, 166.53 minutes (jobs 125484649336, 125641992874
  and 125710788160), sits only 13.47 minutes under it (jobs 125484649336, 125641992874, 125710788160), short of the margin ADR-378 asks, and a job that grows meets its bound again with no step moved.
- Moving the planted suite out of `harness`: rejected because its evidence and its bound pin (SPEC-361 R15) are
  read from the job named `harness`.
- A sibling that boots in its Release boot step alone: rejected because that step budgets each simulator for a
  warm one, a cold boot of both took up to 14.92 minutes (job 125641992874), and the warm script is written only by
  the Debug boot step.
- A checked-in warm script that both jobs run: rejected because it rewrites the boot step of `harness`, re-points
  every census reading of the script and moves six rows to a new file, where a copy keeps every byte of `harness`.
- One shared form by a YAML alias or a composite action: rejected because the census reads the workflow through its
  own reader, which refuses alias forms, and a composite action changes the steps of `harness`, which the lane
  census compares with each lane's app job (SPEC-352 A20).
- Testing both simulators at once to save time: rejected because every wait was measured one simulator at a time.

## Decision Outcome

Chosen: the sibling job, because it is the only option that keeps every step and every byte of `harness` through the
planted suite, keeps the bound of 150, and puts each half inside its own bound by measurement:

- `harness` keeps its first ten steps, then its report and its `harness` upload: worst 103.65 minutes (jobs 125484649336, 125641992874, 125710788160),
  under 150;
- `release-and-app` repeats the first eight steps, worst 16.30 minutes (jobs 125484649336, 125641992874, 125710788160),
  and runs the moved steps: worst 81.70 minutes whole (jobs 125484649336, 125641992874, 125710788160);
- its bound is ADR-378's rule: one and a half times 81.70 is 122.55, rounded up to the ten, 130; its band starts
  at that product rounded up to the minute, 123, and ends at 149, one under the bound of `harness`, so the sibling
  never takes that bound and row S36111's line stays the harness's alone;
- the copies follow the tree's precedent: each TestFlight lane's app job repeats the steps of `harness` from
  the download through the generated project, held equal by its census (SPEC-352 A20).

### Consequences

- Good: the cancellation at the bound cannot recur from the moved steps, and no evidence moves out of `harness`.
- Good: every copied byte is read by a test (SPEC-591 A3), and every step at the base by another (A1).
- Bad: each run pays the repeated steps once more, up to 16.30 minutes (jobs 125484649336, 125641992874 and
  125710788160), and needs a second runner at once to gain wall time.
- Bad: six rows that anchor inside the copied boot step now carry long finds.

### Confirmation

SPEC-591 A1 to A3, A5 and A6, red at the base, with rows S59105, S59106, S59108, S59109, S59111 and S59112. On
each later run, each job's time is read against its own bound.

## More Information

- SPEC-339 R12 and R15 (the warm), SPEC-347 R13, SPEC-348 R20, SPEC-358 R8, SPEC-361 R15, SPEC-352 A20; ADR-378.
- What would make this wrong: a completed `release-and-app` past two thirds of its bound (86:40), which re-derives it
  by the same rule; a `harness` past four fifths of its bound, which reopens that bound by measurement; or a runner where the
  Release boot finds its simulators cold after the repeated boot, which its report rows show.
