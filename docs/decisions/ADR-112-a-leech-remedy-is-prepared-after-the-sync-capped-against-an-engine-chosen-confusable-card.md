---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A leech remedy is prepared after the sync, capped, against an engine-chosen confusable card

## Context and Problem Statement

#47 asks for a remedy per failing card (an explanation from a new angle, a mnemonic and a contrast
against its confusable card) that is ready before the card's next review. SPEC-093 snapshots the
leeches after each sync, with no due day and no card text. An agent run takes minutes and is bounded
by SPEC-043's caps; the host is small, so the number of runs a day matters. The study-duties
template asks for the confusable card by name but does not say who finds it. When is a remedy
prepared, how many at once, and who picks the confusable card?

## Decision Drivers

- "Ready before its next review" is a scheduling promise a test must prove.
- A model's claim is advisory until a deterministic rule accepts it (the W6 plan's content rule).
- The memory budget: agent runs are bounded per run, and nothing is generated while the owner waits.
- No-AI is the default (ADR-054), and SPEC-093's protocol already answers with no AI.

## Considered Options (the alternatives it was chosen against)

- A daily job after the sync, ordered by due day, capped per run, stored per card, with the
  confusable card chosen from engine candidates: chosen, because the soonest reviews are served
  first, the cost per day is bounded, and every choice the model makes is checked against a set.
- Generate on request, when the owner sends `/remedy`: rejected because a run takes minutes, so the
  remedy is not ready at the review, and a request could start unbounded runs.
- Generate every leech in one run: rejected because a large snapshot makes one run long and costly,
  and the caps would stop it halfway with nothing stored.
- Let the model name any card as the confusable one: rejected because a named card cannot be
  checked against the collection, so an invented card would be shown as real.
- Write each remedy into the vault as a note: rejected because the vault layout has no folder for
  it, and a note would copy card text into a second store the owner must erase by hand.

## Decision Outcome

Chosen option: "a daily job after the sync, ordered by due day, capped per run, stored per card,
with the confusable card chosen from engine candidates", because it keeps the promise testable and
the model's choices checkable.

- **When.** The `leech_doctor` job runs after the study day's sync and only when it succeeded.
- **Which.** Unsuspended leeches with no remedy, or with a remedy prepared at fewer lapses, by due
  day, then lapses, then card id.
- **How many.** A configured cap per run, 3 by default.
- **The confusable card.** One of at most 8 other leeches of the same subject, or none. A name
  outside that set withholds the remedy.
- **Where.** `leech_remedies`, one row per card, deleted when the card leaves the snapshot.

### Consequences

- Good, because a card due on the next study day and inside the cap has its remedy before its day
  begins, which a test proves on a scripted runner.
- Good, because the remedy is served by a command and a route and pings nobody.
- Bad, because a confusable card that is not itself a leech is never named. The contrast then sets
  the card against a near miss, which the template allows.
- Bad, because a snapshot larger than the cap takes several days to cover. The order serves the
  soonest reviews first.

### Confirmation

SPEC-112's A4 to A8 and A10 to A13, and its rows S11203 to S11210.

## What would make this wrong

- A snapshot that grows faster than the cap: the queue's age shows it, and the cap is the owner's
  (a question in the W6 plan's hand-back).
- Confusions between a leech and a card that never lapses often: a candidate set built from the
  collection's similar cards would be needed, which is its own decision.

## More Information

SPEC-112, SPEC-093 (the snapshot), ADR-093, SPEC-043 (the caps), SPEC-044 R5 to R7 (memory), and the
W6 schematic `docs/schematics/w6-duty-run-and-its-degradation.md`.
