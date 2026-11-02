# The sealed sync device census

What sealing every sync op body on the device changes, measured at the dev this change is cut from: each context
that reads study data on the server, each of DeckStreak's own tables, each surface the sealing
touches, each record whose sentence it falsifies at the cut-over, and the build items that do the
work. It belongs to SPEC-583 and ADR-602, and `scripts/tests/test_sealed_sync_records.py` holds
its two tables equal to the context map.

A verdict is one of six words:

- `stays`: the server keeps it, fed by the daily record or by data that is not study content.
- `moves`: it moves to the device, and the server keeps none of it.
- `split`: the part over counts, minutes and XP stays; the part that reads content moves.
- `held`: it moves to the device once its parked surface reopens, and not before.
- `retires`: it belongs to the stock-app sync path and ends at the cut-over.
- `unchanged`: the sealing does not touch it.

## 1. The contexts that read study data through `ingest`

The context map's fence names nine contexts that depend on `ingest` (`docs/CONTEXT-MAP.md:16`,
`:17`, `:19`, `:21`, `:24`, `:27`, `:30`, `:33`, `:40`). Their sizes are each crate's tracked files
under `src/` and `tests/`, in bytes, from `git ls-tree -r -l` at dev; a parked file is counted by
name and never opened.

| context | verdict | src files | src bytes | test files | test bytes | parked files | proposed item |
|---|---|---|---|---|---|---|---|
| `analytics` | moves | 8 | 62257 | 5 | 47422 | 0 | #1090 |
| `progression` | split | 21 | 94094 | 27 | 445749 | 0 | #1091 |
| `curriculum` | moves | 7 | 43603 | 8 | 49912 | 0 | #1092 |
| `quests` | split | 8 | 60990 | 9 | 75568 | 0 | #1093 |
| `discipline` | stays | 1 | 685 | 0 | 0 | 0 | #1084 |
| the reading context | held | 8 | 80197 | 7 | 74266 | 0 | #1098 |
| `insights` | moves | 4 | 19010 | 3 | 15889 | 0 | #1094 |
| `coordination` | split | 56 | 315946 | 54 | 771329 | 5 | #1095 |
| `daemon` | split | 11 | 162961 | 17 | 219243 | 4 | #1095 |

- `analytics` moves: the daily rollup, the per-language numbers, the five-pillar score and the
  today snapshot all read review rows.
- `progression` splits: the XP ledger, the settlement and the levels read the daily record's XP
  granted; a badge or a record that reads a deck or a score moves.
- `curriculum` moves: the road to C2, the CEFR bands, Can-Do, the law dues and the leeches read
  card and note content.
- `quests` splits: chests, pity and tokens stay; a quest or a chest earned by a session or a deck
  takes its evidence from the device.
- `discipline` stays: its commitment devices settle on the daily record (one source file at dev).
- the reading context is held: its topic days name card and note ids, and part of its code lies on
  the parked surface, counted by name only; it moves once that surface reopens.
- `insights` moves: its research instruments read the collection.
- `coordination` splits: its jobs stop reading the collection and read the daily record; 5 of its
  files lie on the parked surface, counted by name only.
- `daemon` splits: the composition root drops the `ingest` wiring at the cut-over; 4 of its files
  lie on the parked surface, counted by name only.

Outside the nine, `ingest` itself retires at the cut-over; `streaks`, `economy` and
`notifications` stay over the daily record; `identity` gains the key slots beside its passkeys;
`privacy` changes its export and erase; `habits` keeps its logs, while a minutes entry's note text
moves into a sealed op; and the AI duties of `agent` and `mcp`, which read card text, stop over
sealed data until AI over sealed data has its own design.

## 2. DeckStreak's own tables

The context map's register of DeckStreak's own tables (`docs/CONTEXT-MAP.md:226-273`) holds 46
rows. 44 are listed below with a verdict: 22 stay, 9 move, 3 split, 2 are held, 3 retire and 5 are
unchanged.

