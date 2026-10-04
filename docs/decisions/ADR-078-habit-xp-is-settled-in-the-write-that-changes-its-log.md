---
status: "accepted"
date: "2026-10-03"
decision-makers: "the DeckStreak architect seat"
---

# Habit XP is settled in the write that changes its log

## Context and Problem Statement

SPEC-078 part 078a ports the predecessor's minutes log of book reading (#93): an entry earns
`read:<code>` XP on its study day, a week whose minutes reach the goal earns `readgoal:<code>` on its
first study day, and `/undo` removes the newest entry. ADR-072 settles derived XP into its own table
and lets a recompute only raise a closed day's amount; only the owner's own correction may lower one.
`crates/progression/src/settle.rs` admits nine derived names and refuses every other source before a
write. Five questions stood between the SPEC and the code: whether an entry's settles commit with its
log write or after it, how the recompute's fold re-derives habit XP, how the closed registry admits
a per-course source, what a stale Undo button does, and who announces a level a habit write crosses.

## Decision Drivers

- An undo that lowers a closed day must never be left half done: a recompute stores the larger
  amount on a closed day, so a lowering that did not happen can never be made later.
- The fold settles each study day once, in order, inside that day's write (ADR-071), and a step
  writes no other day's rows.
- `the_derived_registry_and_the_tables_are_pinned_whole` pins the nine derived names whole, and every
  caller of `settle` passes a source and nothing else.
- A button the owner taps acts on what it showed the owner.
- A celebration passes through the one router, once (ADR-041); a level is announced once ever.
- No new edge in the context map: coordination already depends on habits, progression and the
  router.

## Considered Options (the alternatives it was chosen against)

- One write per habit use case: `log_minutes`, `undo_newest` and `undo_entry` each run in one
  `db.write()` (`BEGIN IMMEDIATE`) that holds the log write and its settles of `read:<code>` on the
  entry's day and `readgoal:<code>` on its week's first study day, with the owner's correction as
  their cause and `closed` when the day is before today: chosen because the log and its XP commit
  together or not at all.
- Two writes, rejected because after an undo the fold's heal can never lower a closed day: the log
  write committed first and the settle in a write of its own, with the fold to heal what a failure
  between them left; a recompute keeps the larger amount, so a failure between the two writes
  over-pays that day for good.
- The fold's habit step, chosen because each day's write touches only that day's rows: `HabitsStep`
  in `Phase::DaySteps` inside each evaluated day's write, settling `read:<code>` for every course
  with an entry or a held `read:` row on the day, and `readgoal:<code>` only when the day is its
  week's first study day, from that week's minutes.
- Settling a week's bonus from every later day's evaluation: rejected because it writes another
  day's row inside a day's write, a step the fold's one-settle-per-day order does not take.
- A second registry, chosen because the nine names and their pinned test stand: `DERIVED_PREFIXES`
  (`"read:"` and `"readgoal:"`, one per line), and `is_derived(source)`, which admits one of the
  nine names or a prefix followed by a valid course code, with `settle` refusing
  `!is_derived(source)`; part 078b adds `"write:"` as one more line.
- Turning the nine names into a list of patterns: rejected because it breaks the registry's
  pinned-whole test and every reader of the array.
- Passing the configured `Courses` into `settle`: rejected because it changes the signature for every
  caller and the census's probe on `settle`, to check what the course code's own shape already
  checks.
- The Undo button carries its entry's id, chosen because a button never removes an entry it did not
  show: the button's data is `hb:u:<id>`, and it removes that entry only while it is still the
  newest; otherwise it removes nothing and says to send `/undo`, which removes the newest entry
  whatever its day.
- The predecessor's button, which removes the newest entry whatever it was rendered for: rejected
  because a stale button then removes an entry the owner never saw on it.
- The use case announces the level-up after commit, chosen because every adapter of the use case
  gets the announcement without remembering it: it reads the level before and after its write, and
  after commit calls `announce_level_up(router, before, after, today)`, whose once-ever key
  `level:N` keeps a sync cycle and a habit write from both celebrating one level.
