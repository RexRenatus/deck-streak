# SPEC-001: the campaign: full parity, the flagship readings, and the wave plan

- **Wave:** all (the campaign PRD). **Issue:** the ten wave epics, (issue pending) to (issue pending). **Context(s):** every context.
- **Decided by:** ADR-001, ADR-002, ADR-003, ADR-004, ADR-005, ADR-006, ADR-007, ADR-008, ADR-009,
  ADR-010, ADR-011, ADR-012, ADR-013, ADR-014, ADR-015, ADR-016, ADR-017, ADR-018, ADR-019.
- **Status:** judged. The matrix is generated from one data source together with the issues, so the
  two cannot disagree; `scripts/tests/test_parity_matrix.py` holds it to the population.

## 1. The problem, measured

- **The predecessor.** A private single-user service with 131 features (93 product, 9 platform, 18
  ops, 11 with no production caller), a database of 64 tables at schema version 24, 64 bot
  commands, 29 inline-button families, 12 scheduled jobs and 35 MCP tools. Its only surface is a
  Telegram bot; its charts are drawn on a small host whose memory they spike. Source: the
  predecessor's feature inventory (private), counted by feature record.
- **The flagship never shipped.** The predecessor's nightly pre-study readings fired every night and
  produced nothing: every run refused on a stale-snapshot gate, and the folder the readings were to
  be written to never existed. Source: the second-brain inventory (private), from the run ledger.
- **The second brain.** 22 user-facing features around an Obsidian vault (the readings, drills, the
  daily note, the journal, the weekly synthesis, dashboards, the inbox), whose nightly AI pass has
  been failing; the owner directed that a Claude Code agent through the owner's subscription proxy
  take over its duties.
- **What this campaign delivers.** Every predecessor feature built or excluded with a reason
  (121 built, 10 excluded as inert, each with an owner decision issue); every
  second-brain user-facing feature placed; the daily pre-study readings as the Mini App's first-class
  experience; eleven AI duties under nineteen personas; one public, AGPL-licensed repository.

## 2. Requirements

R1. Every predecessor feature id appears exactly once in Appendix A, either built (with its context,
    wave and issue) or excluded with its reason; every second-brain user-facing feature appears in
    Appendix B; every item the predecessor retired appears in Appendix C with the digest's reason.
R2. Every built row's behaviour is proved against the predecessor: its numbers by the parity
    oracle's goldens of the predecessor's own functions (ADR-012), never re-derived.
R3. The owner's reading decisions (section 9) are binding requirements of the readings SPECs.
R4. The AI agent runs through the owner's subscription proxy on the host, fail-closed, with its
    duties and personas gated by the packs before any output is delivered (ADR-015, CHARTER 16-18).
R5. Every celebration and nudge passes through one router shared by the bot and the Mini App.
R6. Every pack DeckStreak consumes is wired into the gate or CI and green by the end of W7
    (`.packs/wiring.json` and `scripts/box-packs.sh`).
R7. The repository is public, with `dev` protected, `main` changed only by release pull requests,
    GitHub-hosted CI green on both, and secret scanning, push protection and Dependabot on.
R8. `v1.0.0` is tagged on `main` and deployed from the tag, with backups covering the database.
R9. The predecessor runs beside DeckStreak until parity is verified and is retired only on the
    owner's go.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every predecessor feature id is accounted for exactly once in Appendix A | `python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_v9_feature_is_accounted_for_once` |
