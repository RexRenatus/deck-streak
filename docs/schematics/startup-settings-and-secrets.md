# Schematic: a process starts, loads its settings and secrets, and opens the database

Kind: data flow. Read at DeckStreak `main` e05dfa5 (ADR-008, ADR-010, the observability pack's
`logging.rs.template`), and at the predecessor's `27ee2bc` for the rules it ports
(`config.py:Settings.validate`, `config.py:_env_digest_hour`, `logging_redact.py:register`,
`database.py:GamifyStore.connect`). Decided by ADR-020.

```mermaid
flowchart TB
  env[process environment: non-secret settings] -->|read once, in the daemon's main| parse[kernel settings parser]
  creds[$CREDENTIALS_DIRECTORY: one file per credential id] --> loader[kernel credential loader]
  parse -->|missing: SettingsError::Missing names the setting| refuse((refuse start, exit non-zero))
  parse -->|malformed: names setting and shape, never the value| refuse
  parse -->|explicit digest hour before rollover| refuse
  loader -->|missing credential: names its id| refuse
  loader -->|each value of at least the minimum length| redactor[Redactor registry]
  parse --> settings[typed settings: StudyDayRule, digest hour, offload workers, paths]
  redactor --> logging[logging::install: JSON, priority prefix, RUST_LOG]
  logging --> journal[(stdout to journald)]
  settings --> db[Db::open: WAL, synchronous NORMAL, foreign keys, busy timeout]
  db --> migrate[MIGRATOR: migrations/NNNNSS_context_slug.sql]
  migrate --> ready[role continues: api, bot or job]
```

| step | guard | proved by |
|---|---|---|
| settings parse | no value in any error | SPEC-020 A5, A6 |
| digest hour | unset resolves to the larger of 9 and the rollover; explicit earlier is refused | A3, A4 (golden) |
| credential load | only the credentials directory; never an environment variable | A11, A12; `rs.no-secret-env` |
| redaction | registered values and token shapes replaced by the marker, longest first | A13, A14 (golden) |
| database open | the four pragmas, then every migration | A17, A21 |
