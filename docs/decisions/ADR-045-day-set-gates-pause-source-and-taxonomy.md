---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The day set's gates: the last sync first, then the pause computed by the readings from ingest's reviews, from a private taxonomy, into closed states

## Context and Problem Statement

ADR-019 recorded the owner's reading decisions: a reading generates whenever the last sync
succeeded, and after two or more days without study the daily readings pause. It did not say in
which order the gates run, who computes the pause, where the owner's deck taxonomy lives in a
public repository, or how a topic's end state is held so that nothing to read, could not tell and
failed can never blur. The predecessor answered the first by copying the whole collection before it
refused, and the others with deck constants in code.

## Decision Drivers

- Honest states: no new cards, could not tell and failed stay distinct, and a collapse is a test failure.
- The repository is public; deck names and the subjects run are private (constraint 11).
- The readings context depends on the kernel and ingest only; the governor lives in streaks.
- No work, and no collection copy, on a night that is refused anyway.

## Considered Options (the alternatives it was chosen against)

- Gates in the order last sync, then pause, then resolve; the pause computed by the readings from ingest's reviews; the taxonomy in a private file with the predecessor's parse rule as the public algorithm; the end state as one enum whose variants carry their class and reason — chosen: cheap refusals first, no new edge, no private name in the tree, and a type that cannot hold two states.
- Keep a freshness gate on the collection file's age — rejected by the owner (ADR-019): it refused every night the owner did not study, although the sync was healthy.
- Take the pause from the governor's state — rejected because it needs a readings-to-streaks edge the context map does not declare, the governor knows three silent days and not two, and the pause (two days) must begin before the lapse (three days) does.
- Keep the predecessor's deck constants in code — rejected because they are the owner's private deck names and subject list.
- Record could-not-tell as a failure with a reason — rejected because it collapses a broken rail into a failed generation, which the owner's honest-states rule forbids.

## Decision Outcome

Chosen option. The resolution refuses before any collection work when the last sync failed
(`could_not_tell`, `rail_broken`, `sync_failed`) or when the two study days before today have no
qualifying review (`paused`). Only then does it query the scheduler's queue per root. The taxonomy
file names the law roots and bands, the language decks and their term fields, and the writing roots;
the predecessor's parse rule reads it and is proved by goldens generated through a synthetic
adapter. The five states and their closed reasons are one Rust enum stored with its class and reason.

### Consequences

- Good, because a paused or unsynced night costs a query of the sync ledger and of the reviews, not a
  collection copy.
- Good, because the pause and the lapse stay two concepts with two names, as the context map says.
- Bad, because the readings and the governor each count silent days; the counts differ by design
  (two and three), and each is tested against its own rule.

### Confirmation

SPEC-045's tests, the goldens of `prereading.py:resolve_day_sets`, `leeches.py:_law_subject` and
`prereading.py:_digest_for_card_ids`, and the public scrub over the example taxonomy.

## What would make this wrong

- The owner redefines the pause in terms of the governor's lapse (then the pause reads the governor
  through coordination).
- A deck layout the parse rule cannot express appears in the owner's collection (then the taxonomy
  learns an explicit deck-to-topic map).

## More Information

SPEC-045; ADR-019; docs/CONTEXT-MAP.md "Overloaded words"; docs/schematics/reading-lifecycle.md.
