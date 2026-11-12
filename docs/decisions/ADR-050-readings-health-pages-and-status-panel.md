---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Readings health: pages go through the router's alert kind once per state and study day, an absent owner is never a failure, and the owner reads the facts on a status panel

## Context and Problem Statement

The predecessor's lane health was sound as a verdict but loud as a practice: a refused topic
produced a coalesced failure message by code path, outside the quiet-hours policy and on top of a
health page, so an absent owner read as a failure. Its telemetry lived on a metrics endpoint the
owner never opens. DeckStreak's readings refuse for honest reasons (a failed sync, a config fault,
a pause, no new cards). Which of them page, through which path, how often, and where does the
owner see the rest?

## Decision Drivers

- Correct refusals (no new cards, a pause) are not failures and must never page.
- One router for every message the owner receives (constraint 2), and no page without dedupe.
- A page carries no private name: topic keys and deck names are private.
- The owner asked for no spend cap but full telemetry, visible to the owner.

## Considered Options (the alternatives it was chosen against)

- Page only `dark`, `stale`, `armed_and_refusing` and each could-not-tell class, as an `alert` occasion through the router, deduplicated per state and study day; run the check after each generation and at the hourly dead-man watch; show the last run, the states, the census and 30 days of cost on an owner-only status panel — chosen: every page is actionable, none repeats within a study day, and the facts are where the owner looks.
- Keep the predecessor's coalesced per-run failure message — rejected because it reports an absent owner as a failure.
- Page every refusing topic separately — rejected because one broken sync would send one page per topic.
- Send pages outside the router, straight to the alert unit — rejected because they would escape the decision ledger and its dedupe, and the charter routes every message through one router.
- Keep the telemetry on a metrics endpoint only — rejected because the owner does not read it; the panel is the owner's surface, and a metrics endpoint can be added beside it.

## Decision Outcome

Chosen option. The verdict is the predecessor's function, proved by its golden. The triage keeps its
total partition over DeckStreak's closed reasons; the predecessor's `stale_snapshot` and
`owner_absent` have no counterpart because the owner replaced the file-age gate with the last sync
(ADR-019), and an absent owner is `paused`. Pages are `alert` occasions, exempt from quiet hours by
the policy, keyed by state and study day, and they carry states, classes and counts only.

### Consequences

- Good, because an owner who simply did not study hears nothing about the readings but the one
  comeback reading.
- Good, because a broken rail pages once a study day, however many checks see it.
- Bad, because the panel shows deck root names to the owner; they never leave the authenticated
  panel for a page or a log.

### Confirmation

SPEC-050's tests; the goldens of `preread_health.py:verdict`, `undetermined_triage.py:classify_undetermined`
and `preread_census.py:classify`.

## What would make this wrong

- The owner wants a daily readings summary in the digest (then the digest's delivery adds a line, and
  pages stay as they are).
- A class of refusal appears that is neither a broken rail nor a config fault (then the triage gains a
  class by a SPEC amendment, and the partition test fails until it does).

## More Information

SPEC-050; ADR-019; the notifications-policy pack's `alert` kind; the predecessor's
`preread_health.py`, `undetermined_triage.py` and `preread_census.py`.
