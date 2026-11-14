---
status: accepted
decision-makers: "the DeckStreak architect"
---

# A card speaks in its language's own voice until the device's voice list is in, and the list is read once off the main actor

## Context and Problem Statement

`ReviewModel` read the device's installed voices on the main actor before every card step, through
`InstalledVoices.all()` and `AVSpeechSynthesisVoice.speechVoices()`. In a failed UI test on the
iPhone simulator that read held the main thread inside the speech framework's wait until the test
failed, so the app answered no idle wait and no query, and no card showed (SPEC-687, section 1).
The voices feed two things: the engine's choice of voice for a line of speech, and the voice
picker's options. Neither needs the list before the first card shows.

## Decision Drivers

- The main thread never waits for the device's voice list.
- A card shows at once, whatever the state of the list.
- No test, wait, destination or workflow changes.
- The smallest change: one type in a file of the speech role, and one default.

## Decision Outcome, and what each decision was chosen against

### D1. The list is read once per process, off the main actor

`InstalledVoiceList.device` starts one detached read the first time a review asks for it, and keeps
what it lists. Chosen against reading at each step off the main actor, which repeats a read that can
stall, and against starting the read in the app's entry, which adds a file for a list the first card
does not need.

### D2. No step waits: a line speaks in its language's own voice until the list is in

Each step hands the engine the voices listed so far, none until the read is in. With none, the
engine chooses no installed voice, and the player speaks the line in its language's own voice, the
path a chosen voice that is not installed already takes. Chosen against awaiting the read off the
main actor before the step: the main thread would answer, but the card would still wait for the
read, so a stalled read would still outlast the test's own wait. Chosen against no change and more
measurement first, which leaves the card step able to stall.

## Consequences

- Good: no card step and no picker reading waits on the main thread for the device's list.
- Good: the change is one type in `InstalledVoices.swift` and one default in `ReviewModel.swift`,
  and the register keeps both decision counts.
- Bad: until the read is in, a chosen voice is not used, and the picker offers the voices listed so
  far.

## Confirmation

The two acceptance tests of SPEC-687 read red at the commit that adds them with an inert list, which
reads on its caller's actor as the old default did, and green at the commit that moves the read off
the main actor, each by its name on both destinations in the review screen's step of the
`release-and-app` job.

## What would make this wrong

- A process sample of a later stall that shows the main thread in another frame at a card step.
- A device whose language voice cannot speak a line an installed voice can, where the first card's
  speech matters before the list is in.

## More Information

SPEC-687; SPEC-348 R13 and R14; SPEC-506; ADR-407.
