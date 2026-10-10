---
status: proposed
decision-makers: "the DeckStreak architect"
---

# ADR-396: The framework job pins its Xcode as the other jobs do

Decides SPEC-382 R5.

## Context and Problem Statement

Three jobs of the Apple body set the developer directory to the pinned Xcode; the framework job
builds with the runner image's default. When the image changes its default, the framework job's
toolchain changes with no change in the repository, and its timings, and any cache keyed on the
toolchain, change with it. Should the framework job pin its Xcode too?

## Decision Drivers

- Every job of the body builds with the same, named toolchain.
- Any cache keyed on the toolchain must name the toolchain that built an entry from its first save.
- The slices the framework job produces must not change.

## Considered Options (the alternatives it was chosen against)

- Chosen: the framework job's environment sets the developer directory to the same Xcode path the
  other jobs set, because it is the form the other three jobs already use.
- Leaving the image default: lost, because the toolchain, and with it any cache keyed on it, can
  change under a run with no change in the repository.
- Selecting Xcode in a step with `xcode-select`: lost, because a step changes the machine for every
  later step, while an environment entry is read by each tool where it runs and is visible in the
  job's definition.

## Decision Outcome

Chosen option: "the framework job's environment sets the developer directory", because it is the
form the other three jobs already use, it names the toolchain in the workflow, and any cache keyed
on the toolchain then reads the pinned Xcode from its first save.

### Consequences

- Good, because every job of the body builds with one named Xcode.
- Bad, because an image that drops the pinned Xcode now fails the framework job too, as it already
  fails the others; the pin is then changed on purpose, never silently.

### Confirmation

SPEC-382 A5, with mutation row `S38206`, which removes the entry. The slices' library-set rows and
sizes are compared before and after the change on one engine tree.

## What would make this wrong

- A slice whose library set or size differs between the runs before and after the pin, at one
  engine tree, would mean the image default was not the pinned Xcode; the difference is then read
  and the pin kept, because the other jobs already build with it.

## More Information

SPEC-382 R5.
