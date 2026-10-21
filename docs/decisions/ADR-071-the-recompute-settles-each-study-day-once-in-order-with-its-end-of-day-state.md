---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The recompute settles each study day once, in order, with the state it had at its close

## Context and Problem Statement

W3 ports the predecessor's game core: the daily rollup and score, XP, streaks, quests, chests and
coins. The predecessor evaluates several of its rules only for the study day that is current when
a recompute runs (predecessor `27ee2bc`):

- the card snapshot (the workload and mastery pillars, the backlog and the cards due today) is
  taken at the current day's collection day number, and every other day is scored without one
  (`pipeline.py:GamifyPipeline._recompute_day`, its `is_today`);
- the backlog-zero bonus is derived only for the current day (`backlog_zero = is_today and ...`
  in the same function);
- the persisted language streak advances only when the current day holds reviews
  (`pipeline.py:GamifyPipeline._advance_streak`), and the law streak likewise
  (`pipeline_layers/digests.py:DigestsLayer._update_law_streak`);
- session chests and the daily quests read only the current day's reviews
  (`pipeline_layers/loot.py:LootLayer._grant_session_chests`, `LootLayer._update_quests`), and the
  relight reads only the current day's rollup (`pipeline_layers/showcase.py:ShowcaseLayer._relight`).

A rule that needs a day's final state therefore sees it only when a recompute runs while that day
is still the current one. DeckStreak syncs once per study day, just after the rollover, plus the
owner's own triggers (ADR-037). At that scheduled recompute the current study day is the new one,
which holds no review yet. Ported verbatim, the backlog-zero bonus, the card snapshot, chests,
quests and the relight would fire only on days the owner triggers a sync; and the persisted streak
would never record a study day the owner did not sync on, so two such days in a row would break it.

A second behaviour is at stake. The predecessor recomputes every study day of its window, and for
each it first clears the day's study-owned XP sources and derives them again
(`pipeline.py:_STUDY_DAY_XP_SOURCES`, `database.py:GamifyStore.clear_xp_sources`). Once a day is
no longer the current one, its backlog-zero bonus is derived as false, its score is taken without
the card snapshot, its consistency grant is folded over rollups that now include the days after it
(`pipeline_layers/governor.py:GovernorLayer._on_pace_run` reads the most recent rollups), and its
volume pillar uses the current day's baseline (`pipeline.py:GamifyPipeline._baseline`). XP a day
earned while it was current can therefore fall after it closes, with no action of the owner.
CHARTER 5 makes XP structurally unconfiscatable.

How should the recompute evaluate days so that the predecessor's rules keep their meaning under one
sync a study day, and no closed day's XP falls?

## Decision Drivers