| table | owning context | verdict | fed by | proposed item |
|---|---|---|---|---|
| `settings_generation` | `kernel` | stays | service counters, no study data | none |
| `_sqlx_migrations` | `kernel` | unchanged | the schema version table | none |
| `sync_runs` | `ingest` | retires | the stock-app sync path's record; the sealed relay keeps its own | #1086 |
| `ingest_state` | `ingest` | retires | the stock-app sync path's recompute state | #1086 |
| `skip_days` | `ingest` | moves | the skip day becomes a device op; its streak bridge is a clear request | #1096 |
| `skip_card_snapshot` | `ingest` | moves | card scheduling before and after a skip, on the device | #1096 |
| `write_class_stop` | `ingest` | retires | the stock-app write gate | #1086 |
| `cron_fires` | `coordination` | unchanged | job ledger, no study data | none |
| `instrument_reports` | `coordination` | moves | research instruments read the collection | #1094 |
| `xp_ledger` | `progression` | stays | the daily record's XP granted | #1084 |
| `xp_settlement` | `progression` | stays | the daily record's XP granted | #1084 |
| `buffs` | `progression` | stays | the daily record | #1091 |
| `badges_earned` | `progression` | split | count badges on the daily record; content badges on the device | #1091 |
| `records` | `progression` | split | most reviews and minutes on the daily record; the best score on the device | #1091 |
| `streak_state` | `streaks` | stays | the daily record's date and review count | #1084 |
| `freeze_events` | `streaks` | stays | the daily record | #1084 |
| `habit_strength` | `streaks` | stays | the daily record | #1084 |
| `relight_due` | `streaks` | stays | the daily record | #1084 |
| `governor_state` | `streaks` | stays | the daily record | #1084 |
| `coin_ledger` | `economy` | stays | server grants over the daily record | #1084 |
| `economy_state` | `economy` | stays | the shop, no study data | none |
| `language_progress` | `curriculum` | moves | course content and card states | #1092 |
| `band_milestones` | `curriculum` | moves | course content and card states | #1092 |
| `law_dues` | `curriculum` | moves | the law cards due | #1092 |
| `chests` | `quests` | split | chests stay; an earning that reads a session or a deck moves its evidence | #1093 |
| `pity` | `quests` | stays | the chest counters | none |
| `xp_tokens` | `quests` | stays | tokens from chests | none |
| `chest_settings` | `quests` | stays | the learner's chest settings | none |
| `notification_decisions` | `notifications` | stays | the router, over the daily record | #1084 |
| `notification_deliveries` | `notifications` | stays | the router | none |
| `notification_queue` | `notifications` | stays | the router | none |
| `in_app_feed` | `notifications` | stays | the router | none |
| `notification_settings` | `notifications` | stays | the learner's choices | none |
| `owner_last_message` | `notifications` | unchanged | the deprecated bot's | none |
| `reading_runs` | the reading context | held | moves to the device after the parked surface reopens | #1098 |
| `reading_topic_days` | the reading context | held | moves to the device after the parked surface reopens | #1098 |
| `agent_runs` | `agent` | unchanged | run records; AI over sealed data is a later delivery | #1089 |
| `daily_rollup` | `analytics` | moves | each day's numbers from the reviews | #1090 |
| `daily_lang_stats` | `analytics` | moves | each course's reviews per day | #1090 |
| `inbox_captures` | `vault` | unchanged | the owner's vault, not sync data | none |
| `minutes_log` | `habits` | stays | learner input; its note text moves into a sealed op | #1097 |
| `writing_log` | `habits` | stays | learner confirmations, no content | none |
| `passkeys` | `identity` | stays | sign-in credentials; the passkey slots sit beside them | #1082 |
| `sensitive_decks` | `ingest` | moves | deck marks for the AI gate, kept beside the decks on the device | #1096 |

Parked register rows counted, not listed: 2.

Those two rows belong to the vault, are not sync data, and are unchanged. The predecessor's
64-table register (`docs/CONTEXT-MAP.md:148-211`) is the planned import's: it follows each table
this census moves to the device.

## 3. The surfaces the sealing changes

- **The stock-app sync path:** the sync server's store and its two fixed logins
  (`deploy/scripts/sync-server.sh:78-80`; `deploy/systemd/deck-streak-sync-server.service:26`,
  `:30-31`), its daily snapshots (`deploy/scripts/backup.py:50-52`) and their sealed offsite
  archive (ADR-351, `:47-48`). It retires at the cut-over.
- **The private copy:** `ingest` syncs and reads the collection, and writes the skip day back
  (`crates/ingest/src/lib.rs:3-6`; `crates/ingest/src/settings.rs:26`). It retires at the cut-over.
- **The op format and its judge:** the sync core arrives in an open sibling change; SPEC-583 R5
  fixes its clear head, and its judge reads that head alone.
