---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# The evening check-in is one routed message whose parts keep their own kinds

## Context and Problem Statement

At the evening nudge hour three things may want to speak: the stakes preview (`streak_risk`), the
habit check-in (`habit`) and the focus nudge. The predecessor coalesces them into one message, the
raw part when one fires and an "Evening check-in" of two or three otherwise
(`pipeline_layers/focus.py:FocusLayer.run_evening_nudges`, predecessor `27ee2bc`), and #117 keeps
that. Its coordinator sends the joined text itself, outside its own per-kind path, and when nothing
fires it records a second withhold row under a coordinator kind beside the part's own.

SPEC-041 gives DeckStreak one router, and every message goes through it: a decision per occasion,
one kind, one dedupe key, one ledger row, inside one write. A joined message has three kinds, three
keys and one push. How does a message made of several occasions go through a router built for one?

## Decision Drivers

- Every message goes through the one router (SPEC-041, and the notifications-policy pack).
- Each part keeps its own rules: the stakes' evening budget and holdout, the habit's back-off, each
  kind's switch and dedupe.
- The ledger names one decision per part, never two rows for one cause.
- The owner reads one message, not three in a row (#117).

## Considered Options (the alternatives it was chosen against)

- A joined route: each part decided as if routed alone, one push of the passing parts, each recorded under its own kind — chosen, because every part keeps every rule the router applies, and the owner still reads one message.
- Three separate messages, one per part, each routed alone — rejected because the owner receives up to three pushes a minute apart, which #117 exists to stop.
- One kind `evening` for the joined message — rejected because the stakes' budget and holdout, the habit's back-off and each switch belong to the part's kind, and one kind would apply one set of rules to three occasions.
- The coordinator decides the parts and sends the joined text around the router, as the predecessor does — rejected because a message outside the router is a second delivery path, which SPEC-041 and the pack forbid.

## Decision Outcome

Chosen option: "a joined route".

- `route_together(parts)` decides each part in the given order, as `route` would decide it alone,
  inside one write, and each part's claim is held until the push.
- No passing part: nothing is pushed, and each part's own withhold stands. One: its text is pushed
  as it is. Two or three: under the header `⏰ <b>Evening check-in</b>`, joined by blank lines.
- The push carries every passing part's buttons in part order. Each passing part is recorded once,
  under its own kind and key.
- A failed push releases every passing part's claim and records each `no_notifier`, as a single
  occasion's failed push does.

### Consequences

- Good, because each part keeps its kind's switch, budget, holdout, back-off and dedupe.
- Good, because the ledger holds one row per part and per cause, with no coordinator kind.
- Good, because the joined text is the predecessor's, proved by the golden `evening_check_in`.
- Bad, because the router gains a second entry point, which every rule change must keep equal to
  `route`; the tests route each part both ways.
- Bad, because a failed push withholds all the parts together, where three messages could fail one
  at a time.

### Confirmation

SPEC-100's A13 and A14.

## What would make this wrong

- A part whose rules depend on another part's outcome: none does at W5.
- The owner asks for the parts as separate messages: `route` alone then serves, and the joined entry
  point goes.

## More Information

Cites SPEC-041 (the router), ADR-041 (the router core) and ADR-102 (a hold withholds a whole nudge).
SPEC-100 builds it; its schematic is `docs/schematics/nudge-coordinator-and-holdout.md`.
