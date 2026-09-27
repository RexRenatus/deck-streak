---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The kernel keeps the study day as an epoch day under a fixed offset, holds one migration sequence, and installs the one redacting log setup

## Context and Problem Statement

SPEC-020 builds the shared kernel: the study day every context buckets by, the settings and
secrets every process loads, the log setup that must never leak a secret, and the database base
every context writes through. Four choices shape every later wave and are not settled by the
phase-1 ADRs: how the study day is represented and which time library computes it; where
migrations live when every context owns its tables but one database holds them; where the one
logging setup lives; and how settings are parsed.

## Decision Drivers

- The predecessor's study day uses a FIXED UTC offset (`types.py:CollectionConfig.tzinfo`) and the
  rollover hour; CHARTER 8 says its numbers port verbatim, proved by goldens.
- sqlx records every applied migration of a database in ONE `_sqlx_migrations` table and refuses to
  run when an applied migration is missing or edited, unless that check is switched off.
- Six builders add migrations in parallel; numbers must never collide (the phase-1 lesson that
  siblings double-claim numbers).
- A secret must never reach a log line (CHARTER 15), and every binary must log the same way (the
  observability pack's `obs.*` rows).
- A setting's value never appears in an error (SPEC-020's settings rule).

## Considered Options (the alternatives it was chosen against)

- The study day as an epoch day number computed with integer arithmetic under the configured fixed offset — chosen: it is the predecessor's rule exactly, a golden compares integers without a date parser, and the binary ships no time-zone data.
- A date-time crate with a time-zone database (jiff, or chrono with chrono-tz) — rejected because the predecessor's day follows a fixed offset, so a named zone would move the boundary at a daylight-saving change the predecessor never had, and because it adds a dependency and its data for nothing W0 needs.
- One `migrations/` directory at the workspace root, versions `<SPEC number, four digits><sequence, two digits>`, embedded by the kernel's `MIGRATOR` — chosen: versions cannot collide because each SPEC owns its number, the file name carries the owning context, and every context's tests migrate the same full schema.
- A migrations directory per context, each with its own migrator — rejected because the contexts share one `_sqlx_migrations` table: versions collide across directories, and each migrator must `set_ignore_missing(true)`, which switches off the check that an applied migration was not edited.
- Timestamp-numbered migrations — rejected because parallel builders' timestamps order arbitrarily and tie a migration to no SPEC.
- The daemon holding the migrator — rejected because every context's integration tests need the full schema, and they may not depend on the daemon.
- The kernel's `logging::install` (JSON, the sd-daemon priority prefix, `RUST_LOG`, a redacting writer) used by every role of the one binary — chosen: one setup, and the redactor that knows the loaded credentials is in the same crate as the loader that registers them.
- Each binary or role building its own subscriber — rejected because two setups drift, and a setup without the redacting writer leaks.
- A settings crate (figment, envy, config) — rejected because the kernel's settings are a handful of typed values, and a hand-written parser can promise that no error carries a value, which a generic deserializer's messages do not.

## Decision Outcome

Chosen options as above, plus:

- `migrations/<NNNNSS>_<context>_<slug>.sql` is additive, `STRICT`, and every table carries
  `created_at`; the kernel's census (SPEC-020 A21 to A23) holds all three and the ownership
  register together. sqlx applies a lower-numbered migration that arrives after a higher one was
  applied; SPEC-020 measures that (A20) rather than trusting it.
- The kernel owns one table, `settings_generation`: the owner-config generation the change gate
  compares. It is shared because the contexts that write settings and the one that reads the
  generation may not depend on each other.
- **An intended divergence** (ADR-012's `diverges`): the predecessor falls back to its default for
  an UNPARSABLE `ANKI_DIGEST_HOUR`; DeckStreak refuses start on it, like every malformed setting
  (SPEC-020, R10). The unset case ports verbatim (the larger of 9 and the rollover hour). The digest-hour
  golden's unparsable case carries `diverges: ADR-020`.
- `tempfile` is admitted as a dev-dependency for tests that need a temporary database or
  collection; `serde` and `serde_json` are ADR-029's.

### Consequences

- Good, because the study day is one integer everywhere, and screens render it from the server.
- Good, because a migration's number says which SPEC to read.
- Bad, because the kernel embeds every context's SQL; the census and the ownership register keep
  each file's owner explicit.
- Bad, because a fixed offset cannot follow a zone with daylight saving; the liveness job's
  rollover-drift check pages if the host's timers and the configured offset disagree.

### Confirmation

SPEC-020's acceptance tests (A1 to A26) and the rows `rs.no-secret-env`, `obs.subscriber-installed`,
`obs.structured-logs` and ddd `lexicon-locks` in `scripts/check.sh`; ledger-sqlite's portable rows
(`migrations-are-monotonic`, `created-at-everywhere-portable`, `tables-declare-strict`,
`connect-options-in-one-place`) on the box.

## What would make this wrong

- The owner moves to a zone with daylight saving and wants the boundary to follow it (an owner
  decision that supersedes this ADR's fixed offset).
- A context needs a second database file, which would make one migration sequence two.
- A SPEC needs more than 99 migrations, which the two-digit sequence cannot number.

## More Information

SPEC-020; ADR-003; ADR-008; ADR-012; the observability pack's `logging.rs.template`; the
ledger-sqlite pack; `docs/schematics/startup-settings-and-secrets.md`.
