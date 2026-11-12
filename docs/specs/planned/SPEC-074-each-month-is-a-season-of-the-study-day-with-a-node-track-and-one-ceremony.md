# SPEC-074: each month is a season of the study day, with a node track and one ceremony

- **Wave:** W3. **Issue:** #77, #78 (epic #4). **Context(s):** `deck-streak-progression` (the chapter
  of a study day, season XP, prestige, the node target and nodes, `season_nodes`, `season_targets`);
  `deck-streak-coordination` (the recompute's season step: node payouts through economy and streaks,
  the ceremony through the router, the season view); `deck-streak-api`, `deck-streak-bot` and the
  Mini App (the season screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-041 (every celebration passes
  through the one router), ADR-071 (the recompute reaches every study day once, in order), ADR-072
  (season XP sums both XP tables) and ADR-074 (a season is the calendar month of the study day).
- **Prerequisites:** SPEC-040 (the XP ledger), SPEC-041 (the router), SPEC-071 (the fold and its step
  order), SPEC-072 (the XP settlement), SPEC-076 (streaks' freeze grant port), SPEC-080 (crown days
  and race results), SPEC-082 (economy's coin credit) and SPEC-084 (the ceremony's T5 and the nodes'
  tiers). **Mutation band:** `S07400-S07499`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-074.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` c3d769b `crates/progression/src/` holds only `lib.rs`; SPEC-040
  adds the XP ledger and SPEC-072 the XP settlement. No chapter, prestige title, node target or
  ceremony exists, and the season screen the issues describe has no route.
- **What is ported.** The predecessor's `seasons-prestige-chapters` (#77):
  `gamification/seasons.py:season_period`, `prestige_title` and `season_progress`, and
  `pipeline_layers/showcase.py:ShowcaseLayer._month_ceremony`; and its `season-node-track` (#78):
  `gamification/seasons.py:node_target` and `nodes_crossed`, and
  `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes`, with the claim-then-grant of
  `database.py:GamifyStore.claim_season_node` and `coin_ref_exists`.
- **Two boundaries in the predecessor, one here.** The predecessor keys its node track, its season XP
  (`database.py:GamifyStore.get_month_xp`, a sum over ledger rows keyed by study day) and its ceremony
  (which runs when the study day is the first of a month) on the study day's month, and names its
  season view with `season_period`, the local calendar month at midnight. Between local midnight and
  the rollover on a month's first day the view names a month the study day has not reached, and the
  predecessor's own bot hides the node bar there. DeckStreak reads the study day's month everywhere
  (ADR-074, CHARTER 7).
- **Traps a hand port falls into.**
  - `season_period` returns a `YYYY-MM` string, which no golden may carry (SPEC-029 R3): its golden
    records the year and the month as integers.
  - `node_target` takes `int(statistics.median(...))` of the trailing chapters, clamps only when the
    previous target is positive, and applies the floor after the clamp, so the floor always wins.
  - A node never un-crosses: the claim table, not the season XP, remembers a crossing, and a claim
    whose coins were never written is paid at the next evaluation.
  - The chapter title's index is the closed month's `(year × 12 + month) mod 12`; its golden returns
    the index, and the titles are held to the constants golden.
  - Under one sync a study day (ADR-037), the predecessor's "first study day of the month" would be
    missed after downtime; the fold of ADR-071 reaches that day at its settle instead.
- **Corrections to the issues.**
  - #77 says the ceremony fires "on the first non-quiet sync of a new month". The predecessor raises
    it at the first study day of a month (`_month_ceremony` returns unless `today.day == 1`), sends its
    text through the ladder, which holds it in quiet hours, and gates only the keepsake photo on quiet
    hours. Here the ceremony is raised when the fold reaches that day, the router defers it in quiet
    hours, and there is no photo (#126).
  - #77 says season XP is the sum over "the calendar month". It is the sum over the study days of the
    month, the predecessor's own `get_month_xp` domain.
  - #78's freeze drop count counts the reasons chest, weekly quest and season together
    (`database.py:GamifyStore.freeze_drops_in_month`), not the season's drops alone.
- **What the parity oracle proves.** The prestige title at each threshold, the season period as a year
  and a month on both sides of local midnight and of the rollover, the node target over empty, odd
  and even trailing lists with and without a previous target, the crossed count at every boundary,
  one whole node-track evaluation with its payouts, and one whole ceremony, from the predecessor's own
  functions.
- **Prerequisites.** SPEC-040, SPEC-041, SPEC-071, SPEC-072, SPEC-076, SPEC-080, SPEC-082 and
  SPEC-084, as the header names them. The ceremony's counts come from SPEC-080's tables, and its
  tier from SPEC-084's ladder.

## 2. Requirements

Seasons, prestige and the ceremony (#77)

R1. A chapter is the calendar month of a study day (ADR-074): the proleptic Gregorian year and month
    of the kernel's `StudyDay`. Its number is `year × 12 + month − 1`, which keys every stored row and
    every dedupe key, so no key carries a calendar string. The current chapter is the chapter of the
    current study day. For every case of `goldens/season_period.json` outside the class
    `midnight-to-rollover`, the chapter equals the golden's year and month; inside it, the chapter is
    the study day's, the previous month.
R2. A chapter's season XP is the sum of the XP of its study days over SPEC-040's `xp_ledger` and
    SPEC-072's `xp_settlement`, both tracks, the predecessor's `database.py:GamifyStore.get_month_xp`
    domain. Lifetime XP is the sum over both tables and never resets.
R3. The prestige title follows lifetime XP: below 10,000 Novice, below 50,000 Adept, below 150,000
    Expert, below 500,000 Master, else Grandmaster, each with its emoji, and a negative total in the
    lowest tier (`goldens/prestige_title.json`, `gamification/seasons.py:prestige_title`; the four
    thresholds `gamification.seasons.PRESTIGE_NOVICE` to `PRESTIGE_MASTER` in
    `goldens/seasons.constants.json`).
R4. When the fold (ADR-071) reaches the first study day of a month, as the current day or as a
    closed day at its settle, coordination raises one celebration occasion for the chapter that
    closed: event `ceremony` (T5 by the ladder, SPEC-084, and not budget-exempt), dedupe key
    `chapter:<number>`, delivered once ever through the router (SPEC-041) and deferred in quiet hours.
    It states the closed chapter's season XP, its crown days (quests' crown days of its study days,
    SPEC-080), its ghost wins (the won results of the weeks whose Monday falls in the chapter, among
    the six most recent race results, SPEC-080) and the chapter title at the index the golden gives,
    all equal to `goldens/chapter_ceremony.json`.
R5. The twelve chapter titles are the predecessor's copy in its order (`constants.CHAPTER_TITLES` in
    `goldens/seasons.constants.json`), held in one module, `crates/progression/src/chapter.rs`, which a
    test holds equal to the golden.
R6. A second recompute that reaches the same first study day raises nothing new (the router answers
    `already_recorded`), and a chapter that closed during downtime is closed when the fold settles its
    successor's first study day.

The season node track (#78)

R7. A chapter's node target is computed once, at the chapter's first evaluation, and stored in
    `season_targets`; every later evaluation reads it, whatever the trailing XP becomes. It is
    `node_target` over the season XP of the three previous chapters (most recent first) and the
    previous chapter's stored target when that is positive: floored at 5,000, clamped to ±20% of the
    previous target, and floored again (`goldens/node_target.json`,
    `gamification/seasons.py:node_target`; `constants.SEASON_NODE_TARGET_FLOOR` and
    `constants.SEASON_NODE_GROWTH_CLAMP` in `goldens/seasons.constants.json`).
R8. The crossed count is `nodes_crossed(season XP, target, 10)`: at most ten, and none when the
    target or the season XP is not positive (`goldens/nodes_crossed.json`,
    `gamification/seasons.py:nodes_crossed`; `constants.SEASON_NODE_COUNT`).
R9. Each crossed node not yet claimed is claimed once in `season_nodes`, whose key (chapter, node)
    lives in the migration, and pays coins through economy's once-ever credit (SPEC-082's
    `credit_once`): 10 per node and 30
    for node 10 (`economy.json` `coins.earn.season_node` and `season_node_final`, equal to
    `constants.COIN_SEASON_NODE` and `constants.COIN_SEASON_NODE_FINAL`), source `season_node`, ref
    `<chapter>:<node>`, credited at most once per source and ref on any day. A node pays coins, never
    XP, so a payout never raises the season XP that defines the track.
R10. Node 5 (`constants.SEASON_NODE_FREEZE_NODE`) also requests one freeze through streaks'
    `grant_freeze` with the reason `season` (SPEC-076). It is granted only while this month's freeze
    drops by the reasons chest, weekly quest and season are below 1 and the freezes held are below 3
    (`constants.FREEZE_DROP_MONTHLY_CAP`, `constants.STREAK_FREEZE_CAP`), and the claim records whether
    it froze.
R11. A node claimed but not yet credited, after a failure between the claim and the credit, is
    credited at the next evaluation; a node whose credit exists is never credited again; and a node
    never un-crosses: the predecessor's claim-then-grant self-heal.
R12. The crossings of one evaluation raise one celebration naming the top node: event
    `season_node_final` when the top is node 10, else `season_node` (T3 and T2 by the ladder), dedupe
    key `season_node:<chapter>:<top>`. It names the coins that evaluation credited, and a freeze only
    when one was granted.
R13. The node track and the ceremony register one step in phase 7 (awards) of SPEC-071's fold, after
    the derived bonuses and the coin mint: the track reads the day's whole XP, derived bonuses
    included, and pays coins only, as the predecessor evaluates it after its day's recompute, quests
    and chests (`pipeline.py:GamifyPipeline._run_sync_cycle_impl`). The step runs for the chapter of
    every study day the fold reaches, so a chapter's last day is counted before the next chapter's
    target is computed.
R14. One evaluation, its stored target, its crossings, its credits, its freeze and its one
    celebration, equals `goldens/season_nodes_evaluated.json` for every case.

The season screen and the data

R15. `GET /api/season` serves the owner only (SPEC-024): the current chapter's number, year and
    month, its season XP, the prestige title and emoji, lifetime XP, the target, the crossed count, the
    ten nodes each with claimed, coins and froze, and the past chapters, each with its number, title
    index, season XP, crown days and ghost wins, computed on read from the ledgers. Any other caller
    gets 401 or 403 and no data.
R16. The bot's `/season` (SPEC-026's command table) answers with the chapter, season XP, prestige and
    the node bar, and an Open button into the Mini App's season screen, and states the same numbers as
    the route for one study day.
R17. The Mini App's `/season` screen draws the node track as ten nodes, claimed and unclaimed, with
    each node's coins and node 5's freeze, and lists the past chapters; `/season` joins `ROUTES`.
R18. Progression owns `season_nodes` (chapter, node, coins, froze, crossed_at, `created_at`) and
    `season_targets` (chapter, target, `created_at`), both `STRICT`, created by
    `migrations/007401_progression_seasons.sql` (SPEC-020 R15, R18). Each is registered in the context
    map's register of DeckStreak's own tables, declared in `privacy.json` under the category
    `season-track`, given a line in `PRIVACY.md`, listed as exported and erased by progression's
    data-rights port, and seeded in the symmetry test.
R19. Every constant the track and the titles use (the node count, the floor, the clamp, the freeze
    node, both coin amounts, both freeze caps, the four prestige thresholds and the twelve titles) is
    read from `economy.json` where that file declares it, and a test holds each value equal to
    `goldens/seasons.constants.json`.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the prestige title of every case equals the golden of `gamification/seasons.py:prestige_title` (examined count reported, zero refused) | `the_prestige_title_matches_the_predecessors_golden` |
| A2 | the chapter of every case equals the golden of `season_period` as a year and a month, and the study day's month for the `midnight-to-rollover` class | `the_chapter_is_the_month_of_the_study_day` |
| A3 | the node target of every case equals the golden of `gamification/seasons.py:node_target` | `the_node_target_matches_the_predecessors_golden` |
| A4 | the crossed count of every case equals the golden of `gamification/seasons.py:nodes_crossed` | `the_nodes_crossed_match_the_predecessors_golden` |
| A5 | every season constant equals `goldens/seasons.constants.json` and the value `economy.json` declares for it | `the_season_constants_equal_the_predecessors` |
| A6 | a chapter's target is stored at its first evaluation and read unchanged by a later one whose trailing XP differs | `the_target_is_computed_once_per_chapter` |
| A7 | a chapter's season XP sums both XP tables over its study days, both tracks, and no day outside it | `the_season_xp_sums_both_xp_tables_of_the_chapters_study_days` |
| A8 | one evaluation's target, crossings, credits, freeze and celebration equal the golden of `ShowcaseLayer._evaluate_season_nodes` for every case | `the_node_track_pays_as_the_predecessors_golden` |
| A9 | a node is credited once across recomputes and study days, a claimed but uncredited node is credited at the next evaluation, and no node un-crosses | `a_node_is_paid_once_across_recomputes` |
| A10 | node 5's freeze is granted only under both caps, and the celebration names a freeze only when one was granted | `the_node_five_freeze_respects_both_caps` |
| A11 | several crossings in one evaluation raise one celebration naming the top node | `catch_up_crossings_raise_one_celebration` |
| A12 | the next chapter's target counts the settled XP of the previous chapter's last day | `the_next_target_counts_the_previous_chapters_last_day` |
| A13 | the ceremony equals the golden of `ShowcaseLayer._month_ceremony` for every case and is raised once per closed chapter | `the_ceremony_matches_the_predecessors_golden_once_per_chapter` |
| A14 | a ceremony raised inside quiet hours is delivered once, after them | `a_ceremony_in_quiet_hours_is_delivered_once_after_them` |
| A15 | the season route answers the owner and refuses every other caller with no data | `the_season_route_answers_only_the_owner` |
| A16 | `/season` states the same numbers as the route for one study day | `season_shows_the_same_numbers_as_the_route` |
| A17 | the season screen draws claimed and unclaimed nodes, each node's coins and node 5's freeze | `draws claimed and unclaimed nodes with the freeze node marked` |
| A18 | progression's data-rights port lists both season tables as exported and erased, and an erase leaves them empty | `the_season_tables_are_exported_and_erased` |

```acceptance
A1: cargo test -p deck-streak-progression --test seasons_goldens -- --exact the_prestige_title_matches_the_predecessors_golden
A2: cargo test -p deck-streak-progression --test seasons_goldens -- --exact the_chapter_is_the_month_of_the_study_day
A3: cargo test -p deck-streak-progression --test seasons_goldens -- --exact the_node_target_matches_the_predecessors_golden
A4: cargo test -p deck-streak-progression --test seasons_goldens -- --exact the_nodes_crossed_match_the_predecessors_golden
A5: cargo test -p deck-streak-progression --test seasons_goldens -- --exact the_season_constants_equal_the_predecessors
A6: cargo test -p deck-streak-progression --test seasons_store -- --exact the_target_is_computed_once_per_chapter
A7: cargo test -p deck-streak-progression --test seasons_store -- --exact the_season_xp_sums_both_xp_tables_of_the_chapters_study_days
A8: cargo test -p deck-streak-coordination --test seasons_nodes -- --exact the_node_track_pays_as_the_predecessors_golden
A9: cargo test -p deck-streak-coordination --test seasons_nodes -- --exact a_node_is_paid_once_across_recomputes
A10: cargo test -p deck-streak-coordination --test seasons_nodes -- --exact the_node_five_freeze_respects_both_caps
A11: cargo test -p deck-streak-coordination --test seasons_nodes -- --exact catch_up_crossings_raise_one_celebration
A12: cargo test -p deck-streak-coordination --test seasons_nodes -- --exact the_next_target_counts_the_previous_chapters_last_day
A13: cargo test -p deck-streak-coordination --test seasons_ceremony -- --exact the_ceremony_matches_the_predecessors_golden_once_per_chapter
A14: cargo test -p deck-streak-coordination --test seasons_ceremony -- --exact a_ceremony_in_quiet_hours_is_delivered_once_after_them
A15: cargo test -p deck-streak-api --test seasons_routes -- --exact the_season_route_answers_only_the_owner
A16: cargo test -p deck-streak-bot --test seasons_commands -- --exact season_shows_the_same_numbers_as_the_route
A17: pnpm exec vitest run web/app/src/lib/season/NodeTrack.test.ts -t "draws claimed and unclaimed nodes with the freeze node marked"
A18: cargo test -p deck-streak-progression --test seasons_store -- --exact the_season_tables_are_exported_and_erased
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The game-economy, notifications-policy, privacy-gdpr and
accessibility packs stay enforced; no check is deferred or lifted for this delivery, so the private
wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `economy.json`, examining its coin earns and its streak caps: the season node coins and the freeze caps keep the predecessor's values, and a node pays coins, never XP | the game-economy pack |
| B2 | over `notifications-policy.json` and every file under `crates/coordination/src/`: the `ceremony`, `season_node` and `season_node_final` events keep their tiers, and every celebration this delivery raises goes through the one router | the notifications-policy pack |
| B3 | over `privacy.json`, `PRIVACY.md` and `crates/progression/src/data_rights.rs`: the `season-track` category names both tables with its purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |
| B4 | over `web/app/src/routes/season/+page.svelte` and `web/app/src/lib/season/`: the season screen passes the accessibility audit in both Telegram colour schemes | the accessibility pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/progression/src/seasons.rs` | `deck-streak-progression` | added: the chapter of a study day, season and lifetime XP, prestige, the node target and the crossed count |
| `crates/progression/src/chapter.rs` | `deck-streak-progression` | added: the twelve chapter titles and the ceremony's content |
| `crates/progression/src/season_store.rs` | `deck-streak-progression` | added: the repository over `season_targets` and `season_nodes` |
| `crates/progression/src/data_rights.rs` | `deck-streak-progression` | changed: `season_nodes` and `season_targets`, exported and erased |
| `crates/progression/src/lib.rs` | `deck-streak-progression` | changed: the modules above |
| `crates/progression/Cargo.toml` | `deck-streak-progression` | changed only if SPEC-040 left out the golden reader's `serde` and `serde_json` dev-dependencies (SPEC-029 R8) |
| `crates/progression/tests/seasons_goldens.rs` | `deck-streak-progression` | added: A1 to A5 |
| `crates/progression/tests/seasons_store.rs` | `deck-streak-progression` | added: A6, A7, A18 |
| `migrations/007401_progression_seasons.sql` | `deck-streak-progression` | added: both tables, `STRICT`, with `created_at` and their keys |
| `crates/coordination/src/recompute/seasons.rs` | `deck-streak-coordination` | added: the node track and the ceremony, a step of the fold |
| `crates/coordination/src/recompute/mod.rs` | `deck-streak-coordination` | changed: the season step, registered in phase 7 of SPEC-071's fold |
| `crates/coordination/src/seasons/mod.rs` | `deck-streak-coordination` | added: the season view and the past chapters, read from the ledgers |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the season module |
| `crates/coordination/tests/seasons_nodes.rs` | `deck-streak-coordination` | added: A8 to A12 |
| `crates/coordination/tests/seasons_ceremony.rs` | `deck-streak-coordination` | added: A13, A14 |
| `crates/api/src/seasons_routes.rs` | `deck-streak-api` | added: `GET /api/season` |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the season route |
| `crates/api/tests/seasons_routes.rs` | `deck-streak-api` | added: A15 |
| `crates/bot/src/seasons_commands.rs` | `deck-streak-bot` | added: /season |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: the command table and the owner's menu gain /season |
| `crates/bot/tests/seasons_commands.rs` | `deck-streak-bot` | added: A16 |
| `web/app/src/routes/season/+page.svelte` | miniapp | added: the season screen |
| `web/app/src/lib/season/NodeTrack.svelte` | miniapp | added: the node track |
| `web/app/src/lib/season/season.ts` | miniapp | added: the route's client and types |
| `web/app/src/lib/season/NodeTrack.test.ts` | miniapp | added: A17 |
| `web/app/src/lib/routes.ts` | miniapp | changed: /season joins `ROUTES` |
| `docs/CONTEXT-MAP.md` | docs | changed: the register of DeckStreak's own tables gains `season_nodes` and `season_targets` |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | checked: progression's port is registered by SPEC-040, and it is changed only if it is not |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: seeded rows for both tables |
| `privacy.json` | repo | changed: the `season-track` category |
| `PRIVACY.md` | repo | changed: the `season-track` category's line |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `Cargo.lock` | workspace | changed when a manifest above changes |
| `tools/parity-oracle/registry/spec_074.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/prestige_title.json` | repo | added: the golden of `gamification/seasons.py:prestige_title` (function) |
| `tools/parity-oracle/goldens/season_period.json` | repo | added: the golden of `gamification/seasons.py:season_period` (adapter; the year and the month as integers) |
| `tools/parity-oracle/goldens/node_target.json` | repo | added: the golden of `gamification/seasons.py:node_target` (function) |
| `tools/parity-oracle/goldens/nodes_crossed.json` | repo | added: the golden of `gamification/seasons.py:nodes_crossed` (function) |
| `tools/parity-oracle/goldens/season_nodes_evaluated.json` | repo | added: the golden of `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes` (adapter; a stub store) |
| `tools/parity-oracle/goldens/chapter_ceremony.json` | repo | added: the golden of `pipeline_layers/showcase.py:ShowcaseLayer._month_ceremony` (adapter; a stub store and a recording notifier) |
| `tools/parity-oracle/goldens/seasons.constants.json` | repo | added: the constants the track, the ceremony and prestige use (constants) |
| `scripts/mutation-rows.d/S07400-S07499.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-074-each-month-is-a-season-of-the-study-day-with-a-node-track-and-one-ceremony.md` | docs | moved from `docs/specs/planned/` |
| `docs/decisions/ADR-074-the-season-is-the-month-of-the-study-day.md` | docs | changed: accepted |
| `docs/red-first/SPEC-074.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It generates no keepsake image and sends no photo with the ceremony: the keepsake is W7's (#126),
  and its image provider is the owner's decision (#169).
- It renders no tier itself: the ceremony's T5 and the nodes' T2 and T3 renders are the celebration
  ladder's (#120).
- It serves no season through the agent's tools (#157).
- It publishes no chapter, season XP or prestige on the public page (#156).
- It makes no streak share card (#125).
- It imports none of the predecessor's season nodes or targets (#61).

## 6. Risks

- **The chapter titles are product copy held in a golden.** Detected by A5, which holds
  `crates/progression/src/chapter.rs` equal to the constants golden, and by review of the golden's
  `source_commit` against `27ee2bc`.
- **A chapter's last day is not settled before the next chapter's target is computed**, so the target
  reads incomplete XP. Detected by A12, over the fold's step order (ADR-071).
- **A failure between a node's claim and its credit forfeits a payout, or a retry pays twice.**
  Detected by A9, and by the rows on the migration's keys (§9).
- **The two season views disagree during side by side** between local midnight and the rollover on a
  month's first day. Expected by ADR-074, and named by the golden's `midnight-to-rollover` class (A2).
- **A ceremony is raised twice** by two recomputes that reach one first study day. Detected by A13,
  which runs the step twice, and prevented by the router's once-ever dedupe on `chapter:<number>`.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_074.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic; no golden carries a calendar string.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `prestige_title` | `gamification/seasons.py:prestige_title` | function | none: cases below, at and above each threshold, zero and a negative total |
| `season_period` | `gamification/seasons.py:season_period` | adapter | calls it with the case's instant and offset and returns the year and the month parsed from its string; each case also carries a rollover hour for the test, which the adapter does not pass; cases between local midnight and the rollover on a month's first day carry the class `midnight-to-rollover`, beside year-end, mid-month and offset cases |
| `node_target` | `gamification/seasons.py:node_target` | function | none: empty, odd and even trailing lists, a previous target absent, zero and positive, clamps up and down, and the floor over the clamp |
| `nodes_crossed` | `gamification/seasons.py:nodes_crossed` | function | none: zero and negative inputs, each threshold exactly and one XP below, and past the last node |
| `season_nodes_evaluated` | `pipeline_layers/showcase.py:ShowcaseLayer._evaluate_season_nodes` | adapter | a stub store holding the chapter's stored target or none, the three previous chapters' season XP, the previous target, the season XP, the nodes already claimed and credited, the month's freeze drops and the freezes held; it returns the target stored, the crossed count, each node credited with its coins and freeze, and the one celebration's event, top node and whether it names a freeze, with every month key turned into the chapter number |
| `chapter_ceremony` | `pipeline_layers/showcase.py:ShowcaseLayer._month_ceremony` | adapter | a stub store holding the closed chapter's season XP, its crown days and the six most recent race results, a recording notifier, quiet hours off and no image; it returns whether the ceremony fired, the index of the title it chose in `constants.CHAPTER_TITLES`, the season XP, crown days and ghost wins its text states, and its event type, never the title's text |
| `seasons.constants` | `gamification.seasons.PRESTIGE_NOVICE` to `PRESTIGE_MASTER`, `constants.SEASON_NODE_COUNT`, `SEASON_NODE_TARGET_FLOOR`, `SEASON_NODE_GROWTH_CLAMP`, `SEASON_NODE_FREEZE_NODE`, `COIN_SEASON_NODE`, `COIN_SEASON_NODE_FINAL`, `FREEZE_DROP_MONTHLY_CAP`, `STREAK_FREEZE_CAP`, `CHAPTER_TITLES` | constants | none |

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `season_nodes` | `deck-streak-progression` | `migrations/007401_progression_seasons.sql` | its season node claims, one row per claimed node, with the month turned into the chapter number | exported and erased |
| `season_targets` | `deck-streak-progression` | `migrations/007401_progression_seasons.sql` | its per-month node targets, which it kept as settings keys, one row per month | exported and erased |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S07401-TEN-NODES` | `crates/progression/src/seasons.rs` | the node count of ten | `seasons_goldens::the_season_constants_equal_the_predecessors` |
| `S07402-TARGET-FLOOR` | `crates/progression/src/seasons.rs` | the target's floor of 5,000 | `seasons_goldens::the_node_target_matches_the_predecessors_golden` |
| `S07403-GROWTH-CLAMP` | `crates/progression/src/seasons.rs` | the ±20% clamp on the previous target | `seasons_goldens::the_node_target_matches_the_predecessors_golden` |
| `S07404-THE-FLOOR-WINS` | `crates/progression/src/seasons.rs` | the floor applied again after the clamp | `seasons_goldens::the_node_target_matches_the_predecessors_golden` |
| `S07405-PRESTIGE-NOVICE` | `crates/progression/src/seasons.rs` | the first prestige threshold | `seasons_goldens::the_prestige_title_matches_the_predecessors_golden` |
| `S07406-CHAPTER-OF-THE-STUDY-DAY` | `crates/progression/src/seasons.rs` | the chapter read from the study day, never the instant | `seasons_goldens::the_chapter_is_the_month_of_the_study_day` |
| `S07407-FINAL-NODE-COINS` | `economy.json` | node 10's 30 coins | `seasons_goldens::the_season_constants_equal_the_predecessors` |
| `S07408-FREEZE-AT-NODE-FIVE` | `crates/coordination/src/recompute/seasons.rs` | the freeze requested at node 5 only | `seasons_nodes::the_node_five_freeze_respects_both_caps` |
| `S07409-ONE-TARGET-PER-CHAPTER` | `migrations/007401_progression_seasons.sql` | the key on `season_targets.chapter` (a script row; the cargo killer) | `seasons_store::the_target_is_computed_once_per_chapter` |
| `S07410-ONE-CLAIM-PER-NODE` | `migrations/007401_progression_seasons.sql` | the key on `season_nodes (chapter, node)` (a script row; the cargo killer) | `seasons_nodes::a_node_is_paid_once_across_recomputes` |
