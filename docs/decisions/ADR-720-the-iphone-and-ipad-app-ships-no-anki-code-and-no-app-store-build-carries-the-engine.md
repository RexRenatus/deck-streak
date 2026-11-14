---
status: "proposed"
decision-makers: "the owner"
---

# ADR-720: The iPhone and iPad app ships no Anki code, and no App Store build carries the engine

## Context and Problem Statement

DeckStreak's own code is licensed AGPL-3.0-or-later (ADR-018), and the iPhone and iPad app links
Anki's engine, which is under the same licence, through the framework built from the umbrella FFI
crate (ADR-335, ADR-345, ADR-357). Builds reach testers through internal TestFlight only. Whether
the App Store's terms of use place a further restriction on AGPL code is a legal question, and
the repository's store-copyleft check asks that the project's answer be written down in an ADR
before any store lane is relied on. What does DeckStreak do about Anki's AGPL code in the App
Store?

## Decision Drivers

- No party outside the project should decide when DeckStreak may release.
- No licence risk should be carried into a public store listing.
- The question needs an answer now; leaving it open blocks every store plan.
- Internal testing goes on unchanged while the answer takes effect.

## Considered Options (the alternatives it was chosen against)

- Chosen: take Anki's code out of the iOS target, because a store conflict then has no Anki code to apply to, and nobody's permission is needed.
- Ask Anki's copyright holders for written permission to distribute through the store: rejected because a third party would then decide each DeckStreak release.
- Distribute the engine under a custom App Store licence agreement that says the AGPL governs: rejected because that route keeps the very licence risk this record exists to avoid.
- Keep the iOS app on internal TestFlight with no end date: rejected because it leaves the question unanswered.

## Decision Outcome

Chosen option: "take Anki's code out of the iOS target", because it answers the store question
without anyone else's consent and without a licence risk.

- The iPhone and iPad target will contain no code from Anki. The engine leaves that target as
  part of the removal recorded in the ADR titled "Anki's engine leaves DeckStreak in stages, and
  this record supersedes or amends each decision built on it".
- Until the cut-over, the iOS app keeps syncing through the existing bridge that speaks Anki's
  sync protocol; that bridge is retired by the cut-over itself.
- After the removal, no package of Anki's engine is left in the framework the app links, and
  Rex, DeckStreak's own memory model, is the one source of the app's scheduling, reached through
  its documented interface.
- DeckStreak asks Anki's copyright holders for no store permission, and distributes no Anki code
  under a custom App Store licence.
- Until the removal lands, iOS builds go to internal TestFlight testers only, as they do now, and
  no App Store submission includes the engine.
- This record is the project's written position on Anki's AGPL code in the App Store, which the
  store-copyleft check reads. How DeckStreak's own AGPL code meets the store's terms is a
  different question, answered by its own ADR once legal advice is in hand.

### Consequences

- Good, because the store question is settled without waiting on anyone outside the project.
- Good, because internal TestFlight testing continues as it is.
- Bad, because no App Store release can happen before the removal lands.
- Bad, because the store terms for DeckStreak's own code stay open until their own ADR.

### Confirmation

The repository's store-copyleft check finds this ADR, which names both the copyleft code and the
store. The removal step that takes the engine out of the iOS target carries its own test that the
framework's dependencies include no package from Anki's engine.

## What would make this wrong

- The removal stalls and the engine is still in the iOS target when a store release is wanted.
- Anki's copyright holders publish a store exception that covers the engine.
- Legal advice finds the App Store's terms compatible with the AGPL for this use.

## More Information

The ADR titled "Anki's engine leaves DeckStreak in stages, and this record supersedes or amends
each decision built on it" (the removal this record relies on), ADR-018 (the licence), ADR-335,
ADR-345 and ADR-357 (the iOS client, its FFI adapter and the one Rust library it links), and
ADR-599 (what is still Anki's beneath DeckStreak).