- ADR-037's cadence: one scheduled sync per study day after the rollover, plus the owner's
  triggers; raising it waits for cutover (#164).
- The parity oracle (ADR-012): every rule is the predecessor's own function, proved by its golden.
  Only which day a rule is evaluated for, and which state it reads, belongs to the pipeline.
- CHARTER 5 (XP, streaks, levels, badges and CEFR progress are never taken) and CHARTER 7 (the day
  turns over at 04:00 local, and every screen shows the server's study day).
- Idempotency guards travel with their feature (CHARTER 9): a second recompute must change nothing
  it already settled, and pay nothing twice.
- One read of the review window per recompute (SPEC-023), and no per-day work that grows with the
  age of the window.

## Considered Options (the alternatives it was chosen against)

- Settle each closed study day once, in order, with the state it had at its close — chosen: every day is evaluated once as the current day it was, the predecessor's today-only rules keep their meaning, and a later recompute only raises a closed day's pay.
- Port the predecessor's pipeline verbatim — rejected because at the scheduled recompute the current day is the new, empty one, so the today-only rules would fire only on the owner's syncs and the persisted streak would break.
- Move the scheduled sync to just before the rollover — rejected because it moves ADR-037's slot, misses the reviews of the day's last minutes, and still clears each past day's XP at the next recompute.
- Raise the sync cadence now — rejected because ADR-037's condition (b) holds one sync a study day, plus the owner's triggers, until cutover (#164).
- Sync each time the Mini App opens — rejected because ADR-037 decided that opening the app is not an explicit trigger.
- Recompute every window day at each recompute and clear and reinsert its XP, as the predecessor does — rejected because it removes XP a closed day earned (CHARTER 5) and moves a closed day's pay with later days' data.
- Freeze a closed day entirely after its settle — rejected because a review that reaches the sync server late would never count.

## Decision Outcome

Chosen option: "Settle each closed study day once, in order, with the state it had at its close",
because it is the one evaluation order under which every predecessor rule reads what it read while
its day was current, whatever the sync cadence, and it never takes back what a closed day earned.

- **The fold.** After each successful sync, scheduled or the owner's, the recompute
  (`crates/coordination/src/recompute/`) settles every closed study day after the last settled
  one, oldest first, and then evaluates the current study day as far as it has gone.
  - A closed day is settled only by a recompute that follows a successful sync which started after
    that day closed (the study day's sync outcome, ADR-037). Until then it stays owed, and the
    recompute evaluates only the current day, from the copy as it stands. A cycle in which nothing
    changed still settles a day left owed: the start of the study day's first successful sync is an
    obligation's deadline (`owed_settle`), so the change gate runs one recompute after it and none
    once one has run (SPEC-071 §10, "Continuation").
  - The settle cursor is analytics' own record: a settled day's rollup carries the instant it was
    settled, and the last settled day is the cursor. A recompute never settles a day twice.
  - With no settled day at all (the first recompute), every study day of the window is rolled up
    and scored in the predecessor's historical form, with no today-only rule, as the predecessor's
    own first recompute derives its past days; the fold settles from the most recently closed day
    on.
- **A closed day is evaluated as the current day it was.** Its settle runs every registered step
  with that day as the day evaluated: the card snapshot at its own collection day number (the rule
  of `analytics.py:today_day_number`), the volume baseline and the consistency run over the rollups
  before it, the raw streak ending on it, and every today-only rule of the predecessor (the
  backlog-zero bonus, the streak's advance, the relight, chests and quests). Only the most
  recently closed day has an end-of-day card state; a day settled after a gap has none, and keeps
  none, as the predecessor keeps none for a day it never saw.
- **The step order.** Each context registers one step in a fixed phase order: (1) the rollup and
  score (analytics), (2) base XP (progression), (3) the streaks and the governor, (4) the other
  contexts' day steps (habits, focus, quests, chests), (5) the derived bonuses (consistency and
  Ascendant), (6) the coin mint, (7) awards (badges and records). The derived bonuses and the mint
  run after every base source of the day, which is the predecessor's own result once it has
  recomputed a day twice. The evaluation of a day arms that day's Ascendant from the previous day's
  settled grants, as the predecessor arms it before its recompute.
- **After the settle.** A later recompute re-rolls a settled day only when its reviews changed (a
  review that reached the sync server late). It re-scores the day's stored score in the
  predecessor's historical form (no card snapshot, the current day's baseline), which is what the
  predecessor stores for a past day; it never overwrites the day's card state or the score the day
  closed with; and it raises the day's XP, never lowers it (ADR-072).
- **The current day** follows its record at every recompute: its card state is the live snapshot,
  its derived XP rises and falls with the record (ADR-072), and each event it pays (a chest, a
  quest, a relight) carries a key, so the closing settle repeats none of them.

### Consequences

- Good, because the predecessor's rules are evaluated for every study day as they were when that
  day was current, under one sync a study day and under any later cadence.
- Good, because no closed day's XP falls, and every grant has one key that a second recompute
  cannot pay twice.
- Good, because a recompute reads the window once and rolls up only the days whose reviews changed,
  the days it settles and the current day.
- Bad, because a closed day's XP differs from the predecessor's during side by side: the
  predecessor removes a past day's backlog-zero bonus and re-derives its other bonuses, and
  DeckStreak keeps what the day earned at its close. The side-by-side verification (#62) compares
  with this rule named.
- Bad, because a moment that happens during a day (a chest, a quest's completion, the relight) is
  announced at the recompute that settles the day, unless the owner triggers a sync during it
  (ADR-037's consequence).
- Bad, because a review made between the rollover and the settle can lower the closing day's
  backlog in its end-of-day card state.

### Confirmation

SPEC-071's criteria on the fold: each closed day settled once, oldest first, and none twice; the
closing day settled with its end-of-day card state and the score it closed with; no settle before
a successful sync that started after the close; the first recompute's historical backfill; the
phase order; and the incremental re-roll. Each later W3 SPEC proves its own step inside the fold.

## What would make this wrong

- The owner asks for the predecessor's exact past-day XP, removals included: CHARTER 5 is then
  amended first, and this order with it.
- The cadence rises at cutover (#164) so that the current day is recomputed often: the fold still
  holds, but the closing settle then mostly repeats the current day's last evaluation, and a
  cheaper form may be measured.

## More Information

ADR-012; ADR-037; ADR-072; SPEC-023; SPEC-071, which builds the fold; the schematic
`docs/schematics/recompute-settles-each-study-day.md`. The predecessor's
`pipeline.py:GamifyPipeline._recompute_day`, `_days_to_recompute`, `_advance_streak` and
`_baseline`, `pipeline.py:_STUDY_DAY_XP_SOURCES`, `database.py:GamifyStore.clear_xp_sources` and
`pipeline_layers/governor.py:GovernorLayer._on_pace_run`.
