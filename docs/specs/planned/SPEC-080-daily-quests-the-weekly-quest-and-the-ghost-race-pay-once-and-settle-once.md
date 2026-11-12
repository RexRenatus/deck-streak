# SPEC-080: the daily quests, the weekly quest and the ghost race pay once and settle once

- **Wave:** W3. **Issue:** #100, #101, #124 (epic #4). **Context(s):** `deck-streak-quests` (the daily
  arc: minting, the sealed quest, the challenge offers and pick, progress, completion, crown days,
  the feasibility and evidence gates, demotion; the weekly quest; the ghost and its race; their
  tables); `deck-streak-kernel` (the deterministic basis-point draw, ADR-080);
  `deck-streak-coordination` (the recompute's quest and race steps, the payouts through the grant
  port, economy's credit, SPEC-081's chest port and the streaks' freeze port, every message through
  the router); `deck-streak-api` and `deck-streak-bot` (the board, the pick, the race); the Mini App
  (the quest board).
- **Decided by:** ADR-012 (the parity oracle), ADR-041 (the one router), ADR-071 (each study day is
  settled once, in order), and ADR-080 (the reintroduction draw is BLAKE2b in the shared kernel).
- **Prerequisites:** SPEC-071 (the fold; each day's rollup, snapshot and reviews), SPEC-072 (the
  grant port and the day base), SPEC-076 (the streaks' freeze port and the governor's lapse),
  SPEC-081 (the challenge and weekly chests, the Perfect Week settlement), SPEC-082 (economy's
  credit port), SPEC-041 (the router), SPEC-026 (the bot); SPEC-079 and SPEC-083 when they land
  (the focus minutes and the declared skip days, which the recompute passes as none until then, as
  SPEC-049's lapse slice takes them). **Mutation band:** `S08000-S08099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-080.md` (ADR-016).

## 1. The problem, measured

- **Nothing of the arc exists.** `crates/quests/src/` holds only `lib.rs` at `dev` c3d769b;
  SPEC-081 lands first and adds the chest modules this SPEC calls. The kernel has no deterministic
  draw.
- **What is ported.** The predecessor's daily arc (`gamification/quests.py:q2_for`, `q3_offers`,
  `q3_pick`, `progress_for`, `is_done`, `clear_due_is_feasible`, `is_retired`;
  `pipeline_layers/loot.py:LootLayer._update_quests`, `_q2_evidence_suppressed`,
  `_q3_demoted_keys`, `_has_prior_active_day`, `_apply_q3_pick`, `_pay_quest`, `quest_board`;
  `database.py:GamifyStore.median_daily_reviews`, `quest_reintro_draw_bp`,
  `quest_should_reintroduce`, `nudge_draw_bp`), its weekly quest (`gamification/quests.py:weekly_for`,
  `weekly_progress`; `pipeline_layers/loot.py:LootLayer._update_weekly_quest`,
  `_weekly_evidence_suppressed`) and its ghost race (`gamification/ghost.py:build`, `ghost_at`,
  `photo_finish_ok`; `pipeline_layers/ghost_race.py:GhostRaceLayer._update_ghost_race`,
  `_settle_due_races`, `_recompute_win_streak`, `_mint_ghost`, `_announce_race_result`,
  `race_board`), at `27ee2bc`.
- **Rules the issues paraphrase, settled against the code.**
  - The sealed quest reveals at the first recompute its day has any review: the predecessor's
    guard `reviews >= SEAL_REVEAL_REVIEWS or reviews > 0` reduces to one review.
  - `langs3` counts the day's decks studied from the rollup (`quests.py:progress_for` reads
    `decks_studied`), and `mature20` its mature answers.
  - #100's reintroduction chance of 5 percent is 500 basis points of a BLAKE2b draw
    (`database.py:QUEST_REINTRO_EPSILON_BP`), and the draw's input carries the study day as an ISO
    date: `quest_should_reintroduce(seed, key, today.isoformat())`, and the week's first day for a
    weekly key.
  - The ghost's cap is 1.05 times the upper middle of the last four active weeks' sorted totals
    (`ghost.py:build` reads index `len // 2`), not a mean; ties between best weeks go to the first
    in the order the days are given, which the adapter gives ascending.
  - The weekly chest does not move the pity counters, while the challenge chest does; and a weekly
    freeze refused by a cap is dropped, not turned into a token as an Epic's is.
  - `ghost.py:photo_finish_ok` has no caller in the predecessor: its race announces a loss without
    ever naming a photo finish. Issue #124 asks for the gate, so this delivery wires it into the
    loss line; it is the one behaviour here the predecessor never showed.
