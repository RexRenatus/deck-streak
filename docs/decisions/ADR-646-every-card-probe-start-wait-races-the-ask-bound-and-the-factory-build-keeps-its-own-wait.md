---
status: "proposed"
decision-makers: "the owner, the DeckStreak architect"
---

# ADR-646: every card probe start wait races the ask bound, and the factory build keeps its own wait

Decides SPEC-624 (R1 to R5).

## Context and Problem Statement

SPEC-609 bounded every page ask of the card view's probe suite under `Probe.askSeconds`, a value of 120, and named
the waits the suite starts on its own as kept: the listeners each probe starts, the lookup witness, the two rule
list compiles the suite starts and the planted cookie. The listeners' ready wait resumes only on a ready, failed or
cancelled state, so a listener pair that never gets ready held the test, the job and its runner to the job's own
bound, naming no test (#839). Each start wait needs an end, and its stall needs the same name a page ask's stall has:
its test, its label and the probe it stopped on.

## Decision Drivers

- A stalled start fails one test by name and records the probe it stopped on; it never holds the job to the job's
  bound.
- Nothing is weakened: no card, view, wait, reading, bound or assertion changes, and nothing is retried or skipped.
- One race and one stall reading: a start wait reuses SPEC-609's one-shot guard and stall, with no second copy.
- The listener and witness types stay unchanged, and the criterion's seam reads the same helper `show` calls.

## Considered Options (the alternatives it was chosen against)

- **Chosen: one start helper at the call sites, `Probe.started`,** because it is one race, it leaves
  `Listeners.swift` unchanged, and the planted seam reads the same helper `show` calls.
- A timer inside `Listeners.ready` and `Witness.start`: rejected because it is a second copy of the race beside
  SPEC-609's, on the listeners' own queues; it builds the stall off the main actor, where the probe record lives;
  and it leaves no seam a test can hold without changing the listener types.
- Bounding the factory's build at its five call sites: rejected here because it wraps the product API the factory
  suite judges; it is left to its own delivery.
- A retry of a stalled start: rejected because a retry is a weakening.

## Decision Outcome

Chosen: one start helper at the call sites, because it ends every wait the suite starts on its own with the same
stall a page ask has, names the probe, and changes no card, wait or assertion:

- D1: every wait the suite starts on its own, the listeners, the witness, its two rule list compiles and the planted
  cookie, races `Probe.askSeconds` through one start helper, `Probe.started`, which reuses SPEC-609's one-shot guard
  and stall unchanged.
- D2: a start wait is no page ask, and it is not counted in the `card probe asks:` line.
- D3: the owner of a started object, the listeners in `show` and the witness in its test, registers its cancel
  before the start, so a stall ends the object and its late finish is dropped.
- D4: the factory's own build keeps its wait: the rule list compile inside `CardWebViewFactory.makeCardWebView` is
  the product API the factory suite judges, and this test-only delivery does not wrap it.

The bound is the ask bound because a listener pair, a witness, a compile and a cookie each finish far inside one page
ask's allowance on a running simulator, and one bound keeps one stall reading.

### Consequences

- Good: a listener pair that never gets ready fails its test by name, with the probe it stopped on, at the ask bound
  instead of the job's.
- Good: one guard and one stall serve page asks and start waits alike, and the asks line keeps its meaning.
- Bad: a stall inside the factory's own build still holds the job to its bound.
- Bad: the timer is a task on the main actor, so a main thread that WebKit itself blocks still holds the job.
- Neutral: a stalled start's operation goes on until its owner's cancel, and its late finish is dropped; the test has
  already failed.

### Confirmation

SPEC-624 A1, red at the commit that carries the start helper as an inert seam with no bound, and green at the commit
that adds the bound, with the test and its seam unchanged.

## More Information

- SPEC-624; SPEC-609 R1 to R6, kept, with its section 5's second bullet superseded for the five start waits; SPEC-355
  R10, SPEC-349 R5 and SPEC-361 R16, kept.
- What would make this wrong: a start wait measured near the bound on either simulator, which would call for its own
  value in its own delivery; or a hang inside the factory's own build, which this bound does not reach.
