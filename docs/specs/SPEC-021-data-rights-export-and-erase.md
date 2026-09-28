# SPEC-021: export and erase run over every context's port, touch the same tables, and leave nothing recoverable

- **Wave:** W0. **Issue:** #14 (epic #1). **Context(s):** `deck-streak-privacy` (the engine), `deck-streak-coordination` (the registry of every port, the export and erase use cases), `deck-streak-daemon` (the `data` role), `repo` (`privacy.json`, `PRIVACY.md`, the Litestream and journald templates).
- **Decided by:** ADR-008 (one database, each context owns its tables), ADR-010 (backups), ADR-002 (the registry crosses contexts, so it lives in coordination), ADR-020 (the data-rights port, one migration sequence).
- **Status:** judged: delivered with its tests and `docs/red-first/SPEC-021.md` (ADR-016). The
  delivery made R2 to R8, A1, A2 and the manifest exact where the code decided them, amended the
  manifest by two files, each with its reason, and reworded §6 under the public-prose rule (§7).

## 1. The problem, measured

- **The predecessor's invariant** (predecessor `27ee2bc`, names only; its privacy policy): the
  tables `database.py:GamifyStore.erase_all_user_data` touches and the tables
  `database.py:GamifyStore.dump_all` returns are provably equal, by a test that enumerates the live
  schema and fails on a table neither exported, erased nor exempt (`test_privacy_completeness.py`);
  singletons are RESET in place, never deleted (`database.py:_ERASE_SINGLETONS`); the schema-version
  table and `cron_fires` are exempt from both, so an erase can never re-arm the catch-up
  double-send guard; offsite copies that survive an erase are disclosed, not hidden. CHARTER 13
  makes all of it binding.
- **What DeckStreak has at this SPEC's base** (W0's tables, each behind its context's port):
  `settings_generation` (kernel, reset in place), `_sqlx_migrations` (kernel, exempt),
  `sync_runs` (ingest, exported and erased), `ingest_state` (ingest, reset in place), `cron_fires`
  (coordination, exempt). No engine runs the ports, no `privacy.json` exists, and the privacy-gdpr
  pack is pending on this issue in `.packs/wiring.json`.
- **What the pack demands once `privacy.json` exists** (the privacy-gdpr pack; every row runs, and
  a red one fails the gate): the inventory rows (`inventory-valid`, `schema-covered`,
  `retention-bounded`, `purpose-limited`), and also the rights, design and public rows over the
  export and erase code, the policy, the Litestream file and the journald drop-in the inventory
  names. `inventory-valid` requires the `backups`, `logs` and `policy` blocks, and every file they
  name must exist.
- **Personal data outside the database.** The private copy of the collection (the owner's own Anki
  data, refreshed from their sync server) and the journal; and, in memory only, the owner's
  Telegram id in a session (SPEC-024).

**Order.** After SPEC-020 (the port), SPEC-022 and SPEC-023 (the ingest tables), SPEC-025 (the
`deckstreakd` binary and its `roles` test target, which the `data` role joins), SPEC-027
(`cron_fires`, whose exemption this proves on the real table) and SPEC-028 (the `/about` route this
SPEC names as a policy entry point). SPEC-026 lands after it: the bot's `/export` and `/delete` call
this SPEC's use cases, and add the bot's entry point to `privacy.json`. From here on, a delivery that
creates a table adds it to its port and to `privacy.json` in the same change (A2 and A8 fail until
it does).

## 2. Requirements

R1. `coordination::data_rights_registry::ports` returns every stateful context's `DataRights` port
    (at W0: kernel, ingest, coordination); `export_all` and `erase_all` are the one pair of use cases
    every surface calls.
R2. `privacy::export` reads each port's exported and reset tables and returns one JSON document:
    `schema` `deckstreak.export.v1`, then one key per table holding its rows (a singleton's one row
    included). Exempt tables are absent.
R3. `privacy::erase` runs in ONE `BEGIN IMMEDIATE` transaction on a connection with
    `PRAGMA secure_delete = ON`: `DELETE FROM` each exported table, the declared reset row written
    over each singleton, nothing done to an exempt table. After it commits: `VACUUM`, then
    `PRAGMA wal_checkpoint(TRUNCATE)`, so neither the database file nor its WAL holds an erased
    value. A port that fails rolls the whole erase back.
