---
status: proposed
decision-makers: "the DeckStreak architect"
---

# The native column admits the normal sync and the native adapter carries a sync facade

## Context and Problem Statement

ADR-356 D2 gave the core one ordinary table with a native and a web mark per row, and SPEC-364 put
the normal sync, `BackendSyncService.SyncCollection` (1,5), in the web column alone. The app keeps
the host key in one Keychain item (ADR-358) and signs in, but cannot sync. SPEC-503 asks that the
iPhone and iPad client sync its collection as the web client does: the normal sync with media off
and the endpoint guard, the handshake before it, and a first sync through the full-sync choice. This
ADR decides how the native side reaches those, and amends ADR-356 D2.

## Decision Drivers

- One rule for both clients: the guard, the handshake and the choice already live in the core
  (ADR-358, ADR-368, ADR-385); the native side must reach them, never copy them.
- The native adapter links no protocol crate (`crates/ffi/Cargo.toml`), and a new dependency would
  change the lock file and the graph census.
- The app's Swift stays thin: only the session file imports the adapter, and each file keeps its
  decision budget (SPEC-347 R11).
- The one-way gesture is built only in an adapter's entry file (ADR-356 D4).

## Considered Options

### How the native client reaches the normal sync

- Admit (1,5) in the native column and the native allow-list: chosen, because the core's `run`
  already guards the endpoint, refuses media and checks the handshake for (1,5) on any transport.
- A native-only door beside `run`: rejected, because it would copy `run`'s guard and handshake
  order into a second path the table does not see.

### Where the native sync's messages are built

- In the core, as plain values (`native_sync`): chosen, because the adapter then names no protocol
  type and needs no new dependency, and the request's `sync_media: false` is fixed in one place.
- In the app's Swift codec: rejected, because the app would build a request whose media flag the
  core must then trust, and the Swift budget would grow.
- In the adapter, with the protocol crates as new dependencies: rejected, by the lock file and the
  graph census.

### What the adapter offers the app

- A sync facade on `Engine`, one method per step the web engine exports (`sync`, `unsynced`,
  `full_sync_count`, `full_sync_confirm`, `full_sync_cancel`), each answering records the app maps
  field for field: chosen, because the web client's steps are the same steps and the app decides
  nothing about the choice.
- The core's choice states handed to Swift: rejected, because the app would then sequence the
  choice itself.

## Decision Outcome

- **D1. The column.** (1,5) is `native: true`; the native allow-list carries it (eleven pairs), and
  the parity test holds the two equal. This amends ADR-356 D2: the normal sync is no longer the
  web's alone.
- **D2. The facade.** `crates/ffi/src/engine.rs` carries the sync facade. `Engine::sync` reads the
  statement at the endpoint's origin when the login's rule keeps the endpoint, hands it to the core,
  and runs the core's normal sync request through `run`. The choice's steps run through
  `one_way` in the model's order, its stage held in the engine between the owner's taps, and its
  files named in the folder the app gives.
- **D3. The upload waits.** The native client has no snapshot answer, so the app offers the
  download only, and an upload through the facade meets the core's snapshot rule.
- **D4. The key.** The app reads the host key from its Keychain item before each sync and passes
  only the key; no password is stored.

### Consequences

- Good: the native and web clients share every sync rule in the core; the app adds one model and
  one view within its budgets.
- Bad: the choice's server copies and backups accumulate on the device until retention is built.
- Neutral: the facade's sentences are the adapter's, so the app shows them as they are.

### Confirmation

SPEC-503's A1 to A6 and its mutation rows in S503.

## What would make this wrong

- The native client gains a snapshot answer: D3 is amended and the app offers the upload.
- The engine's normal sync takes more than one call: the facade's `sync` becomes a sequence.

## More Information

ADR-356, ADR-358, ADR-368, ADR-375, ADR-385, ADR-388; SPEC-347, SPEC-364, SPEC-374, SPEC-377.
