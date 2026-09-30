# Context map

DeckStreak's bounded contexts, the edges between them, the one name each concept has, and who
owns every table. This document is **binding** (the ddd pack, ADR-002). The crate graph IS the
map: each Rust context is one Cargo package, so an import the map does not allow is a compile
error, and the box-run ddd probe holds every manifest equal to the fence below in both
directions. A change that needs an edge the map lacks is a design question, answered by an ADR
that amends this file in the same change. Never add an edge to make code compile.

## The map

```context-map
deck-streak-kernel        (shared kernel: ids, study day, track, verdict, gate class, errors, config, Db, data-rights port)  depends on: nothing
deck-streak-ingest        (anti-corruption layer for Anki: sync, read, change gate, new-card queue, skip day)  depends on: kernel
deck-streak-identity      (Telegram initData, the owner allow-list, sessions, linked sign-in)  depends on: kernel
deck-streak-analytics     (daily rollup, per-language stats, five-pillar score, today snapshot)  depends on: kernel, ingest
deck-streak-progression   (XP ledger and grant port, levels, tiers, badges, records, seasons)  depends on: kernel, ingest
deck-streak-streaks       (language and law streaks, freezes, governor, relight)  depends on: kernel
deck-streak-curriculum    (Road to C2, CEFR, strands, goal, memory, Can-Do, law track, leeches)  depends on: kernel, ingest
deck-streak-economy       (coin wallet, the only confiscable stake, loss cap, shop)  depends on: kernel
deck-streak-quests        (daily and weekly quests, chests and pity, tokens, smoke bombs, ghost race)  depends on: kernel, ingest
deck-streak-habits        (minutes logs, writing confirmation, habit board, habit analytics)  depends on: kernel
deck-streak-focus         (focus timer, deep-work accounting)  depends on: kernel
deck-streak-discipline    (opt-in commitment devices)  depends on: kernel, ingest
deck-streak-markets       (self-prediction markets, Oracle ladder)  depends on: kernel
deck-streak-notifications (the one router for bot and Mini App, the ladder, budgets, quiet hours)  depends on: kernel
deck-streak-readings      (the daily pre-study readings)  depends on: kernel, ingest
deck-streak-vault         (anti-corruption layer for the second-brain vault)  depends on: kernel
deck-streak-agent         (AI duties through the headless runner, personas, the output gate)  depends on: kernel
deck-streak-insights      (research instruments, chart series)  depends on: kernel, ingest
deck-streak-publishing    (scrubbed public achievement export)  depends on: kernel
deck-streak-privacy       (export, erase and purge over every context's data-rights port)  depends on: kernel
deck-streak-coordination  (use cases and scheduled jobs across contexts)  depends on: kernel, ingest, identity, analytics, progression, streaks, curriculum, economy, quests, habits, focus, discipline, markets, notifications, readings, vault, agent, insights, publishing, privacy
deck-streak-api           (axum HTTPS adapter for the Mini App)  depends on: kernel, identity, notifications, coordination
deck-streak-bot           (Telegram Bot API adapter)  depends on: kernel, identity, notifications, coordination
deck-streak-daemon        (composition root: the deckstreakd binary)  depends on: kernel, ingest, identity, analytics, progression, streaks, curriculum, economy, quests, habits, focus, discipline, markets, notifications, readings, vault, agent, insights, publishing, privacy, coordination, api, bot
deck-streak-migration     (one-off import of v9's schema 24, planned)  depends on: kernel, analytics, progression, streaks, curriculum, economy, quests, habits, focus, discipline, markets, notifications, readings, vault
miniapp   web/app/src     (the SvelteKit Mini App)  depends on: nothing internal
landing   web/site/src    (the Astro landing page, planned)  depends on: nothing internal
```

`deck-streak-migration` and `landing` are PLANNED: they have no code yet, so the probe counts
them and judges none of their edges until the wave that builds them (W8 and W7).

## How the contexts relate

The graph has three layers, and an edge only ever points down.

1. **The shared kernel** (`kernel`) admits what two contexts that may not depend on each other
   both need: identifiers, the study day and its 04:00 rollover calendar, the track (language or
   law), the `Verdict` type, the error type, typed configuration and credentials, the clock, the
   SQLite repository base, and the data-rights port every stateful context implements. Domain
   logic placed in the kernel for convenience is a context hiding in shared code.
2. **The domain contexts** depend on the kernel, and on `ingest` where they compute from Anki
   reviews, cards or queues. No domain context depends on another domain context. What crosses
   between them (an XP grant after a reading is marked read, a coin fine after a broken
   contract, a celebration after a level-up) is a use case of `coordination`, which calls each
   context in order and holds none of their rules.
3. **The adapters and the root.** `api` and `bot` translate a Mini App request or a Telegram
   update into a `coordination` use case, and each implements the notification router's
   transport port for its surface. `daemon` is the composition root: it builds every context's
   adapters, joins ports across contexts (a trait from one context implemented for a type of
   another lives in `crates/daemon/src/wiring.rs`, behind a newtype, because Rust's orphan rule
   makes the root the only crate that can hold it), and nothing depends on it.