R4. The set of tables `export` returns equals the set `erase` touches, over the real registry, and
    every table of the migrated schema (the SQLite internal tables aside) is declared by exactly one
    port; both are tests that print their examined count and refuse zero.
R5. `privacy.json` (schema `phx.privacy.v1`) declares a category for every table a port exports or
    erases, with its data, source, stores, purpose, Article 6(1) basis, retention, export and erase:
    `sync-history` (`sync_runs.*`, erase `delete`), `ingest-state` (`ingest_state.*`, erase
    `anonymise`, because a reset is an update in place) and `service-counters`
    (`settings_generation.*`, erase `anonymise`); retention `until: account-deletion` for each, so
    no purge job is needed at W0. It names `migrations/*.sql`, the ports' and the engine's source as
    the export and erase code (format `json`), `deploy/litestream.yml` with a backups window of
    `P3D`, `deploy/journald.conf.d/deck-streak.conf` with a logs window of `P14D`, and `PRIVACY.md`
    with its entry points (`README.md`, and the Mini App's `/about` route).
R6. `deploy/litestream.yml` is a Litestream 0.5 template with one replica, a global snapshot
    interval of 24h and retention of 48h (inside the declared `P3D`), and no replica-level
    retention; its destination is a placeholder the private rail fills. The journald drop-in sets
    `MaxRetentionSec=14day` and a `SystemMaxUse=` cap. Both are templates; nothing is installed.
R7. `PRIVACY.md` gives one line per category naming its id, basis and retention; states the backup
    and log windows; and discloses what an erase does not reach: the Litestream replica for its
    window, the journal for its window, the private collection copy (the owner's own Anki data,
    which the next sync would restore; removing it means removing the sync credential, an
    owner-setup step), and the cron-fire ledger (kept, and pruned after 90 days, so an erase cannot
    re-arm a double send). The offsite backup and its window are added when W2 builds them.
R8. `deckstreakd data export` writes the export to standard output; `deckstreakd data erase` erases
    only with `--confirm ERASE` (the predecessor's confirmation word) and refuses otherwise with code
    2.
R9. `.packs/wiring.json` moves `privacy-gdpr` to `enforced`; `defaults-private`, whose subject is
    the settings screen's default file, is listed under `deferred_rows` with that screen's issue.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the exported tables equal the erased tables over every registered port (examined count, zero refused) | coordination `data_rights_symmetry` test |
| A2 | every table of the migrated schema is declared by exactly one port | coordination `data_rights_symmetry` test |
| A3 | erase resets singletons in place | privacy `erase` test |
| A4 | erase leaves `cron_fires` and `_sqlx_migrations` untouched | coordination `data_rights_symmetry` test |
| A5 | an erased value is absent from the database file and its WAL | privacy `erase` test; privacy-gdpr `erase-effective` |
| A6 | a failing port rolls the whole erase back | privacy `erase` test |
| A7 | the export is one JSON object keyed by table, singletons included, exempt tables absent | privacy `export` test; privacy-gdpr `export-complete` |
| A8 | `privacy.json` names every table a port exports or erases, in exactly one category | coordination `data_rights_symmetry` test; privacy-gdpr `inventory-valid`, `schema-covered` |
| A9 | the policy discloses every copy an erase cannot reach, and each category's basis and retention | `test_privacy_policy.py`; privacy-gdpr `policy-published`, `copies-bounded` |
| A10 | the `data` role erases only with the confirmation word | daemon `roles` test |

```acceptance
A1: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact the_exported_tables_equal_the_erased_tables_over_every_port
A2: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact every_table_of_the_schema_is_declared_by_exactly_one_port
A3: cargo test -p deck-streak-privacy --test erase -- --exact erase_resets_singletons_in_place
A4: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact erase_leaves_the_cron_fire_ledger_and_the_schema_table_untouched
A5: cargo test -p deck-streak-privacy --test erase -- --exact an_erased_value_is_absent_from_the_database_file_and_its_wal
A6: cargo test -p deck-streak-privacy --test erase -- --exact a_failing_port_rolls_the_whole_erase_back
A7: cargo test -p deck-streak-privacy --test export -- --exact the_export_is_one_json_object_per_table
A8: cargo test -p deck-streak-coordination --test data_rights_symmetry -- --exact privacy_json_names_every_table_the_ports_export_or_erase
A9: python3 -m unittest discover -s scripts/tests -p test_privacy_policy.py -k the_policy_discloses_every_copy_an_erase_cannot_reach
A10: cargo test -p deck-streak-daemon --test roles -- --exact the_data_role_erases_only_with_the_confirmation_word
```

A5 plants a value no other test uses, erases, and searches both files' bytes for it; it also
searches before the erase, so the test cannot pass on a value that was never written. The privacy
tests use synthetic ports over a temporary database; the coordination tests use the real registry
over a fully migrated temporary database.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/privacy/Cargo.toml`, `crates/privacy/src/lib.rs` | `deck-streak-privacy` | changed |
| `crates/privacy/src/export.rs` | `deck-streak-privacy` | added |
| `crates/privacy/src/erase.rs` | `deck-streak-privacy` | added |
| `crates/privacy/tests/export.rs`, `erase.rs` | `deck-streak-privacy` | added: A3, A5 to A7 |
| `crates/coordination/src/data_rights_registry.rs`, `crates/coordination/src/lib.rs` | `deck-streak-coordination` | added or changed: the registry, `export_all`, `erase_all` |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | added: A1, A2, A4, A8 |
| `crates/daemon/src/role_data.rs`, `crates/daemon/src/main.rs` | `deck-streak-daemon` | added or changed: the `data` role |
| `crates/daemon/tests/roles.rs` | `deck-streak-daemon` | changed: A10, in the roles test target SPEC-025 adds |
| `privacy.json` | repo | added |
| `PRIVACY.md` | repo | added |
| `README.md` | repo | changed: a privacy section linking `PRIVACY.md` |
| `deploy/litestream.yml` | repo | added: template |
| `deploy/journald.conf.d/deck-streak.conf` | repo | added: template |
| `scripts/tests/test_privacy_policy.py` | repo | added: A9 |
| `.packs/wiring.json` | repo | changed: privacy-gdpr enforced, one row deferred |
| `docs/schematics/data-rights-export-and-erase.md` | repo | added |
| `docs/red-first/SPEC-021.md` | repo | added |
| `changelog.d/` fragment | repo | added |
| `Cargo.lock` | repo | changed: the privacy crate's dependency edges, no version (§7) |
| `scripts/tests/test_deploy_templates.py` | repo | changed: the Litestream unit's advisory waits on its issue (§7) |

## 5. What this does NOT do

- It adds no bot command: `/privacy`, `/export` and `/delete` call this SPEC's use cases and name
  the bot as a policy entry point (#19).
- It adds no settings screen, so the default-settings row waits for it (#57).
- It installs no replica, bucket or journald setting on any host, and builds no daily backup or
  offsite copy; the policy gains their windows when they exist (#44).
- It purges nothing by age: no W0 category keeps data for a fixed period; the ledger's own pruning
  is the scheduler's (#20).
- It imports no predecessor data (#61).
- It publishes nothing: the public achievement export has its own scrub (#156).

## 6. Risks

- **A later table is added without its port or category.** A2 and A8 fail in that delivery's gate,
  and privacy-gdpr's `schema-covered` refuses an undeclared personal column.
- **`VACUUM` on a live database.** It needs free disk equal to the database's size and holds the
  write lock while it runs; DeckStreak's database holds derived rows, never the collection,
  so it stays small, and an erase is a rare owner action.
- **The journald drop-in is host-wide.** `MaxRetentionSec` and `SystemMaxUse` are journald's own
  settings, not one unit's, so installing the drop-in is a change on the host that is the owner's
  gate (#161); the first deploy makes it only with the owner's go (#42).
- **An erase while a sync job runs.** Both write through `BEGIN IMMEDIATE`, so they serialise on the
  write lock; the erase then resets `ingest_state`, and the next cycle recomputes from an empty
  anchor, which is the intended result.

## 7. Amendments at delivery

- **§6: the journald risk is reworded under the public-prose rule.** The planned text described what
  else a host runs; the risk now says only that the drop-in's settings are journald's own and that
  installing it is a change on the host behind the owner's gate (#161), which the first deploy makes
  with the owner's go (#42).
- **R2: the export is a type of its own.** `privacy::export` returns `privacy::Export`, one JSON
  object (`as_json`, and `to_line` for the `data` role), so coordination's registry and the daemon
  name no serde type and neither manifest changes. Its keys are the schema key and one per table,
  in serde_json's sorted order, so `schema` need not come first; the document is one object either
  way.
- **R2, R3: the declarations are checked before anything runs.** A table two ports declare, or one
  named `schema`, the export's own key, is refused by name before a row is read or written; and a
  port whose export returns a table it does not declare, returns one twice, or leaves one out is
  refused by its context and the table. The export reads every port in one read transaction, so
  the document is one snapshot.
- **R3: the erase checks every port's work before it commits.** Inside the erase's transaction each
  port's own export is read back: every table it exports must be empty and every singleton must hold
  exactly one row carrying its reset values, or the erase is refused by the context and the table
  and rolls back. `PRAGMA secure_delete = ON` is read back, and a connection where it does not
  answer 1 is refused before anything is deleted. The connection keeps `secure_delete` on in the
  pool. A compaction that fails after the commit is its own refusal, named apart from every refusal
  that erased nothing; a checkpoint that meets a reader reports busy, is not an error, and the
  erase's report and the `data` role's log say the log was held
  (`docs/schematics/data-rights-export-and-erase.md`).
- **R4, A1, A2: measured, the predecessor's way.** A table counts as exported when the export
  carries every one of its rows (101 seeded rows where a table takes rows, so an export that pages
  or limits its read comes up short), and as erased when none of its seeded rows survives the erase
  unchanged. A1 also holds both sets equal to the tables the ports declare exported or reset, and
  A2 also holds each table's one port equal to its owner in the ownership register.
- **R5: what else `privacy.json` carries.** Its export and erase code globs are
  `crates/*/src/data_rights.rs`, so a later context's port joins by its module's name, and the
  engine's two files; its `code` globs are the Rust and the Mini App sources the pack reads for
  client storage, contact requests and log calls. Each category's source is `derived` and its basis
  `contract`. Its `public.allow` names ten literals the pack's scrub reads as addresses in files
  this SPEC does not touch: eight systemd unit instance names, by their `@instance.type` tails, a
  release `v1.2.3.4` and a Python slice. SPEC-054 R2 passed the same unit names in the
  repository's own scrub.
- **R6: the templates' other settings.** The Litestream template also sets the global `validation`
  interval (Litestream 0.5's periodic check of its LTX files), so the durable-services advisory
  that asks for it holds. Its replica URL is `${DECKSTREAK_REPLICA_URL}`, a placeholder with no
  scheme that Litestream expands from the environment, so the deferred `backup.offsite` row stays
  red until the backups issue builds the copy (#44). The journald cap is `SystemMaxUse=256M`.
- **R7: the policy says what is.** `PRIVACY.md` names no offsite backup: none exists, and a public
  page shows what is, never what is planned (CHARTER 12). The offsite copy and its window join the
  policy with the delivery that builds them (#44).
- **R8: the `data` role is a module of the binary.** `crates/daemon/src/role_data.rs` is compiled
  into `deckstreakd` through `main.rs`, as the `job` role is, so the daemon's library is unchanged.
  Any erase other than exactly `data erase --confirm ERASE` is a usage refusal, code 2, before the
  database is opened. The export is the one line of standard output that does not open with a
  journal priority, written through the kernel's offload, since the daemon's tokio carries no
  standard-stream feature.
- **Manifest: `Cargo.lock` and SPEC-032's advisory test.** The privacy crate gains five dependency
  edges and no version: `serde_json` (ADR-029), `sqlx` and `thiserror` (ADR-003), and `tempfile` and
  `tokio` in its tests (ADR-020). `scripts/tests/test_deploy_templates.py` required every
  durable-services advisory to be waived; the Litestream template raises `litestream-unit-missing`,
  whose subject, the unit that runs `litestream replicate`, is the backups issue's (#44), so the
  test now expects exactly that departure while it waits, and fails once it stops firing. Its
  exemption for the journal's size cap is gone, because the drop-in sets `SystemMaxUse=`. The
  schematic was planned with the SPEC, so the delivery changes it rather than adding it.
