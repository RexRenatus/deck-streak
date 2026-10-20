---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-394: The repository keeps no timing history, baseline or budget

Decides SPEC-382 section 5's timing exclusion, with ADR-393.

## Context and Problem Statement

Once each run records its own timing (ADR-393, the timing record), the question is where runs are
compared with one another: does the repository hold a history of durations, a baseline or a budget
that a run is judged against?

## Decision Drivers

- The Apple job body holds read-only contents permission and reads no secret.
- A check that fails on elapsed time on a hosted runner would fail for reasons the change did not
  cause.
- Nothing the body tests or how it gates changes.

## Considered Options (the alternatives it was chosen against)

- Chosen: no history, baseline or budget in the repository, because each run's timing record is
  an artifact of that run and needs no write permission, no evicting store and no new check.
- A branch of published pages holding the history: lost, because it needs write permission, which
  the body does not hold.
- A cache entry holding the history: lost, because the cache store evicts entries that go unused,
  so the history would vanish.
- A committed baseline file: lost, because every change of a duration becomes a commit, and the
  file drifts from the runs.
- A budget step that fails a run over a time limit: lost, because it would be a new flaky check.
- A separate timing workflow that writes durations into a public summary on every run: lost,
  because it adds a workflow and a runner per run for what the per-run artifact already holds.

## Decision Outcome

Chosen option: "no history, baseline or budget in the repository", because it needs no write
permission, no evicting store and no new check, and each run's artifact already carries its own
timing.

### Consequences

- Good, because no permission, secret, workflow or check is added, and no run fails on elapsed
  time.
- Bad, because no check in the repository fails when a run grows slower; a slower run is found only
  by reading the timing records across runs.

### Confirmation

The census's existing permission rule holds the body's contents permission read-only, so no step
can write a branch. SPEC-382 adds no cache save. Review of each change confirms that no step
compares a duration with a limit.

## What would make this wrong

- A slowdown that reaches a job's bound before anyone reads the records across runs would show that
  reading the artifacts is not enough; the answer is then a decision of its own, chosen against
  the options above, never a step added quietly to this body.

## More Information

SPEC-382 section 5. ADR-393 (the timing record).
