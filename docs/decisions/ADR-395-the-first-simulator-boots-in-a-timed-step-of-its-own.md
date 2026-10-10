---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-395: The first simulator boots in a timed step of its own

Decides SPEC-382 R4.

## Context and Problem Statement

The first test step boots the iPhone simulator implicitly, so the first session of each run carries
an excess that cannot be split into boot on one side and install and launch on the other. Where
should the boot happen so that it is measured, without changing what the tests run on?

## Decision Drivers

- One simulator at a time: the planted suite's load waits were measured that way.
- No new step runs before "the project, generated", whose prefix the census holds.
- The boot must fail loudly when the pinned device is missing or ambiguous.

## Considered Options (the alternatives it was chosen against)

- Chosen: a separate step after "the project, generated" boots the first simulator of the first
  test step, waits until it has booted and writes its seconds, because it measures the boot and
  leaves the iPad to be booted by `xcodebuild` as before.
- Booting both simulators early: lost, because it adds memory pressure on the path the load-wait
  gate guards.
- Booting in the background and waiting just before the first test: lost for now, because it
  overlaps work the step would otherwise wait for, but it cannot be judged until the boot's own
  share is measured.
- Leaving the boot inside the first test step: lost, because the excess stays unattributed.

## Decision Outcome

Chosen option: "a separate timed step after the project is generated", because it measures the
boot, leaves the iPad's boot where it is, keeps one simulator booted at a time and adds nothing to
the test steps.

- The step selects the one available device that matches the pinned iPhone name and OS, and fails
  unless exactly one matches.
- It boots the device, waits for the boot to complete (`xcrun simctl bootstatus <device> -b`) and
  writes its seconds to the report directory; the report gains its row.

### Consequences

- Good, because boot time becomes a series of its own, and the first test step's time is its
  tests'.
- Bad, because a boot failure now fails its own step rather than the first test step; it is the
  same class of failure as before.

### Confirmation

SPEC-382 A4: one boot step follows "the project, generated", boots the first simulator only, waits
for it and writes its seconds; mutation row `S38205` removes the wait.

## What would make this wrong

- A first test step whose own time does not fall once the boot is outside it would show the excess
  was not the boot; the step is then read again before any other boot decision is drawn from it.
- A load wait that fails on a simulator booted ahead of its first test, where it held when booted
  by `xcodebuild`, would show the early boot changes what the tests run on; the step is then
  removed rather than the wait changed.

## More Information

SPEC-382 R4.
