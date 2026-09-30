# SPEC-020: the kernel gives every context the study day, a clock, settings, secrets, redacted logs, the database base and the data-rights port

- **Wave:** W0. **Issue:** #11, #12, #13 (epic #1). **Context(s):** `deck-streak-kernel`.
- **Decided by:** ADR-002 (the kernel is the shared kernel), ADR-003 (sqlx, tokio, tracing, thiserror), ADR-008 (one SQLite database, WAL, `BEGIN IMMEDIATE`, sqlx migrations), ADR-010 (secrets as systemd credentials), ADR-012 (goldens), ADR-029 (the golden reader, `serde`), and this SPEC's ADR-020 (the study day as an epoch day under a fixed offset; one migration sequence; the logging setup in the kernel).
- **Status:** judged: delivered with its tests, `docs/red-first/SPEC-020.md`, and three goldens
  (`digest_hour`, `redaction`, `kernel.constants`) generated at the predecessor's `27ee2bc`. The
  delivery made R3 and R10 to R15 exact where the code decided them (§7).

## 1. The problem, measured

- **Nothing exists yet.** `crates/kernel/src/lib.rs` holds only its crate documentation and lint
  attributes, and `crates/kernel/Cargo.toml` declares no dependency (read at `main` e05dfa5). Every
  other W0 SPEC needs something only the kernel may hold, because the contexts that need it may not
  depend on each other (CONTEXT-MAP, layer 1).
- **The predecessor's rules this ports** (predecessor `27ee2bc`, names only):
  - the study day, `analytics.py:study_day`: the local date of an instant in a FIXED UTC offset
    (`types.py:CollectionConfig.tzinfo`), after subtracting the rollover hour; defaults
    `constants.py:DEFAULT_ROLLOVER_HOUR` and `DEFAULT_DIGEST_HOUR`; bounds 0 to 23 and -720 to
    +840 minutes (`config.py:Settings.validate`);
  - the digest hour: an unset value resolves to the larger of the default and the rollover hour,
    so only an explicitly set digest hour earlier than the rollover is refused at start
    (`config.py:_env_digest_hour`, `config.py:Settings.validate`);
  - secrets registered with a log filter before use, and every log line scrubbed of them and of
    Telegram token shapes (`logging_redact.py:register`, `SecretRedactingFilter`, the constants
    `_REDACTED`, `_TELEGRAM_TOKEN_RE`, `_MIN_SECRET_LEN`);
  - blocking work on one bounded worker pool (`offload.py:run_offloaded`,
    `OFFLOAD_MAX_WORKERS`, `SLOW_OFFLOAD_MS`);
  - the connection's pragmas: WAL, `synchronous=NORMAL`, foreign keys on, busy timeout
    `database.py:DB_BUSY_TIMEOUT_MS` (`database.py:GamifyStore.connect`);
  - the owner-config generation the change gate compares, bumped by every value-changing runtime
    setting (`pipeline.py`, key `sync_gate_config_generation`).
- **What the parity oracle proves.** The study day against `goldens/study_day.json` (SPEC-029);
  the digest-hour resolution against `goldens/digest_hour.json` (an adapter over
  `config.py:_env_digest_hour`); the scrubbed text against `goldens/redaction.json` (an adapter over
  `logging_redact.py:SecretRedactingFilter`); every constant above against
  `goldens/kernel.constants.json`. This SPEC registers the last three in
  `tools/parity-oracle/registry/spec_020.py`.
- **The skeleton's debt this closes.** `.packs/wiring.json` defers the ddd pack's `lexicon-locks`
  row to this issue because the skeleton declares no identifier.

**Order.** SPEC-029 lands before this SPEC (its A1 reads `goldens/study_day.json` through the reader
SPEC-029 adds). This SPEC is then the first Rust delivery of W0 and runs alone among them: every
other Rust SPEC of W0 needs it. SPEC-028 (web only) and SPEC-030 (scripts only) need nothing from it
and may run beside it; after it, SPEC-022 and SPEC-025 start in parallel.

## 2. Requirements

