---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The data store: one SQLite database in WAL mode, migrated with sqlx, and a proved import of v9's schema 24

## Context and Problem Statement

The predecessor keeps its state in one 2 MB SQLite file at `user_version = 24`, replicated by
Litestream. DeckStreak needs a store that fits a small VM, survives a crash, and can take over the
predecessor's state at cutover without trusting anyone's reading of it.

## Decision Drivers

- The golden path `data`: SQLite in WAL mode and Litestream 0.5 (0.5.2 or later).
- The ledger-sqlite practice: `BEGIN IMMEDIATE`, foreign keys on, a busy timeout, `created_at` on every table, migrations that are immutable once shipped.
- The data-migration pack: a plan mapping all 64 source tables, a parity oracle, a dry run by default, a read-only source, a verified backup first, rollback.

## Considered Options (the alternatives it was chosen against)

- A new SQLite database for DeckStreak, owned table by table per context, migrated by sqlx, and a one-off import of v9's schema 24 proved by the data-migration pack — chosen: fits the VM, the stack, and the owner's parity bar.
- Reuse the predecessor's database file in place — rejected because both services would write it during the side-by-side period, and its schema carries shapes the port fixes.
- PostgreSQL — rejected because the radar holds it at assess until a second app server or sustained concurrent writers, neither of which exists.

## Decision Outcome

Chosen option: DeckStreak's database is its own file under its unit's `StateDirectory=`, opened
by the kernel's repository base (WAL, `synchronous=NORMAL`, foreign keys, a busy timeout,
`BEGIN IMMEDIATE` for writes) and migrated by sqlx at start. Each context owns its tables
(docs/CONTEXT-MAP.md). The v9 import (W8) follows `data-migration.json`: a schema snapshot pinned
at `user_version = 24`, every table mapped, goldens from v9's own functions, a dry run, a
Litestream restore verified before the apply, integrity and foreign-key checks after, and a
rollback that discards the target while v9 keeps serving.

### Consequences

- Good, because the two services never share a writable file.
- Bad, because the import is its own wave of work; the parity oracle makes it provable rather than trusted.

### Confirmation

The data-migration pack's rows once `data-migration.json` lands (W8); ledger-sqlite's rows on the box; the database base's own tests in W0.

## What would make this wrong

- Sustained concurrent writers appear (they would move the radar's PostgreSQL entry).
- The import cannot reconcile a table's counts under its declared rule (the counts proof).

## More Information

The ledger-sqlite and data-migration packs; ADR-010 for backups; ADR-011 for cutover.
