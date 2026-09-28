---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The law streak is evaluated on every study day, so it decays when law study stops

## Context and Problem Statement

The law track keeps its own streak: consecutive study days with at least one law review, bridged
by declared skip days, with no freezes and no comeback (the predecessor's feature `law-streak`,
#82). The predecessor computes it with `analytics.py:bridged_streak`, which answers correctly for
any day it is asked about, but its caller, `pipeline_layers/digests.py:DigestsLayer._update_law_streak`,
returns early on every day without a law review. So the stored law streak is rewritten only on law
study days: once law study stops, it keeps its last value until the next law study day, and every
surface that reads it (the law block that leads the home screen, `/today`, the digests) shows a
streak that has already ended. The schematic `docs/schematics/streaks-and-governor-state-machine.md`
already records that this defect is fixed, not ported. How does DeckStreak keep the law streak true
without inventing a rule the predecessor never had?

## Decision Drivers

- The eleven anti-goals include no dishonest copy (CHARTER 10): no surface may show a streak that
  has ended.
- The math ports verbatim (CHARTER 8): the streak's value must still be the predecessor's own
  function, proved by its golden.
- The law track has no freezes and no comeback by the predecessor's design: the caller's own
  documentation keeps those mechanics on the language track.
- Law is primary: the law block leads the home screen whenever there is law activity (#134, #69),
  so a stale value would lead with a false number.
- The recompute settles each study day once, in order (ADR-071), so the streak can be evaluated
  for every day, with law study or without.

## Considered Options (the alternatives it was chosen against)

- Evaluate `bridged_streak` on every settled study day and on the current study day, law study or not — chosen: the stored value is always what the predecessor's own function says the law streak is on that day, and only the caller's early return is dropped.
- Copy the predecessor's caller, which writes the law streak only on law study days — rejected because once law study stops the stored value never decays, and the law block would lead the home screen with a streak that has ended (dishonest copy, CHARTER 10).
- Store only the longest law streak and compute the current one on every read — rejected because every reader would need the window's law study days, which only the recompute reads, and the settle's later steps read the current value too.
- Decay the law streak through the language streak's gap classifier and freezes — rejected because the law track has no freezes by the predecessor's design, and borrowing the classifier would invent a freeze rule the owner never had.
- Zero the law streak only when a break marker is written — rejected because it needs a state the predecessor lacks and still shows a stale value between the last law study day and the marker.

## Decision Outcome

Chosen option: "Evaluate `bridged_streak` on every settled study day and on the current study
day", because it keeps the predecessor's function and its golden unchanged and removes only the
early return that made the stored value lie.

- At each recompute, the fold (ADR-071) sets the law row's current value to
  `bridged_streak(law study days, day, declared skip days)` for every study day it settles, in
  order, and then for the current study day. The longest is the larger of itself and each current
  value; the last law study day is the latest day with a law review.
- `bridged_streak` treats the day it is asked about as unfinished: a day with no law review yet
  keeps the run that ended the day before, and the study day after a missed day that was not a
  declared skip reads zero until a law review starts a new run.
- The law row's freezes stay at the start value, and nothing consumes or grants one.

### Consequences

- Good, because the law streak on every surface equals the predecessor's own function for that
  day, proved by `goldens/bridged_streak.json`.
- Good, because no state and no rule is added: only a call site changes.
- Bad, because during side by side the predecessor's bot keeps showing its stale law streak while
  DeckStreak shows a smaller, true one; the side-by-side verification treats this row as a
  recorded divergence (ADR-011, #62).
- Bad, because a law review that reaches the sync server after its day has settled does not rejoin
  the law streak, as for the language streak (SPEC-076, section 6).

### Confirmation

SPEC-076's acceptance tests: the law streak equals the golden of `analytics.py:bridged_streak` on
every settled and current day (A9), it reads zero after a study day with no law review and no
declared skip (A10), and no freeze is spent from or granted to the law row (A11); and the mutation
row that restores the early return (S07607).

## What would make this wrong

- The owner decides the law block should keep showing the last law run while law study pauses:
  then the view adds that value beside the streak, and the rule stays.
- The owner gives the law track freezes: then the law streak moves to the language streak's gap
  classifier with its own freezes, by a new decision.

## More Information

SPEC-076; ADR-071; the predecessor's `analytics.py:bridged_streak` and
`pipeline_layers/digests.py:DigestsLayer._update_law_streak` at `27ee2bc`;
`docs/schematics/streaks-and-governor-state-machine.md` and
`docs/schematics/streaks-governor-and-freezes-in-w3.md`; #82.
