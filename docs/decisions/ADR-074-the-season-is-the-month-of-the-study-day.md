---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A season is the calendar month of the study day, on every screen and in every rule

## Context and Problem Statement

A season, which the product also calls a chapter, is one calendar month: it accumulates season XP,
carries a personal node track, and closes with one ceremony. The predecessor (`27ee2bc`) draws that
month two ways. Its node track (`pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes`),
its season XP (`database.py:GamifyStore.get_month_xp`, a sum over ledger rows keyed by study day) and
its chapter ceremony (`pipeline_layers/showcase.py:ShowcaseLayer._month_ceremony`, which runs when
the study day is the first of a month) all key the month on the study day. Its season view names
the current season with `gamification/seasons.py:season_period(now_sec, tz_offset_minutes)`, the
local calendar month of the wall clock, which turns at local midnight. The two disagree between
local midnight and the rollover on a month's first day: the view names a month the study day has
not reached, shows that month's season XP (none yet), and the predecessor's own bot hides the node
bar in that window rather than show a stale one (`bot.py:CommandBot._process_update`, its `season`
branch).
DeckStreak's study day turns over at 04:00 local and every screen shows the server's study day
(CHARTER 7, ADR-020). Which boundary does a DeckStreak season use?

## Decision Drivers

- CHARTER 7: every screen shows the server's study day, never a calendar day.
- One boundary for the season view, the node track, the season XP and the ceremony, so the view
  never needs a hidden-bar patch.
- The parity oracle proves the predecessor's functions (ADR-012); a deliberate divergence is recorded
  by its golden's class, as ADR-020 records the digest hour's.
- One scheduled sync per study day, just after the rollover, plus the owner's triggers (ADR-037): the
  boundary must not depend on when a sync runs.

## Considered Options (the alternatives it was chosen against)

- The calendar month of the study day, everywhere — chosen: the view, the node track, the season XP and the ceremony share one boundary, and it differs from `season_period` only between local midnight and the rollover on a month's first day, where CHARTER 7 requires the study day's month.
- The predecessor's split, `season_period` for the view and the study-day month for the rest — rejected because the view would name a month the study day has not reached (CHARTER 7) and would need the predecessor's hidden-bar patch to stay honest.
- The UTC calendar month — rejected because it moves the boundary by the owner's offset, away from both the predecessor's local month and the study day.
- The calendar month of the instant a recompute runs — rejected because the boundary would depend on when a sync runs: an owner's sync between midnight and the rollover would open a month the scheduled sync has not.

## Decision Outcome

Chosen option: "The calendar month of the study day, everywhere", because it is the one boundary
the charter allows and the predecessor itself uses for everything but its view's label.

- **A chapter is the calendar month of a study day.** The kernel's `StudyDay` is an epoch day
  (ADR-020); its chapter is the proleptic Gregorian year and month of that day. A chapter's number
  is `year × 12 + month − 1`, an opaque integer that keys every stored row and every dedupe key
  (`chapter:<number>`, `season_node:<number>:<node>`), so no key carries a calendar string.
- **The season view, the node track, the season XP and the ceremony read the chapter of the study
  day.** The current chapter is the chapter of the current study day; the ceremony closes the chapter
  of the study day before the first study day of a month.
- **The golden records the divergence.** `goldens/season_period.json` carries the predecessor's
  `season_period` as a year and a month, never a `YYYY-MM` string. Its cases between local midnight
  and the rollover on a month's first day carry the class `midnight-to-rollover`; DeckStreak's test
  asserts the predecessor's month for every other case, and the study day's month (the previous
  month) for those.

### Consequences

- Good, because one boundary holds on every screen and in every rule, and the season view never
  names a month the study day has not reached.
- Good, because the node track and the ceremony keep the predecessor's own boundary exactly.
- Bad, because between local midnight and the rollover on a month's first day, DeckStreak's season
  view names the previous month where the predecessor's view names the new one; during side by side
  the two views disagree for those hours, which the golden's class names.

### Confirmation

SPEC-074's acceptance tests: the golden of `gamification/seasons.py:season_period` for every case,
the `midnight-to-rollover` class asserting the study day's month; the node track and the ceremony
keyed by the chapter number; and the ceremony raised once, at the first study day of a month.

## What would make this wrong

- The owner asks for chapters that turn at local midnight whatever the study day says: then the
  view follows `season_period` exactly, and CHARTER 7 gains a recorded exception.
- The rollover hour moves to 0: then the two boundaries coincide and the class is empty.

## More Information

SPEC-074; ADR-012; ADR-020; ADR-037; ADR-071; `docs/schematics/seasons-state-machine.md` and
`docs/schematics/seasons-at-the-study-day.md`; the predecessor's `gamification/seasons.py:season_period`,
`pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes` and `_month_ceremony`.
