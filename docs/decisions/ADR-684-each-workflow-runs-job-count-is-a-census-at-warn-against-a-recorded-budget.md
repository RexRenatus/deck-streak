---
status: "accepted"
---

# ADR-684: each workflow run's job count is a census at warn against a recorded budget

Decides SPEC-662. Cites ADR-450 (D6, a new rule starts at warn and moves by its own delivery),
ADR-436 and SPEC-422 R14 (the limit holds for one matrix, measured by `matrix-limit.yml`), and
SPEC-362 R3 (the mutation legs' ceiling). Amends nothing.

## Context

A matrix generates at most 256 jobs, and the committed record of `matrix-limit.yml` holds a run
of more than 256 jobs (SPEC-422 R15), so the platform bounds one matrix and not one run. Nothing
in the tree counts the jobs each workflow's run can start; a change that adds jobs to a run is
read by no check.

## Decision Drivers

- No rule already in force is lowered, and no workflow changes.
- A new rule starts at warn and is promoted by its own delivery (ADR-450 D6).
- Every check prints what it examined and fails on an empty population, which is VOID.

## Decisions

**D1. One count rule, a lower bound.** Each job is one; a static matrix is its combinations after
`exclude` and then `include`; a call of a workflow of the same directory is that workflow's count
once per caller combination; a job a condition or a path filter can skip is counted; a matrix
computed at run time or any other call is counted once and named undecided (SPEC-662 R2).

**D2. A budget per run, the run being one workflow file and one event.** `BUDGET` holds each run's
figure, today's count, with its reason; `UNDECIDED` holds each job the rule cannot count, with
what bounds it (SPEC-662 R6, R7).

**D3. Warn first.** `LEVEL` is warn; each run past or outside its budget, each unlisted undecided
job, each entry with no reason and each stale entry prints. Moving to fail is its own delivery
under ADR-450 D6's rule, and a demotion at fail is a weakening (SPEC-662 R9).
A stale entry fails at warn as well as at fail, because an entry that nothing matches is a wrong
record (SPEC-436 R21, SPEC-662 R9).

**D4. The house reader.** The census reads each workflow through the loader and the reader of
`scripts/tests/test_ci_workflows.py`, so a form the reader refuses fails as the hardening tests do.

## What it was chosen AGAINST

- One budget per event over every workflow it starts: rejected because a raise would not name the
  workflow that caused it.
- The 256 jobs of one matrix as each run's budget: rejected because the platform bounds a matrix,
  not a run, and the figure would pass every run today.
- Deciding the computed matrices from the plan's constants: rejected because those constants
  already hold their own bounds in their own tests.
- A stale entry that only prints at warn: rejected because an entry that nothing matches is a
  wrong record, not a finding to wait on.

## Consequences

- Good: a change that adds jobs to a run prints a WARN line naming the run and its figure.
- Bad: at warn a run past its budget passes; the line prints in the hygiene job's log.
- Bad: a computed matrix stays undecided; its bound lives where its own SPEC holds it.

## What would make this wrong

- The reader refuses a workflow GitHub runs: the census fails closed by name, never a skip.
- The budget's lines are wrong in more than 1 of 5 cases: D3's precision falls under ADR-450 D6.

## More Information

SPEC-662; ADR-450; ADR-436; SPEC-422; SPEC-362; #1382.
