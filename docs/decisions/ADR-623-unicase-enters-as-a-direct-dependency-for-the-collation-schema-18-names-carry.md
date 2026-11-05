# ADR-623: unicase enters as a direct dependency for the collation schema-18 names carry

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-602 |
| builds on | ADR-622 |
| measured at | DeckStreak `dev` at the commit this pull request branches from |

## Context

Anki's schema-18 file declares its name columns with `COLLATE unicase`, a collation the file does not carry: a
connection that has not registered it cannot run any statement that orders or compares by it, and fails with "no
such collation sequence". The engine registers the collation with the `unicase` crate's case-insensitive ordering.
The golden `anki.collation.order` (40 cases) pins the order the engine's collation gives.

## Decision

`unicase` 2.6.0 enters the workspace's `[workspace.dependencies]` as a direct dependency, named only by
`deck-streak-anki-format`. The defensive opener registers the collation `unicase` on every connection it makes,
comparing two names by `UniCase::new` on each side, so the order the crate reads is the engine's.

- **Version:** 2.6.0, the version the lock already holds through the engine; the lock gains no package.
- **Licence:** `MIT/Apache-2.0`, read from the crate's manifest in the cargo cache.
- **Size:** the `.crate` file is 23478 bytes, read from the cargo cache.
- **Why the engine's comparator:** a collection's order is the engine's order; any other comparator would sort a
  name pair the engine keeps apart differently, and the collation golden would read red.

## Consequences

- The collation test reads the golden's order through the crate's connection, and a planted control opens the same
  file with rusqlite alone, registering nothing, and fails with "no such collation sequence".
- A later `unicase` release moves the order only with an engine change that moves the golden too.