The study day, the clock and the value types (#11)

R1. `StudyDay` is an epoch day number: for an instant `t` (epoch milliseconds), a rollover hour `h`
    and a UTC offset of `m` minutes, it is `floor((t + m*60000 - h*3600000) / 86400000)`, and it
    equals the output of every case of `goldens/study_day.json`, rollover, negative and offset
    classes included. No time-zone database is used (ADR-020).
R2. `StudyDayRule` holds the rollover hour (0 to 23, default `DEFAULT_ROLLOVER_HOUR`) and the UTC
    offset in minutes (-720 to +840; the repository's default is 0, and the owner's offset is
    private configuration), parsed from `DECKSTREAK_ROLLOVER_HOUR` and
    `DECKSTREAK_UTC_OFFSET_MINUTES`.
R3. The digest hour comes from `DECKSTREAK_DIGEST_HOUR`; unset, it is the larger of
    `DEFAULT_DIGEST_HOUR` and the rollover hour, equal to `goldens/digest_hour.json`. An explicitly
    set digest hour earlier than the rollover hour refuses start with
    `SettingsError::DigestBeforeRollover`, naming both settings and neither value (CHARTER 7). A
    malformed digest hour is refused like any malformed setting (R10); the predecessor fell back to
    its default instead, and ADR-020 records that divergence.
R4. The repository's defaults for the rollover and digest hours equal the hours of
    `notifications-policy.json`'s `digest.rollover` and `digest.at`, held equal by a test, so the
    one value in two files cannot drift.
R5. `StudyDay` renders as an ISO calendar date (proleptic Gregorian, for the API) and parses back
    to the same day, with integer arithmetic only.
R6. `Clock` is a port with one method, the current instant as `UtcMillis`. `SystemClock` is the only
    reader of the system time in the workspace's production code; `ManualClock` (public, for tests
    and replays) is set and advanced explicitly. Every kernel function that depends on the time
    takes a clock or an instant.
R7. The kernel's ids are newtypes with no arithmetic and no conversion from a bare integer except a
    named constructor: `TelegramUserId` (the owner, the bot and the router all name it, and
    identity, bot and notifications may not depend on each other) and `UtcMillis`.
R8. `Track` is `Language` or `Law`, serialised as `language` and `law`.
R9. `Verdict<R>` is `#[must_use]`, with `Pass` and `Refuse(R)`; a `compile_fail` doc test on the
    type shows that dropping one does not compile under the workspace's `unused_must_use = "deny"`.

