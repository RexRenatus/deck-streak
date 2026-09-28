# SPEC-086: today is one snapshot, read the same by the home screen and the bot

- **Wave:** W3. **Issue:** #69 (epic #4). **Context(s):** `deck-streak-coordination` (the today
  view, a read model joining the owners' ports); `deck-streak-api` (`GET /api/today`);
  `deck-streak-bot` (`/today`); the Mini App (`web/app`, the home screen's hero).
- **Decided by:** ADR-002 (a read model that joins contexts lives in coordination and holds none of
  their rules), ADR-005 (the Mini App), ADR-012 (the parity oracle proves the snapshot) and ADR-071
  (the current study day is evaluated as far as it has gone).
- **Prerequisites:** SPEC-071 (the rollup, the score and the grade), SPEC-072 (the level, its
  title and the Ascendant buff), SPEC-073 (the next milestone), SPEC-076 (the streaks and the
  governor), SPEC-077 (the courses' bands and the law block), SPEC-082 (the coin balance), SPEC-051
  (the Today screen and its readings cards), SPEC-024 (the owner's session), SPEC-026 (the bot's
  command table). **Mutation band:** `S08600-S08699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-086.md` (ADR-016).

## 1. The problem, measured

- **No snapshot exists.** At `dev` c3d769b the Mini App's Today (`web/app/src/routes/+page.svelte`)
  shows no game state, `crates/coordination/src/` has no today view, and the bot has no `/today`.
  Every value the snapshot carries is owned by another W3 SPEC; this one composes them, which is
  why it is built last.
- **What is ported** (predecessor `27ee2bc`): the snapshot
  (`pipeline_layers/read_api.py:ReadApiLayer.snapshot`), which backs the predecessor's `/today`
  (`bot.py:CommandBot.handle_command('today')`) and its read-only dashboard; and the rule that the
  law block leads `/today` whenever the law track is active, which is the predecessor's
  `telegram.py:render_law_block` rendering a block at all (it renders nothing unless the law track
  has lifetime XP, a streak, XP today or an active leech).
- **Traps a hand port falls into.**
  - `minutes_today` and each course's `mastery_pct` are Python's `round(x, 1)`, which rounds the
    exact binary value of `x`: a value that looks like a tie rounds by what the float really is. A
    port that multiplies by ten and rounds disagrees on those values; the golden carries them.
  - The predecessor's law block names the owner's law deck in its heading; DeckStreak's block names
    no deck, so the golden of the law-first rule records only whether the block renders, never its
    text.
  - The snapshot's day is the server's study day: one minute before the rollover it is still the
    previous day.
- **Corrections to the issue.** #69 lists `true_retention` among the snapshot's keys; the
  predecessor returns 0.0 for a day with no answered review, and DeckStreak renders it as null
  (SPEC-071 R10). The home-screen notes of #73 (the Ascendant chip), #76 (the next milestone card),
  #83 (the governor's state) and #84 (a welcome-back line after a lapse) are composed here, on the
  one home screen, from the data their SPECs serve.
- **What the parity oracle proves.** The snapshot's keys and values for seeded synthetic state, the
  one-place rounding with its tie class, and the law block's presence over synthetic law payloads.
- **Prerequisites.** Listed in the header.

## 2. Requirements

R1. Coordination's today view (`crates/coordination/src/today/view.rs`) composes one snapshot for
    the current study day from each owner's port: analytics' score, grade, reviews, minutes and
    retention (SPEC-071); progression's level, title, emoji, total XP, XP into the level and XP
    for the next level (SPEC-072); streaks' language streak, heat, freezes and longest streak
    (SPEC-076); economy's coin balance (SPEC-082); and each course's name, flag, current band and
    mastery (SPEC-077). It holds none of their rules, and it writes no table.
R2. The snapshot's keys and values equal the golden of
    `pipeline_layers/read_api.py:ReadApiLayer.snapshot` for every case: `day`, `score`, `grade`,
    `reviews_today`, `true_retention`, `minutes_today`, `streak`, `streak_heat`, `freezes`,
    `longest_streak`, `level`, `level_title`, `level_emoji`, `total_xp`, `xp_into_level`,
    `xp_for_next`, `coins` and `languages`, each course with its `name`, `flag`, `current_band` and
    `mastery_pct`. `minutes_today` and `mastery_pct` are rounded to one place as the predecessor
    rounds them (`round(x, 1)` on the exact binary value). A day with no answered review carries a
    null retention (SPEC-071 R10).
R3. The day is the server's study day under the configured rule (CHARTER 7): the API renders it as
    its ISO date (SPEC-020 R5), and no surface derives it from the device's clock.
R4. The law block (SPEC-077) leads the snapshot whenever the law track is active, and is omitted,
    never rendered with zeros, otherwise. The law track is active exactly when the golden
    `law_block_present` proves the predecessor's `telegram.py:render_law_block` renders a block:
    when the track has lifetime XP, a streak, XP today or an active leech. The block names no deck.
R5. `GET /api/today` serves the snapshot and, when the law track is active, the law block, to the
    owner's session only (SPEC-024).
R6. `/today` answers with the numbers `GET /api/today` returns for the same study day, the law
    block first when the law track is active, and one button that opens the Mini App's home.
    `/today` keeps its name; the predecessor has no `/status`.
R7. The Mini App's home (`/`, Today) gains a hero above SPEC-051's readings cards: the score ring
    with its grade, the streak flame with its heat, the level bar, the coin balance, and the law
    block first when the law track is active. The hero also shows, from their SPECs' reads, the
    Ascendant chip on an Ascendant day (SPEC-072, #73), the next milestone card (SPEC-073, #76),
    and the governor's state, with a welcome-back line on the first day after a lapse (SPEC-076,
    #83, #84). It shows the snapshot's study day, never the device's date.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the snapshot's keys and values equal the golden of `ReadApiLayer.snapshot` for every case (examined count reported, zero refused) | `the_snapshot_matches_the_predecessors_golden` |
| A2 | `minutes_today` and `mastery_pct` round to one place as the predecessor rounds, on the golden's tie class | `the_one_place_rounding_matches_the_predecessor_on_ties` |
| A3 | the law block leads the snapshot when the law track is active, and is omitted, never zeroed, otherwise, as the golden `law_block_present` decides | `the_law_block_leads_when_law_is_active_and_is_omitted_otherwise` |
| A4 | with the clock at one minute before and at the rollover, the snapshot's day is the previous and then the new study day | `the_snapshot_day_is_the_servers_study_day` |
| A5 | the today view names no table in a query (a census over `crates/coordination/src/today/`, examined count reported, with a planted fixture it refuses) | `the_today_view_writes_no_table` |
| A6 | `GET /api/today` serves every snapshot key, with an absent retention as null | `the_today_route_serves_every_snapshot_key` |
| A7 | `GET /api/today` answers only the owner's session: 401 or 403 and no data otherwise | `the_today_route_answers_only_the_owner` |
| A8 | `/today` reports the numbers `GET /api/today` returns for the same study day | `today_reports_the_numbers_the_today_route_returns` |
| A9 | `/today` puts the law block first when the law track is active, and carries one button into the Mini App | `today_puts_the_law_block_first_and_offers_the_mini_app` |
| A10 | the home hero puts the law block first when the law track is active | `puts the law block first when law is active` |
| A11 | the home hero shows the snapshot's study day, never the device's date | `shows the server's study day, not the device's date` |

```acceptance
A1: cargo test -p deck-streak-coordination --test today_view -- --exact the_snapshot_matches_the_predecessors_golden
A2: cargo test -p deck-streak-coordination --test today_view -- --exact the_one_place_rounding_matches_the_predecessor_on_ties
A3: cargo test -p deck-streak-coordination --test today_view -- --exact the_law_block_leads_when_law_is_active_and_is_omitted_otherwise
A4: cargo test -p deck-streak-coordination --test today_view -- --exact the_snapshot_day_is_the_servers_study_day
A5: cargo test -p deck-streak-coordination --test today_view -- --exact the_today_view_writes_no_table
A6: cargo test -p deck-streak-api --test today_routes -- --exact the_today_route_serves_every_snapshot_key
A7: cargo test -p deck-streak-api --test today_routes -- --exact the_today_route_answers_only_the_owner
A8: cargo test -p deck-streak-bot --test today_commands -- --exact today_reports_the_numbers_the_today_route_returns
A9: cargo test -p deck-streak-bot --test today_commands -- --exact today_puts_the_law_block_first_and_offers_the_mini_app
A10: pnpm exec vitest run web/app/src/lib/today/TodayHero.test.ts -t "puts the law block first when law is active"
A11: pnpm exec vitest run web/app/src/lib/today/TodayHero.test.ts -t "shows the server's study day, not the device's date"
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. No pack's state changes for this delivery: the
accessibility pack stays enforced, the ux-laws pack keeps its current state, and no row is
deferred for it.

| id | criterion | decided by |
|---|---|---|
| B1 | over `web/app/src/routes/+page.svelte` and `web/app/src/lib/today/`: the home screen passes in both of Telegram's colour schemes, the score ring and the level bar carry their values as text, and the law block's order is the order a screen reader reads | the accessibility pack |
| B2 | over the copy in `web/app/src/lib/today/` and `crates/bot/src/today_commands.rs`: no loss framing of the streak, no countdown or urgency, and no guilt in the welcome-back line | the ux-laws pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/coordination/src/today/mod.rs` | `deck-streak-coordination` | added |
| `crates/coordination/src/today/view.rs` | `deck-streak-coordination` | added: the snapshot, the one-place rounding, the law-first order |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the today module |
| `crates/coordination/tests/today_view.rs` | `deck-streak-coordination` | added: A1 to A5 |
| `crates/api/src/today_routes.rs` | `deck-streak-api` | added: `GET /api/today` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the today route behind the owner's session |
| `crates/api/tests/today_routes.rs` | `deck-streak-api` | added: A6, A7 |
| `crates/bot/src/today_commands.rs` | `deck-streak-bot` | added: the /today command and its button |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain the /today command |
| `crates/bot/tests/today_commands.rs` | `deck-streak-bot` | added: A8, A9 |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the today view's ports, joined in the composition root |
| `web/app/src/routes/+page.svelte` | miniapp | changed: Today gains the hero above the readings cards |
| `web/app/src/lib/today/TodayHero.svelte` | miniapp | added: the score ring, the streak flame, the level bar, the coins and the chips |
| `web/app/src/lib/today/LawBlock.svelte` | miniapp | added: the law block |
| `web/app/src/lib/today/today.ts` | miniapp | added: the today route's client and types |
| `web/app/src/lib/today/TodayHero.test.ts` | miniapp | added: A10, A11 |
| `tools/parity-oracle/registry/spec_086.py` | repo | added: this SPEC's registrations |
| `tools/parity-oracle/goldens/today_snapshot.json` | repo | added: the golden of `pipeline_layers/read_api.py:ReadApiLayer.snapshot` (adapter) |
| `tools/parity-oracle/goldens/law_block_present.json` | repo | added: the golden of `telegram.py:render_law_block`'s presence (adapter) |
| `scripts/mutation-rows.d/S08600-S08699.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-086-today-is-one-snapshot-for-the-home-screen-and-the-bot.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-086.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It serves no HTML dashboard: SPEC-025 excluded it, and the Mini App replaces it (#21).
- It serves no agent read tool for the snapshot (#157).
- It pins no daily widget (#121).
- It sends no morning brief, and no Ascendant line in one (#122).
- It adds no block to the daily digest (#129).
- It draws no chart on the home screen (#152).

## 6. Risks

- **The one-place rounding disagrees on a value that looks like a tie.** Detected by A2, whose
  golden class carries such values, and by the row `S08601-MINUTES-ONE-PLACE`.
- **A composed value drifts from its owner's own screen.** Each value is read through its owner's
  port, never recomputed here (A5), and A8 holds the bot and the API to one reading.
- **The law block renders with zeros on a day the law track is idle.** Detected by A3 and A10.
- **The device's date leaks into the hero near the rollover.** Detected by A4 and A11.
- **A prerequisite SPEC lands after this one.** The build order puts this SPEC last; a missing port
  fails the composition root's build, never the snapshot silently.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_086.py` (SPEC-029), generated on the owner's
checkout of the predecessor at `27ee2bc`.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `today_snapshot` | `pipeline_layers/read_api.py:ReadApiLayer.snapshot` | adapter | a stand-in layer whose stub store returns the case's rollup, total XP, streak state, coin balance and course progress, with the layer's current day and heat refresh patched to the case's; it returns the snapshot with the day as its epoch day and the grade as a label and an emoji. Class: `tie` (minutes and mastery that look like ties) |
| `law_block_present` | `telegram.py:render_law_block` | adapter | the case's synthetic law payload (lifetime XP, streak, XP today, active leeches, dues, mastery); it returns only whether a block renders, never its text, because the predecessor's heading names the owner's law deck. Class: `idle` |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S08601-MINUTES-ONE-PLACE` | `crates/coordination/src/today/view.rs` | the minutes round to one place as the predecessor rounds | `today_view::the_one_place_rounding_matches_the_predecessor_on_ties` |
| `S08602-MASTERY-ONE-PLACE` | `crates/coordination/src/today/view.rs` | each course's mastery rounds to one place as the predecessor rounds | `today_view::the_snapshot_matches_the_predecessors_golden` |
| `S08603-LAW-LEADS` | `crates/coordination/src/today/view.rs` | the law block comes first when the law track is active | `today_view::the_law_block_leads_when_law_is_active_and_is_omitted_otherwise` |
| `S08604-SNAPSHOT-KEY-NAMES` | `crates/api/src/today_routes.rs` | the serialized key names of the snapshot | `today_routes::the_today_route_serves_every_snapshot_key` |
| `S08605-TODAY-OWNER-ONLY` | `crates/api/src/today_routes.rs` | the route requires the owner's session | `today_routes::the_today_route_answers_only_the_owner` |
