---
status: "proposed"
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-631: every card probe ask races one bound, and a stall is a named failure

Decides SPEC-609 (R1 to R5).

## Context and Problem Statement

The card view's probe suite waited on each WebKit completion with no bound of its own: `Probe.evaluate`, the load
wait's ready ask and `Probe.snapshot` each awaited a continuation that only WebKit resumes, and `Probe.poll` bounds
only the time between its checks. A page that stopped answering held one `apple / harness` job to its own bound,
naming no test (job 115216068432). Each ask needs an end, and a stall needs a name: its test, its ask and the probe
it stopped on.

## Decision Drivers

- A stall fails one test by name and records its ask and its probe; it never holds the job to the job's bound.
- Nothing is weakened: no card, view, wait, reading, bound or assertion changes, and nothing is retried or skipped.
- A cold first WebKit start, which the warm-up allows `Probe.warmUpSeconds`, is no stall.
- The bound is measured, per test and per simulator, so its margin is read, never guessed.

## Considered Options (the alternatives it was chosen against)

- **Chosen: a bound per ask, `Probe.askSeconds`, a value of 120, raced in one bounded helper,** because every wait
  of the suite that only WebKit can end is a page ask, and one helper bounds each the same way.
- XCTest's own time allowance per test: rejected here because it needs a workflow change and names no probe; it is
  left to its own delivery.
- A bound per probe, one `show` call: rejected because a probe's own window and deadlines vary by card, so one value
  would be either loose or a new red.
- A retry of a stalled ask: rejected because a retry is a weakening.

## Decision Outcome

Chosen: a bound per ask, because it ends every wait that only WebKit can end, names the ask and the probe, and
changes no card, wait or assertion:

- D1: every page ask the suite waits on, each `evaluateJavaScript` answer and each `takeSnapshot` completion, goes
  through one bounded helper under `Probe.askSeconds`, a value of 120, and the answer phase keeps its own
  non-blocking ask under `answerSeconds`.
- D2: a stall throws `ProbeStall`, which ends the test and names the ask and the probe; a reading after a stall is
  never taken, because it would measure nothing.
- D3: a one-shot guard on the main actor resumes the ask's continuation once, with whichever of the answer and the
  bound comes first, cancels the bound's timer on an answer, and drops a late answer.
- D4: each test prints one `card probe asks:` line, its ask count and its slowest ask against the bound.

The value is twice `Probe.warmUpSeconds`, a value of 60, the most the warm-up allows WebKit's first process start
(SPEC-361 R16, ADR-372 D11), so a cold start is no stall.

### Consequences

- Good: a page that stops answering fails one test by name, with the ask and the probe it stopped on, at the ask
  bound instead of the job's.
- Good: the `card probe asks:` lines give each test's slowest ask against the bound on each simulator.
- Bad: a cold start beyond the bound reads as a stall, a red where there was a slow pass.
- Bad: the timer is a task on the main actor, so a main thread that WebKit itself blocks still holds the job.
- Neutral: a late answer after a stall is dropped; the test has already failed.

### Confirmation

SPEC-609 A1 and A2, red at the commit that carries the helper as an inert seam with no bound, and green at the
commit that adds the bound, with both tests unchanged.

## More Information

- SPEC-609; SPEC-349 R11 and R12 and SPEC-361 R16, kept with every bound they name.
- What would make this wrong: measured `card probe asks:` lines whose slowest ask nears the bound on either
  simulator, which would call for a new value in its own delivery; or a hang that is no page ask, such as a
  listener's start, which this bound cannot end.