Settings, credentials, logs and the offload (#12)

R10. Settings are parsed once, at start, from an environment map that the caller passes in (the
    daemon's `main` is the one reader of the process environment), into typed values. A missing
    required setting refuses start with `SettingsError::Missing` naming the setting; a malformed
    one with `SettingsError::Malformed` naming the setting and the shape it expects. No error, log
    line or panic message ever carries a setting's value.
R11. A secret is read only as `<credentials directory>/<credential id>`, the directory being the
    `$CREDENTIALS_DIRECTORY` systemd passes, handed in by the caller. No production code reads a
    secret-named environment variable (the rust-service row `rs.no-secret-env`, enforced). A
    missing credential refuses start with `CredentialError::Missing` naming its id. One trailing
    newline is trimmed.
R12. Every credential of at least `_MIN_SECRET_LEN` characters is registered with the `Redactor`
    before the loader returns it. The redactor replaces every registered value, longest first, and
    every match of `_TELEGRAM_TOKEN_RE` with `_REDACTED`; its output equals every case of
    `goldens/redaction.json`.
R13. `logging::install(redactor)` installs the one subscriber of a process: JSON events on stdout,
    each line prefixed with its sd-daemon priority (`<3>` error, `<4>` warn, `<6>` info, `<7>`
    debug and trace), the level taken from `RUST_LOG` (info when unset), written through a writer
    that applies the redactor to every line before it leaves the process. A second install returns
    an error; it never panics. (The observability pack's `logging.rs.template`; rows
    `obs.subscriber-installed`, `obs.structured-logs`, `obs.journal-priority`.)
R14. `Offload` runs blocking work on tokio's blocking pool behind a semaphore of
    `DECKSTREAK_OFFLOAD_WORKERS` permits (default `OFFLOAD_MAX_WORKERS`). A call names its
    operation, and a call slower than `SLOW_OFFLOAD_MS` logs one WARN event with the operation and
    its duration, never its arguments.

The database base and the data-rights port (#13)

R15. `Db::open` opens DeckStreak's database with `journal_mode=WAL`, `synchronous=NORMAL`,
    `foreign_keys=ON` and a busy timeout of `DB_BUSY_TIMEOUT_MS`, then applies the embedded
    migrations (`MIGRATOR`, `sqlx::migrate!` over `migrations/`, with `crates/kernel/build.rs`
    printing `cargo:rerun-if-changed` for that directory) before it returns.
R16. Every write goes through `Db::write`, which begins its transaction with `BEGIN IMMEDIATE`
    (`begin_with`), so two writers serialise on the write lock under the busy timeout instead of
    failing mid-transaction.
R17. `Db::open_foreign_read_only` opens any other SQLite file (the Anki collection copy) with
    `mode=ro` and `query_only`, never changing its journal mode. Only `crates/kernel/src/db.rs`
    constructs SQLite connect options or a pool (ledger-sqlite `connect-options-in-one-place`,
    `no-raw-connection-outside-repository-base`).
R18. Migrations live in one directory, `migrations/`, named `<SPEC number, four digits><sequence,
    two digits>_<owning context>_<slug>.sql` (ADR-020); a migration is additive; sqlx applies a
    lower-numbered migration that arrives after a higher one was applied (measured by a test, not
    assumed). Every table has `created_at` and is `STRICT`.
R19. The kernel owns one table, `settings_generation` (one row), created by
    `migrations/002001_kernel_settings_generation.sql`. `Db::bump_settings_generation` increments
    it inside a write; the change gate (SPEC-023) reads it. It is registered in docs/CONTEXT-MAP.md,
    whose ownership register gains a section for DeckStreak's own tables, and `_sqlx_migrations` is
    recorded there as the schema-version table that replaces the predecessor's `schema_versions`.
R20. `DataRights` is the port every stateful context implements: it names its context, declares
    each of its tables once with a `Disposition` (`ExportAndErase`, `ResetInPlace` with the reset
    row, or `Exempt` with a reason), exports its rows as JSON values, and erases inside a
    transaction the caller holds. The kernel refuses a declaration that names a table twice or
    exempts one without a reason. The kernel's own port resets `settings_generation` in place and
    exempts `_sqlx_migrations` (CHARTER 13).
R21. The ddd pack's `lexicon-locks` row judges the kernel's identifiers and is green, and its
    deferral to this issue is removed from `.packs/wiring.json`: no kernel identifier says
    `calendar` (the study day's replaced word, docs/LEXICON.md).
R22. `.env.example` names every setting this SPEC reads, with a neutral example value or none.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the study day equals the predecessor's for every golden case, across the rollover | `study_day` test over `goldens/study_day.json` |
| A2 | a study day renders as its ISO date and parses back | `study_day` test |
| A3 | an explicit digest hour before the rollover hour is refused by name, without values | `settings` test |
| A4 | an unset digest hour resolves as the predecessor's does | `settings` test over `goldens/digest_hour.json` |
| A5 | a missing setting is refused by name | `settings` test |
| A6 | a malformed setting is refused by name, and its value appears nowhere in the error | `settings` test |
| A7 | the default rollover and digest hours equal notifications-policy.json's | `settings` test |
| A8 | the kernel's constants equal the predecessor's | `constants` test over `goldens/kernel.constants.json` |
| A9 | a manual clock carries a study day across the rollover without sleeping | `clock` test |
| A10 | `Verdict` is declared `#[must_use]` (and the `compile_fail` doc test on it runs in the doctest stage) | `verdict` test; `cargo test --doc` |
| A11 | a secret is read from the credentials directory and never from the environment | `credentials` test; rust-service `rs.no-secret-env` |
| A12 | a missing credential refuses start by its id | `credentials` test |
| A13 | a logged secret leaves the process as the redaction marker in the JSON line | `logging` test |
| A14 | the redactor's output equals the predecessor's for every golden case | `logging` test over `goldens/redaction.json` |
| A15 | each JSON line opens with its journal priority | `logging` test; observability `obs.journal-priority` |
| A16 | the offload runs at most its bound of blocking tasks at once | `offload` test |
| A17 | opening the database sets WAL, `synchronous=NORMAL`, foreign keys and the busy timeout | `db` test |
| A18 | two concurrent writers serialise without a lost update | `db` test |
| A19 | a foreign SQLite file opened read-only refuses a write | `db` test |
| A20 | a lower-numbered migration added after a higher one was applied is applied | `db` test |
| A21 | every table of the migrated schema has `created_at` and is `STRICT`, and the embedded migrations are exactly the files in `migrations/` (examined count, zero refused) | `schema` test |
| A22 | a planted table without `created_at` is refused by the same census | `schema` test |
| A23 | every migration names the context that owns its tables in the ownership register | `schema` test |
| A24 | a data-rights declaration names each table once with a disposition | `data_rights` test |
| A25 | an exempt table without a reason is refused | `data_rights` test |
| A26 | the kernel's port resets `settings_generation` in place and exempts the schema table | `data_rights` test |

```acceptance
A1: cargo test -p deck-streak-kernel --test study_day -- --exact the_study_day_matches_the_predecessors_golden
A2: cargo test -p deck-streak-kernel --test study_day -- --exact a_study_day_renders_as_its_iso_date_and_parses_back
A3: cargo test -p deck-streak-kernel --test settings -- --exact an_explicit_digest_hour_before_the_rollover_hour_is_refused_by_name
A4: cargo test -p deck-streak-kernel --test settings -- --exact an_unset_digest_hour_resolves_as_the_predecessors_golden
A5: cargo test -p deck-streak-kernel --test settings -- --exact a_missing_setting_is_refused_by_name
A6: cargo test -p deck-streak-kernel --test settings -- --exact a_malformed_setting_is_refused_by_name_without_its_value
A7: cargo test -p deck-streak-kernel --test settings -- --exact the_default_rollover_and_digest_hours_equal_the_notifications_policy
A8: cargo test -p deck-streak-kernel --test constants -- --exact the_kernel_constants_equal_the_predecessors
A9: cargo test -p deck-streak-kernel --test clock -- --exact a_manual_clock_carries_a_study_day_across_the_rollover_without_sleeping
A10: cargo test -p deck-streak-kernel --test verdict -- --exact the_verdict_type_is_declared_must_use
A11: cargo test -p deck-streak-kernel --test credentials -- --exact a_secret_is_read_from_the_credentials_directory_and_never_from_the_environment
A12: cargo test -p deck-streak-kernel --test credentials -- --exact a_missing_credential_refuses_start_by_its_id
A13: cargo test -p deck-streak-kernel --test logging -- --exact a_logged_secret_leaves_the_process_as_the_redaction_marker
A14: cargo test -p deck-streak-kernel --test logging -- --exact the_redaction_matches_the_predecessors_golden
A15: cargo test -p deck-streak-kernel --test logging -- --exact each_json_line_opens_with_its_journal_priority
A16: cargo test -p deck-streak-kernel --test offload -- --exact the_offload_runs_at_most_its_bound_of_blocking_tasks_at_once
A17: cargo test -p deck-streak-kernel --test db -- --exact opening_the_database_sets_wal_normal_sync_foreign_keys_and_the_busy_timeout
A18: cargo test -p deck-streak-kernel --test db -- --exact two_concurrent_writers_serialise_without_a_lost_update
A19: cargo test -p deck-streak-kernel --test db -- --exact a_foreign_sqlite_file_opened_read_only_refuses_a_write
A20: cargo test -p deck-streak-kernel --test db -- --exact a_lower_numbered_migration_added_after_a_higher_one_is_applied
A21: cargo test -p deck-streak-kernel --test schema -- --exact every_table_of_the_schema_has_created_at_and_is_strict
A22: cargo test -p deck-streak-kernel --test schema -- --exact a_planted_table_without_created_at_is_refused
A23: cargo test -p deck-streak-kernel --test schema -- --exact every_migration_names_the_context_that_owns_its_tables
A24: cargo test -p deck-streak-kernel --test data_rights -- --exact a_data_rights_declaration_names_each_table_once_with_a_disposition
A25: cargo test -p deck-streak-kernel --test data_rights -- --exact an_exempt_table_without_a_reason_is_refused
A26: cargo test -p deck-streak-kernel --test data_rights -- --exact the_kernel_port_resets_the_settings_generation_and_exempts_the_schema_table
```

No test sleeps: A9 and A16 drive a `ManualClock` and channels. A18 runs two tasks against one file in
a temporary directory. Tests that read goldens include SPEC-029's reader with `#[path]`.

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/kernel/Cargo.toml` | `deck-streak-kernel` | changed: sqlx (runtime-tokio, sqlite, migrate, macros), tokio, thiserror, tracing, tracing-subscriber (fmt, json, env-filter), serde, serde_json; dev: tempfile, tokio (macros, rt) |
| `crates/kernel/build.rs` | `deck-streak-kernel` | added: rerun when `migrations/` changes |
| `crates/kernel/src/lib.rs` | `deck-streak-kernel` | changed: the modules below |
| `crates/kernel/src/study_day.rs` | `deck-streak-kernel` | added: `StudyDay`, `StudyDayRule`, ISO rendering |
| `crates/kernel/src/clock.rs` | `deck-streak-kernel` | added: `Clock`, `SystemClock`, `ManualClock`, `UtcMillis` |
| `crates/kernel/src/ids.rs` | `deck-streak-kernel` | added: `TelegramUserId` |
| `crates/kernel/src/track.rs` | `deck-streak-kernel` | added |
| `crates/kernel/src/verdict.rs` | `deck-streak-kernel` | added, with the `compile_fail` doc test |
| `crates/kernel/src/error.rs` | `deck-streak-kernel` | added: `KernelError`, `SettingsError`, `CredentialError` |
| `crates/kernel/src/settings.rs` | `deck-streak-kernel` | added: the typed parser and the kernel's settings |
| `crates/kernel/src/credentials.rs` | `deck-streak-kernel` | added |
| `crates/kernel/src/redact.rs` | `deck-streak-kernel` | added: `Redactor` and the redacting writer |
| `crates/kernel/src/logging.rs` | `deck-streak-kernel` | added: `install` |
| `crates/kernel/src/offload.rs` | `deck-streak-kernel` | added |
| `crates/kernel/src/db.rs` | `deck-streak-kernel` | added: `Db`, `MIGRATOR`, the read-only opener, the settings generation |
| `crates/kernel/src/data_rights.rs` | `deck-streak-kernel` | added: the port, `Disposition`, the kernel's own port |
| `crates/kernel/tests/study_day.rs`, `settings.rs`, `constants.rs`, `clock.rs`, `verdict.rs`, `credentials.rs`, `logging.rs`, `offload.rs`, `db.rs`, `schema.rs`, `data_rights.rs` | `deck-streak-kernel` | added: A1 to A26 |
| `crates/kernel/tests/fixtures/migrations/` | `deck-streak-kernel` | added: the planted migrations A20 and A22 use |
| `migrations/002001_kernel_settings_generation.sql` | `deck-streak-kernel` | added |
| `.sqlx/` | workspace | added: the offline query cache for the kernel's queries |
| `Cargo.toml` | workspace | changed: sqlx, tokio, thiserror, tracing, tracing-subscriber (ADR-003) and tempfile (ADR-020) in `[workspace.dependencies]` |
| `Cargo.lock` | workspace | changed |
| `tools/parity-oracle/registry/spec_020.py` | repo | added: the digest-hour and redaction adapters, the kernel constants |
| `tools/parity-oracle/goldens/digest_hour.json`, `redaction.json`, `kernel.constants.json` | repo | added: generated on the owner's checkout |
| `docs/CONTEXT-MAP.md` | repo | changed: the ownership register gains DeckStreak's own tables (`settings_generation`, `_sqlx_migrations`) |
| `docs/LEXICON.md` | repo | changed: glossary rows for the settings generation and the data-rights port |
| `.env.example` | repo | added: the kernel's settings, by name |
| `.packs/wiring.json` | repo | changed: the ddd `lexicon-locks` deferral removed |
| `docs/schematics/startup-settings-and-secrets.md` | repo | changed: the order a role starts in, as built |
| `docs/decisions/ADR-020-kernel-time-schema-and-log-foundations.md` | repo | changed: accepted, with the decisions the delivery made |
| `docs/red-first/SPEC-020.md` | repo | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It creates no context's tables but its own: each SPEC registers the tables it creates with its
  port and the ownership register (#15, #16, #20).
- It builds no export or erase over the ports, and writes no `privacy.json` (#14).
- It starts no binary: the daemon's roles call `logging::install` and read the process environment
  (#18, #24).
- It validates no Telegram payload and pins no owner; it only names the id type
  (#17).
- It samples no host memory around slow offloads, as the predecessor's rail did; the memory watch
  owns memory pressure (#24).
- It reports no degraded optional secret on a health route: a missing required credential refuses
  start, and readiness is the API's (#18).
- It reaches no secret manager: the private rail provisions systemd credentials (#41).
- It declares no XP type or grant port (#26).

## 6. Risks

- **The committed `.sqlx/` cache goes stale.** A changed query without a refreshed cache fails the
  clippy and test stages with "no cached data for this query"; ledger-sqlite's
  `sqlx-offline-cache-is-committed` reads it on the box. The builder runs `cargo sqlx prepare
  --workspace` before committing.
- **`STRICT` refuses a type sqlx maps.** A migration test (A20, A21) runs every migration against a
  fresh file, so a refused type fails the delivery that wrote it, not a deploy.
- **The fixed offset meets a zone with daylight saving.** By design (ADR-020): the predecessor's
  offset is fixed. A host whose timers drift from the configured rollover is paged by the liveness
  job's rollover-drift check (#20 builds it; see SPEC-027).
- **A secret shorter than `_MIN_SECRET_LEN` is not redacted**, as in the predecessor. Every
  credential W0 reads is longer; the token-shape pattern catches the bot token whatever its
  registration.
- **A log line is written before the redactor knows a secret.** Credentials are loaded, and so
  registered, before `logging::install` returns control to the role; a test that logs during
  loading would read the marker (A13 covers the order).
- **A migration is added without the kernel recompiling.** `build.rs`'s `rerun-if-changed` makes the
  kernel rebuild; if it did not, A21 would fail, because it holds the embedded versions equal to
  the files in `migrations/`.

## 7. Amendments at delivery

- **R3: a divergence is a case's class, and it covers Python-only spellings.** The generator writes
  a case's `class` and has no `diverges` key to write, so the digest-hour golden marks ADR-020's
  divergence as `class: unparsable` and `class: python-only`, and A4 holds each such case to a
  refusal. Python's `int` also reads a digit separator (`1_0`) and digits outside ASCII; DeckStreak
  refuses them as malformed, as it refuses every value that is not an optional sign and ASCII
  digits. A4 decides the other cases by the predecessor's own value: equal where it resolved an hour
  its validation accepts, refused by name where it resolved one before the rollover, refused as
  malformed where it resolved one outside 0 to 23.
- **R10: the environment map is `settings::Environment`, and A5's required setting is
  `CREDENTIALS_DIRECTORY`.** The kernel's own four settings all have defaults; the credentials
  directory is the setting a role that loads a credential cannot start without, so A5 refuses its
  absence by name, and any context's required setting reads through the same parser.
- **R11: "never from the environment" is measured on a child process** whose real environment
  carries the credential's name and value while its credentials directory holds no such file; the
  loader refuses the credential as missing. `rs.no-secret-env` holds the production code.
- **R12: the token pattern is matched by hand, with Python's `\d`.** No regular-expression crate is
  admitted, and Python's `re` reads `\d` as every Unicode decimal digit. The redaction golden holds
  one token per run of decimal digits of the predecessor's interpreter (Unicode 15.0.0), and a test
  holds the redactor's table equal to them character by character; numeric characters that are not
  decimal digits (Roman numerals, superscripts) are proved not to make a token.
- **R13: the writer also scrubs a secret as a JSON string or a Rust debug string escapes it,** so a
  secret holding a quote or a backslash cannot pass inside a JSON line; `Redactor::redact` stays the
  predecessor's exact scrub, and `Redactor::redact_line` is the writer's. A line carries no
  timestamp: journald stamps each line, and the system time is read by `SystemClock` alone (R6).
  A13 and A15 are measured on a child process that installs the logging, so the global subscriber
  and the real stdout are a role's.
- **R14: a call at `SLOW_OFFLOAD_MS` warns, and its duration counts the wait for a worker,** as the
  predecessor's rail compared (`>=`) and timed it.
- **R15: `Db::open_with`** opens with a migrator other than `MIGRATOR`, for the fixtures A20 and A22
  apply; production opens with `Db::open`.

Amendment (2026-09-28): R2's upper bound is pinned by a test of its own (#222).
`every_hour_past_23_is_refused_and_23_is_admitted` (`crates/kernel/tests/study_day.rs`), added at
936d033, admits 23 as itself, refuses every value from 24 to 255 as `None`, and refuses a rollover
or a digest hour of 24 as `SettingsError::Malformed`, by its name and an hour's shape. Row S02005
holds it, with the mutant `hour <= 24` inside `Hour::new`, where cargo-mutants never looks: 27.1.0
lists 187 mutants of `crates/kernel/src/study_day.rs` and none inside `Hour::new`. The test was not
red at 3f916ce, where the code already refused 24. With the mutant installed on that tree it was
red by assertion, selecting one test (`24 was admitted as an hour`: left `Some(Hour(24))`, right
`None`), while A1's golden test and A6's malformed-setting test both passed. The file was then
restored byte for byte, its sha256
`3668d29d0cf56a289719a20a89df5d0b27f113f6549de1355d027dbd3d246854` before and after.

## 8. Amendment, 2026-09-30: the predecessor's register counts to its prose

Issue #420. The ownership register in `docs/CONTEXT-MAP.md` starts from the predecessor's 64 tables,
and two rows that belong to DeckStreak's own schema (`xp_settlement` and `buffs`) had been added to
it, so it named 66 rows (65 unique names) where its prose counts 64. Nothing read the register, and
the v9 import maps the predecessor's tables one by one from it.

R16. Every count the section's prose states (a number beside `tables`, `rows` or `names`) equals
the unique names of the register it describes; no name repeats inside one register; and a name in
both the predecessor's register and DeckStreak's own is a carried table, which both give the same
owning context. A register with a qualifier in its header (`v9 table`) pairs with the count
sentence that says the same word before its number, and a sentence or a qualified register that
pairs with nothing is refused by name. The predecessor's register loses the two rows, and the
section "DeckStreak's own tables" is unchanged byte for byte.

Two readings of the class are stated because the file decides them. Five names sit in both
registers on purpose (`sync_runs`, `xp_ledger`, `cron_fires`, `daily_rollup`, `daily_lang_stats`),
tables the predecessor had and DeckStreak keeps under the same name, so the rule is "same owner in
both", and a strict disjointness would refuse the file the issue asks for. DeckStreak's own register
states no count, so a row dropped from it is not expressible as a mismatch.

The check is `scripts/tests/test_context_map_registers.py`. Its population was generated from the
file: one altered copy per register row (dropped, repeated, moved to the other register) and one per
count sentence (one more, one fewer), 257 in all. 236 turned the check red by an assertion, and the
21 that did not are the rows dropped from DeckStreak's own register. The file was unchanged by
sha256 after every copy.

Files: `scripts/tests/test_context_map_registers.py` (new), `docs/CONTEXT-MAP.md` (two rows
removed), `docs/red-first/SPEC-020.md`, `changelog.d/predecessor-register-420.md`.

## 9. Acceptance criteria of the 2026-09-30 amendment

| id | criterion | decided by |
|---|---|---|
| A27 | the predecessor's register holds as many unique names as its prose counts, repeats none, and shares with DeckStreak's own register only names both give the same owner | `python3 -m unittest discover -s scripts/tests -p test_context_map_registers.py` |

```acceptance
A27: python3 -m unittest discover -s scripts/tests -p test_context_map_registers.py
```
