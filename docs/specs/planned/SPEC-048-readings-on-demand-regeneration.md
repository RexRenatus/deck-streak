# SPEC-048: the owner regenerates a listed topic with one tap, and one lock keeps each topic to a single writer

- **Wave:** W1. **Issue:** #34 (epic #2). **Context(s):** `deck-streak-readings` (the pick token, the topic lock, the regeneration's outcome rules); `deck-streak-coordination` (the use case, and the lock on the nightly path); `deck-streak-api` and `deck-streak-bot` (the two taps).
- **Decided by:** ADR-006 (owner-only requests and updates), ADR-010 (the API, the bot and each job
  run as separate units), ADR-019 (the owner's on-demand ruling: tap to pick), and ADR-048 (the lock's
  mechanism and the pick token).
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-048.md` (ADR-016).

## 1. The problem, measured

- **The predecessor never shipped it.** Its tap-to-pick command lived on an unmerged branch, and the
  seams it needed were disarmed, one of them raising "not implemented" (the second-brain inventory,
  private). Its design had the rules that matter: tap to pick and never a typed topic; one named lock
  so the nightly run and an on-demand request are never two writers of one note; the owner's read
  state and tick survive a regeneration.
- **Why the lock must cross processes.** The API, the bot and each scheduled job run as separate
  units (ADR-010), so an on-demand tap in the Mini App or the bot and the nightly run are different
  processes; a lock inside one process serialises nothing.
- **Prerequisites.** SPEC-046 (the generation and its gates), SPEC-042 (the body replacement and the
  archive), SPEC-047 (the read state that must survive), SPEC-025 and SPEC-026 (the API and the bot's
  callback handling). SPEC-051 and SPEC-052 draw the buttons that issue these taps.

## 2. Requirements

R1. A regeneration names a pick token, never a topic. A token is `<study day ordinal>-<first 12
    hexadecimal digits of the SHA-256 of the topic key>`, made only of `[0-9a-f-]`, and it is valid
    only for a topic listed on the current study day. A token from another study day, or one that
    matches no listed topic, is refused with `not_listed`. Text typed after a command names nothing
    and regenerates nothing.
R2. Each topic has one lock, taken by every writer of that topic's reading: the nightly generation
    and an on-demand regeneration. The lock is an exclusive advisory lock on the file
    `<lock directory>/readings-<topic hash>.lock`, where the lock directory is one directory shared
    by every unit that writes readings, given by configuration; it is taken with `try_lock`, and it is
    released when its holder closes the file or exits (ADR-048).
R3. An on-demand regeneration answers at once: `started` when it took the lock (the generation then
    runs off the request path), or `busy` when another writer holds it. It never waits on the request
    path and never starts a second writer.
R4. The nightly generation waits for a held topic lock at most one reading's wall clock (620
    seconds, SPEC-043), and if the lock is still held it leaves the topic to its holder, whose outcome
    becomes the topic's state.
R5. A regeneration runs SPEC-046's generation on the day set resolved at that moment, with the same
    gates, the one repair and no placeholder. It keeps the last-sync gate and ignores the pause: the
    owner's tap is always honoured.
R6. When the regenerated day set has the same digest, the reading keeps its id, read state, studied
    state and XP; its text becomes the new text with its version raised by one; and its vault note's
    body is replaced by SPEC-042's body replacement, which keeps both box lines byte for byte and
    refuses a note whose body the owner edited.
R7. When the day set changed, the new text is a new reading with its own id; the previous reading
    keeps its read state and XP, and its vault note is archived under its own day with its ticks
    intact (a taken archive name gets a numeric suffix, SPEC-042).
R8. A regeneration whose output fails its repair writes nothing: the standing reading, its text and
    its vault note are unchanged, and the attempt and its reason are recorded for the Mini App to show
    beside the reading that stands.
R9. `POST /api/readings/pick/{token}/regenerate` and the bot's callback `rg:<token>` run the same use
    case for the authenticated owner only; the API answers `202` with `started` or `409` with `busy`,
    and the bot answers the callback with the same word.
R10. No identifier this delivery declares in the readings context says `lane`, `preread` or
    `prestudy` (the lexicon's locks for `topic` and `reading`).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | from the Mini App, only a token of a topic listed today regenerates; a token of another day or of no listed topic is refused with `not_listed` | `only_a_listed_topic_regenerates_from_the_mini_app` |
| A2 | from the bot, only a callback token of a listed topic regenerates, and text typed after the command regenerates nothing | `only_a_listed_topic_regenerates_from_the_bot` |
| A3 | a regeneration while the nightly run holds the topic's lock answers `busy` and starts nothing | `a_regeneration_during_the_nightly_run_answers_busy` |
| A4 | many concurrent writers of one topic never run two generations at once (the fake runner records the peak per topic) | `two_writers_of_one_topic_never_run_at_once` |
| A5 | the read state, the studied state and the vault note's parsed `I read it` tick survive a regeneration of the same day set | `the_read_state_and_the_vault_tick_survive_a_regeneration` |
| A6 | a lock held by a process that exits is free for the next writer | `a_crashed_holder_never_leaves_the_lock_held` |
| A7 | the owner's tap regenerates while the daily readings are paused | `the_owners_tap_regenerates_even_while_paused` |
| A8 | a regeneration that fails its repair leaves the standing reading and its vault bytes unchanged | `a_failed_regeneration_leaves_the_standing_reading` |
| A9 | a changed day set gives a new reading and archives the previous note with its ticks | `a_changed_day_set_archives_the_old_note_with_its_ticks` |
| A10 | the nightly run waits for a held lock at most 620 seconds and then leaves the topic to its holder (paused test time) | `the_nightly_run_leaves_a_held_topic_to_its_holder` |

```acceptance
A1: cargo test -p deck-streak-api --test readings_regenerate -- --exact only_a_listed_topic_regenerates_from_the_mini_app
A2: cargo test -p deck-streak-bot --test regenerate_callback -- --exact only_a_listed_topic_regenerates_from_the_bot
A3: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact a_regeneration_during_the_nightly_run_answers_busy
A4: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact two_writers_of_one_topic_never_run_at_once
A5: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact the_read_state_and_the_vault_tick_survive_a_regeneration
A6: cargo test -p deck-streak-readings --test lock -- --exact a_crashed_holder_never_leaves_the_lock_held
A7: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact the_owners_tap_regenerates_even_while_paused
A8: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact a_failed_regeneration_leaves_the_standing_reading
A9: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact a_changed_day_set_archives_the_old_note_with_its_ticks
A10: cargo test -p deck-streak-coordination --test readings_regenerate -- --exact the_nightly_run_leaves_a_held_topic_to_its_holder
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/readings/src/lock.rs` | `deck-streak-readings` | added: the topic lock |
| `crates/readings/src/pick.rs` | `deck-streak-readings` | added: the pick token |
| `crates/readings/src/regenerate.rs` | `deck-streak-readings` | added: same identity or new reading |
| `crates/readings/src/lib.rs` | `deck-streak-readings` | changed |
| `crates/readings/tests/lock.rs` | `deck-streak-readings` | added |
| `crates/coordination/src/readings/regenerate.rs` | `deck-streak-coordination` | added |
| `crates/coordination/src/readings/generate.rs` | `deck-streak-coordination` | changed: the nightly path takes the topic lock |
| `crates/coordination/tests/readings_regenerate.rs` | `deck-streak-coordination` | added |
| `crates/api/src/readings_routes.rs` | `deck-streak-api` | changed: the regenerate route |
| `crates/api/tests/readings_regenerate.rs` | `deck-streak-api` | added |
| `crates/bot/src/commands/regenerate.rs` | `deck-streak-bot` | added: the `rg:` callback's handler |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: registers the `rg:` callback beside the delete command's confirming callback |
| `crates/bot/tests/regenerate_callback.rs` | `deck-streak-bot` | added |
| `docs/specs/SPEC-048-readings-on-demand-regeneration.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-048-the-topic-lock-is-a-file-lock.md` | docs | added |
| `docs/red-first/SPEC-048.md` | docs | added |

## 5. What this does NOT do

- It draws no button: the Mini App's Regenerate and the bot's `/prestudy` listing issue the tokens
  (#37, #38).
- It regenerates no past day's reading; history is read-only (#37).
- It schedules no nightly run (#39).
- It pages nobody when a regeneration fails; the reason is shown, and the health check judges the
  day (#36).

## 6. Risks

- **Two units lock two different files** because each has its own private state directory. Detected
  at start: every unit that writes readings checks that the configured lock directory exists and is
  writable, and refuses otherwise; the deploy templates give the units one shared directory
  (SPEC-053, #42).
- **A hung holder keeps a topic busy.** Bounded by the run's wall clock and the unit's runtime
  bound; meanwhile the owner sees `busy`, never a second writer.
- **A lock file on a network file system** would not lock. The state directory is local disk on the
  host (ADR-010).
- **A stale listing's token** names yesterday. It carries the study day, so it is refused (A1, A2).
