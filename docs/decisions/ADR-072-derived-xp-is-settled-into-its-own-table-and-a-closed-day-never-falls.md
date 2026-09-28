---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# XP derived from the study record is settled into its own table, and a closed study day's settled XP never falls

## Context and Problem Statement

SPEC-040 built progression's grant port over one table, `xp_ledger`, and ADR-040 made it
append-only: a grant is written once, its amount is unsigned, and the port offers no update, debit
or delete. SPEC-040 left one thing open on purpose: it "does not clear and re-derive study-owned
sources on each recompute" (its section 5, citing #70 and #71).

The predecessor keeps every kind of XP in one ledger and rewrites part of it at every recompute
(predecessor `27ee2bc`). `pipeline.py:GamifyPipeline._recompute_day` deletes the study-owned sources
of each day it recomputes (`database.py:GamifyStore.clear_xp_sources` over
`pipeline.py:_STUDY_DAY_XP_SOURCES`) and writes them again with
`database.py:GamifyStore.upsert_xp_grant`, whose `ON CONFLICT ... DO UPDATE` replaces an amount in
place. The habit sources are re-derived the same way after each entry or undo
(`pipeline_layers/habits.py:HabitsLayer._recompute_reading_xp` and `_recompute_writing_xp`,
`pipeline_layers/focus.py:FocusLayer._recompute_focus_xp`), and so is a double-XP token's bonus
(`pipeline_layers/loot.py:LootLayer._recompute_token_xp`).

Three facts follow, each read in that code:
- a day's review XP must follow its reviews as they arrive, including reviews synced late;
- the owner's undo of a reading entry, or a writing confirmation toggled off, takes back exactly the
  XP it earned;
- the recompute evaluates `backlog_zero` only for the current study day and scores a past day
  without its card state, so once a day closes the recompute removes its `backlog_zero` grant, and
  removes its `score90` grant whenever the past-day score falls below 90. The owner's total, and
  with it the level, falls at the rollover.

CHARTER 5 makes XP structurally unconfiscatable. How should DeckStreak store the XP it derives from
the study record, so that the amount can follow the record, without a debit path and without a
closed day's XP ever falling?

## Decision Drivers

- CHARTER 5: XP is never confiscable, so no path may lower a closed day's XP.
- ADR-040's guarantees stand for every grant: append-only, once-guarded, unsigned, one census.
- The record changes after a grant: reviews arrive late, a day's value is provisional until the day
  closes (ADR-071), and the owner corrects their own manual entries.
- Parity: every derived amount equals the predecessor's function (ADR-012), and the level counts
  every XP the owner holds.