- **Traps.**
  - Both rotations use Python's proleptic ordinal (`date.toordinal()`), which is the epoch day plus
    719163. Modulo 3 the phase is the same; modulo 4 (the challenge offers) and the weekly
    `ordinal // 7` it is not, so a port that rotates by the epoch day offers the wrong challenges on
    three days in four.
  - The draw's digest length is BLAKE2b's own parameter (Python's `digest_size=8`), not the first 8
    bytes of a longer digest, and the ISO date in its input must be the kernel's rendering of the
    study day, zero-padded (SPEC-020 R5).
  - Under one scheduled sync a study day, just after the rollover (ADR-037), a day's reviews reach
    DeckStreak at the next day's settle unless the owner triggers a sync, so a quest completed in the
    afternoon is paid and celebrated the next morning (ADR-071). The noon auto-pick reads the
    recompute's own clock in the predecessor; at a closed day's settle its noon has passed.
- **What the parity oracle proves.** The draw and the reintroduction; both rotations; progress,
  completion, the pick, feasibility, the retirement rule and the median; the three evidence gates;
  the weekly quest's settlement; the ghost, its value at a weekday, the photo-finish gate, the
  settlement sweep and the announcement; and the constants.
- **Prerequisites.** SPEC-071, SPEC-072, SPEC-076, SPEC-081, SPEC-082, SPEC-041 and SPEC-026 before
  it; SPEC-079 and SPEC-083 whenever they land.

## 2. Requirements

The deterministic draw (ADR-080)

R1. The kernel's `draw_bp(seed, kind, reference)` returns a number from 0 to 9999: BLAKE2b with an
    8-byte digest over the UTF-8 bytes of `seed|kind|reference`, read as a big-endian unsigned
    integer, modulo 10000. It equals `goldens/draw_bp.json` (`database.py:nudge_draw_bp`), whose
    references carry no date.
R2. A suppressed quest key is reintroduced for a reference when
    `draw_bp(seed, "quest_reintro", "<key>:<ISO date>") < 500`, with the seed `quest-reintro-v1`
    (`goldens/quests.constants.json`: `database.QUEST_REINTRO_SEED_DEFAULT`,
    `database.QUEST_REINTRO_EPSILON_BP`), where the ISO date is the study day's for a daily key and
    the week's first study day's for a weekly key. It equals `goldens/quest_reintro_draw_bp.json` and
    `goldens/quest_should_reintroduce.json`.

The daily arc (#100)

R3. The quest step registers in phase 4 of SPEC-071's fold (ADR-071). It mints a study day's quests
    at the first recompute that reads that day while it is current, one row per slot: the first
    quest `reviews15` with the target 15 (`quests.Q1_KEY`, `quests.Q1_TARGET`), a sealed second quest
    from `q2_for(day, suppressed)`, and the challenge offers from
    `q3_offers(day, yesterday's reviews, drop_clear_due, demoted)`. It never mints quests, and so no
    crown day, for a closed study day that no recompute read while it was current, nor for the past
    days the first recompute backfills.
R4. The rotations use the proleptic ordinal of the study day, its epoch day plus 719163. The second
    quest and the offers equal `goldens/q2_for.json` and `goldens/q3_offers.json`.
R5. The second quest's evidence gate suppresses a key with no evidence over the 30 days before the
    study day (`database.QUEST_GATE_WINDOW_DAYS`): `focus25` when it has no completion and no focus
    entry in the window; `langs3` and `mature20` when they have no completion and no rollup of a day
    with an answered review (`answered > 0`) shows their decks studied or mature answers above 0, a
    NULL value being unknown and never 0; each suppressed key is reintroduced when R2's draw passes;
    and the pool keeps at least 1 selectable key (`quests.QUEST_MIN_POOL`) by yielding back the keys
    least recently native to the rotation. It equals `goldens/q2_evidence_suppressed.json`, the
    board's reduced-pool flag included. While no focus source is wired (before SPEC-079), the step
    passes `focus25` as suppressed without a draw, because no focus timer exists yet to satisfy it.
R6. `clear_due` is offered only when the day's backlog is at most 3.0
    (`quests.CLEAR_DUE_BACKLOG_MULTIPLE`) times the median of the daily reviews in the rollups of the
    30 days before it (`database.QUEST_GATE_WINDOW_DAYS`); a window with no rollup gives a median of
    0.0. It equals `goldens/clear_due_is_feasible.json` and `goldens/median_daily_reviews.json`.
R7. A challenge key is demoted when its completion rate over the window's active days is at most
    0.0 (`constants.QUEST_RETIREMENT_RATE`) with at least 5 offers (`constants.MIN_OFFERS_FOR_RATE`);
    a demoted key is held back, never deleted, and returned while fewer than two offers remain. It
    equals `goldens/is_retired.json` and `goldens/q3_demoted_keys.json`, the board's evidence per
    demoted key included.
R8. The owner picks one offer, once per study day, from the bot or the Mini App. An unpicked day is
    picked by `q3_pick(offers, comeback)` once the day's noon has passed (`quests.Q3_AUTOPICK_HOUR`,
    12 local): at a recompute of the current study day, when the recompute's local hour is at least
    12; at a closed day's settle, always. It is a comeback day when yesterday had no review and an
    active day lies in the window before it. The pick equals `goldens/q3_pick.json`.
R9. The sealed quest is revealed at the first recompute at which its study day has a review
    (`quests.SEAL_REVEAL_REVIEWS` with the predecessor's `reviews > 0` guard). At a recompute of the
    current study day the reveal is announced through the router as the celebration event `quest`
    (dedupe key `seal:<epoch day>`); at a closed day's settle the seal lifts without an
    announcement, because the moment it announced has passed and the completion names the quest. A
    sealed quest shows no progress.
R10. Progress and completion are `progress_for(key, target, stats)` and `is_done(key, target, stats)`
    over the day's stats: its reviews, decks studied, focus minutes (none until SPEC-079), mature
    answers, true retention and answered reviews from its rollup, its due and backlog counts from
    its snapshot (the end-of-day snapshot at a closed day's settle, ADR-071), and yesterday's
    reviews. They equal `goldens/quest_progress.json` and `goldens/quest_is_done.json`.
R11. Each completion is claimed once by its slot's reward flag, in the same write as the claim:
    the first and second quests grant 20 and 40 XP (`quests.QUEST_XP`) through the grant port as
    `quest:q1` and `quest:q2` (scope `per-day`, track `language`) on the quest's study day, which
    the day's base counts (SPEC-072's `goldens/day_base_xp.json`); every completion credits 10 coins
    (`constants.COIN_QUEST_COMPLETE`) through economy's credit port with the source `quest` and the
    reference `<epoch day>:<slot>`; the challenge quest grants SPEC-081's challenge chest. The first
    and second quests' completions are the celebration event `quest` (dedupe key
    `quest_done:<epoch day>:<slot>`), and the challenge quest's is the event `quest` carrying its
    chest's Open button.
R12. A study day whose three slots are complete is one crown day, recorded once, and the celebration
    event `quest_all` (dedupe key `crown:<epoch day>`).
R13. A declared skip day voids its quests: no mint, no evaluation, no payout and no crown. Undoing
    the skip restores their evaluation at the next recompute.

The weekly quest (#101)

R14. A week runs Monday to Sunday by study day (`habits.py:week_bounds`). Its quest is
    `weekly_for(week start, suppressed)`: `crowns5`, `reviews600` or `focus200`, rotated by the
    week start's proleptic ordinal divided by 7, modulo 3. `crowns5` is suppressed when the gate's
    30 days (`database.QUEST_GATE_WINDOW_DAYS`) hold no crown day and no completion, and `focus200`
    when they hold no focus entry and no completion, unless R2's draw reintroduces it for the week;
    `reviews600` is never suppressed.
    They equal `goldens/weekly_for.json` and `goldens/weekly_evidence_suppressed.json`; while no focus
    source is wired, `focus200` is passed as suppressed without a draw, as R5's `focus25` is.
R15. The weekly progress is the week's crown days, its reviews from the rollups, or its focus minutes
    from blocks not abandoned (none until SPEC-079), per `weekly_progress`
    (`goldens/weekly_progress.json`). A complete week is claimed once: SPEC-081's weekly chest, and a
    freeze through the streaks' `grant_freeze` with the reason `weekly_quest` only while this calendar
    month's dropped freezes are fewer than 1 and fewer than 3 are held, with no token in its place.
    The settlement equals `goldens/weekly_quest_settled.json`. The completion is the celebration
    event `quest` (dedupe key `weekly_done:<epoch day of the week's start>`) carrying the chest's
    Open button.
R16. At each Monday's recompute or settle, the step settles last week's Perfect Week through
    SPEC-081 with last week's crown days and declared skip days.

The ghost race (#124)

R17. The race step registers in phase 4 of the fold and runs once per recompute, for the current
    study day. It counts each study day's distinct cards among its study reviews, from the reviews
    the recompute reads.
R18. The current week's ghost is minted once by `build(day cards, week start)`: the best week by
    total within the 26 weeks before it (`ghost.LOOKBACK_WEEKS`), needing at least 4 active weeks
    (`ghost.MIN_HISTORY_WEEKS`), capped at the integer part of 1.05 (`ghost.GHOST_CAP_FRAC`) times
    the upper middle of the last four active weeks' sorted totals, with its daily cumulative series
    scaled down when capped. It equals `goldens/ghost_build.json`, and the ghost's value at a
    weekday equals `goldens/ghost_at.json`.
R19. Each recompute sweeps the 4 weeks before the current one: a week with a ghost is recorded once
    (your total, the ghost's, and won when yours is greater) unless it is a rest week, one with 2 or
    more declared skip days; nothing is swept while the reviews hold no study day; the win streak is
    the run of wins from the newest result. It equals `goldens/race_settled.json`.
R20. Outside the governor's lapse (SPEC-076), last week's result is announced once: a win is the
    celebration event `ghost_win` (dedupe key `ghost:<epoch day of that week's start>`), or `record`
    when the win streak exceeds 3 (`ghost.WIN_STREAK_DECAY`); a loss is announced only when the owner
    studied that week, as the event `quest` (dedupe key `ghost_loss:<epoch day of that week's
    start>`), and names a photo finish only when `photo_finish_ok(yours, the ghost's)`: a gap of at
    most 5 cards or 5 percent (`ghost.PHOTO_FINISH_CARDS`, `ghost.PHOTO_FINISH_FRAC`). The choice of
    event equals `goldens/race_announced.json` and the gate `goldens/photo_finish_ok.json`. A week in
    a lapse still settles, and announces nothing.
R21. Outside a lapse and outside a rest week, at a recompute of the current study day, a change of
    the week's leader (you, when your cumulative distinct cards are at least the ghost's at today's
    weekday) is the event `quest` (dedupe key `ghost_lead:<epoch day of the week's start>:<epoch day>`),
    at most once a study day, as the predecessor's lead line.

Surfaces, tables and names

R22. The API serves the board (the day's quests with progress and what is left, the offers and the
    pick, the weekly quest, the week's crown days, the reduced-pool flag, the demoted challenge keys
    with their active-day evidence), takes the owner's pick, and serves the race (the ghost's daily
    series, capped or not, and its value today; your daily series and its value today; the recent
    results): GET /api/quests, POST /api/quests/q3 and GET /api/race, each for the owner's session
    only.
R23. The bot keeps the predecessor's `/quests` (the board, with a pick button per offer) and `/race`
    (the race as text with an Open in app button; the chart is drawn in the Mini App).
R24. The Mini App's `/quests` shows the board, the challenge choice card, the weekly card and the race
    card's numbers; SPEC-085's chart component draws the race when it lands. The route joins `ROUTES`.
R25. The quests context gains seven tables, each `STRICT` with `created_at`, created by
    `migrations/008001_quests_daily_weekly_and_race.sql`: `quests` (keyed by study day and slot),
    `quest_offers`, `weekly_quests`, `crown_days`, `ghosts` (with the week's own daily series and
    its leader), `race_results` and `quest_state` (one row: the reintroduction seed and the win
    streak). Each is registered in the context map, declared in `privacy.json` and listed in the
    quests data-rights port, exported and erased; `quest_state` is reset in place (the seed to its
    default, the win streak to 0).
R26. `blake2` is admitted by ADR-080 in `[workspace.dependencies]` and named by the kernel's manifest
    alone; `draw_bp` lives in `crates/kernel/src/draw.rs`, and the quests context calls it.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the kernel's basis-point draw equals the golden | `the_basis_point_draw_matches_the_predecessors_golden` |
| A2 | the reintroduction draw and verdict equal the goldens, the study day written as its ISO date | `the_reintroduction_draw_matches_the_predecessors_goldens` |
| A3 | the second quest and the challenge offers rotate by the proleptic ordinal, equal to the goldens | `the_daily_rotations_match_the_predecessors_goldens` |
| A4 | the weekly quest rotates by the week start's ordinal, equal to the golden | `the_weekly_rotation_matches_the_predecessors_golden` |
| A5 | progress, completion, the pick, feasibility and the retirement rule equal the goldens | `quest_progress_and_completion_match_the_predecessors_goldens` |
| A6 | the second quest's evidence gate equals the golden, the pool floor and the reduced flag included | `the_sealed_quest_gate_matches_the_predecessors_golden` |
| A7 | the challenge gates equal the goldens: the median, feasibility and demotion with its evidence | `the_challenge_offer_gates_match_the_predecessors_goldens` |
| A8 | the weekly gate equals the golden, and `reviews600` is never suppressed | `the_weekly_gate_matches_the_predecessors_golden` |
| A9 | the weekly quest settles as the golden: progress, one claim, the Epic chest, the capped freeze | `the_weekly_quest_settles_as_the_predecessors_golden` |
| A10 | the quest constants equal the golden and `economy.json`'s quest XP and quest coins | `the_quest_constants_equal_the_golden_and_economy_json` |
| A11 | a study day's quests are minted once while it is current, and never for a closed day no recompute read nor for a past day the first recompute backfills | `quests_are_minted_once_while_their_day_is_current` |
| A12 | a closed day's settle auto-picks its challenge, lifts the seal silently and pays each completion once | `a_closed_day_settles_its_quests_once` |
| A13 | a completion grants its XP and coins once and a crown day is recorded and celebrated once | `a_completion_pays_once_and_a_crown_day_once` |
| A14 | a declared skip day voids its quests, and undoing it restores them | `a_skip_day_voids_its_quests` |
| A15 | the ghost and its value at a weekday equal the goldens, cold start, cap and ties included | `the_ghost_matches_the_predecessors_golden` |
| A16 | the photo-finish gate equals the golden | `the_photo_finish_gate_matches_the_predecessors_golden` |
| A17 | the settlement sweep and the announcement equal the goldens, rest weeks and the quieter win included | `race_settlement_and_announcement_match_the_predecessors_goldens` |
| A18 | a week in a lapse settles and announces nothing | `a_lapse_week_settles_silently` |
| A19 | a lead change is raised at most once a study day | `a_lead_change_is_raised_once_a_study_day` |
| A20 | the quest and race routes answer the owner's session only, and a second pick is refused | `the_quest_routes_answer_only_the_owner` |
| A21 | `/quests` and `/race` answer the owner with the board and the race | `quests_and_race_answer_the_owner` |
| A22 | the board shows a sealed quest without its progress | `shows a sealed quest without its progress` |
| A23 | the seven tables are exported and erased, `quest_state` reset in place | `the_quest_tables_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-kernel --test quests_draw -- --exact the_basis_point_draw_matches_the_predecessors_golden
A2: cargo test -p deck-streak-quests --test quests_gates -- --exact the_reintroduction_draw_matches_the_predecessors_goldens
A3: cargo test -p deck-streak-quests --test quests_rotation -- --exact the_daily_rotations_match_the_predecessors_goldens
A4: cargo test -p deck-streak-quests --test quests_rotation -- --exact the_weekly_rotation_matches_the_predecessors_golden
A5: cargo test -p deck-streak-quests --test quests_progress -- --exact quest_progress_and_completion_match_the_predecessors_goldens
A6: cargo test -p deck-streak-quests --test quests_gates -- --exact the_sealed_quest_gate_matches_the_predecessors_golden
A7: cargo test -p deck-streak-quests --test quests_gates -- --exact the_challenge_offer_gates_match_the_predecessors_goldens
A8: cargo test -p deck-streak-quests --test quests_gates -- --exact the_weekly_gate_matches_the_predecessors_golden
A9: cargo test -p deck-streak-quests --test quests_weekly -- --exact the_weekly_quest_settles_as_the_predecessors_golden
A10: cargo test -p deck-streak-quests --test quests_progress -- --exact the_quest_constants_equal_the_golden_and_economy_json
A11: cargo test -p deck-streak-coordination --test quests_steps -- --exact quests_are_minted_once_while_their_day_is_current
A12: cargo test -p deck-streak-coordination --test quests_steps -- --exact a_closed_day_settles_its_quests_once
A13: cargo test -p deck-streak-coordination --test quests_steps -- --exact a_completion_pays_once_and_a_crown_day_once
A14: cargo test -p deck-streak-coordination --test quests_steps -- --exact a_skip_day_voids_its_quests
A15: cargo test -p deck-streak-quests --test race_ghost -- --exact the_ghost_matches_the_predecessors_golden
A16: cargo test -p deck-streak-quests --test race_ghost -- --exact the_photo_finish_gate_matches_the_predecessors_golden
A17: cargo test -p deck-streak-quests --test race_settle -- --exact race_settlement_and_announcement_match_the_predecessors_goldens
A18: cargo test -p deck-streak-coordination --test race_steps -- --exact a_lapse_week_settles_silently
A19: cargo test -p deck-streak-coordination --test race_steps -- --exact a_lead_change_is_raised_once_a_study_day
A20: cargo test -p deck-streak-api --test quests_routes -- --exact the_quest_routes_answer_only_the_owner
A21: cargo test -p deck-streak-bot --test quests_commands -- --exact quests_and_race_answer_the_owner
A22: pnpm exec vitest run web/app/src/lib/quests/board.test.ts -t "shows a sealed quest without its progress"
A23: cargo test -p deck-streak-quests --test quests_rights -- --exact the_quest_tables_are_exported_and_erased
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. No pack's state changes for this delivery: every pack
named here stays enforced, with no row deferred.

| id | criterion | decided by |
|---|---|---|
| B1 | over `notifications-policy.json` and every delivery call under `crates/`, the quest, crown and race messages reach the owner only through the one router | the notifications-policy pack |
| B2 | over `economy.json`, the quest XP and the quest coins stay the predecessor's values, and no quest reward is sold | the game-economy pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `crates/quests/src/data_rights.rs`, each of the seven new tables has a category with its purpose, basis, retention, export and erase | the privacy-gdpr pack |
| B4 | over `web/app/src/routes/quests/` in both colour schemes, the board meets WCAG 2.2 AA | the accessibility pack |
| B5 | over the quest and race strings in `web/app/messages/*.json` and the message texts in `crates/coordination/src/quests/`, no copy fabricates a near miss or a false urgency, and a loss is stated as a neutral fact | the ux-laws pack |
| B6 | over `Cargo.toml` and every crate manifest under `crates/`, the crate graph still equals the context map, the kernel gaining no internal dependency | the ddd probe |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/src/draw.rs` | `deck-streak-kernel` | added: the basis-point draw (ADR-080) |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the draw module |
| `crates/kernel/Cargo.toml` | `deck-streak-kernel` | changed: `blake2` (ADR-080) |
| `crates/kernel/tests/quests_draw.rs` | `deck-streak-kernel` | added |
| `crates/quests/src/arc.rs` | `deck-streak-quests` | added: the pools, the rotations, progress and completion, the pick |
| `crates/quests/src/gates.rs` | `deck-streak-quests` | added: feasibility, the evidence gates, the reintroduction, demotion |
| `crates/quests/src/weekly.rs` | `deck-streak-quests` | added: the weekly quest |
| `crates/quests/src/race.rs` | `deck-streak-quests` | added: the ghost, the settlement, the win streak, the announcement's choice |
| `crates/quests/src/quest_store.rs` | `deck-streak-quests` | added: the repository over the seven tables |
| `crates/quests/src/data_rights.rs` | `deck-streak-quests` | changed: the seven tables join the port |
| `crates/quests/src/lib.rs` | `deck-streak-quests` | changed: the modules above |
| `crates/quests/tests/quests_rotation.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/quests_progress.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/quests_gates.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/quests_weekly.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/quests_rights.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/race_ghost.rs` | `deck-streak-quests` | added |
| `crates/quests/tests/race_settle.rs` | `deck-streak-quests` | added |
| `migrations/008001_quests_daily_weekly_and_race.sql` | `deck-streak-quests` | added: the seven tables, `STRICT`, with `created_at` (SPEC-020 R15, R18) |
| `crates/coordination/src/quests/mod.rs` | `deck-streak-coordination` | added: the pick, the board and race views |
| `crates/coordination/src/quests/messages.rs` | `deck-streak-coordination` | added: the quest, crown, weekly and race occasions and their texts |
| `crates/coordination/src/recompute/quests.rs` | `deck-streak-coordination` | added: the quest step and its Monday settlement, in phase 4 of SPEC-071's fold |
| `crates/coordination/src/recompute/race.rs` | `deck-streak-coordination` | added: the race step for the current study day, in phase 4 of the fold |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the two steps registered in phase 4 |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the quests module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the quests port lists its new tables |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for each of the seven tables |
| `crates/coordination/tests/quests_steps.rs` | `deck-streak-coordination` | added |
| `crates/coordination/tests/race_steps.rs` | `deck-streak-coordination` | added |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the quest step joined to the grant, credit, chest and freeze ports |
| `crates/api/src/quests_routes.rs` | `deck-streak-api` | added: the board, the pick and the race |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the quest routes mounted behind the owner's session |
| `crates/api/tests/quests_routes.rs` | `deck-streak-api` | added |
| `crates/bot/src/quests_commands.rs` | `deck-streak-bot` | added: the quests and race commands and the pick callback |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the two commands in the table and the owner's menu, the pick's callback prefix |
| `crates/bot/tests/quests_commands.rs` | `deck-streak-bot` | added |
| `web/app/src/routes/quests/+page.svelte` | miniapp | added: the board |
| `web/app/src/lib/quests/api.ts` | miniapp | added: the quest routes' client |
| `web/app/src/lib/quests/QuestBoard.svelte` | miniapp | added: the quests, the challenge choice card, the weekly card |
| `web/app/src/lib/quests/RaceCard.svelte` | miniapp | added: the race's numbers, and the chart once SPEC-085 lands |
| `web/app/src/lib/quests/board.test.ts` | miniapp | added |
| `web/app/src/lib/routes.ts` | miniapp | changed: the quests route joins `ROUTES` |
| `web/app/messages/*.json` | miniapp | changed: the quest, crown and race strings, in each locale's catalog |
| `Cargo.toml` | workspace | changed: `[workspace.dependencies]` gains `blake2` (ADR-080) |
| `tools/parity-oracle/registry/spec_080.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/draw_bp.json` | repo | added: the golden of `database.py:nudge_draw_bp` (§7) |
| `tools/parity-oracle/goldens/quest_reintro_draw_bp.json` | repo | added: the golden of `database.py:quest_reintro_draw_bp` (§7) |
| `tools/parity-oracle/goldens/quest_should_reintroduce.json` | repo | added: the golden of `database.py:quest_should_reintroduce` (§7) |
| `tools/parity-oracle/goldens/q2_for.json` | repo | added: the golden of `gamification/quests.py:q2_for` (§7) |
| `tools/parity-oracle/goldens/q3_offers.json` | repo | added: the golden of `gamification/quests.py:q3_offers` (§7) |
| `tools/parity-oracle/goldens/q3_pick.json` | repo | added: the golden of `gamification/quests.py:q3_pick` (§7) |
| `tools/parity-oracle/goldens/quest_progress.json` | repo | added: the golden of `gamification/quests.py:progress_for` (§7) |
| `tools/parity-oracle/goldens/quest_is_done.json` | repo | added: the golden of `gamification/quests.py:is_done` (§7) |
| `tools/parity-oracle/goldens/clear_due_is_feasible.json` | repo | added: the golden of `gamification/quests.py:clear_due_is_feasible` (§7) |
| `tools/parity-oracle/goldens/is_retired.json` | repo | added: the golden of `gamification/quests.py:is_retired` (§7) |
| `tools/parity-oracle/goldens/median_daily_reviews.json` | repo | added: the golden of `database.py:GamifyStore.median_daily_reviews` (§7) |
| `tools/parity-oracle/goldens/q2_evidence_suppressed.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._q2_evidence_suppressed` (§7) |
| `tools/parity-oracle/goldens/q3_demoted_keys.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._q3_demoted_keys` (§7) |
| `tools/parity-oracle/goldens/weekly_for.json` | repo | added: the golden of `gamification/quests.py:weekly_for` (§7) |
| `tools/parity-oracle/goldens/weekly_progress.json` | repo | added: the golden of `gamification/quests.py:weekly_progress` (§7) |
| `tools/parity-oracle/goldens/weekly_evidence_suppressed.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._weekly_evidence_suppressed` (§7) |
| `tools/parity-oracle/goldens/weekly_quest_settled.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer._update_weekly_quest` (§7) |
| `tools/parity-oracle/goldens/ghost_build.json` | repo | added: the golden of `gamification/ghost.py:build` (§7) |
| `tools/parity-oracle/goldens/ghost_at.json` | repo | added: the golden of `gamification/ghost.py:ghost_at` (§7) |
| `tools/parity-oracle/goldens/photo_finish_ok.json` | repo | added: the golden of `gamification/ghost.py:photo_finish_ok` (§7) |
| `tools/parity-oracle/goldens/race_settled.json` | repo | added: the golden of `pipeline_layers/ghost_race.py:GhostRaceLayer._settle_due_races` (§7) |
| `tools/parity-oracle/goldens/race_announced.json` | repo | added: the golden of `pipeline_layers/ghost_race.py:GhostRaceLayer._announce_race_result` (§7) |
| `tools/parity-oracle/goldens/quests.constants.json` | repo | added: the constants golden (§7) |
| `tools/parity-oracle/goldens/ghost.constants.json` | repo | added: the constants golden (§7) |
| `scripts/mutation-rows.d/S08000-S08099.json` | repo | added: the hand-proved rows (§9) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the seven tables; the kernel's line names the basis-point draw |
| `privacy.json` | repo | changed: the quests, weekly quest and race categories |
| `PRIVACY.md` | repo | changed: one line per new category |
| `docs/schematics/quests-and-chests-lifecycle.md` | docs | added by the W3 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/specs/SPEC-080-daily-quests-the-weekly-quest-and-the-ghost-race-pay-once-and-settle-once.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-080-the-quest-reintroduction-draw-is-blake2b-in-the-shared-kernel.md` | docs | changed: accepted |
| `docs/red-first/SPEC-080.md` | docs | added |
| `Cargo.lock` | workspace | changed |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no morning brief and no challenge offer message in one; the Mini App's board and
  `/quests` show the offers until the brief exists (#122).
- It changes no widget's quest lines or weekly footer (#121).
- It builds the smoke bomb's rule, inventory and copy nowhere here: SPEC-081 owns them, and this
  delivery only calls their settlement on Mondays (#104).
- It draws no race chart; SPEC-085's component draws it from this delivery's series (#152).
- It offers no quest through the agent's tools (#157).
- It raises no quest or race fanfare beyond the router's celebrations (#128).
- It builds no nudge holdout, which will reuse the kernel's draw (#132).
- It builds no screen that changes the reintroduction seed (#57).
- It imports none of the predecessor's quests, offers, crown days, ghosts or race results (#61).

## 6. Risks

- **A port rotates by the epoch day.** Detected by `goldens/q3_offers.json` and
  `goldens/weekly_for.json`, whose cases cover every residue of the ordinal (A3, A4), and by the
  hand-proved row `S08007-ORDINAL-OFFSET`.
- **The draw's input or digest differs.** An ISO date without zero padding, a separator other than
  `|`, or the first 8 bytes of a 64-byte digest each change every draw; detected by
  `goldens/draw_bp.json` and `goldens/quest_reintro_draw_bp.json` (A1, A2).
- **A quest completed in the afternoon is paid the next morning.** Under one daily sync the day's
  reviews reach DeckStreak at its settle unless the owner triggers a sync (ADR-037, ADR-071); the
  board shows when the day was last read, and the owner's `/sync` brings it forward.
- **A day whose scheduled sync failed has no quests.** Minting happens only while a day is current
  (R3), as the predecessor mints only at a sync; the failed sync is recorded and paged (SPEC-027),
  and the next day mints normally.
- **The photo-finish line is new behaviour.** The predecessor never named a photo finish; #124 asks
  for it, and the line states only the two totals and the gate's verdict. Reviewed against the
  ux-laws pack (B5).
- **A focus quest is offered before focus exists.** Prevented by R5 and R14 while no focus source is
  wired; once SPEC-079 lands, the predecessor's gate applies.
- **A double payout on a re-run.** Each completion is claimed by its slot's flag in the same write as
  its payout (R11), and each grant has its own once key; detected by A12 and A13.

## 7. Parity goldens

Each adapter builds what JSON cannot carry and calls the predecessor; none computes a rule
(ADR-029). A stub store is a stand-in holding only the case's rows, as `registry/spec_023.py`'s
`with_a_stub_store` is. Every day is an epoch day number; where the predecessor reads a date
string, the adapter writes the case's epoch day as its ISO date before the call, and no golden holds
a date (SPEC-029 R3).

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `draw_bp` | `database.py:nudge_draw_bp` | function | none; seeds, kinds and references carry no date |
| `quest_reintro_draw_bp` | `database.py:quest_reintro_draw_bp` | adapter | the reference `<key>:<ISO date>` from the case's key and epoch day |
| `quest_should_reintroduce` | `database.py:quest_should_reintroduce` | adapter | the same reference; class `epsilon` for draws either side of 500 |
| `q2_for` | `gamification/quests.py:q2_for` | adapter | a `date` from the epoch day and a suppressed set; classes by ordinal residue |
| `q3_offers` | `gamification/quests.py:q3_offers` | adapter | a `date`, the drop flag and a demoted set; classes by ordinal residue |
| `q3_pick` | `gamification/quests.py:q3_pick` | function | none; class `comeback` |
| `quest_progress` | `gamification/quests.py:progress_for` | adapter | a `QuestStats` from the case |
| `quest_is_done` | `gamification/quests.py:is_done` | adapter | a `QuestStats` from the case; class `retention` |
| `clear_due_is_feasible` | `gamification/quests.py:clear_due_is_feasible` | function | none |
| `is_retired` | `gamification/quests.py:is_retired` | function | none; class `floor` |
| `median_daily_reviews` | `database.py:GamifyStore.median_daily_reviews` | adapter | a temporary store database seeded with the case's rollups; classes `even`, `empty`, `gap` |
| `q2_evidence_suppressed` | `pipeline_layers/loot.py:LootLayer._q2_evidence_suppressed` | adapter | a stand-in layer over a stub store holding completions, focus rows and rollups; returns the suppressed keys and the reduced flag; class `floor` |
| `q3_demoted_keys` | `pipeline_layers/loot.py:LootLayer._q3_demoted_keys` | adapter | a stub store holding the active days' challenge rows; returns the demoted keys and their evidence |
| `weekly_for` | `gamification/quests.py:weekly_for` | adapter | a `date` for the week start and a suppressed set |
| `weekly_progress` | `gamification/quests.py:weekly_progress` | function | none |
| `weekly_evidence_suppressed` | `pipeline_layers/loot.py:LootLayer._weekly_evidence_suppressed` | adapter | a stub store holding completions, crown days and focus rows |
| `weekly_quest_settled` | `pipeline_layers/loot.py:LootLayer._update_weekly_quest` | adapter | a stand-in layer over a stub store holding the week's crowns, rollups, focus rows, freezes held, this month's drops and the claim; the smoke-bomb call recorded; returns the progress, the claim, the chest and the freeze |
| `ghost_build` | `gamification/ghost.py:build` | adapter | a dictionary of `date` to distinct cards, inserted in ascending order; returns the source week, the series, the total and the cap flag, or none; classes `cold`, `cap`, `tie` |
| `ghost_at` | `gamification/ghost.py:ghost_at` | adapter | a `Ghost` from the case |
| `photo_finish_ok` | `gamification/ghost.py:photo_finish_ok` | function | none; class `boundary` |
| `race_settled` | `pipeline_layers/ghost_race.py:GhostRaceLayer._settle_due_races` | adapter | a stub store holding ghosts, skip days and results; returns the results recorded and the win streak; classes `rest`, `empty` |
| `race_announced` | `pipeline_layers/ghost_race.py:GhostRaceLayer._announce_race_result` | adapter | a stub store holding the newest result and the win streak, and a recorder for the celebration and the plain line; returns which event, key or line |
| `quests.constants` | `gamification/quests.py`, `constants.py` and `database.py` | constants | `quests.Q1_KEY`, `Q1_TARGET`, `QUEST_XP`, `Q2_POOL`, `Q3_POOL`, `WEEKLY_POOL`, `SEAL_REVEAL_REVIEWS`, `Q3_AUTOPICK_HOUR`, `CLEAR_DUE_BACKLOG_MULTIPLE`, `QUEST_MIN_POOL`; `constants.COIN_QUEST_COMPLETE`, `MIN_OFFERS_FOR_RATE`, `QUEST_RETIREMENT_RATE`; `database.QUEST_GATE_WINDOW_DAYS`, `QUEST_REINTRO_SEED_DEFAULT`, `QUEST_REINTRO_EPSILON_BP` |
| `ghost.constants` | `gamification/ghost.py` | constants | `ghost.GHOST_CAP_FRAC`, `MIN_HISTORY_WEEKS`, `LOOKBACK_WEEKS`, `PHOTO_FINISH_CARDS`, `PHOTO_FINISH_FRAC`, `WIN_STREAK_DECAY` |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `quests` | `deck-streak-quests` | `migrations/008001_quests_daily_weekly_and_race.sql` | its quests, keyed by study day and slot, their instants in epoch milliseconds | exported and erased |
| `quest_offers` | `deck-streak-quests` | the same migration | its challenge offers and picks | exported and erased |
| `weekly_quests` | `deck-streak-quests` | the same migration | its weekly quests, keyed by the week's first study day | exported and erased |
| `crown_days` | `deck-streak-quests` | the same migration | its crown days | exported and erased |
| `ghosts` | `deck-streak-quests` | the same migration | its ghosts, with the week's own daily series and its leader from its per-week settings | exported and erased |
| `race_results` | `deck-streak-quests` | the same migration | its race results | exported and erased |
| `quest_state` | `deck-streak-quests` | the same migration | its reintroduction seed and ghost win streak settings | reset in place: the seed to its default, the win streak to 0 |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08001-FIRST-QUEST-TARGET-FIFTEEN` | `crates/quests/src/arc.rs` | the first quest asks for 15 reviews | `quests_progress::the_quest_constants_equal_the_golden_and_economy_json` |
| `S08002-SECOND-QUEST-PAYS-FORTY` | `crates/quests/src/arc.rs` | the second quest pays 40 XP | `quests_progress::the_quest_constants_equal_the_golden_and_economy_json` |
| `S08003-A-QUEST-PAYS-TEN-COINS` | `crates/quests/src/arc.rs` | each completion credits 10 coins | `quests_progress::the_quest_constants_equal_the_golden_and_economy_json` |
| `S08004-CLEAR-DUE-MULTIPLE-THREE` | `crates/quests/src/gates.rs` | `clear_due` needs a backlog within 3 times the median | `quests_gates::the_challenge_offer_gates_match_the_predecessors_goldens` |
| `S08005-REINTRODUCED-BELOW-FIVE-HUNDRED` | `crates/quests/src/gates.rs` | a suppressed key returns on a draw below 500 | `quests_gates::the_reintroduction_draw_matches_the_predecessors_goldens` |
| `S08006-GATE-WINDOW-THIRTY-DAYS` | `crates/quests/src/gates.rs` | the evidence window is 30 days | `quests_gates::the_sealed_quest_gate_matches_the_predecessors_golden` |
| `S08007-ORDINAL-OFFSET` | `crates/quests/src/arc.rs` | the rotations read the proleptic ordinal, the epoch day plus 719163 | `quests_rotation::the_daily_rotations_match_the_predecessors_goldens` |
| `S08008-DRAW-DIGEST-EIGHT-BYTES` | `crates/kernel/src/draw.rs` | the draw's BLAKE2b digest is 8 bytes long | `quests_draw::the_basis_point_draw_matches_the_predecessors_golden` |
| `S08009-GHOST-CAP-ONE-POINT-OH-FIVE` | `crates/quests/src/race.rs` | the ghost is capped at 1.05 times the recent middle | `race_ghost::the_ghost_matches_the_predecessors_golden` |
| `S08010-ONE-QUEST-PER-SLOT` | `migrations/008001_quests_daily_weekly_and_race.sql` | the key that holds one quest per study day and slot (a script-mutation row whose cargo killer is in `deck-streak-coordination`) | `quests_steps::quests_are_minted_once_while_their_day_is_current` |