- **Export and erase:** `crates/privacy/src/lib.rs:3-14` exports every port's tables as one JSON
  document and erases them in one transaction. Over sealed data the server exports its own tables,
  the daily records, the key records and the sealed ops as ciphertext, and the device exports the
  plaintext; the erase deletes the sealed ops, the media objects, the key records and the list
  versions, so no copy can be opened again.
- **Backups and archives:** the database's daily copies and its replica (`deploy/scripts/backup.py:43-48`;
  `deploy/litestream.yml:9-19`) hold sealed bodies once the relay lands, and a restore rehearsal
  then checks integrity, never content.
- **The identity and passkey surfaces an unlock rides:** the `passkeys` table, keyed by the
  Telegram user id (`migrations/035901_identity_passkeys.sql:10-20`); the ceremonies
  (`crates/identity/src/passkeys.rs:22-24`); the web client's ceremony JSON, which posts
  `extensions: {}` (`web/app/src/lib/passkeys.ts:140-170`); today's web sync key, sealed in the
  Worker (ADR-374, `:22`, `:60`); and the app's Keychain item
  (`ios/App/Sources/SyncCredentialStore.swift:9`, `:38`).
- **The privacy page:** what it keeps (`PRIVACY.md:3-6`, `:8-54`), the private copy and the sync
  server's store (`PRIVACY.md:56-62`), the web app's sealed key and its backups
  (`PRIVACY.md:64-71`, `:73-76`), what an erase does not reach (`PRIVACY.md:97-124`) and what
  reaches a model (`PRIVACY.md:126-135`).
- **The tests and rows that pin them:** `scripts/tests/test_privacy_policy.py` (each copy an erase
  cannot reach, `:86-97`, `:116-138`); `scripts/tests/test_context_map_register.py` and
  `crates/kernel/tests/schema.rs` (the register and the migrations' owners);
  `crates/coordination/tests/data_rights_symmetry.rs` (export and erase symmetry);
  `web/app/src/lib/passkeys.test.ts:122`, `:131` (the posted `extensions: {}`); and the passkey rows
  of `scripts/mutation-rows.d/S35900-S35999.json`.

## 4. Records whose sentences the sealing falsifies at the cut-over

Each sentence is true at dev, because it describes the stock-app path, and the cut-over item
amends it when that path retires (#1086):

- `ARCHITECTURE.md:9`: DeckStreak syncs a private copy of its owner's collection.
- ADR-009 (`:12`) and ADR-037 (`:11`): the service syncs and reads a private copy.
- SPEC-021 (`:30`, `:47`): the private copy is personal data outside the database, and the export
  reads every port's tables.
- ADR-336 (`:6`): the web client's fallback is a study collection on the server.
- ADR-351 (`:47-48`): the sync server's archive is sealed to the owner's offline key.
- ADR-374 (`:22`, `:60`, `:250-251`) and SPEC-363: today's web sync key.
- ADR-358 (`:59-60`) and SPEC-347: today's Keychain item for the sync host key.
- ADR-392 (`:16-17`): an AI duty's input carries card text.
- `PRIVACY.md:56-62`, `:64-71`, `:97-124` and `:126-135`: the private copy and the sync server's
  store, the web app's sealed key, the sync server's snapshots and what reaches a model.
- `crates/ingest/src/lib.rs:3-6`: the private copy and the skip day's write.

## 5. Proposed build items

Each is a proposed issue; the census's `proposed item` cells and SPEC-583 section 5 cite them.

- #1080: the sealing core in Rust for both apps: the envelope, the padding, the slots,
  the HPKE wraps, the device list, their test vectors, a TLA+ model of a rotation racing a push and
  a join, and a Lean proof of the padding rule and the record's bounds.
- #1081: the account key, the device key and the passkey PRF slot on the iPhone and iPad
  app.
- #1082: the account key, the device key and the passkey PRF slot in the web app, with
  the posted ceremony JSON pinned to `extensions: {}`.
- #1083: the relay that stores sealed ops, slots, list versions, media objects and device
  checkpoints, and that depends on no cipher crate.
- #1084: the daily record's route and checks, and the streaks, freezes, coins, quests and
  push budgets it feeds.
- #1090, #1091, #1092,
  #1093, #1094, #1095, #1096 and
  #1097: each context's move, by the verdicts above.
- #1098: the reading context's move, held until its parked surface reopens.
- #1085: export and erase over sealed data.
- #1086: the cut-over: the stock-app path's end, the staged privacy text installed, and
  the records of section 4 amended.