**Anti-corruption layers.** Four contexts own a foreign model so the rest of the workspace never
learns it:

| context | foreign model it translates | what it publishes |
|---|---|---|
| `ingest` | Anki's collection schema, its sync protocol, its unicase and protobuf quirks | reviews, cards, notes, decks, the day's new-card queue |
| `vault` | the second brain's file contract (paths, frontmatter, rails, temp names) | staged duty runs, reading archive writes, drill and inbox records |
| `identity` | Telegram's `initData` and user object | the caller, and whether they are the owner |
| `agent` | the headless Claude Code runner and its output | gated duty outputs, or a fail-closed verdict |

**Upstream suppliers.** Three domain contexts are suppliers to many use cases: `progression`
(the idempotent XP grant port), `economy` (the only confiscable stake) and `notifications` (the
one router). They are reached through `coordination`, never directly from a sibling context, so
the bot and the Mini App share one code path and cannot double-celebrate.

**The front end.** `miniapp` is TypeScript under `web/app/src`; it depends on no internal
context and reaches the backend only over the HTTPS API. `landing` is the public, SEO-gated
Astro site (planned).

## Ubiquitous language

One concept has one name in the code, the schema and the prose of the context that owns it. The
lexicon is [LEXICON.md](LEXICON.md); the box-run ddd probe's `lexicon-locks` class refuses an
identifier in a scoped context that says a replaced word.

## Ownership register

Every table is owned by the context that WRITES it; reads say nothing. A table with two writing
contexts is a design defect: a second writer is either the owner called through its port, or
evidence the two contexts are one.

Each wave registers the tables it creates, in the SPEC that creates them. The register below is
the starting assignment of v9's 64 tables, which the v9 import (W8) maps table by table
(`data-migration.json`). DeckStreak's own schema may split, merge or rename a table; the SPEC
that does so updates this register in the same change.

| v9 table | owning context | privacy class in v9 |
|---|---|---|
| `schema_versions` | `kernel` | exempt |
| `sync_runs` | `ingest` | user-data |
| `review_cursor` | `ingest` | singleton |
| `daily_rollup` | `analytics` | user-data |
| `xp_state` | `progression` | singleton |
| `xp_ledger` | `progression` | user-data |
| `xp_settlement` | `progression` | user-data |
| `buffs` | `progression` | user-data |
| `streak_state` | `streaks` | singleton (all tracks reset) |
| `badges_earned` | `progression` | user-data |
| `notifications` | `notifications` | user-data |
| `deck_names` | `ingest` | user-data |
| `language_progress` | `curriculum` | user-data |
| `band_milestones` | `curriculum` | user-data |
| `coaching_kv` | `curriculum` | user-data |
| `reading_log` | `habits` | user-data |
| `writing_log` | `habits` | user-data |
| `daily_lang_stats` | `analytics` | user-data |
| `leech_snapshot` | `curriculum` | user-data |
| `leech_remediation` | `curriculum` | user-data |
| `focus_log` | `focus` | user-data |
| `focus_timer` | `focus` | singleton |
| `skip_days` | `ingest` | user-data |
| `skip_card_snapshot` | `ingest` | user-data |
| `settings_kv` | (split: see note) | user-data |
| `coin_ledger` | `economy` | user-data |
| `inventory` | `quests` | user-data |
| `xp_tokens` | `quests` | user-data |
| `habit_strength` | `streaks` | user-data |
| `freeze_events` | `streaks` | user-data |
| `celebration_log` | `notifications` | user-data |
| `governor_state` | `streaks` | singleton |
| `penalty_ledger` | `economy` | user-data |
| `buffs` | `progression` | user-data |
| `records` | `progression` | user-data |
| `committed_windows` | `discipline` | user-data |
| `window_events` | `discipline` | user-data |
| `chests` | `quests` | user-data |
| `pity` | `quests` | singleton |
| `quests` | `quests` | user-data |
| `quest_offers` | `quests` | user-data |
| `weekly_quests` | `quests` | user-data |
| `crown_days` | `quests` | user-data |
| `widget_state` | `notifications` | user-data |
| `wagers` | `discipline` | user-data |
| `tripwire_events` | `discipline` | user-data |
| `tripwire_state` | `discipline` | singleton |
| `sprints` | `discipline` | user-data |
| `ghosts` | `quests` | user-data |
| `race_results` | `quests` | user-data |
| `contracts` | `discipline` | user-data |
| `contract_days` | `discipline` | user-data |
| `contract_changes` | `discipline` | user-data |
| `pardons` | `discipline` | user-data |
| `beeminder_posts` | `discipline` | user-data |
| `hardmode_windows` | `discipline` | user-data |
| `season_nodes` | `progression` | user-data |
| `drill_xp_grants` | `vault` | user-data |
| `market_positions` | `markets` | user-data |
| `session_debrief` | `notifications` | user-data |
| `nudge_ablation` | `notifications` | user-data |
| `cron_fires` | `coordination` | exempt (never erased: erasure must not re-arm the catch-up double-send guard) |
| `preread_notes` | `readings` | user-data |
| `preread_runs` | `readings` | user-data |
| `preread_run_events` | `readings` | user-data |
| `can_do_unlocks` | `curriculum` | user-data |