| A2 | every excluded row carries its reason and its owner decision issue | `python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_exclusion_carries_its_reason` |
| A3 | every built row names a context the context map declares and a wave of the plan | `python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_built_row_names_a_context_and_a_wave` |
| A4 | every built row names its issue, and the issue is in the manifest | `python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_built_row_names_its_issue` |
| A5 | every second-brain user-facing feature is accounted for in Appendix B | `python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_second_brain_feature_is_accounted_for` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_v9_feature_is_accounted_for_once
A2: python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_exclusion_carries_its_reason
A3: python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_built_row_names_a_context_and_a_wave
A4: python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_built_row_names_its_issue
A5: python3 -m unittest discover -s scripts/tests -p test_parity_matrix.py -k every_second_brain_feature_is_accounted_for
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-001-campaign-prd-parity-and-waves.md` | campaign | added |
| `docs/parity/v9-feature-ids.txt` | campaign | added: the population of predecessor feature ids |
| `docs/issues-manifest.json` | campaign | added: each row's issue number and id |
| `docs/github/milestones.txt` | campaign | added: one milestone per wave |
| `scripts/tests/test_parity_matrix.py` | campaign | added |
| `scripts/create-issues.py` | campaign | added: idempotent, paced, scrubbed issue creation |
| `scripts/github-setup.sh` | campaign | added: labels, milestones, rulesets, security settings |

## 5. What this does NOT do

- It serves one owner per deployment, never several learners at once ((issue pending)).
- It reads the owner's own Anki sync server and never logs in to AnkiWeb ((issue pending)).
- It builds no countdown, exam date or forward-looking timeline, anywhere ((issue pending)).
- It revives none of the predecessor's inert features without the owner's decision (one issue each, for example (issue pending)).
- It builds the vault to Anki flashcard bridge only after parity and the readings ((issue pending)).

## 6. Risks

- **A feature's parity is claimed, not proved.** Detected by the parity oracle's goldens and by
  closing each issue only with an evidence comment (the criterion, the command, its output).
- **The host's memory or disk runs out beside the predecessor.** Detected by the memory watch and
  the host budget (ADR-010); a resize is an owner decision.
- **The agent's path to the proxy fails often.** Detected by the readings' lane health and the
  digest's degraded-coaching line; the deterministic fallbacks keep the owner informed.
- **A private value leaks into the public repository or an issue.** Detected by the public scrub
  in the gate and before every issue is posted, and by the history scan before the repository is
  made public.

## 7. The wave plan

Waves run in order, with W2's owner-gated deploy running beside W3. A wave's SPECs are written by
the wave's architect turn before its builders are dispatched; W0 and W1 are specified now
(`docs/specs/planned/`). Builders at once are bounded by the build machine (about six compiling
builders).

| wave | goal | DeckStreak units | v9 rows with their own issue | builders at once |
|---|---|---|---|---|
| W0 | Foundation: kernel, config and secrets, database base, identity (Telegram initData), API and bot shells, Anki ingest, scheduler and cron ledger, data-rights framework, Mini App shell, parity-oracle harness | 15 | 0 | 1, then 6 |
| W1 | Flagship: the daily pre-study readings, with the AI agent core, the vault adapter core, the XP grant port and the notification router core | 14 | 0 | 5, then 6 |
| W2 | First deploy: readings live on the VM side by side with v9 (host scrub, units, HTTPS, the proxy tunnel, backups), behind owner gates | 6 | 0 | 2 (owner-gated) |
| W3 | Game core parity: analytics and score, XP and levels, streaks, economy, quests and chests, focus, habits, skip day | 0 | 34 | 6 |
| W4 | Curriculum and insight parity: Road to C2, law track, leeches, research instruments, client-side charts | 0 | 28 | 6 |
| W5 | Engagement parity: the full celebration ladder, digests, nudges, comeback, discipline devices, prediction markets | 0 | 23 | 6 |
| W6 | Second brain and the AI duties: drills, vault bridge, inbox, daily note, weekly synthesis, leech doctor, writing tutor, conversation partner, practice questions, digest coaching | 11 | 6 | 6 |
| W7 | Surfaces and polish: settings, linked sign-in, landing page, public achievement page, share art, accessibility and CJK polish, every pack enforced | 4 | 3 | 4 |
| W8 | Data migration and cutover: v9 schema 24 import, side-by-side verification, cutover, v1.0.0 | 4 | 0 | 2 |
| W9 | After parity: the vault to Anki flashcard bridge | 1 | 0 | 1 |

## 8. The architecture, in one page

The backend is one Rust workspace with a crate per bounded context (docs/CONTEXT-MAP.md,
ADR-002): a shared kernel; domain contexts that depend only on the kernel and on `ingest`; a
`coordination` layer of use cases and jobs; `api` and `bot` adapters; and the `daemon`
composition root. The Mini App is a SvelteKit SPA behind Caddy on the host (ADR-005, ADR-007),
authenticated by Telegram `initData` pinned to the owner (ADR-006). Data is one SQLite database in
WAL mode, replicated by Litestream (ADR-008, ADR-010). Anki data arrives by syncing a private
collection copy with Anki's own engine (ADR-009). The AI agent is headless Claude Code through the
owner's subscription proxy over a reverse tunnel, gated by the packs (ADR-015). Details:
ARCHITECTURE.md and docs/schematics/.

## 9. The owner's reading decisions (binding)

- **Staleness:** readings generate whenever the last sync succeeded. After two or more days
  without study, daily readings pause, and the owner gets one comeback reading per lapse, inside
  the three-message comeback cap.
- **The read tap:** the Mini App's "I read it" writes the vault note's box only on the owner's tap;
  code never ticks it on its own and never clears it.
- **XP:** 40 when a reading is marked read, plus 60 when at least 80% of its new cards are reviewed
  within two study days; at most 100 per reading; no new streak; granted once per reading.
- **Length and cap:** a law reading's primer scales with its topic's new-card count within 800 to
  1500 words, and there is no daily cap on the number of readings.
- **Surface:** the Mini App is the primary reading surface; the same reading is archived to the
  vault.
- **Drafts:** the vault to Anki card bridge comes after parity and the readings (W9); the countdown
  is not built.
- **Rules carried over:** fail loud and never write a placeholder; on demand is tap to pick; code
  never destroys the owner's tick; no spend cap but full telemetry, with the agent's per-run caps as
  a separate safety bound; vault writes are atomic and pass the rails; no exam dates or timelines.

## 10. Appendix A: the parity matrix (predecessor features)

A built row's issue is the issue that delivers it: its own, or the DeckStreak work unit (Appendix
D) that covers it.

| v9 feature | name | category | context | wave | disposition | issue |
|---|---|---|---|---|---|---|
| `anki-sync-download` | Headless Anki sync from the self-hosted sync server | platform | ingest | W0 | build | (issue pending) |
| `collection-read-ingest` | Read-only collection ingest (bounded window) + deck scope | platform | ingest | W0 | build | (issue pending) |
| `sync-change-gate` | Sync change gate (skip the recompute when nothing changed) | platform | ingest | W0 | build | (issue pending) |
| `study-day-calendar` | Rollover-aware study day and collection calendar | platform | kernel | W0 | build | (issue pending) |
| `daily-rollup-metrics` | Daily metrics rollup + card-state snapshot | product | analytics | W3 | build | (issue pending) |
| `per-language-daily-stats` | Per-language daily Anki stats | product | analytics | W3 | build | (issue pending) |
| `daily-score` | Five-pillar daily score (0-100) and grade band | product | analytics | W3 | build | (issue pending) |
| `today-snapshot` | Today snapshot (/today) and status reads | product | analytics | W3 | build | (issue pending) |
| `xp-ledger-and-levels` | XP ledger, per-review XP, daily bonuses, level curve and titles | product | progression | W3 | build | (issue pending) |
| `law-tier-xp` | Law-track Bloom-tier XP multiplier | product | progression | W3 | build | (issue pending) |
| `consistency-multiplier` | Consistency XP multiplier (on-pace run with tier-down) | product | progression | W3 | build | (issue pending) |
| `phoenix-ascendant-buff` | Phoenix Ascendant next-day buff | product | progression | W3 | build | (issue pending) |
| `badges-catalog` | Badge catalog (40 static) + CEFR band-up badges | product | progression | W3 | build | (issue pending) |
| `personal-records` | Personal records (best score, most reviews, most minutes) | product | progression | W3 | build | (issue pending) |
| `milestone-ladder` | Next milestone ladder | product | progression | W3 | build | (issue pending) |
| `seasons-prestige-chapters` | Monthly seasons, prestige titles and chapter ceremonies | product | progression | W3 | build | (issue pending) |
| `season-node-track` | Season node track (10 coin checkpoints per month) | product | progression | W3 | build | (issue pending) |
| `leaderboard-personal` | Personal leaderboard read | product | progression | W3 | build | (issue pending) |
| `xp-exchange-rates` | XP exchange-rate readout (XP per graduation by source) | product | progression | W3 | build | (issue pending) |
| `language-streak-freezes` | Language study streak with freezes, heat tiers and lapse decay | product | streaks | W3 | build | (issue pending) |
| `law-streak` | Independent law-track streak | product | streaks | W3 | build | (issue pending) |
| `habit-strength-governor` | Habit strength and the anti-abandonment governor | product | streaks | W3 | build | (issue pending) |
| `relight-ritual` | Relight ritual (first day back after a lapse) | product | streaks | W3 | build | (issue pending) |
| `road-to-c2` | Road to C2: per-language CEFR progress and band-up celebrations | product | curriculum | W4 | build | (issue pending) |
| `forecast-velocity` | Road-to-C2 forecast (velocity and ETA to next band) | product | curriculum | W4 | build | (issue pending) |
| `adaptive-daily-goal` | Adaptive daily review goal | product | curriculum | W4 | build | (issue pending) |
| `skill-strands-weak-spots` | Skill strands and weak spots | product | curriculum | W4 | build | (issue pending) |
| `cross-language-balance` | Cross-language study balance | product | curriculum | W4 | build | (issue pending) |
| `memory-health-calibration` | Memory health, retention calibration, input readiness, stability depth, mature trend | product | curriculum | W4 | build | (issue pending) |
| `forward-obligation-horizon` | Forward obligation horizon (the empty calendar) | product | curriculum | W4 | build | (issue pending) |
| `can-do-ladder` | Can-Do ladder (fine-grained reward ladder in the owner's own words) | product | curriculum | W4 | build | (issue pending) |
| `reading-log` | Reading habit log (/read) with weekly goal and XP | product | habits | W3 | build | (issue pending) |
| `writing-log` | Writing habit (daily K/J/C confirmation) | product | habits | W3 | build | (issue pending) |
| `habit-board` | Habit board (/habits) and habit digest block | product | habits | W3 | build | (issue pending) |
| `reading-analytics` | Reading analytics fused with Anki (/readstats, /readtrend, /correlate) | product | habits | W3 | build | (issue pending) |
| `habit-nudge` | Evening habit check-in (with lapse back-off) | product | notifications | W5 | build | (issue pending) |
| `focus-timer` | Pomodoro focus timer with live countdown and auto cycles | product | focus | W3 | build | (issue pending) |
| `focus-xp-badges-stats` | Deep-work XP, streak, stats board and focus nudge | product | focus | W3 | build | (issue pending) |
| `daily-quests` | Daily quest arc (Q1, sealed Q2, owner-picked Q3, crown days) | product | quests | W3 | build | (issue pending) |
| `weekly-meta-quest` | Weekly meta-quest | product | quests | W3 | build | (issue pending) |
| `session-chests` | Session chests (variable-ratio loot with pity) and the overnight vault | product | quests | W3 | build | (issue pending) |
| `double-xp-tokens` | Double-XP tokens | product | quests | W3 | build | (issue pending) |
| `smoke-bombs-perfect-week` | Perfect Week smoke bombs | product | quests | W3 | build | (issue pending) |
| `instant-loop-free-spin` | Instant loop: AnkiMobile-open acknowledgement and free spin | product | discipline | W5 | build | (issue pending) |
| `coin-wallet` | Coin wallet and economy rules | product | economy | W3 | build | (issue pending) |
| `coin-shop` | Coin shop (/shop) | product | economy | W3 | build | (issue pending) |
| `skip-day` | Skip / cheat day (the one Anki write path) | product | ingest | W3 | build | (issue pending) |
| `committed-windows` | Committed study windows | product | discipline | W5 | build | (issue pending) |
| `doomscroll-tripwire` | Doomscroll tripwire rail (sensor, verdicts, sprints, rung ladder, canary) | product | discipline | W5 | build | (issue pending) |
| `confess` | Honor-system confession (/confess) | product | discipline | W5 | build | (issue pending) |
| `streak-wager` | Double-or-nothing streak wager (/wager) | product | discipline | W5 | build | (issue pending) |
| `commitment-contracts` | Commitment contracts with a weakening horizon and Sunday stake review | product | discipline | W5 | build | (issue pending) |
| `panic-pardon-rearm` | Panic switch, monthly pardon and re-arm | product | discipline | W5 | build | (issue pending) |
| `hard-mode` | Hard mode (Trial by Fire evening window) | product | discipline | W5 | build | (issue pending) |
| `beeminder-money-rung` | Beeminder real-money rung | product | discipline | W5 | build | (issue pending) |
| `evening-nudge-coordinator` | Evening nudge coordinator + stakes preview (streak risk) | product | notifications | W5 | build | (issue pending) |
| `last-chance-nudge` | Last-chance deadline ping (22:00) | product | notifications | W5 | build | (issue pending) |
| `self-prediction-markets` | Self-prediction markets and the Oracle ladder (/predict, /oracle) | product | markets | W5 | build | (issue pending) |
| `celebration-ladder` | Celebration escalation ladder (T0-T5) with budgets, honesty cap, quiet-hour deferral and outage breakers | product | notifications | W5 | build | (issue pending) |
| `quiet-hours` | Quiet hours | product | notifications | W1 | build | (issue pending) |
| `pinned-widget` | Pinned daily widget (ambient dashboard) | product | notifications | W5 | build | (issue pending) |
| `morning-brief` | Morning brief | product | notifications | W5 | build | (issue pending) |
| `comeback-protocol` | Comeback protocol (lapse-mode morning messages and lapse digest) | product | notifications | W5 | build | (issue pending) |
| `ghost-race` | Ghost-of-past-self weekly race (/race) | product | quests | W3 | build | (issue pending) |
| `streak-share-cards` | Streak milestone share cards (AI art) | product | progression | W7 | build | (issue pending) |
| `ai-keepsake-art` | AI keepsake art (Nano Banana Pro via OpenRouter) | product | progression | W7 | build | (issue pending) |
| `historical-landmarks` | Historical landmarks (anniversaries and every 25th study day) | product | notifications | W5 | build | (issue pending) |
| `milestone-pings` | Milestone pings (badges, level-ups, queue zero) | product | notifications | W5 | build | (issue pending) |
| `daily-digest` | Daily digest (just-closed study day) | product | notifications | W5 | build | (issue pending) |
| `weekly-report` | Weekly report (Sunday) | product | notifications | W5 | build | (issue pending) |
| `session-debrief` | Felt-difficulty session debrief | product | notifications | W5 | build | (issue pending) |
| `nudge-ablation` | Nudge ablation (holdout experiment) and withhold ledger | product | notifications | W5 | build | (issue pending) |
| `leech-remediation` | Leech remediation workflow | product | curriculum | W4 | build | (issue pending) |
| `law-track-summary` | Law-track summary block (dual-track display, law primary) | product | curriculum | W4 | build | (issue pending) |
| `lsat-section-board` | LSAT section coverage board (/lsat) | product | curriculum | W4 | build | (issue pending) |
| `law-drills` | Law drills: browse, answer via Telegram, graded post-back XP | product | vault | W6 | build | (issue pending) |
| `illusion-ledger` | The Illusion Ledger (recognition vs production retention gap) | product | insights | W4 | build | (issue pending) |
| `echo-test` | The Echo Test (same-note sibling contamination) | product | insights | W4 | build | (issue pending) |
| `price-of-a-day-off` | The Price of a Day Off (rest-day dose-response) | product | insights | W4 | build | (issue pending) |
| `hanzi-dividend` | The Hanzi Dividend (Chinese-Japanese character overlap) | product | insights | W4 | build | (issue pending) |
| `instrument-bench-ii` | Instrument Bench II (Preset Audit, Lateness Ceiling, Bake-Off) | product | insights | W4 | build | (issue pending) |
| `dark-fields` | Dark Fields (authored content no template renders) | product | insights | W4 | build | (issue pending) |
| `dead-air` | Dead Air (idle time between cards) | product | insights | W4 | build | (issue pending) |
| `the-runway` | The Runway (unseen pool in years) | product | insights | W4 | build | (issue pending) |
| `contradiction-docket` | The Contradiction Docket | product | insights | W4 | build | (issue pending) |
| `fluency-trap-audit` | Fluency check (/fluency) | product | insights | W4 | build | (issue pending) |
| `divestment-day` | Divestment Day (/divest) | product | insights | W4 | build | (issue pending) |
| `tilt-test` | Tilt Test (/tilt) | product | insights | W4 | build | (issue pending) |
| `deal-the-hand` | Deal the Hand (/hand) | product | insights | W4 | build | (issue pending) |
| `sabbatical-clock` | Sabbatical Decay Clock (/park <lang>) | product | insights | W4 | build | (issue pending) |
| `other-hand-census` | The Other Hand (/otherhand) due-date provenance census | product | insights | W4 | build | (issue pending) |
| `charts-and-resources` | Charts (/chart) and MCP chart resources | product | insights | W4 | build | (issue pending) |
| `vault-stats-bridge` | Vault stats bridge (nightly stats JSON into the second-brain vault) | product | vault | W6 | build | (issue pending) |
| `inbox-capture` | Telegram media capture to the vault inbox | product | vault | W6 | build | (issue pending) |
| `vaultops-trigger` | On-demand vault-ops run (/vaultops) | product | agent | W6 | build | (issue pending) |
| `preread-lane` | Nightly pre-reading lane (AI primers into 12-Readings) with two-box tracking and health checks | product | readings | W1 | build | (issue pending) |
| `public-achievement-publish` | Public achievement publishing (scrubbed, opt-in) | product | publishing | W7 | build | (issue pending) |
| `inert-churn-tax` | Churn Tax (same-day repeat-answer accounting) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-latency-debt` | Latency Debt exam-clock audit - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-two-button-grading` | Two-Button Grading audit (collapsed grade vocabulary) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-shuffle-test` | Shuffle Test (blocked vs interleaved study order) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-birth-cohort` | Birth Cohort (intake-day batch size vs lifetime cost) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-unclaimed-effort` | Unclaimed Effort Ledger (study the filters never saw) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-track-crowdout` | Law vs language track crowd-out statistics - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-cue-ledger` | Cue ledger (per-app cost of doomscrolling) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-optimiser-ledger` | Optimiser Ledger + Preset Census (FSRS preset eligibility) - INERT | inert | - | - | exclude: inert in v9 | (issue pending) |
| `inert-peak-day` | Peak Day taper (law retrievability at an exam date) - library only | inert | - | - | exclude: inert in v9; it also needs an exam date, which the owner's no-dates rule forbids | (issue pending) |
| `inert-reading-rollover-signal` | Per-topic reading rollover lapse signal - no surface | inert | readings | W1 | build | (issue pending) |
| `mcp-server` | MCP server (FastMCP, Streamable HTTP) | platform | agent | W6 | build | (issue pending) |
| `mcp-bearer-auth` | Fail-closed bearer auth for the law/drill MCP tools | platform | agent | W6 | build | (issue pending) |
| `http-health-metrics-dashboard` | Health, readiness, Prometheus metrics and HTML dashboard | ops | daemon | W0 | build | (issue pending) |
| `data-export-erase` | Data export and erasure (data rights) with a privacy completeness gate | platform | privacy | W0 | build | (issue pending) |
| `telegram-bot-runtime` | Telegram command bot runtime (long poll, owner gate, menu, safety caps) | platform | bot | W0 | build | (issue pending) |
| `telegram-notifier` | Outbound Telegram notifier | platform | bot | W0 | build | (issue pending) |
| `cron-ledger-and-scheduler` | Scheduler layout, shared sync guard and cron-fire ledger | ops | pipeline | W0 | build | (issue pending) |
| `startup-catchup` | Startup catch-up of missed notification fires | ops | pipeline | W0 | build | (issue pending) |
| `liveness-deadman-watch` | Hourly dead-man watch, rollover-drift and digest-window checks | ops | pipeline | W0 | build | (issue pending) |
| `sync-failure-alerting` | Sync failure alerting and sync_runs history | ops | pipeline | W0 | build | (issue pending) |
| `db-maintenance` | Daily DB maintenance | ops | pipeline | W0 | build | (issue pending) |
| `sd-notify-watchdog` | systemd notify watchdog (liveness only) | ops | daemon | W0 | build | (issue pending) |
| `offload-rails` | Offload rails and latency budget (keep blocking work off the event loop) | ops | kernel | W0 | build | (issue pending) |
| `secrets-resolution` | Secret resolution (GCP Secret Manager via ADC, env fallback) and degraded-secret tracking | ops | kernel | W0 | build | (issue pending) |
| `logging-redaction` | Secret-redacting structured logging (journald JSON with syslog priorities) | ops | kernel | W0 | build | (issue pending) |
| `nightly-backup-offsite` | Nightly backup with offsite copy, dedup, Litestream age check and temp sweep | ops | deploy | W2 | build | (issue pending) |
| `litestream-replication` | Litestream continuous WAL replication | ops | deploy | W2 | build | (issue pending) |
| `restore-drills` | Weekly restore drills (local backup + Litestream replica) | ops | deploy | W2 | build | (issue pending) |
| `deploy-provision-harden` | Deploy, provision and VM hardening scripts | ops | deploy | W2 | build | (issue pending) |
| `card-state-backfill-tool` | One-shot card-state history recovery tool | ops | migration | W8 | build | (issue pending) |
| `syllabus-band-export` | CEFR band syllabus export + parity gate | ops | curriculum | W4 | build | (issue pending) |
| `ci-pipeline` | GitHub Actions CI (lint, types, tests, anki compat, supply chain) | ops | repo | W0 | build | (issue pending) |
| `test-tmp-retention` | pytest tmp_path retention policy | ops | repo | W0 | build | (issue pending) |

## 11. Appendix B: the second brain's user-facing features

| second-brain feature | name | context | wave | disposition | delivered by |
|---|---|---|---|---|---|
| `SB-U1` | Daily pre-study readings, law form | readings | W1 | build | DS-W1-06 ((issue pending)) DS-W1-07 ((issue pending)) DS-W1-12 ((issue pending)) |
| `SB-U2` | Language pre-reading form | readings | W1 | build | DS-W1-06 ((issue pending)) DS-W1-07 ((issue pending)) |
| `SB-U3` | Reading completion: read and studied | readings | W1 | build | DS-W1-08 ((issue pending)) DS-W1-12 ((issue pending)) |
| `SB-U4` | Rollover, archive and carried nights | readings | W1 | build | DS-W1-03 ((issue pending)) DS-W1-11 ((issue pending)) DS-W1-12 ((issue pending)) |
| `SB-U5` | On-demand regeneration, tap to pick | readings | W1 | build | DS-W1-09 ((issue pending)) DS-W1-13 ((issue pending)) |
| `SB-U6` | The comeback reading | readings | W1 | build | DS-W1-10 ((issue pending)) |
| `SB-U7` | Law drill generation | agent | W6 | build | DS-W6-01 ((issue pending)) |
| `SB-U8` | Drill answering | vault | W6 | build | DS-W6-10 ((issue pending)) law-drills |
| `SB-U9` | Drill grading and feedback | agent | W6 | build | DS-W6-01 ((issue pending)) |
| `SB-U10` | Drill XP post-back | vault | W6 | build | law-drills |
| `SB-U11` | The daily note | agent | W6 | build | DS-W6-04 ((issue pending)) |
| `SB-U12` | The journal (capture only) | vault | W6 | build | DS-W6-11 ((issue pending)) |
| `SB-U13` | The weekly synthesis | agent | W6 | build | DS-W6-04 ((issue pending)) |
| `SB-U14` | Vault dashboards as native progress screens | miniapp | W6 | build | DS-W6-10 ((issue pending)) |
| `SB-U15` | Subject maps: unit-progress views | miniapp | W6 | build | DS-W6-10 ((issue pending)) |
| `SB-U16` | Concept notes | - | - | exclude: remains an owner-curated vault artifact; the second-brain inventory recommends keeping it in the vault (inventory section 2.1, U16) | - |
| `SB-U17` | Inbox capture and filing | vault | W6 | build | inbox-capture DS-W6-05 ((issue pending)) |
| `SB-U18` | On-demand vault-ops run | agent | W6 | build | vaultops-trigger DS-W6-09 ((issue pending)) |
| `SB-U19` | Vault to Anki flashcard bridge | vault | W9 | build | DS-W9-01 ((issue pending)) |
| `SB-U20` | Bar and LSAT countdown | - | - | exclude: the owner decided it is not built and stays a draft; no dates or timelines anywhere | (issue pending) |
| `SB-U21` | Digests and nudges carrying second-brain data | notifications | W6 | build | daily-digest weekly-report DS-W6-08 ((issue pending)) |
| `SB-U22` | Curated public page | publishing | W7 | build | public-achievement-publish |

## 12. Appendix C: retired in the predecessor (excluded, with the digest's reason)

| retired item | what | the digest's reason |
|---|---|---|
| `R-01` | Rest-token and comeback guards (backlog C8) | superseded by the discipline engine program; the backlog says do not port it as a separate feature |
| `R-02` | Voice logging and an i18n catalog for the reading habit | explicit won't-do, with the trigger for revisiting recorded in v9's ADR-010 |
| `R-03` | Atomic full-download swap | closed as a false premise: Anki already stages and renames atomically; only the backup's swap lock and orphan sweep carry over (ingest and backups) |
| `R-04` | Gentle, standard or strict tier-down knob | cut: safety constants stay out of owner-mood-reachable state |
| `R-05` | Temptation bundling, send-time personalisation, a real-money reward channel | optional balance items deliberately left out of every shipped spec |
| `R-06` | Two superseded instruments of the split-fate instrument spec | superseded by later instrument specs; only the surviving instrument ports |
| `R-07` | The preset census's original design | superseded by its rescope; the shipped shape is the one that ports |
| `R-08` | The enhancement loop's scheduling meta-work | process, not a product feature |

## 13. Appendix D: DeckStreak's own work units

| work unit | title | context | wave | SPEC | issue |
|---|---|---|---|---|---|
| `DS-W0-01` | Kernel: study day, clock, ids, track and verdict | kernel | W0 | SPEC-020 | (issue pending) |
| `DS-W0-02` | Kernel: configuration, credentials and redacting logs | kernel | W0 | SPEC-020 | (issue pending) |
| `DS-W0-03` | Kernel: the SQLite base, migrations and the data-rights port | kernel | W0 | SPEC-020 | (issue pending) |
| `DS-W0-04` | Privacy: export and erase, proved symmetric | privacy | W0 | SPEC-021 | (issue pending) |
| `DS-W0-05` | Ingest: the measured sync engine spike and the Anki sync | ingest | W0 | SPEC-022 | (issue pending) |
| `DS-W0-06` | Ingest: the read-only collection read, deck scope and change gate | ingest | W0 | SPEC-023 | (issue pending) |
| `DS-W0-07` | Identity: Telegram initData validation and the owner pin | identity | W0 | SPEC-024 | (issue pending) |
| `DS-W0-08` | API: the axum service shell with health, limits and graceful shutdown | api | W0 | SPEC-025 | (issue pending) |
| `DS-W0-09` | Bot: the Telegram transport, owner gate, menu and notifier | bot | W0 | SPEC-026 | (issue pending) |
| `DS-W0-10` | Coordination: the scheduler, cron-fire ledger, catch-up and liveness | coordination | W0 | SPEC-027 | (issue pending) |
| `DS-W0-11` | Mini App: the shell, the Telegram wrapper, tokens and the API handshake | miniapp | W0 | SPEC-028 | (issue pending) |
| `DS-W0-12` | Parity oracle: the golden generator, format and reader | coordination | W0 | SPEC-029 | (issue pending) |
| `DS-W0-13` | Repository hygiene: CI, test hygiene and the pack runner | repo | W0 | SPEC-030 | (issue pending) |
| `DS-W0-14` | Observability: logging, traces, the alert unit and the SLO skeleton | daemon | W0 | SPEC-031 | (issue pending) |
| `DS-W0-15` | Deploy templates: units, the Caddy block, the host budget | deploy | W0 | SPEC-032 | (issue pending) |
| `DS-W1-01` | Progression: the XP ledger and its idempotent grant port | progression | W1 | SPEC-040 | (issue pending) |
| `DS-W1-02` | Notifications: the one router, quiet hours and the withhold ledger | notifications | W1 | SPEC-041 | (issue pending) |
| `DS-W1-03` | Vault: the adapter core, atomic writes, rails and the readings date tree | vault | W1 | SPEC-042 | (issue pending) |
| `DS-W1-04` | Agent: the headless runner, prompt composition, the output gate and fail-closed degradation | agent | W1 | SPEC-043 | (issue pending) |
| `DS-W1-05` | Agent: the persona engine, private roster and subject memory | agent | W1 | SPEC-044 | (issue pending) |
| `DS-W1-06` | Readings: the day set from the scheduler's queue, topics and honest states | readings | W1 | SPEC-045 | (issue pending) |
| `DS-W1-07` | Readings: generation through the persona, the gates, and one repair retry | readings | W1 | SPEC-046 | (issue pending) |
| `DS-W1-08` | Readings: the read tap, the studied measure and reading XP | readings | W1 | SPEC-047 | (issue pending) |
| `DS-W1-09` | Readings: on-demand regeneration, tap to pick, one lock | readings | W1 | SPEC-048 | (issue pending) |
| `DS-W1-10` | Readings: the comeback reading, one per lapse | readings | W1 | SPEC-049 | (issue pending) |
| `DS-W1-11` | Readings: lane health, triage and transparency | readings | W1 | SPEC-050 | (issue pending) |
| `DS-W1-12` | Mini App: Today's readings, the reader and the history | miniapp | W1 | SPEC-051 | (issue pending) |
| `DS-W1-13` | Bot: the readings command and the morning readings line | bot | W1 | SPEC-052 | (issue pending) |
| `DS-W1-14` | Coordination: the readings jobs | coordination | W1 | SPEC-053 | (issue pending) |
| `DS-W2-01` | Host: inventory, backup and the scrub list for the owner | deploy | W2 | per-wave | (issue pending) |
| `DS-W2-02` | The private deploy rail: credentials, configuration and the guards | deploy | W2 | per-wave | (issue pending) |
| `DS-W2-03` | First deploy: units, the Caddy block and HTTPS | deploy | W2 | per-wave | (issue pending) |
| `DS-W2-04` | The agent's path: the reverse tunnel and its device key | deploy | W2 | per-wave | (issue pending) |
| `DS-W2-05` | Backups: Litestream, the daily backup and the restore drill | deploy | W2 | per-wave | (issue pending) |
| `DS-W2-06` | Readings live: the vault folder, the first nightly run and the first reading | deploy | W2 | per-wave | (issue pending) |
| `DS-W6-01` | Agent duty: the drill coach (generation and grading) | agent | W6 | per-wave | (issue pending) |
| `DS-W6-02` | Agent duty: the leech doctor | agent | W6 | per-wave | (issue pending) |
| `DS-W6-03` | Agent duty: the writing tutor | agent | W6 | per-wave | (issue pending) |
| `DS-W6-04` | Agent duties: the daily note and the weekly synthesis | agent | W6 | per-wave | (issue pending) |
| `DS-W6-05` | Agent duty: the inbox curator | agent | W6 | per-wave | (issue pending) |
| `DS-W6-06` | Agent duty: the conversation partner, in the bot and the Mini App | agent | W6 | per-wave | (issue pending) |
| `DS-W6-07` | Agent duty: practice questions | agent | W6 | per-wave | (issue pending) |
| `DS-W6-08` | Agent duty: the daily digest's coaching | agent | W6 | per-wave | (issue pending) |
| `DS-W6-09` | Agent: the ported vault-ops skill and the on-demand run | agent | W6 | per-wave | (issue pending) |
| `DS-W6-10` | Mini App: the drill workspace and the progress screens | miniapp | W6 | per-wave | (issue pending) |
| `DS-W6-11` | Vault: journal-shaped capture through the inbox | vault | W6 | per-wave | (issue pending) |
| `DS-W7-01` | Mini App: the settings screen for every runtime setting | miniapp | W7 | per-wave | (issue pending) |
| `DS-W7-02` | Identity: linked sign-in with Google, Apple and passkeys | identity | W7 | per-wave | (issue pending) |
| `DS-W7-03` | The public landing page | landing | W7 | per-wave | (issue pending) |
| `DS-W7-04` | Every pack enforced | repo | W7 | per-wave | (issue pending) |
| `DS-W8-01` | The v9 import: plan, oracle and runbook | migration | W8 | per-wave | (issue pending) |
| `DS-W8-02` | Side-by-side verification and the cutover checklist | coordination | W8 | per-wave | (issue pending) |
| `DS-W8-03` | Cutover and the predecessor's retirement | deploy | W8 | per-wave | (issue pending) |
| `DS-W8-04` | Release v1.0.0 | repo | W8 | per-wave | (issue pending) |
| `DS-W9-01` | The vault to Anki flashcard bridge | vault | W9 | per-wave | (issue pending) |
