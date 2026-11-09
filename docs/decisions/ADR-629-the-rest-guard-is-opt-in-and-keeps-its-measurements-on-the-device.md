---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The rest guard is opt-in and keeps its readings on the device

## Context and Problem Statement

#924 asks for an opt-in rest guard in the study clients that keeps its readings on the device. To
suggest a rest, the guard reads the open session's own timings. Readings of how long and how fast
a learner studies can hint at tiredness, which sits close to health data, so where they are kept,
who may see them and how they are erased must be decided before anything is built. Where does the
rest guard keep its readings, what may it read, and how does the learner take it back?

## Decision Drivers

- Data protection by design and by default: personal data is off until the learner asks for it.
- Consent that can be withdrawn as easily as it was given, and erasure at once when it is.
- Data minimisation and storage limitation: only what the suggestion needs, only for the day.
- No health data, no other app's data, and nothing sent to the service.
- One declared inventory: every store of personal data is in `privacy.json`.

## Considered Options (the alternatives it was chosen against)

- An opt-in guard whose readings stay in a declared store on the device, erased when it is turned off and never sent — chosen: it gives the suggestion what it needs and keeps the readings with the learner.
- Turn the guard on by default — rejected because data protection by default keeps personal data off until the learner asks, and consent must be given, not assumed.
- Keep the readings on the service — rejected because the suggestion needs nothing the service holds, and a reading the service never receives cannot leak from it.
- Read a health store or a wearable's readings — rejected because health data is a special category, and the suggestion needs only the session's own timings.
- Read another app's data, such as screen time — rejected because it is not the learner's study, and the guard would need a permission no study client asks for.
- Keep the readings in the collection store beside the device's working state — rejected because that store syncs, and the readings would leave the device.
- Keep the readings until the learner erases them — rejected because the suggestion needs only the current study day.

## Decision Outcome

Chosen option: "an opt-in guard whose readings stay in a declared store on the device, erased when
it is turned off and never sent", because it holds every driver above at no cost to the
suggestion.

- **Off until turned on.** The rest guard is off by default, in the web app and in the iPhone and
  iPad app. Turning it on is the learner's consent, given in the session where the guard explains
  what it reads.
- **What it reads.** The open session's own timings on this device: the minutes studied today, the
  minutes since the last break, and each answer's time in the open block. It reads nothing from a
  health store or from any other app's data.
- **Where it keeps them.** A store on the device, apart from the collection that syncs, declared in
  `privacy.json` as a device store with consent as its basis, the switch that turns the guard off
  as its withdrawal, and a retention of the current study day.
- **Never sent.** No request either client sends carries a reading, and the service has no route
  that accepts one.
- **Withdrawal.** Turning the guard off erases its store at once. Erasing the account erases it
  too.
- **What it does.** An in-session suggestion to rest. It never stops a review, never changes a
  card, and sends no message outside an open session.

### Data-protection impact note

- **The processing.** Timings of the learner's own study, on the learner's own device, to suggest a
  rest during a session.
- **Why it is noted.** Study timings can hint at tiredness, an inference close to health data, so
  the processing is weighed before it is built.
- **The measures.** Opt-in consent; withdrawal in one switch that erases at once; readings kept for
  the current study day only; nothing sent to the service; no health store and no other app's
  data; no decision made about the learner beyond a suggestion the learner may ignore.
- **The residual risk.** Low: the readings never leave the device, are deleted daily, and reveal no
  more than the study log the learner already keeps.
- **What reopens it.** Any change that sends a reading, reads a health store or another app's data,
  or keeps readings past the study day needs a new assessment and a new ADR.

### Consequences

- Good, because the learner's readings stay with the learner, and turning the guard off leaves
  nothing behind.
- Good, because the service holds no tiredness signal, so it has none to export, leak or erase.
- Bad, because the guard cannot suggest a rest from what another device saw today.
- Bad, because each client keeps its own store, so the two clients' guards may suggest differently.

### Confirmation

SPEC-607's acceptance tests: the rest guard is off until turned on and its withdrawal erases it,
neither client sends a reading, and the store is a declared device store with consent, a
withdrawal and a bounded retention while neither client reads a health store; and the
privacy-gdpr pack's rows over `privacy.json`.

## What would make this wrong

- The learner asks for the guard to follow them across devices: the readings would then have to be
  sent, which needs a new assessment and a new ADR.
- A suggestion built from the session's timings proves no use to the learner: the guard is then
  removed with its store.

## More Information

SPEC-607; ADR-628 (the block and its one timer); `privacy.json`; `docs/schematics/focus-blocks.md`;
#924.