`settings_kv` is split by owner: each runtime setting moves to the context that reads it (quiet
hours and celebration intensity to `notifications`, chests per day and the vault hour to
`quests`, tripwire and hard-mode settings to `discipline`, and so on), and each becomes visible on
the Mini App's settings screen. `cron_fires` stays exempt from export and erase, so an erasure can
never re-arm the catch-up double-send guard.

### DeckStreak's own tables

The tables DeckStreak's own migrations create, each registered by the SPEC that creates it. Every
migration lives in the one `migrations/` directory, named `<SPEC number, four digits><sequence, two
digits>_<owning context>_<slug>.sql` (ADR-020), and `crates/kernel/tests/schema.rs` holds the
context each migration names equal to the owner this register gives each table it creates.

| table | owning context | created by | export and erase |
|---|---|---|---|
| `settings_generation` | `kernel` | `migrations/002001_kernel_settings_generation.sql` (SPEC-020); the courses digest by `migrations/007102_kernel_courses_digest.sql` (SPEC-071) | reset in place: the generation back to 0 and the courses digest cleared |
| `_sqlx_migrations` | `kernel` | sqlx, when `Db::open` applies the migrations | exempt: the schema version table, which replaces the predecessor's `schema_versions` |
| `sync_runs` | `ingest` | `migrations/002201_ingest_sync_runs.sql` (SPEC-022) | exported and erased |
| `ingest_state` | `ingest` | `migrations/002301_ingest_state.sql` (SPEC-023) | reset in place: the anchor, the rescore flag, the refused request (SPEC-128) and the window's base cleared |
| `cron_fires` | `coordination` | `migrations/002701_coordination_cron_fires.sql` (SPEC-027) | exempt: an erase must never re-arm the catch-up double-send guard |
| `xp_ledger` | `progression` | `migrations/004001_progression_xp_ledger.sql` (SPEC-040) | exported and erased |
| `xp_settlement` | `progression` | `migrations/007201_progression_xp_settlement.sql` (SPEC-072) | exported and erased |
| `buffs` | `progression` | `migrations/007202_progression_buffs.sql` (SPEC-072) | exported and erased |
| `notification_decisions` | `notifications` | `migrations/004101_notifications_router.sql` (SPEC-041) | exported and erased |
| `notification_deliveries` | `notifications` | `migrations/004101_notifications_router.sql` (SPEC-041) | exported and erased |
| `notification_queue` | `notifications` | `migrations/004101_notifications_router.sql` (SPEC-041) | exported and erased |
| `in_app_feed` | `notifications` | `migrations/004101_notifications_router.sql` (SPEC-041) | exported and erased |
| `notification_settings` | `notifications` | `migrations/004101_notifications_router.sql` (SPEC-041) | exported and erased |
| `owner_last_message` | `notifications` | `migrations/008401_notifications_owner_last_message.sql` (SPEC-084) | reset in place: no message |
| `reading_runs` | `readings` | `migrations/004501_readings_topic_days_and_runs.sql` (SPEC-045) | exported and erased |
| `reading_topic_days` | `readings` | `migrations/004501_readings_topic_days_and_runs.sql` (SPEC-045) | exported and erased |
| `readings` | `readings` | `migrations/004601_readings_and_attempts.sql` (SPEC-046) | exported and erased |
| `reading_attempts` | `readings` | `migrations/004601_readings_and_attempts.sql` (SPEC-046) | exported and erased |
| `agent_runs` | `agent` | `migrations/004301_agent_runs.sql` (SPEC-043) | exported and erased |
| `daily_rollup` | `analytics` | `migrations/007101_analytics_daily_rollup.sql` (SPEC-071) | exported and erased |
| `daily_lang_stats` | `analytics` | `migrations/007101_analytics_daily_rollup.sql` (SPEC-071) | exported and erased |

## Overloaded words, held apart

| word | meaning here | never means |
|---|---|---|
| reading | one pre-study text for one topic on one study day (`readings`) | the minutes-of-reading habit, which `habits` calls a minutes log |
| lapse | the governor's episode of three or more zero-review study days, with a lapse id (`streaks`) | the two-day reading pause, which `readings` calls a pause |
| day | a study day, which turns over at 04:00 local (`kernel`) | a calendar day |
| duty | one kind of AI work the agent performs (`agent`) | a scheduled job, which `coordination` calls a job |
| track | language or law (`kernel`) | a reading topic |
| tier | a celebration tier T0 to T5 (`notifications`) or a Bloom tier T1 to T4 (`progression`); the two are always qualified | a subscription level |
