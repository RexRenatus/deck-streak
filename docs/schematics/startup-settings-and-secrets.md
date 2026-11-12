# Schematic: a process starts, loads its settings and secrets, and opens the database

Kind: data flow. Read at DeckStreak `main` e05dfa5 (ADR-008, ADR-010, the observability pack's
`logging.rs.template`), and at the predecessor's `27ee2bc` for the rules it ports
(`config.py:Settings.validate`, `config.py:_env_digest_hour`, `logging_redact.py:register`,
`database.py:GamifyStore.connect`). Decided by ADR-020, and built by SPEC-020's delivery in the
order below: the logging is installed before a setting is read, so even a refusal to start leaves
the process as a JSON line with its priority (SPEC-031 R1), and every secret the loader reads is
registered in the one registry the log writer reads live. SPEC-066 (ADR-067) adds the
loader's refusal of an empty credential, which fails start exactly as a missing one does.

```mermaid
flowchart TB
  start[a role of deckstreakd starts] --> redactor[Redactor: one shared registry, empty]
  redactor --> logging[logging::install: JSON, the priority prefix, RUST_LOG, the redacting writer]
  logging --> journal[(stdout to journald)]
  redactor -. read live by the writer .-> logging
  env[process environment, read once by the daemon's main] --> parse[Environment, then KernelSettings::from_env]
  logging --> parse
  parse -->|missing: SettingsError::Missing names the setting| refuse((refuse start, exit non-zero))
  parse -->|malformed: names the setting and its shape, never the value| refuse
  parse -->|explicit digest hour before the rollover| refuse
  parse --> directory[CredentialsDirectory from CREDENTIALS_DIRECTORY]
  directory --> loader[CredentialLoader: one file per credential id, one trailing newline trimmed]
  loader -->|missing credential: names its id| refuse
  loader -->|empty credential, zero bytes or only the trimmed newline: names its id| refuse
  loader -->|each value of at least 4 characters, before it is returned| redactor
  parse --> settings[KernelSettings: StudyDayRule, digest hour, offload workers]
  settings --> db[Db::open: WAL, synchronous NORMAL, foreign keys, busy timeout]
  db --> migrate[MIGRATOR: migrations/NNNNSS_context_slug.sql, embedded at build]
  migrate --> ready[the role continues: api, bot or job]
  ready --> writes[every write: Db::write, BEGIN IMMEDIATE]
```

| step | guard | proved by |
|---|---|---|
| logging installed first | JSON lines, each opening with its journal priority; a second install is an error | SPEC-020 A15 |
| settings parse | no value in any error; blank is unset | A5, A6 |
| digest hour | unset resolves to the larger of 9 and the rollover; explicit earlier is refused | A3, A4 (golden) |
| credential load | only the credentials directory; never an environment variable | A11, A12; `rs.no-secret-env` |
| empty credential | refused by its id once the one trailing newline is trimmed, before the redactor registers anything | SPEC-066 A1 to A3 |
| redaction | registered values and token shapes replaced by the marker, longest first, live | A13, A14 (golden) |
| database open | the four pragmas, then every migration | A17, A21 |
| writes | two writers serialise on the write lock; none is lost | A18 |
