# SPEC-076: the language and law streaks, their freezes, the governor and the relight hold across every study day

- **Wave:** W3. **Issue:** #81, #82, #83, #84 (epic #4). **Context(s):** `deck-streak-streaks` (the
  language streak and its freezes, the one freeze port and its caps, the law streak, habit strength,
  the governor with its stored anchor, the standby notice rule and the relight rule; it extends
  SPEC-049's `lapse.rs`); `deck-streak-coordination` (the recompute's streak step, the freeze port's
  use case, the relight's grant and celebration, the views); `deck-streak-api` (the streak and
  governor routes); `deck-streak-bot` (`/streak`); the Mini App (`web/app`, the streak screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-040 (the relight pays through the
  grant port, once), ADR-041 (the relight's celebration goes through the one router), ADR-071 (the
  recompute settles each study day once, in order) and ADR-076 (the law streak is evaluated on every
  study day, so it decays).
- **Prerequisites:** SPEC-071 (the fold and each study day's reviews), SPEC-040 (the grant port),
  SPEC-041 (the router), SPEC-049 (the lapse slice this SPEC extends), SPEC-024 (the owner's
  session) and SPEC-026 (the bot's command table). **Mutation band:** `S07600-S07699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-076.md` (ADR-016).

## 1. The problem, measured

- **What exists.** At `dev` c3d769b, `crates/streaks/src/` holds only `lib.rs`. SPEC-049 (W1)
  plans `crates/streaks/src/lapse.rs`, a pure function that returns the open lapse and its id, and
  `crates/coordination/src/lapse.rs`, its one caller. No streak, freeze, strength or governor state
  exists, and no table of the streaks context.
- **What is ported.** The predecessor's `gamification/streak.py` (`classify_gap`, `update_on_study`,
  `decay_on_lapse`, `heat_for`, `badge_view`), `skip.py:real_misses`, `analytics.py:bridged_streak`,
  `pipeline.py:GamifyPipeline._advance_streak` and `_freeze_events_for`,
  `gamification/strength.py:advance`, `gamification/governor.py:assess`,
  `pipeline_layers/governor.py:GovernorLayer._update_strength` and `_update_governor` (the silence
  walk, the stored anchor, the standby notice), `pipeline_layers/showcase.py:ShowcaseLayer._relight`,
  and the freeze grants of four callers: `pipeline_layers/economy.py:EconomyLayer.buy_item`,
  `pipeline_layers/loot.py:LootLayer._update_weekly_quest` and `LootLayer.pick_epic_prize`, and
  `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes`.
- **The trap one sync a day opens.** The predecessor advances the language streak only for the
  study day that is current at its recompute (`_advance_streak`: `update_on_study` when that day has
  a study review, `decay_on_lapse` otherwise). DeckStreak's one scheduled sync runs just after the
  rollover (ADR-037), when the current study day is new and empty, so a verbatim port would never
  record the day before, and two such days would break the streak of an owner who studies daily and
  never triggers a sync. The fold (ADR-071) runs the same transitions for every study day it settles,
  in order, which is the sequence the predecessor's own calls produced one day at a time.
- **The defect ADR-076 fixes.** The predecessor's law streak is written only on a day with a law
  review (`pipeline_layers/digests.py:DigestsLayer._update_law_streak` returns early otherwise), so
  once law study stops the stored law streak never decays. `bridged_streak` itself is correct for
  any day; DeckStreak asks it on every settled and current study day.
- **Four words, held apart.** The streak counts consecutive study days; a freeze bridges one real
  miss; a lapse is the governor's episode of three or more silent study days with a lapse id
  (SPEC-049, docs/LEXICON.md); the relight is the first study day that ends a lapse with enough
  reviews.
- **What the parity oracle proves.** The streak's gap classifier and both transitions, the heat
  tiers and the comeback view, real misses, the bridged streak, the freeze events of a transition,
  the monthly drop gate and its reasons, strength, the verdict, the anchor past the walk, the standby
  notice's rule, the relight, and every constant (section 7).
- **Corrections to the issues.**
  - #82's first criterion ("updated only on a law study day") states the predecessor's defect; R11
    states the fixed rule and A10 asserts the decay (ADR-076).
  - #81's freeze events "plus shop, chest, weekly_quest and season grants": the predecessor writes
    those from their four callers; here each caller requests them through one port (R7 to R9), which
    holds both caps, including the monthly drop cap the predecessor's callers each checked.
  - #83's standby announcement: the rule is built and proved (R17); the notice is raised by the first
    discipline device, because no penalty can fire before it (section 5).
  - #84's relight reads the return day's whole review count at its settle (ADR-071), where the
    predecessor read the count at its first recompute of the return day.
- **Prerequisites.** SPEC-071 lands first: it owns the fold, each study day's review counts and the
  raw streak (`analytics.py:raw_streak`). SPEC-049's slice must exist, because R15 extends it
  without changing its answer for any run shorter than the walk. SPEC-040 and SPEC-041 carry the
  relight's grant and celebration. The skip days stay empty until the skip day exists (#108).

## 2. Requirements

The language streak and its freezes (#81)

R1. `streak_state` holds at most one row per track, `language` or `law`. A missing row reads as the
    track's start state: current 0, longest 0, `constants.STREAK_START_FREEZES` (1) freezes, no last
    study day, the comeback not armed (`goldens/streaks.constants.json`).
R2. The streak step registers in phase 3 of SPEC-071's fold (R19 there). For every study day the
    fold settles, in order, and then for the current study day, the language row advances once: by
    `update_on_study` when the day has a study review and by `decay_on_lapse` otherwise, each with
    the declared skip days, equal to the goldens of
    `gamification/streak.py:update_on_study` and `decay_on_lapse`. Both classify the gap through
    `classify_gap` (golden): bootstrap (current is the larger of 1 and the observed raw streak, which
    coordination takes from analytics), same day (no change), continue, freeze (one real miss with a
    freeze held spends one) and break (current 1). A real miss is a missed study day that is not a
    declared skip day (`skip.py:real_misses`, golden). The days the first recompute backfills with
    no today-only rule (SPEC-071 R17) advance neither track; the first day the fold settles then
    finds an empty language row and bootstraps it from that day's raw streak, as the predecessor's
    first run bootstrapped from the current day's.
R3. A freeze is earned when the new current is a multiple of `constants.STREAK_DAYS_PER_FREEZE` (7)
    on a day that is not a break, and the freezes stay between 0 and `constants.STREAK_FREEZE_CAP`
    (3), as the golden of `update_on_study` holds.
R4. A day without study zeroes the streak only when its real misses reach 2, the point where no freeze
    bought later that day could save it, and never changes the last study day, the freezes or the
    longest (golden of `decay_on_lapse`). A break of a streak of at least
    `constants.STREAK_COMEBACK_MIN` (7) arms the comeback, and the view the badges read shows it armed
    only while the streak is alive (golden of `badge_view`).
R5. The heat of a streak is `heat_for` over `constants.STREAK_HEAT`: tiers at 1, 7, 30, 100 and 365
    days, each with the predecessor's emoji (goldens of `heat_for` and the constants).
R6. A transition's freeze events are written with its state in one write, exactly once: `consumed`
    (-1), `streak_earn` (+n) and a `streak_break` marker (0), equal to the golden of
    `pipeline.py:GamifyPipeline._freeze_events_for`. A second recompute of a settled day writes no
    event: the same day is the classifier's no-op, and a day without study writes only when a
    persisted field changed, as `_advance_streak` does.

The freeze port (#81, and every freeze source)

R7. Only the streaks context writes `streak_state` and `freeze_events`. Every freeze that is not the
    streak's own earn comes through `grant_freeze(study day, reason)`, which coordination offers to
    other contexts' use cases: a shop purchase (`shop`, SPEC-082), an Epic chest's choice (`chest`,
    SPEC-081), the weekly quest's reward (`weekly_quest`, SPEC-080) and season node 5 (`season`,
    SPEC-074). A grant adds one freeze to the language row and its `freeze_events` row in one write.
R8. The port refuses every reason while the language row holds `constants.STREAK_FREEZE_CAP` (3)
    freezes. It refuses a drop, a reason of `chest`, `weekly_quest` or `season`, when the freezes
    those reasons granted in the calendar month of the study day it names (for a chest's choice, the
    chest's own study day) already reach `constants.FREEZE_DROP_MONTHLY_CAP` (1), counting positive
    deltas only; a `shop` purchase is capped by the hold alone. The counter, its reasons and the
    chest's fallback equal the golden `freeze_drop_gate`, whose stand-in runs the predecessor's own
    `database.py:GamifyStore.freeze_drops_in_month`.
R9. A refusal names the cap that stopped it, so each caller does what the predecessor did: the
    chest's choice becomes a Double-XP token, labelled as such (SPEC-081), the weekly quest and node 5
    pay without the freeze (SPEC-080, SPEC-074), and the shop writes no coin movement (SPEC-082).

The law streak (#82)

R10. The law streak counts consecutive study days with at least one review of a law-track card
    (SPEC-023's track). Declared skip days bridge it, neither counting nor breaking it; it has no
    freezes and no comeback.
R11. For every study day the fold settles, and then for the current study day, with a law review or
    without one, the law row's current is `bridged_streak(law study days, day, skip days)`, its
    longest the larger of itself and that value, and its last study day the latest day with a law
    review (golden of `analytics.py:bridged_streak`; ADR-076). This replaces the predecessor's write
    on law study days only, which never decayed.
R12. The law row keeps its start freezes: nothing spends one from it or grants one to it.

Habit strength and the governor (#83)

R13. Habit strength folds every day from the first study day, one day at a time:
    `s' = s × m + studied × (1 − m)` with `m = 0.5^(1 / constants.STRENGTH_HALF_LIFE_DAYS)` (13),
    the previous value clamped to 0..1 (goldens of `gamification/strength.py:advance` and the
    constants, which carry `gamification.strength.STRENGTH_DECAY`). A day counts as studied when it
    has a study review; a declared skip day with none counts as not studied, as in the predecessor's
    fold. The first recompute folds every day from the window's first study day and stores each day
    inside `pipeline_layers/governor.py:_STRENGTH_PERSIST_DAYS` (400) days of it (constants golden),
    as the predecessor's first run does; each later settle folds one day from the value
    `habit_strength` holds for the day before.
R14. The verdict is `assess(strength, silent run)` (golden of `gamification/governor.py:assess`):
    standby when strength is below `constants.STRENGTH_ARM_THRESHOLD` (0.6), lapse when the silent
    run reaches `constants.LAPSE_AFTER_SILENT_DAYS` (3), armed when neither holds.
R15. The silent run counts back from the day over consecutive days with no study review; a declared
    skip day neither counts nor ends it; the walk stops after
    `pipeline_layers/governor.py:_SILENCE_WALK_CAP_DAYS` (120) days (constants golden). SPEC-049's
    `crates/streaks/src/lapse.rs` gains the stored anchor as an input used only when the walk runs out
    of days; for every shorter run its answer is unchanged (SPEC-049's golden `lapse_episode`).
R16. `governor_state` (one row) stores, as of the last settled study day, the open lapse's anchor
    (its lapse id, an epoch day, or none), whether the verdict is standby, and the study day of the
    last standby notice. When the walk runs out of days without reaching a study day, a stored anchor
    at or before the walk's horizon is kept, and the recomputed one is written otherwise, equal to the
    golden `lapse_anchor_beyond_the_walk`. A study day clears the anchor. The current study day's
    verdict is read from the state settled for the day before and the day's reviews so far, and is
    stored only when the day settles.
R17. The standby notice falls due only when the verdict enters standby from a settled day that was
    not standby, outside a lapse, outside quiet hours, and not within 7 days of the stored notice day,
    equal to the golden `standby_notice`. The rule is a pure function of the streaks context; no
    notice is raised in this wave (section 5).

The relight (#84)

R18. The relight runs in phase 3, after the governor's step, so its XP is in the day's base before
    phase 5's derived bonuses and phase 6's mint read it. When the fold settles a study day and the
    governor state settled for the day before held an open lapse, the day relights when its study reviews reach `constants.RELIGHT_CARDS` (3): it grants
    `constants.RELIGHT_XP` (100) XP through SPEC-040's grant port (source `relight:<epoch day>`, scope
    `once`, track `language`) and raises one celebration through the router (event `record`, dedupe
    key `relight:<epoch day>`), equal to the golden of `ShowcaseLayer._relight`. The count is the day's
    whole count at its settle (ADR-071); a return day that already reached it at a mid-day recompute
    was granted then, and its settle finds the grant written.
R19. A relight happens once per episode: a second recompute of the return day neither grants (the
    port answers `AlreadyGranted`) nor celebrates (the router's once-ever dedupe) again.

Views and surfaces

R20. `GET /api/streak` serves, to the owner's session only, both tracks (current, longest, heat, and
    for the language track its freezes and their cap), the window's study days with their freeze,
    skip and break markers, and what is at stake if the current study day ends with no study review:
    `freeze` when a freeze would cover the missed day, `break` when none is held, `none` when the day
    already has a study review or the streak is zero, taken from `classify_gap` for the next study
    day. During a lapse it also serves `constants.RELIGHT_CARDS`.
R21. `GET /api/governor` serves, to the owner's session only, the verdict (armed, standby or lapse),
    the strength, the lapse's anchor, and why the governor is disarmed when it is.
R22. The bot's `/streak` shows both tracks, the law track first when it has activity, with the
    language track's heat and freezes.
R23. The Mini App's `/streak` screen shows both tracks side by side, law first when the law track has
    activity; the calendar with its markers; the at-stake line, stated as the rule (a freeze covers one
    missed day) with no loss or shame wording; a governor chip that says armed, standby or lapse and
    why; and, in a lapse, how many reviews relight it.

Data rights

R24. The migration `migrations/007601_streaks_state_and_governor.sql` creates `streak_state`,
    `freeze_events`, `habit_strength` and `governor_state`, each `STRICT` with `created_at` (SPEC-020
    R15, R18). `streak_state`, `freeze_events` and `habit_strength` are exported and erased, so after
    an erase each track reads its start state (R1); `governor_state` is one seeded row, reset in place
    to no anchor, not standby and no notice day. Each is registered in the context map's ownership
    register, declared in `privacy.json` and listed in the streaks data-rights port.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the language streak's transitions equal the goldens of `update_on_study`, `classify_gap` and `real_misses` for every case (examined count reported, zero refused) | `the_language_streak_transitions_match_the_parity_goldens` |
| A2 | a day without study zeroes the streak only at two real misses and never changes the last study day, the freezes or the longest, equal to the golden of `decay_on_lapse` | `a_day_without_study_breaks_only_an_unsavable_streak` |
| A3 | the heat and the comeback view equal the goldens of `heat_for` and `badge_view` | `the_heat_and_the_comeback_view_match_the_parity_goldens` |
| A4 | the streak constants equal the constants golden, and `economy.json`'s `streak`, `governor` and `xp.bonuses.relight` values equal them | `the_streak_constants_and_economy_json_match_the_predecessors` |
| A5 | each transition writes its freeze events with its state, equal to the golden of `_freeze_events_for`, and a second recompute of a settled day writes none | `a_settled_day_writes_its_freeze_events_once` |
| A6 | with one scheduled sync a day and a study review every day, the language streak grows by one each day with no owner sync | `a_streak_studied_daily_survives_one_sync_a_day` |
| A7 | the freeze port refuses every reason at the hold cap, refuses a second chest, weekly quest or season freeze in one calendar month, and still grants a shop freeze after a drop | `the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap` |
| A8 | the drop gate's reasons, its monthly count and the chest's fallback equal the golden `freeze_drop_gate` | `the_drop_gate_matches_the_parity_golden` |
| A9 | the law streak equals the golden of `bridged_streak` on every settled and current day of a fixture | `the_law_streak_matches_the_parity_golden` |
| A10 | after a study day with no law review and no declared skip, the law streak reads zero, where the predecessor's caller kept its last value | `the_law_streak_decays_without_law_study` |
| A11 | no freeze is spent from or granted to the law row | `the_law_row_never_spends_or_receives_a_freeze` |
| A12 | habit strength and the verdict equal the goldens of `strength.py:advance` and `governor.py:assess` | `strength_and_the_verdict_match_the_parity_goldens` |
| A13 | past the walk the anchor equals the golden `lapse_anchor_beyond_the_walk`, and every shorter run gives SPEC-049's episode and id | `the_anchor_beyond_the_walk_matches_the_parity_golden` |
| A14 | every day of one episode keeps one anchor across recomputes | `one_episode_keeps_its_anchor_across_recomputes` |
| A15 | the standby notice falls due exactly as the golden `standby_notice` says | `the_standby_notice_rule_matches_the_parity_golden` |
| A16 | the relight rule equals the golden of `_relight`: 100 XP and one celebration keyed by the day at 3 reviews or more, nothing below | `the_relight_rule_matches_the_parity_golden` |
| A17 | a second recompute of a return day neither grants nor celebrates the relight again | `a_lapse_relights_once_per_episode` |
| A18 | a return day whose third review lands after a mid-day recompute still relights at its settle | `a_return_day_relights_at_its_settle_with_its_whole_count` |
| A19 | after an erase, each track reads its start state and the governor row holds its reset values | `an_erase_returns_both_tracks_to_their_start_state` |
| A20 | the streak and governor routes answer the owner's session only, and any other caller gets 401 or 403 and no data | `the_streak_routes_answer_only_the_owner` |
| A21 | `/streak` shows both tracks, law first when the law track has activity | `streak_shows_both_tracks_law_first_when_law_is_active` |
| A22 | the streak screen shows law first when law is active, the at-stake line, and a governor chip that says why it is disarmed | `shows law first and says why the governor is disarmed` |

```acceptance
A1: cargo test -p deck-streak-streaks --test streak_goldens -- --exact the_language_streak_transitions_match_the_parity_goldens
A2: cargo test -p deck-streak-streaks --test streak_goldens -- --exact a_day_without_study_breaks_only_an_unsavable_streak
A3: cargo test -p deck-streak-streaks --test streak_goldens -- --exact the_heat_and_the_comeback_view_match_the_parity_goldens
A4: cargo test -p deck-streak-streaks --test streak_goldens -- --exact the_streak_constants_and_economy_json_match_the_predecessors
A5: cargo test -p deck-streak-coordination --test streak_fold -- --exact a_settled_day_writes_its_freeze_events_once
A6: cargo test -p deck-streak-coordination --test streak_fold -- --exact a_streak_studied_daily_survives_one_sync_a_day
A7: cargo test -p deck-streak-streaks --test freeze_port -- --exact the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap
A8: cargo test -p deck-streak-streaks --test freeze_port -- --exact the_drop_gate_matches_the_parity_golden
A9: cargo test -p deck-streak-streaks --test streak_law -- --exact the_law_streak_matches_the_parity_golden
A10: cargo test -p deck-streak-coordination --test streak_fold -- --exact the_law_streak_decays_without_law_study
A11: cargo test -p deck-streak-streaks --test streak_law -- --exact the_law_row_never_spends_or_receives_a_freeze
A12: cargo test -p deck-streak-streaks --test governor_goldens -- --exact strength_and_the_verdict_match_the_parity_goldens
A13: cargo test -p deck-streak-streaks --test governor_goldens -- --exact the_anchor_beyond_the_walk_matches_the_parity_golden
A14: cargo test -p deck-streak-coordination --test streak_fold -- --exact one_episode_keeps_its_anchor_across_recomputes
A15: cargo test -p deck-streak-streaks --test governor_goldens -- --exact the_standby_notice_rule_matches_the_parity_golden
A16: cargo test -p deck-streak-streaks --test relight_rule -- --exact the_relight_rule_matches_the_parity_golden
A17: cargo test -p deck-streak-coordination --test relight_settle -- --exact a_lapse_relights_once_per_episode
A18: cargo test -p deck-streak-coordination --test relight_settle -- --exact a_return_day_relights_at_its_settle_with_its_whole_count
A19: cargo test -p deck-streak-streaks --test streak_rights -- --exact an_erase_returns_both_tracks_to_their_start_state
A20: cargo test -p deck-streak-api --test streak_routes -- --exact the_streak_routes_answer_only_the_owner
A21: cargo test -p deck-streak-bot --test streak_commands -- --exact streak_shows_both_tracks_law_first_when_law_is_active
A22: pnpm exec vitest run web/app/src/lib/streak/streak-screen.test.ts -t "shows law first and says why the governor is disarmed"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, game-economy, ux-laws and
accessibility packs stay enforced, and no row is deferred for this delivery.

| id | criterion | decided by |
|---|---|---|
| B1 | every table this delivery creates is declared under a category with its data, purpose, basis, retention and erase mode, over `privacy.json`, `migrations/007601_streaks_state_and_governor.sql` and `crates/streaks/src/data_rights.rs`, examining all four tables | the privacy-gdpr pack |
| B2 | the streak and governor values the engine is held equal to (A4) still equal the reference economy, over `economy.json`'s `streak` and `governor` sections | the game-economy pack |
| B3 | the streak copy offers forgiveness with no loss, shame or pressure wording, over `web/app/src/routes/streak/+page.svelte`, every file under `web/app/src/lib/streak/` and `crates/bot/src/streak_commands.rs` | the ux-laws pack |
| B4 | the streak screen passes the accessibility audit in both of Telegram's colour schemes, over the `/streak` route that `web/app/src/lib/routes.ts` lists | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/streaks/src/lib.rs` | `deck-streak-streaks` | changed: the modules below |
| `crates/streaks/src/constants.rs` | `deck-streak-streaks` | added: the streak, freeze, strength, governor and relight constants, proved by the golden |
| `crates/streaks/src/streak.rs` | `deck-streak-streaks` | added: the gap classifier, both transitions, the heat, the comeback view and a transition's freeze events |
| `crates/streaks/src/law.rs` | `deck-streak-streaks` | added: the law streak, asked on every day (ADR-076) |
| `crates/streaks/src/freeze.rs` | `deck-streak-streaks` | added: the freeze port, the hold cap and the monthly drop cap |
| `crates/streaks/src/strength.rs` | `deck-streak-streaks` | added: the strength fold |
| `crates/streaks/src/governor.rs` | `deck-streak-streaks` | added: the verdict, the stored anchor and the standby notice rule |
| `crates/streaks/src/relight.rs` | `deck-streak-streaks` | added: the relight rule |
| `crates/streaks/src/lapse.rs` | `deck-streak-streaks` | changed: the stored anchor past the walk, unchanged for every shorter run |
| `crates/streaks/src/store.rs` | `deck-streak-streaks` | added: the repository over the four tables, through the kernel's base |
| `crates/streaks/src/data_rights.rs` | `deck-streak-streaks` | added: the streaks data-rights port |
| `crates/streaks/Cargo.toml` | `deck-streak-streaks` | changed: the workspace dependencies it uses (`sqlx`, `thiserror`, `tokio`); `serde` and `serde_json` stay dev-dependencies (SPEC-029 R8) |
| `migrations/007601_streaks_state_and_governor.sql` | `deck-streak-streaks` | added: the four tables, `STRICT`, the governor row seeded |
| `crates/streaks/tests/streak_goldens.rs` | `deck-streak-streaks` | added: A1 to A4 |
| `crates/streaks/tests/freeze_port.rs` | `deck-streak-streaks` | added: A7, A8 |
| `crates/streaks/tests/streak_law.rs` | `deck-streak-streaks` | added: A9, A11 |
| `crates/streaks/tests/governor_goldens.rs` | `deck-streak-streaks` | added: A12, A13, A15 |
| `crates/streaks/tests/relight_rule.rs` | `deck-streak-streaks` | added: A16 |
| `crates/streaks/tests/streak_rights.rs` | `deck-streak-streaks` | added: A19 |
| `crates/coordination/src/recompute/streaks.rs` | `deck-streak-coordination` | added: the streak step of the fold (both tracks, strength, the governor, the relight) |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: registers the streak step in phase 3 of SPEC-071's fold |
| `crates/coordination/src/freeze.rs` | `deck-streak-coordination` | added: the one freeze use case other contexts' use cases call |
| `crates/coordination/src/streak_views.rs` | `deck-streak-coordination` | added: the streak and governor views |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the modules above |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the streaks port joins the registry |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for the four tables |
| `crates/coordination/tests/streak_fold.rs` | `deck-streak-coordination` | added: A5, A6, A10, A14 |
| `crates/coordination/tests/relight_settle.rs` | `deck-streak-coordination` | added: A17, A18 |
| `crates/api/src/streak_routes.rs` | `deck-streak-api` | added: `GET /api/streak`, `GET /api/governor` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the two routes, behind the owner's session |
| `crates/api/tests/streak_routes.rs` | `deck-streak-api` | added: A20 |
| `crates/bot/src/streak_commands.rs` | `deck-streak-bot` | added: the streak command |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the streak command joins the command table and the owner's menu |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the streak command joins the census of command replies and callers |
| `crates/bot/tests/streak_commands.rs` | `deck-streak-bot` | added: A21 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the streaks store and the freeze port joined to coordination |
| `web/app/src/routes/streak/+page.svelte` | miniapp | added: the streak screen |
| `web/app/src/lib/streak/StreakScreen.svelte` | miniapp | added: both tracks, the calendar and the at-stake line |
| `web/app/src/lib/streak/streak.ts` | miniapp | added: the two routes' client |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the streak routes module joins the crate |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the streak commands module joins the crate |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: the command table names the streak command |
| `crates/bot/tests/messages/help.msg.json` | `deck-streak-bot` | changed: the help message lists the streak command |
| `crates/bot/tests/messages/start.msg.json` | `deck-streak-bot` | changed: the start message lists the streak command |
| `crates/coordination/src/relight.rs` | `deck-streak-coordination` | added: the relight's grant and its celebration key |
| `crates/coordination/src/sync_cycle.rs` | `deck-streak-coordination` | changed: the celebration is routed after the fold's commit |
| `crates/progression/src/ledger.rs` | `deck-streak-progression` | changed: `grant_on` writes a grant on the caller's connection |
| `crates/streaks/src/replay.rs` | `deck-streak-streaks` | added: the replay of the streaks and the governor over the study days |
| `web/app/messages/en.json` | miniapp | changed: the streak screen's messages |
| `web/app/src/lib/api.ts` | miniapp | changed: the client for the two routes |
| `web/app/src/lib/startapp.ts` | miniapp | changed: the streak screen joins the start-parameter routes |
| `web/app/src/lib/startapp.test.ts` | miniapp | changed: the start-parameter test names the streak screen |
| `web/app/src/lib/streak/streak-screen.test.ts` | miniapp | added: A22 |
| `web/app/src/lib/routes.ts` | miniapp | changed: the streak screen's route joins `ROUTES` |
| `tools/parity-oracle/registry/spec_076.py` | repo | added: this SPEC's registrations (SPEC-029's registry) |
| `tools/parity-oracle/goldens/classify_gap.json` | repo | added: the golden of `gamification/streak.py:classify_gap` (adapter; section 7) |
| `tools/parity-oracle/goldens/update_on_study.json` | repo | added: the golden of `gamification/streak.py:update_on_study` (adapter) |
| `tools/parity-oracle/goldens/decay_on_lapse.json` | repo | added: the golden of `gamification/streak.py:decay_on_lapse` (adapter) |
| `tools/parity-oracle/goldens/heat_for.json` | repo | added: the golden of `gamification/streak.py:heat_for` (function) |
| `tools/parity-oracle/goldens/badge_view.json` | repo | added: the golden of `gamification/streak.py:badge_view` (adapter) |
| `tools/parity-oracle/goldens/real_misses.json` | repo | added: the golden of `skip.py:real_misses` (adapter) |
| `tools/parity-oracle/goldens/bridged_streak.json` | repo | added: the golden of `analytics.py:bridged_streak` (adapter) |
| `tools/parity-oracle/goldens/freeze_events_for.json` | repo | added: the golden of `pipeline.py:GamifyPipeline._freeze_events_for` (adapter) |
| `tools/parity-oracle/goldens/freeze_drop_gate.json` | repo | added: the golden of `pipeline_layers/loot.py:LootLayer.pick_epic_prize` (adapter over a temporary store database) |
| `tools/parity-oracle/goldens/strength_advance.json` | repo | added: the golden of `gamification/strength.py:advance` (function) |
| `tools/parity-oracle/goldens/governor_assess.json` | repo | added: the golden of `gamification/governor.py:assess` (adapter) |
| `tools/parity-oracle/goldens/lapse_anchor_beyond_the_walk.json` | repo | added: the golden of `pipeline_layers/governor.py:GovernorLayer._update_governor` past the walk (adapter over a stub store) |
| `tools/parity-oracle/goldens/standby_notice.json` | repo | added: the golden of `pipeline_layers/governor.py:GovernorLayer._update_governor`'s notice (adapter over a stub store) |
| `tools/parity-oracle/goldens/relight.json` | repo | added: the golden of `pipeline_layers/showcase.py:ShowcaseLayer._relight` (adapter over a stub store) |
| `tools/parity-oracle/goldens/streaks.constants.json` | repo | added: the constants golden (constants) |
| `scripts/mutation-rows.d/S07600-S07699.json` | repo | added: the hand-proved rows (section 9) |
| `crates/streaks/tests/open_lapse_bound.rs` | `deck-streak-streaks` | added: A41, the generated population that pins the open lapse walk against the earlier loop (section 16) |
| `scripts/mutation-rows.d/S04900-S04999.json` | repo | changed: three lapse rows re-anchored on the counted range, ids and killers unchanged (section 16) |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains the four tables |
| `privacy.json` | repo | changed: the categories `streaks` and `governor` |
| `PRIVACY.md` | repo | changed: one line for each of the two categories |
| `.sqlx/` | workspace | changed: the offline query cache for the new queries |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-076-the-language-and-law-streaks-freezes-the-governor-and-the-relight-hold-across-every-study-day.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-076-the-law-streak-is-evaluated-every-study-day-so-it-decays.md` | docs | changed: accepted |
| `docs/red-first/SPEC-076.md` | docs | added |
| `changelog.d/feat-streaks-076.md` | repo | added: the changelog fragment |
| `crates/streaks/tests/population_rules.rs` | `deck-streak-streaks` | added: the generated populations of A26 to A32 |
| `crates/streaks/tests/store_effects.rs` | `deck-streak-streaks` | added: the store's read-back populations (A33) |
| `crates/streaks/tests/silence_walk_bound.rs` | `deck-streak-streaks` | added: the bounded walk against a test-only copy of the earlier loop, and its cap edge (A38, A39) |
| `crates/coordination/tests/streak_views.rs` | `deck-streak-coordination` | added: the views' populations (A34) |
| `scripts/mutation-equivalent.d/deck-streak-streaks.json` | repo | added: the eight equivalent mutants of `civil_month` |
| `web/app/src/lib/streak/streak.test.ts` | repo | added: the date rule and the readers' populations (A40) |
| `web/app/src/lib/streak/streak-screen-population.test.ts` | repo | added: the screen's populations (A40) |
| `web/app/src/lib/api-streak.test.ts` | repo | added: the client's call record (A40) |
| `web/app/src/routes/streak.test.ts` | repo | added: the page's call record (A40) |

## 5. What this does NOT do

- It sells no freeze: the one-tap purchase from the streak screen is the shop's (#107).
- It composes nothing on the home screen: the governor chip and the welcome-back line after a lapse
  are the home screen's, built from this SPEC's routes (#69).
- It sends no standby notice: before the first discipline device no penalty can fire, and that
  device raises the notice through the router (#109).
- It suppresses no discipline device in standby or a lapse: wagers void (#112), windows and the
  tripwire stand down (#109, #110), and markets hide (#119).
- It sends no comeback protocol message and no lapse digest (#123), and no evening stakes line
  (#117).
- It shows strength on no pinned widget (#121).
- It sends no streak share card (#125) and no milestone ping beyond the relight's celebration
  (#128).
- It writes no streak line into the daily digest or the weekly report (#129, #130).
- It declares no skip day: the fold passes an empty skip set until the skip day exists (#108).
- It serves no MCP streak tool (#157) and no public law page (#156).
- It imports none of the predecessor's streak, freeze, strength or governor rows (#61).

## 6. Risks

- **A review that syncs after its day has settled.** It raises that day's rollup and XP but does not
  rejoin either streak, as in the predecessor, whose streak moved only while a day was current.
  Detected by A6's fixture of one sync a day; an owner's `/sync` before the rollover counts a late
  device's reviews in time (ADR-037).
- **The governor departs from SPEC-049's slice.** Detected by A13, which runs every run shorter than
  the walk through SPEC-049's golden, and by SPEC-049's own A11.
- **A freeze source writes the streak row itself**, for example a chest's choice in the quests
  context. Prevented by the crate graph (quests may not depend on streaks) and detected by A7, whose
  port is the one writer.
- **The law streak's decay reads as a regression during side by side,** where the predecessor's bot
  still shows its stale value. Recorded by ADR-076 as a divergence for the side-by-side
  verification (#62).
- **The standby notice never falls due at the scheduled settle,** which runs inside quiet hours by
  default. Recorded for the discipline wave, which decides when the notice is raised (#109).
- **Floating point in the strength fold drifts from the predecessor's.** The port multiplies in the
  predecessor's order in `f64`, and the golden's cases run long folds whose value crosses the arm
  threshold (A12).
- **The Mini App and the bot disagree on what is at stake.** Both read `GET /api/streak`; A21 and A22
  run the same fixture.

## 7. Parity goldens

All are registered in `tools/parity-oracle/registry/spec_076.py` and generated on the owner's
checkout of the predecessor at `27ee2bc` (SPEC-029). Every day is an epoch day number and every
instant epoch milliseconds; no golden holds a calendar date.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `goldens/classify_gap.json` | `gamification/streak.py:classify_gap` | adapter | a `StreakState` from the case's fields, its last study day and the skip days from epoch day numbers; returns the outcome |
| `goldens/update_on_study.json` | `gamification/streak.py:update_on_study` | adapter | the state, the day, the observed raw streak and the skip days as above; returns the new state, its last study day as an epoch day, and its transient flags |
| `goldens/decay_on_lapse.json` | `gamification/streak.py:decay_on_lapse` | adapter | as above; cases at one and two real misses, with and without a freeze, across skip days |
| `goldens/heat_for.json` | `gamification/streak.py:heat_for` | function | none |
| `goldens/badge_view.json` | `gamification/streak.py:badge_view` | adapter | a `StreakState`; returns the comeback flag the view shows |
| `goldens/real_misses.json` | `skip.py:real_misses` | adapter | the last study day, the day and the skip days from epoch day numbers |
| `goldens/bridged_streak.json` | `analytics.py:bridged_streak` | adapter | the law study days, the day asked about and the skip days from epoch day numbers; cases on a law day, on a day with no law review yet, after a missed day, and with skip days at each end of a run |
| `goldens/freeze_events_for.json` | `pipeline.py:GamifyPipeline._freeze_events_for` | adapter | two `StreakState`s and a day; returns the event rows, each day an epoch day |
| `goldens/freeze_drop_gate.json` | `pipeline_layers/loot.py:LootLayer.pick_epic_prize` | adapter | a stand-in layer over a temporary store database seeded with an opened Epic chest on the case's day, the case's freeze events and held freezes; its `freeze_drops_in_month` is the predecessor's own and records the reasons it is asked for; returns whether the freeze or the token was paid, and those reasons |
| `goldens/strength_advance.json` | `gamification/strength.py:advance` | function | none; cases include values outside 0..1 |
| `goldens/governor_assess.json` | `gamification/governor.py:assess` | adapter | returns the verdict's fields and `armed` |
| `goldens/lapse_anchor_beyond_the_walk.json` | `pipeline_layers/governor.py:GovernorLayer._update_governor` | adapter | a stand-in layer over a stub store holding the study days, the skip days and the stored governor state, with the relight and the notifier stubbed; returns the anchor written, as an epoch day; every case's silence is longer than the walk, with no stored anchor, with one at the horizon, and with one newer than the horizon |
| `goldens/standby_notice.json` | `pipeline_layers/governor.py:GovernorLayer._update_governor` | adapter | the same stand-in with a recording notifier and the quiet-hours check patched to the case's flag; returns whether a notice was sent and the notice day stored |
| `goldens/relight.json` | `pipeline_layers/showcase.py:ShowcaseLayer._relight` | adapter | a stub store with the day's review count and any relight grant already written, and a recording `celebrate`; returns the grant's amount and the celebration's event type and key, the key's day written as its epoch day number |
| `goldens/streaks.constants.json` | `constants.py` and two modules | constants | `constants.STREAK_START_FREEZES`, `STREAK_FREEZE_CAP`, `STREAK_DAYS_PER_FREEZE`, `STREAK_COMEBACK_MIN`, `STREAK_HEAT`, `FREEZE_DROP_MONTHLY_CAP`, `STRENGTH_HALF_LIFE_DAYS`, `STRENGTH_ARM_THRESHOLD`, `LAPSE_AFTER_SILENT_DAYS`, `RELIGHT_CARDS`, `RELIGHT_XP`; `gamification.strength.STRENGTH_DECAY`; `pipeline_layers.governor._SILENCE_WALK_CAP_DAYS` and `_STRENGTH_PERSIST_DAYS` |

SPEC-049's `goldens/lapse_episode.json` covers every run shorter than the walk and is not registered
again.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `streak_state` | `streaks` | `migrations/007601_streaks_state_and_governor.sql` (SPEC-076) | `streak_state`, one row per track, its last study day as an epoch day | exported and erased; a missing row reads as the track's start state |
| `freeze_events` | `streaks` | the same migration | `freeze_events`, one row per grant, spend or break marker, its day as an epoch day | exported and erased |
| `habit_strength` | `streaks` | the same migration | `habit_strength`, one strength per study day | exported and erased |
| `governor_state` | `streaks` | the same migration | the `governor_state` singleton, its anchor and notice day as epoch days | reset in place: no anchor, not standby, no notice day |

## 9. Mutation rows

The band is `S07600-S07699`, in `scripts/mutation-rows.d/S07600-S07699.json`. Each row's killer
selects exactly one test, and each is proved with its file restored byte for byte (SPEC-039).

| row | target | what it guards | killer |
|---|---|---|---|
| `S07601-A-TRACK-STARTS-WITH-ONE-FREEZE` | `crates/streaks/src/constants.rs` | a new track starts with one freeze | `streak_goldens::the_streak_constants_and_economy_json_match_the_predecessors` |
| `S07602-A-FREEZE-EVERY-SEVEN-STREAK-DAYS` | `crates/streaks/src/constants.rs` | a freeze is earned every 7 streak days | `streak_goldens::the_language_streak_transitions_match_the_parity_goldens` |
| `S07603-THE-HOLD-CAP-IS-THREE` | `crates/streaks/src/constants.rs` | no more than 3 freezes are held | `freeze_port::the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap` |
| `S07604-TWO-REAL-MISSES-BREAK-A-STREAK` | `crates/streaks/src/streak.rs` | a day without study breaks only an unsavable streak | `streak_goldens::a_day_without_study_breaks_only_an_unsavable_streak` |
| `S07605-ONE-DROP-A-MONTH` | `crates/streaks/src/constants.rs` | one drop-style freeze a calendar month | `freeze_port::the_freeze_port_holds_the_hold_cap_and_the_monthly_drop_cap` |
| `S07606-THE-SHOP-IS-NOT-A-DROP` | `crates/streaks/src/freeze.rs` | the drop reasons are chest, weekly quest and season, never the shop | `freeze_port::the_drop_gate_matches_the_parity_golden` |
| `S07607-THE-LAW-STREAK-IS-ASKED-EVERY-DAY` | `crates/coordination/src/recompute/streaks.rs` | the law step runs on a day with no law review (ADR-076) | `streak_fold::the_law_streak_decays_without_law_study` |
| `S07608-STRENGTH-HALVES-IN-THIRTEEN-DAYS` | `crates/streaks/src/constants.rs` | the strength half-life | `governor_goldens::strength_and_the_verdict_match_the_parity_goldens` |
| `S07609-ONE-STANDBY-NOTICE-A-WEEK` | `crates/streaks/src/governor.rs` | no notice within 7 days of the last | `governor_goldens::the_standby_notice_rule_matches_the_parity_golden` |
| `S07610-THE-RELIGHT-NEEDS-THREE-REVIEWS` | `crates/streaks/src/constants.rs` | a return day relights at 3 reviews | `relight_rule::the_relight_rule_matches_the_parity_golden` |
| `S07611-THE-SILENCE-WALK-READS-ONE-PAST-THE-CAP` | `crates/streaks/src/governor.rs` | the walk counts up to one day past the cap | `silence_walk_bound::the_walk_stops_at_the_cap_from_the_specs_words` |

## 10. Amendment, 2026-09-29: the prerequisite is SPEC-049's lapse slice, not the whole of SPEC-049

Made under ADR-088, insert-only: every earlier byte is kept in order, and this section is the only
insertion.

- Where this SPEC names SPEC-049 (the header, section 1, R15, A13, and sections 6 and 7), it names
  the slice SPEC-049 section 7 defines: R12 to R15, decided by its A11, A12, A13 and A15. SPEC-049's
  remainder, the comeback reading, is not a prerequisite of this SPEC, and nothing in this SPEC
  reads SPEC-046, SPEC-047, SPEC-048, SPEC-051 or SPEC-052.
- The slice leaves SPEC-049 in `docs/specs/planned/` (ADR-016). This prerequisite is therefore met
  when `dev` holds `crates/streaks/src/lapse.rs`, `crates/coordination/src/lapse.rs`,
  `tools/parity-oracle/registry/spec_049.py` and `tools/parity-oracle/goldens/lapse_episode.json`,
  and SPEC-049's A11, A12, A13 and A15 pass there. A dispatch check reads those, not the folder
  SPEC-049 is in.
- The slice's function takes no stored anchor. R15 adds it, so the manifest's rows marked `changed`
  for the two `lapse.rs` files change the slice's files.

## 11. Amendment, 2026-09-29: an empty history is answered as the predecessor answers it

Made on the orchestrator's ruling at re-dispatch, insert-only, after section 10. SPEC-049's own
answer and its criterion stay as they are: the slice's function answers no lapse for a window with
no anchor, and this layer, which holds the stored anchor, answers an empty history through it.

R25. When the study set is empty, the layer answers as the predecessor's governor does: the silence
    walk reaches its 120-day cap, so the lapse is anchored on the stored anchor when one exists and
    is not later than the horizon (today minus 120 days), and otherwise on the horizon. The rule
    lives in `crates/streaks/src/lapse.rs` beside the slice's function and leaves the slice's
    answers unchanged.

The criterion of this amendment, A23, is stated in section 14.

```acceptance
A23: cargo test -p deck-streak-streaks --test governor_goldens -- --exact an_empty_history_is_anchored_as_the_predecessor_anchors_it
```

## 12. Amendment, 2026-09-29: the relight is granted on the fold's connection and celebrated after its commit

Made on the orchestrator's ruling at the third dispatch, insert-only, after section 11. It corrects
R18's wording only where the code proved it incomplete: the fold holds one write transaction while
its steps run, so a step that opens a second writer for the grant, or asks the router to record a
send, waits on a lock its own caller holds.

R26. The relight's XP is written on the fold's own connection. `crates/progression/src/ledger.rs`
    gains `grant_on(connection, request, at)`, which carries the grant port's two queries verbatim, so
    the offline query cache is unchanged, and `SqliteXpLedger::grant` opens its write, calls
    `grant_on` and commits. The streak step calls `grant_on` in phase 3, on the fold's write, so the
    relight's XP is in the day's base before phase 5's derived bonuses and phase 6's mint read it in
    the same recompute (R18). The source, scope and track stay R18's (`relight:<epoch day>`, `once`,
    `language`).
R27. The relight's celebration is routed after the fold commits. The streak step answers the relight
    as due, and the caller routes it after the fold runs, as the level-up is announced. The policy has
    no `record` kind, so the celebration is routed under the policy's `celebration` kind (once-ever
    dedupe) with the key `relight:<epoch day>`; `record` stays the predecessor's event name inside the
    relight rule's answer, which the golden asserts. The caller routes on every settle that
    qualifies, whether the grant answered `Granted` or `AlreadyGranted`: the router's dedupe gives one
    send per episode, and a crash between the commit and the route is recovered at the next
    recompute. `notifications-policy.json` gains no kind.

Manifest additions: `crates/progression/src/ledger.rs` (changed: `grant_on`), and the test file
`crates/coordination/tests/relight_settle.rs` gains the two criteria of section 13.

## 13. Acceptance criteria of the 2026-09-29 relight amendment

| id | criterion | decided by |
|---|---|---|
| A24 | a recompute that relights completes with the grant written on the fold's connection, on a pool that holds one connection, so a second writer would hang | `a_relight_is_granted_on_the_folds_connection` |
| A25 | a second recompute of the return day routes the celebration again and the router answers it as already sent, so exactly one send is recorded | `a_second_recompute_routes_the_relight_and_one_send_is_recorded` |

```acceptance
A24: cargo test -p deck-streak-coordination --test relight_settle -- --exact a_relight_is_granted_on_the_folds_connection
A25: cargo test -p deck-streak-coordination --test relight_settle -- --exact a_second_recompute_routes_the_relight_and_one_send_is_recorded
```

## 14. Acceptance criteria of the 2026-09-29 empty-history amendment

| id | criterion | decided by |
|---|---|---|
| A23 | an empty history is answered with the stored anchor or the horizon, equal to the golden `lapse_anchor_beyond_the_walk`'s empty-history cases | `an_empty_history_is_anchored_as_the_predecessor_anchors_it` |

## 15. Acceptance criteria of the 2026-09-30 mutation-hardening amendment

Insert-only. Every rule of the streaks crate, its views, routes and reply is pinned by ONE
population per function, generated at test time, counted and asserted, with an oracle written from
this SPEC's words and never by calling the function it judges. The class rules:

- Date arithmetic (`civil_month`): every day from 135000 days before the epoch to 48300 after, with
  probes at month, year and era distances, must share a calendar month exactly when the SPEC's
  calendar says so.
- Comparisons at a boundary (`freeze_events_for`, the law row, `anchor_beyond_the_walk`, the
  standby notice, `persists`): the boundary less one, the boundary and the boundary plus one, for
  every input axis.
- Boolean rules: the full truth table of the inputs, every subset of the days the run reads.
- Deleted arms, fields and lists: a stored-then-read population whose oracle is a raw read-back of
  the row, so an arm, a field or a list that is dropped changes what is read.
- Views, routes and commands: status, body shape and stored effect per handler, and every refusal
  names its reason (`database_not_open`, `streak_unreadable`).
- The silence walk (R11): the walk is bounded. It reads at most `SILENCE_WALK_CAP_DAYS + 1` days
  by a counted `for`, and the other counter loops (`language`, `law`, `fold`, `bridged_streak`) run
  over the range of epoch days they read, so no mutant of a counter can spin. `open_lapse` ends by
  `checked_sub` over a finite window and is left as it was: converting it would change what it
  answers at the smallest epoch day. The bounded walk answers what the earlier open loop answered,
  measured against a test-only copy of that loop.
- The streak screen, its readers and its page (web): the date rule over a population of valid and
  invalid dates and non-string lookalikes; the readers over every bad body, count and verdict; the
  screen over the law runs, freeze combinations, stakes and governor cases; the client and page over
  the calls they make.

| id | criterion | decided by |
|---|---|---|
| A26 | every day falls in the calendar month the drop cap reads | `every_day_falls_in_the_calendar_month_the_cap_reads` |
| A27 | the freeze events follow the flags and the freezes gained | `the_freeze_events_follow_the_flags_and_the_freezes_gained` |
| A28 | the law row follows the run over every assignment of eight days | `the_law_row_follows_the_run_over_every_assignment_of_eight_days` |
| A29 | the language row and its events follow the rules over every subset of twelve days | `the_language_row_and_events_follow_the_rules_over_every_subset_of_twelve_days` |
| A30 | the heat tier counts the thresholds reached for every length | `the_heat_tier_counts_the_thresholds_reached_for_every_length` |
| A31 | the fold stores today, yesterday and the first run's window | `the_fold_stores_today_yesterday_and_the_first_run_window` |
| A32 | the silence walk and the stored anchor follow the run at every distance | `the_silence_walk_and_the_stored_anchor_follow_the_run_at_every_distance` |
| A33 | what the store writes is what it reads back, for the state, the strength, the governor row, the events, the outside freezes and a freeze added | `store_effects` |
| A34 | the views name what is at stake and the heat for every stored pair, read a missing row as the start state and give the governor's verdict for every stored row | `streak_views` |
| A35 | the streak step carries its name, and the outside freezes join the language row within zero and three | `the_step_is_named_for_the_fold_report`, `the_outside_freezes_join_the_language_row_within_zero_and_three` |
| A36 | the law leads only above zero and the noun follows the freezes | `the_law_leads_only_above_zero_and_the_noun_follows_the_freezes` |
| A37 | the streak routes name why they cannot answer | `the_streak_routes_name_why_they_cannot_answer` |
| A38 | the bounded silence walk answers what the earlier loop answered | `the_bounded_walk_answers_what_the_reference_walk_answers` |
| A39 | the walk stops at the cap, from the SPEC's words | `the_walk_stops_at_the_cap_from_the_specs_words` |
| A40 | the web mutation check of the five streak files reports survived 0 and no coverage 0 apart from the recorded equivalents | `web/app/src/lib/streak/streak.test.ts`, `streak-screen-population.test.ts`, `api-streak.test.ts`, `routes/streak.test.ts` |

```acceptance
A26: cargo test -p deck-streak-streaks --test population_rules -- --exact every_day_falls_in_the_calendar_month_the_cap_reads
A27: cargo test -p deck-streak-streaks --test population_rules -- --exact the_freeze_events_follow_the_flags_and_the_freezes_gained
A28: cargo test -p deck-streak-streaks --test population_rules -- --exact the_law_row_follows_the_run_over_every_assignment_of_eight_days
A29: cargo test -p deck-streak-streaks --test population_rules -- --exact the_language_row_and_events_follow_the_rules_over_every_subset_of_twelve_days
A30: cargo test -p deck-streak-streaks --test population_rules -- --exact the_heat_tier_counts_the_thresholds_reached_for_every_length
A31: cargo test -p deck-streak-streaks --test population_rules -- --exact the_fold_stores_today_yesterday_and_the_first_run_window
A32: cargo test -p deck-streak-streaks --test population_rules -- --exact the_silence_walk_and_the_stored_anchor_follow_the_run_at_every_distance
A33: cargo test -p deck-streak-streaks --test store_effects
A34: cargo test -p deck-streak-coordination --test streak_views
A35: cargo test -p deck-streak-coordination --test streak_fold -- --exact the_step_is_named_for_the_fold_report
A36: cargo test -p deck-streak-bot --test streak_commands -- --exact the_law_leads_only_above_zero_and_the_noun_follows_the_freezes
A37: cargo test -p deck-streak-api --test streak_routes -- --exact the_streak_routes_name_why_they_cannot_answer
A38: cargo test -p deck-streak-streaks --test silence_walk_bound -- --exact the_bounded_walk_answers_what_the_reference_walk_answers
A39: cargo test -p deck-streak-streaks --test silence_walk_bound -- --exact the_walk_stops_at_the_cap_from_the_specs_words
A40: pnpm exec vitest run web/app/src/lib/streak/streak.test.ts -t "refuses a body that is not an object, or that lacks any key"
```

The silence walk's bound is not a timeout. Two alternatives were rejected: leaving the open loop
and raising the job's timeout, because a mutant that never advances its counter then costs the
whole budget of the shard and hides the mutants behind it; and a per-test timeout, because it makes
the answer depend on the machine's speed and kills a spinning test without pinning the rule.

Eight mutants of `civil_month` in `freeze.rs` are equivalent and are recorded in
`scripts/mutation-equivalent.d/deck-streak-streaks.json`: each relabels the month or the year
without moving two days across a month boundary, which was measured over the whole range above.

Manifest additions: `crates/streaks/tests/population_rules.rs`, `store_effects.rs` and
`silence_walk_bound.rs`; `crates/coordination/tests/streak_views.rs`; the streak fold, bot and api
test files gain the criteria above; `crates/streaks/src/governor.rs`, `replay.rs`, `strength.rs` and
`law.rs` (changed: the counter loops are bounded); the row `S07611`; and the record file above.

## 16. Amendment, 2026-09-30: the open lapse walks a counted range, and three rules gain a fence

Insert-only: every earlier byte is kept in order, and this section is the only insertion.

- `open_lapse` (SPEC-049 R12, R13) is bounded as the other walks are. It reads the window's days by
  a counted range, newest first, and answers nothing when the walk reaches the smallest epoch day,
  as the earlier `checked_sub` step did. This supersedes the sentence of section 15 that leaves it
  as it was. That loop ended, but its mutant `while number < window_start` does not end for a today
  before the window's first day, and a conversion that keeps the answer at the smallest epoch day
  exists: A41 proves it against a test-only copy of the earlier loop.
- The bot's `/streak` reply is judged over runs that differ from bests (A42). A36 writes each row
  with its best equal to its run, so it cannot tell the two apart: a reply led by the law best, or
  one that shows a track's best as its run, passes A36 and the whole bot crate.
- The second half of A35, the outside freezes joining the language row within zero and three, is
  fenced by its own test (A43). A35's fence runs only the step's name.

## 17. Acceptance criteria of the 2026-09-30 open-lapse amendment

| id | criterion | decided by |
|---|---|---|
| A41 | the bounded lapse walk answers what the earlier loop answered, at every origin, the smallest and largest epoch days among them | `the_bounded_lapse_walk_answers_what_the_reference_walk_answers` |
| A42 | each line of the reply names its own run and best, and the law leads by its run | `each_line_names_its_own_run_and_best_and_the_law_leads_by_its_run` |
| A43 | the outside freezes join the language row within zero and three | `the_outside_freezes_join_the_language_row_within_zero_and_three` |

```acceptance
A41: cargo test -p deck-streak-streaks --test open_lapse_bound -- --exact the_bounded_lapse_walk_answers_what_the_reference_walk_answers
A42: cargo test -p deck-streak-bot --test streak_commands -- --exact each_line_names_its_own_run_and_best_and_the_law_leads_by_its_run
A43: cargo test -p deck-streak-coordination --test streak_fold -- --exact the_outside_freezes_join_the_language_row_within_zero_and_three
```

Manifest additions: `crates/streaks/src/lapse.rs` (changed: `open_lapse` walks a counted range);
`crates/streaks/tests/open_lapse_bound.rs` (added: A41); `crates/bot/tests/streak_commands.rs`
(changed: A42).

## 18. Amendment, 2026-09-30: R20, R21 and R23 as this delivery serves them

Insert-only: every earlier byte is kept in order, and this section is the only insertion.

- The calendar is not in this delivery. `GET /api/streak` serves no study days and no freeze, skip
  or break markers, and the streak screen draws no calendar. This supersedes R20's "the window's
  study days with their freeze, skip and break markers", R23's "the calendar with its markers", and
  the words "the calendar" in section 4's row for `web/app/src/lib/streak/StreakScreen.svelte`. The
  calendar is #486.
- The relight count is served by `GET /api/governor` as `relight_cards`, during a lapse only, and
  not by `GET /api/streak`. This supersedes R20's last sentence.
- No route serves a reason string for a disarmed governor. `GET /api/governor` serves the verdict,
  `standby`, `lapse` and `lapse_since`, and the streak screen states the reason from them. That is
  how R21's "why the governor is disarmed when it is" is met.

## 19. Amendment, 2026-09-30: the relight's celebration is due in the grant's own write, and five sentences are corrected

Insert-only: every earlier byte is kept in order, and this section is the only insertion.

R27 (restated). The relight's celebration is due in the grant's own write. The streak step records
    the relight's study day in `relight_due` in the fold's per-day write that grants the relight
    (R26), so the day is due exactly when that write commits, and a write that rolls back leaves no
    day due. After the fold's commit, the cycle reads every day still due, oldest first, and
    routes each under R27's kind and key (`celebration`, `relight:<epoch day>`). Once the router has
    decided a day, as sent or as already sent, the day leaves the list. A day whose route fails
    stays due, and so does a day whose route a restart interrupts, so the next cycle routes it. This
    supersedes R27's answer held in memory and its sentence on crash recovery: a day answered before
    its write committed could be celebrated after that write rolled back, and a day held in memory
    was lost at a restart.
R28. Over a fold that fails at any step after the streak step, a later sync in which the day still
    qualifies or no longer qualifies, and a restart between any two steps: at most one celebration
    is sent for each relight day (S1); no celebration is sent for a day whose grant did not commit
    (S2); and every committed grant is celebrated while recomputes keep running (L1). The proof of
    the three is #477.
R29. The migration `migrations/007602_streaks_relight_due.sql` creates `relight_due`, `STRICT` with
    `created_at` (SPEC-020 R15, R18). It is exported and erased, registered in the context map's
    ownership register, declared in `privacy.json` and listed in the streaks data-rights port. This
    adds to R24.
R30. R20's `none` for a day that already has a study review is served: the streak view, and so
    `GET /api/streak`, answers `none` for a track whose last study day is the open study day,
    whatever its run and freezes (A49).
R31. Every value `GET /api/streak` and `GET /api/governor` serve is judged, by the api's route test
    and by the web reader's test, over one generated population in which each pair of served values
    differs in some member, so a route or a reader that puts one served value in another's place is
    red (A47, A48). The bot's reply is judged the same way by A42.
R32. The silence walk, the law bridge and the law replay are defined on the study days an instant
    maps to, whose epoch day numbers lie within 2^37 of the epoch. The silence walk steps at most
    `SILENCE_WALK_CAP_DAYS + 1` days below today, and the bridge and the replay one day, so none of
    them leaves the day type there. A today closer than that to the smallest epoch day is outside
    their domain, and each function's doc says so; `open_lapse` alone answers there (section 16).
    The proof of the walks' arithmetic is #478.

- Section 10's last bullet names "the two `lapse.rs` files". This delivery changes only
  `crates/streaks/src/lapse.rs`; `crates/coordination/src/lapse.rs` is read and unchanged.
- `open_lapse`'s doc now states its one exception: a walk that reaches the smallest epoch day
  without meeting a study review answers no lapse (section 16, A41).
- A30's named mutant, the heat's `days >= *threshold` replaced by `days > *threshold`, matches two
  sites. The failure the red-first record quotes is `heat_tier`'s; `heat_for`'s mutant passes A30
  and fails A3. The record's addendum of this date names both.
- A33's store population gains the due list. `put_relight_due`, `relight_due` and
  `clear_relight_due` are called only from coordination, and a mutant is judged by its own
  package's tests, so `store_effects` reads each back with a raw query: a day is kept at its first
  write, the list is read oldest first, a cleared day leaves it, and a write that rolls back leaves
  no day due.

Manifest additions: `migrations/007602_streaks_relight_due.sql` (added: R29);
`crates/streaks/src/store.rs` (changed: the due list is written, read and cleared);
`crates/streaks/src/data_rights.rs` (changed: the due list is exported and erased);
`crates/coordination/src/recompute/streaks.rs` (changed: the step records the due day in the
grant's write); `crates/coordination/src/relight.rs` and `crates/coordination/src/sync_cycle.rs`
(changed: the cycle routes the stored due days and clears each decided one);
`crates/coordination/src/streak_views.rs` (changed: R30); `crates/streaks/src/governor.rs`,
`law.rs`, `replay.rs` and `lapse.rs` (changed: their docs state the domain of R32 and section 16);
`crates/coordination/tests/relight_order.rs` (added: A44 to A46);
`crates/coordination/tests/relight_settle.rs`, `crates/coordination/tests/data_rights_symmetry.rs`
and `crates/streaks/tests/streak_rights.rs` (changed: the stored due list);
`crates/streaks/tests/store_effects.rs` (changed: A33 reads the due list back);
`crates/coordination/tests/streak_views.rs` (changed: A49); `crates/api/tests/streak_routes.rs`
(changed: A47);
`web/app/src/lib/streak/streak-served-population.test.ts` (added: A48); `privacy.json`,
`PRIVACY.md` and `docs/CONTEXT-MAP.md` (changed: R29); the offline query cache (five queries
added); the rows `S07617` to `S07621`; and `docs/red-first/SPEC-076.md` (changed: the addendum of
this date).

## 20. Acceptance criteria of the 2026-09-30 relight-order amendment

| id | criterion | decided by |
|---|---|---|
| A44 | a fold that fails after the relight's grant, followed by a sync in which the day no longer qualifies, sends no celebration | `a_celebration_is_sent_only_for_a_grant_that_committed` |
| A45 | a restart between the fold's commit and the route leaves the day due, and the next cycle sends its one celebration | `a_crash_between_the_commit_and_the_route_is_recovered_at_the_next_cycle` |
| A46 | over every failure point, later-sync outcome and restart, each committed grant is celebrated once and no other day is | `every_failure_point_later_sync_and_crash_keeps_one_celebration_per_committed_grant` |
| A47 | the streak and governor routes serve each value in its own place, over a generated population in which each pair of served values differs | `every_served_value_is_read_where_each_pair_of_served_values_differs` |
| A48 | the web reader reads each served value into its own place, over the same population | `web/app/src/lib/streak/streak-served-population.test.ts` |
| A49 | the view puts nothing at stake on a track whose open study day has a study review, over every run, freeze count and studied flag of both tracks | `a_studied_day_puts_nothing_at_stake_on_either_track` |

```acceptance
A44: cargo test -p deck-streak-coordination --test relight_order -- --exact a_celebration_is_sent_only_for_a_grant_that_committed
A45: cargo test -p deck-streak-coordination --test relight_order -- --exact a_crash_between_the_commit_and_the_route_is_recovered_at_the_next_cycle
A46: cargo test -p deck-streak-coordination --test relight_order -- --exact every_failure_point_later_sync_and_crash_keeps_one_celebration_per_committed_grant
A47: cargo test -p deck-streak-api --test streak_routes -- --exact every_served_value_is_read_where_each_pair_of_served_values_differs
A48: pnpm exec vitest run web/app/src/lib/streak/streak-served-population.test.ts
A49: cargo test -p deck-streak-coordination --test streak_views -- --exact a_studied_day_puts_nothing_at_stake_on_either_track
```

## 21. Amendments: the files the relight-order amendment adds

The relight-order amendment (sections 18 to 20) adds three files that no row of section 4 names:
`crates/coordination/tests/relight_order.rs` (added: A44 to A46),
`migrations/007602_streaks_relight_due.sql` (added: R29, the relight's due day stored in the
grant's own write) and `web/app/src/lib/streak/streak-served-population.test.ts` (added: A48).