- The v9 import (#61) maps the predecessor's one ledger into DeckStreak's tables row by row.

## Considered Options (the alternatives it was chosen against)

- A settlement table beside the grant ledger, written by one `settle` operation for a closed registry of derived sources — chosen: grants keep ADR-040's append-only guarantee, and the one table that may change holds only amounts recomputed from the owner's own record.
- A `settled` scope in `xp_ledger` whose rows an update may replace — rejected because it breaks ADR-040's append-only guarantee and SPEC-040's census, which hold that the port never updates a grant.
- Append-only deltas, one new row per change of a derived amount — rejected because an owner's undo would need a negative row, which the unsigned amount forbids, and the table would grow at every recompute.
- Deriving the XP on every read from the rollups and the logs, storing none — rejected because each level read would re-derive the whole window, and a day's XP by source would depend on data progression does not own.
- The predecessor's clear and reinsert in one table — rejected because it is a delete path over XP, and it lowers a closed day's XP at the rollover.
- Letting a later recompute lower a closed day's settled amount, as the predecessor's recompute does — rejected because the owner's total would fall at a rollover (CHARTER 5).

## Decision Outcome

Chosen option: "a settlement table beside the grant ledger", because it lets the derived amounts
follow the record exactly as the predecessor's functions compute them, while no grant is ever
rewritten and no closed day's XP can fall.

- **The table.** Progression owns `xp_settlement`: one row per study day, source and track, an
  unsigned amount (`CHECK (amount >= 0)`), whether the study day had closed when the row was last
  written, and `created_at`. It is `STRICT`, unique on (study day, source, track), and created by
  SPEC-072's migration. No other crate names it in a query.
- **The operation.** `settle` takes a study day, a source, a track, an amount and a cause (a
  recompute, or the owner's correction of a manual entry), in one `BEGIN IMMEDIATE` write through the
  kernel's repository base:
  - a source outside the derived registry is refused before any write;
  - while the study day is open, the amount replaces the stored one: the day's value follows the
    record, and may fall, as the predecessor's does for the current day;
  - once the study day has closed, a recompute stores the larger of the stored and the recomputed
    amount, so it raises the row and never lowers it; the owner's correction replaces it.
- **The registry.** The derived sources are a closed list, extended only by the SPEC that derives
  each: SPEC-072's `reviews`, `reviews_law`, `studied`, `backlog_zero`, `streak`, `score90`,
  `graduations`, `consistency` and `ascendant`; SPEC-078's `read:<code>`, `readgoal:<code>`,
  `write:<code>` and `write:all`; SPEC-079's `focus`, `focusgoal:` (on the week's first study day)
  and `focus_combo`; and
  SPEC-081's `2x:<token>`. A day in a source is its epoch day number, never a date.
- **The grants.** The grant port and `xp_ledger` do not change (ADR-040): quests, chests, the relight,
  band-ups and the readings grant through them.
- **The readers.** The total and the level are read from both tables and never stored (SPEC-040 R7,
  R8). The day base, which the consistency bonus and the coin mint read, is the predecessor's
  `database.py:GamifyStore.day_base_xp` over both tables (SPEC-072).
- **Who settles.** Only coordination's recompute steps and the owner-correction use cases call
  `settle`; a census in SPEC-072 refuses any other caller, so no fine, stake or penalty path can reach
  it.
- **The import.** The v9 import (#61) writes the predecessor's derived sources to `xp_settlement`, as
  closed days, and every other source to `xp_ledger`.

### Consequences

- Good, because a recompute replayed on the same record writes the same amounts, and an owner's undo
  takes back exactly the XP its entry earned.
- Good, because ADR-040's guarantees and SPEC-040's census hold unchanged for every grant.
- Good, because no rollover lowers the owner's total or level.
- Bad, because XP lives in two tables, so every reader of XP (the level, the day base, a day's XP by
  source, the exchange readout, a season's XP, the export) reads both.
- Bad, because an open day's derived XP is provisional and can fall before the day closes, so a
  screen that shows today's XP says it settles at the day's close.
- Bad, because a closed day keeps a `backlog_zero` or `score90` bonus the predecessor would remove at
  the rollover, so DeckStreak's total can exceed the predecessor's during side by side; the
  side-by-side verification (#62) compares XP by source and expects that difference.

### Confirmation

SPEC-072's acceptance tests: a closed day raised and never lowered by a recompute, the owner's
correction replacing it, the open day following its record, a source outside the registry refused,
the level read from both tables, and the census of the table and of `settle`'s callers. SPEC-072's
mutation rows hold the raise-only rule and the table's unique key.

## What would make this wrong

- The owner decides that a closed day's bonus may be withdrawn after all: CHARTER 5 would be amended
  first, by an owner decision recorded in an ADR.
- A derived source turns out to need a once-ever scope: it then belongs to the grant port.
- Summing two tables on each read becomes a measured latency problem: the API's SLO (SPEC-031) pages,
  and a cache is a separate, measured delivery.

## More Information

SPEC-072; SPEC-040 and ADR-040 (the grant port); ADR-071 (a closed study day is settled once, in
order); CHARTER 5; SPEC-078, SPEC-079 and SPEC-081 extend the registry; the v9 import (#61); the
side-by-side verification (#62).
