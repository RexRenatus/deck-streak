---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A persona created in the app speaks only after the persona-core gates pass it

## Context and Problem Statement

A card meets a persona only when its deck is mapped to one in the private roster (the owner's
answer PER-01). The owner also asked to create further personas inside the app. CHARTER 18 makes
personas public templates and a private roster, and ADR-044 keeps the roster as a private file the
owner approves once (#163), delivered by the private deploy rail. A persona created in the app is
a new kind of write to that private corpus: a template choice, its filled slots and voice, and
its deck map. Every shipped persona passed the persona-core gates before it could speak. How does
a persona made in the app join the roster without loosening any of that?

## Decision Drivers

- No persona speaks before it passes the same gates the shipped personas passed.
- Names, bios, subjects and weak spots never enter this repository or any public text
  (CHARTER 11, 18).
- A persona reads its own subject's memory only, and never the journal (CHARTER 18, ADR-044).
- A created persona runs on the deployment's one AI route: the owner's route behind its gates, or
  an API key elsewhere (ADR-054).
- What the owner creates, the owner can export and erase (CHARTER 13).

## Considered Options (the alternatives it was chosen against)

- Create a persona in the app from a public template, held as a draft in the service's private state until the persona-core gates pass its probe outputs, then mapped to decks beside the deployed roster — chosen because the owner gets in-app creation and every created persona passes the shipped personas' gates before it speaks.
- No in-app creation, the roster edited only in the private deploy rail — rejected because the owner asked for in-app creation (PER-01).
- A created persona that speaks at once, its outputs gated one by one — rejected because the shipped personas pass the persona-core review once before they speak, and a created one must pass the same.
- Created personas kept in this repository as templates — rejected because a filled persona is roster data, which CHARTER 18 keeps private.

## Decision Outcome

Proposed option: in-app creation from a public template, a draft until the gates pass.

- **Creation.** The owner picks one of the repository's public templates in the app and fills its
  slots: a name, a voice and a subject. The service stores the result as a draft entry in its
  own private state, beside the roster file the private rail delivers and never inside it, and
  never in this repository.
- **The gates.** A draft generates probe outputs for its subject, and the persona-core gates judge
  them as they judged the shipped personas: the output contract, no dates, memory scope, no human
  claim and the scrubber. The draft becomes a roster entry only when every gate passes, and its
  outputs keep passing the gates before delivery (CHARTER 17).
- **The deck map.** The deck-to-persona map accepts created entries by an opaque id, so no
  persona name appears in a public log, a metric label or any public text.
- **Memory.** A created persona reads its own subject's memory only, through ADR-044's
  subject-scoped reader, and never the journal.
- **The route.** It runs on the deployment's AI route (ADR-054), under the same caps as every
  persona duty.
- **Data rights.** Created entries are the owner's data: exported and erased with the rest
  (CHARTER 13), with a data-rights entry in the delivery that builds them.
- **The review.** The campaign's security review covers created personas before the first one
  speaks.

### Consequences

- Good, because the owner can add a mentor for a new subject without a deploy.
- Good, because a created persona is held to exactly the shipped personas' gates.
- Bad, because the roster now has two sources, the deployed file and the created entries, and the
  deck map reads both.
- Bad, because each draft spends AI turns on probe outputs before it can speak.

### Confirmation

- The gate tests: a draft whose probe output fails any gate stays a draft and is never mapped.
- A test that no persona name reaches a log line, a metric label or the public export.
- The data-rights census with the created entries' table.

## What would make this wrong

- The persona-core gates cannot judge a persona from probe outputs alone and need a human reading
  of the roster, as the owner's one approval (#163) did; creation would then end in an owner
  approval, not in the gates.

## More Information

- SPEC-334 (R16; the in-app persona creation exclusion in section 5, which a later delivery
  builds).
- ADR-044 (the private roster), ADR-054 (the AI route), CHARTER 11, 13, 17 and 18, #163.