- The bot adapter announcing the level-up: rejected because every later adapter (part 078b's toggle,
  part 078c's routes) must then remember to.
- Leaving the level-up to the next sync cycle: rejected because that cycle reads `before` after the
  habit write and never sees the crossing, so the level is never announced.
- Recording the closed-day residue: an undo that lowers a closed day's `read:` amount leaves the
  day's derived bonuses and the coins minted from its base as they were settled at its close, and
  SPEC-078 names this in an exclusion citing #93: chosen because it couples habits to nothing else.
- An undo that re-derives the closed day's bonuses and mint, rejected because it makes a habit use
  case reach into two other contexts' settled history: progression's derived bonuses and economy's
  mint for the closed day.

## Decision Outcome

Chosen options: "one write per habit use case", "the fold's habit step settling the week's bonus on
its first study day only", "`DERIVED_PREFIXES` and `is_derived`", "the Undo button carries its
entry's id", "the use case announces the level-up after commit" and "recording the closed-day
residue".

The study week runs from the local Monday's 04:00 rollover to the next: its first study day is
`d - (d + 3).rem_euclid(7)` over the kernel's study days, as `crates/streaks/src/calendar.rs` computes
its weeks. The amounts are habits' own constants in `crates/habits/src/minutes.rs`, held equal to
the predecessor's by a golden; `economy.json` gains no key, because the game-economy reference
refuses one it does not name.

### Consequences

- Good, because an entry, an undo and a recompute serialise on the write base, and each leaves the
  settled amounts equal to the rule over the log it committed.
- Good, because an undo lowers a closed day exactly, in the same write that removed the entry.
- Good, because a settle the fold finds missing on a day it evaluates is healed with the same rule's
  value.
- Good, because the registry's nine names and their test are unchanged, and a source such as
  `read:QAA` or `reading:read:r1` is still refused before any write.
- Good, because a stale button removes nothing.
- Bad, because a closed day's derived bonuses and minted coins keep the base they read when an undo
  later lowers that day's reading XP (the recorded residue, #93).
- Bad, because a past day whose settle a failure left undone heals only when the fold evaluates that
  day again, which it does for days with reviews and for today.
- Bad, because an Undo button on an older reply answers stale even when the owner meant the newest
  entry, and the owner then sends `/undo`.

### Confirmation

SPEC-078's tests: an entry and its undo leave every settled amount as it was, a closed day's and an
earlier week's included (A5); a stale button removes nothing (A5b, A27); a row left with no settle
is settled by the fold's step (A6); entries and a recompute on two connections end at the rule over
the final log (A6b); a crossing is announced once (A30); `read:qaa` and `readgoal:qaa` settle and
malformed sources are refused (A23); the production fold registers the step (A24). The formal entry
`formal/tla/HabitXpFollowsItsLog/` checks `ReadXpFollowsTheLog`, `GoalBonusFollowsItsWeek` and
`ALevelUpIsCelebratedOnce`, and one witness is the settle written apart from its undo.

## More Information

What would make this wrong: a habit write path that cannot share the settle's write, such as an
adapter that writes `minutes_log` outside `coordination::habits`. SPEC-078 (R3, R4, R5, R18 and its
section 5); ADR-041; ADR-071; ADR-072; ADR-087; the predecessor's
`pipeline_layers/habits.py:HabitsLayer.record_reading`, `undo_last_reading` and
`_recompute_reading_xp`, and `bot.py:CommandBot._callback_log_reading` at `27ee2bc`; #93.

## Amendment, 2026-10-03: part 078b (#94)

SPEC-078 part 078b adds the writing habit and the habit badges (SPEC-078 section 13). The writing
confirmation is a third habit write beside an entry and an undo, the fold gains a writing step, and
phase 7 gains a second awarding step. Each choice below names the option it beat.

### Considered options

- One write per toggle, holding the log change and both settles, chosen because a confirmation
  cleared on a day that later closes must lower its `write:` rows in the write that cleared it, as
  an undo lowers `read:` (the decision above, unchanged).
- A toggle whose settles are a write of their own, rejected because a failure between the two
  writes leaves a cleared day paid, and a recompute never lowers a closed day (ADR-072).
- A chip that carries its study day, `hb:w:<code>:<epoch day>`, chosen because a tap after the
  rollover then toggles nothing rather than a day the owner was not shown.
- A chip with no day, rejected because after the rollover it toggles today, a day its checklist
  never showed.
- A separate writing step in its own file, chosen because part 078a's habit step, its row S07813
  and its cover `recompute/habits.rs anchor=evaluate` stay as they landed and proved.
- Writing settles added to part 078a's habit step, rejected because it re-anchors a landed row and
  re-stamps a landed cover, and a second `evaluate` in one file resolves no anchor.
- Refusing a writing course coded `all` at start, through the existing `Malformed` refusal, chosen
  because its `write:<code>` would be the bonus's source `write:all`, and every role shares the
  refusal while no caller matches a new variant.
- Renaming the bonus's source, rejected because the settled history and the predecessor's import
  both name `write:all`.
- A new `CoursesError::Reserved` variant, rejected because `Malformed` with a new shape keeps every
  caller and adds no Display arm.
- No bonus and no streak over an empty writing set, chosen because a bonus for confirming nothing
  pays XP the mint reads; the predecessor's subset test pays it.
- Porting the predecessor's empty-set answers, rejected because they pay the bonus and the reading
  badges for no course at all.
- Habit badges awarded by the fold only, in a second phase-7 step before the badge step, chosen
  because one actor awards and the offers already drain every unmarked badge through the router.
- An award after each owner write as well, rejected because it is a second award-and-celebrate actor
  that the AwardOnce model does not hold.
- The badge context of the evaluated day, chosen because a settle that catches up an owed day judges
  it by its own week.
- The wall clock's today, rejected because a catch-up settle would judge an owed day by a later week.
- Named reply methods `Commands::confirm_writing` and `Commands::clear_writing` with census entries,
  answering the commands `/write` and `/unwrite`, chosen because the one-router census names every
  reply method, and the admission is recorded once.
- Reply methods named `write` and `unwrite`, rejected because the census counts every `.write(` in
  the handler's module as a call of the reply, so the capture download's stream write in
  `Commands::save_media` would read as an unnamed caller, and naming that caller would be a false
  edge.
- Replies built inside `on_message`, rejected because the census could not name them by method.
- A chip tap answered with a new message, chosen because the edit the bot has carries no keyboard.
- An in-place edit of the checklist, rejected for this part and filed as its own successor (#601).
- The six constants added to the constants golden, A14's test gaining six arms, chosen because the
  golden is the one record of the predecessor's values and the test stays exact.
- A second constants test for this part's values, rejected because two tests would then hold one
  golden.
- A Lean proof of the writing day's rule and of `all_confirmed_days`, chosen because the rule decides
  XP the mint reads and its empty-set arm departs from the predecessor, so the parity golden cannot
  judge that arm.
- The parity golden alone, rejected because it closes every case but the empty-set arm, and that is
  the arm that would pay.

### Decision outcome

Chosen: one write per toggle, the chip that carries its day, the separate writing step, `all`
refused at start, no bonus and no streak over an empty set, the fold as the one awarder of habit
badges over the evaluated day's context, named reply methods with their census entries, a new
message per tap, the grown constants golden, and Lean beside the TLA+ model.

### Consequences

- Good, because a toggle, an entry, an undo and a recompute serialise on the write base, and each
  leaves the settled `write:` amounts equal to the rule over the log it committed.
- Good, because a chip tapped after the rollover changes nothing.
- Good, because no configuration pays the writing bonus, an ink badge or a reading badge for no
  course.
- Bad, because a habit badge is awarded only when the fold next runs, not at the moment of the write
  that earned it.
- Bad, because a tap leaves the old checklist in the chat and answers below it (#601).

### Confirmation

SPEC-078 section 14: A7 to A10d, A14b, A19, A19a, A19b, A22b, A23b and A32 to A37. The formal entry
`formal/tla/HabitXpFollowsItsLog/` gains `WritingXpFollowsItsConfirmations` and
`AChipTogglesOnlyItsOwnDay`, each with its witness; `formal/tla/AwardOnce/` covers the habit badges
step; `lean/HabitWriting` proves the writing day's rule and the all-confirmed days.

What would make this wrong: a writing path that cannot share the settle's write, such as an adapter
that writes `writing_log` outside `coordination::habits::writing`, or a second awarder of habit
badges outside the fold. The predecessor's `habits.py:writing_day_xp`, `writing_streak` and
`evaluate_habit_badges` at `27ee2bc`; #94.

### Correction (verify round 2), 2026-10-04
- Four of part 078a's private helpers, `settle_owner`, `level_before` and `celebrate` in `coordination::habits::minutes` and
  `settle_habit` in `coordination::recompute::habits`, widened to `pub(super)`, chosen because the writing toggle and the
  writing step then settle, measure and celebrate through the code part 078a proved, and no row's anchor or covered span moves.
- Copies of those helpers in the writing modules, rejected because two copies of one settle rule are free to drift apart.
