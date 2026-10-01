---
status: accepted
date: "2026-10-01"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The streak calendar is derived on read from the settled days, over a bounded window

## Context and Problem Statement

SPEC-076 R20 and R23 name a streak calendar: the window's study days with their freeze, skip and
break markers, drawn on the Mini App's streak screen. The first delivery left it out (section 18)
and it is #486. The predecessor serves a streak calendar of its own, with no markers. Which window
does it serve, which day does each marker sit on, and where do the study days it marks come from?

## Decision Drivers

- A marker on a day other than its own is a wrong fact about the owner's streak, so a day's markers
  must be decided by one rule that every path shares.
- The streak is replayed from its study days, so the markers should come from that replay and never
  from a second copy of its rules.
- SPEC-020's data-rights rows are owed for every table, so a new table is a real cost.
- The route answers in one read of the store, so the calendar and the counts beside it are one moment.

## Considered Options (the alternatives it was chosen against)

D1, parity first:

- Search the predecessor for a calendar, and bind to it if it serves one: chosen, because the
  predecessor's behaviour is the product's source of truth (#486). The predecessor serves one:
  `charts.streak_calendar`, the last 26 weeks of study days, binary (a day is studied when its
  rollup has reviews above zero, and its reviews count study events only), from the Monday on or
  before the served day minus 181 days through the served day, with no track split and no markers.
  Its window and its studied layer bind with a golden, which the parity oracle's own generator
  writes by calling it through an adapter in `registry/spec_076.py`
  (`tools/parity-oracle/goldens/streak_calendar.json`), and D2 yields to it. The markers have no
  predecessor and stay under D3.
- The studied layer is judged as a union: each track serves exactly the predecessor's window, in
  order, and the union of the two tracks' study days equals the predecessor's studied set, because
  the predecessor keeps one day set and DeckStreak keeps one per track. A difference is allowed
  only where it is named here, by day class and reason; an unnamed one fails A63. The named
  classes, none of which A63's golden cases hold:
  - A closed day whose study reviews were deleted after it closed: DeckStreak keeps it studied,
    because a closed day's settled amount is only ever raised (`settle.rs`), while the predecessor's
    rollup takes the recount whenever that day is recomputed. Read from the code, not run.
  - The open day before its writer has run: neither serves from the review log. DeckStreak's route
    reads the settled rows, which the fold writes when a sync is folded; the predecessor's chart
    reads its stored rollups (`recent_days`), which its pipeline writes when it recomputes. Until
    its own writer has run, each serves the open day as not yet studied.
  - A card moved between tracks after a day closed: it changes one track's day set and not the
    union, so it is a difference between the tracks and never between the union and the
    predecessor.

D2, the window:

- The predecessor's window: from the Monday on or before the served day minus
  `CALENDAR_LOOKBACK_DAYS` (181, `CALENDAR_WEEKS` (26) weeks less the served day) through the served
  day, 182 to 188 days by weekday: chosen, because D1 binds the predecessor's behaviour and the
  predecessor serves this window. It starts on a Monday, so the screen lays it out in whole weeks,
  and it bounds the body for any history length (#486).
- The 35 days ending at the served day, a constant: rejected, because ruling 1 binds the
  predecessor's window (D1), and the predecessor serves 26 weeks.
- The whole history: rejected, because the body then grows without bound with every day studied.
- The current calendar month: rejected, because the window would be one day long on the first of the
  month and would change its length through the year.

D3, the day a marker sits on:

- Each marker on its own day: chosen, because each then sits on the day the owner would point at, and
  the replay already decides all three (freeze on the covered miss, break on `broke_today`, skip on
  each skip day) (#486).
- The freeze on the return day, where the replay spends it: rejected, because the return day is a
  study day and the miss it hides would show as nothing.
- The language track's `break` where its replay records it: chosen, because that is its own day:
  the day after the second real miss of a live run (the day whose lapse records it), or a return
  day after a break the lapse path did not record.
- The law track's freeze: not served, because the law track holds no freezes (R10). Its `break` sits
  on the day after the first real miss of a live run, the day its replay resets the run.
- A `break` on the miss itself: rejected, because on both tracks the replay records the break on a
  later day, and a marker sits on the day its own replay records it.

D4, where the study days come from:

- Settled review XP per track: chosen, because it is stored per track (`xp_settlement` rows, `reviews`
  on `language` and `reviews_law` on `law`, amount above nothing)
  and written by the same fold that writes the freeze events, and it is the set of study days of the
  track. `review_xp` is zero only for an event that is not a study event, and the smallest XP of a
  study event is the economy's base of 10 times the lowest ease (again, 0.5), maturity (new, 1.0),
  type (filtered, 0.7, the lowest) and tier (untagged or T1, 1.0) multipliers, 3.5, which rounds
  half to even to 4. So a day's settlement is above nothing if and only if the day has a study event
  of that track.
  The fold settles a day for every evaluation, an open day's row replaced as it grows (SPEC-072 R6).
- `daily_rollup` and `daily_lang_stats`: rejected, because the first holds no track and the second
  is not the study-day set the streak fold reads.
- The revlog window the fold itself reads: rejected, because the collection is the ingest's to read
  and not the route's, and the route would then open a second store.
- A stored marker table or a stored day set: rejected, because it would owe SPEC-020's data-rights
  rows and could drift from the replay (see D5).

D5, markers derived on read:

- The markers are computed on read from the replay over the settled days: chosen, because a stored
  marker could disagree with the replay.
- A stored marker table: rejected, as above.

## Decision Outcome

Chosen: the predecessor's window, its studied layer held by a golden, the three markers on the days
D3 names, and the study days read from the settled XP of each track, all in one read transaction
beside `streak_state`.

### Consequences

- Good, because one pure function in `deck-streak-streaks` owns the days and the markers and the
  route only reads and shapes them.
- Good, because no table is added.
- Bad, because the study days are a proxy: a recompute that lowers a closed day's amount leaves it
  as it was (the raise-only rule), so a deleted review keeps its day studied until the owner's
  correction. The fold's own streak replay reads the revlog and not the XP, and A59 holds the two
  equal for a constructed population.

### Confirmation

A57 to A64 (SPEC-076 section 28): the pure function over a population built by construction, the
route over the real store after the fold, the settlement's equality with the study days, the open
day, the screen's reading of each marker from its own cell, the law track's break day, the route's
window and union against the predecessor's golden, and the screen's whole weeks.

## More Information

#486, SPEC-076 R20 and R23, ADR-076, ADR-072.
