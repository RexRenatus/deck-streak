---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The web client runs the engine in the browser on WASM, with a server-side study collection as the fallback

## Context and Problem Statement

The owner chose that a web study session runs Anki's engine in the browser (SPEC-334 R5; the
owner's answer ANK-02), where the recommendation had been a study collection on the server. The
engine has never been built for `wasm32`: its crate carries an in-crate sync server, a
multi-threaded async runtime with file-system and signal features, multi-threaded compression and
a parallel iterator crate, and it reaches SQLite through a native library, none of which builds
for the browser as it stands. Engine changes are carried on a fork that ADR-058 limits to exactly
one commit. How does the browser run the engine, what is the fallback if it cannot, and what
happens to the fork?

## Decision Drivers

- One engine on every surface (ADR-335): no second implementation of Anki's scheduler or
  collection.
- The browser holds its own copy and syncs like any Anki client, so no third copy of the
  collection lives on the host.
- A WASM build carries a measured hot path and a gzipped size budget before it is adopted.
- The browser may evict site storage, so unsynced reviews must never sit there silently.
- The engine's pin stays reproducible and its difference from the upstream tag stays checkable
  (ADR-058).

## Considered Options (the alternatives it was chosen against)

- The engine compiled to WASM, in a Worker over OPFS storage — chosen because the owner chose it (ANK-02), the browser then syncs like any Anki client, and no study copy of the collection, with its lock contention, lives on the host.
- A study collection on the server, behind a study API — kept as the fallback because it needs no `wasm32` build, and rejected as the first choice because it keeps a third copy of the collection on the host and routes every answer through the server.
- A JavaScript implementation of the scheduler and collection — rejected because it is a second engine that drifts from Anki's, so every behaviour would be re-derived rather than inherited.
- No study in the browser, the iPhone and iPad client only — rejected because the owner wants to study on the web, which needs no Apple build to reach a device.

## Decision Outcome

Proposed option: the engine compiled to WASM, in a Worker over OPFS storage. **The decision
outcome comes from the browser engine spike**, which is time-boxed and ends in GO or NO-GO. It
builds the engine for `wasm32` from a branch of the fork, records the gzipped size, and opens,
answers and undoes on a synthetic collection in a Worker over OPFS, in Chromium and WebKit under
Playwright. On GO, the delivery that records it sets this ADR to `accepted` with the measurements
and the size budget. On NO-GO, this ADR records the fallback as its outcome, and the web engine
delivery builds the server-side study collection instead; SPEC-334's row 1.4 holds either way.

- **Storage.** The Worker keeps the collection in OPFS through a SQLite build for the browser,
  and asks for persistent storage (`navigator.storage.persist()`). Because the browser may still
  evict it, the client syncs at session start and session end and warns while reviews are
  unsynced.
- **The budget.** CI holds the WASM bundle to a gzipped size budget that the spike sets from its
  measurement, and the page's content security policy admits WebAssembly compilation and nothing
  else new.
- **FSRS-7.** The isolated FSRS-7 crate (ADR-338) compiles to the same target and runs beside the
  engine in the Worker.
- **ADR-058's one-fix fork becomes a multi-patch fork.** The browser build needs changes to the
  engine that upstream does not carry: `cfg(target_arch = "wasm32")` gates around the in-crate
  sync server, the runtime's multi-threaded, file-system and signal features, multi-threaded
  compression and the parallel iterator crate, and a SQLite route that works in the browser. The
  fork's branch carries ADR-058's one commit plus these, each patch one commit with its own name,
  its reason and its own removal condition. The fork then differs from the upstream tag by exactly
  the listed patches, and every rule ADR-058 sets for the one commit holds for each of them: the
  `[patch]` entry pinned by `rev`, a tag on the fork, nothing force-pushed or deleted while it is
  pinned, the upstream tag kept in the dependency line, and each patch re-applied and checked on
  every Anki bump. The delivery that adds the second patch appends a note to ADR-058 naming this
  ADR and the list.

### Consequences

- Good, because a web session studies offline after its first sync, and the host keeps no study
  copy of the collection.
- Good, because the browser runs the same engine as the iPhone and iPad client, behind the same
  allow-list (ADR-337).
- Bad, because the fork grows from one patch to several, and each Anki bump re-applies every one
  of them until upstream carries it.
- Bad, because the first page load downloads the engine, bounded by the size budget.
- Bad, because browser storage can be evicted, so the client must sync at both ends of a session
  and say when it has not.

### Confirmation

- The spike's record: the build, the gzipped size, and open, answer and undo in Chromium and
  WebKit.
- The CI size-budget check on the WASM bundle.
- The web sync screens' tests: a sync at session start and end, and the warning while reviews are
  unsynced.

## What would make this wrong

- The engine cannot be gated for `wasm32` without changes too deep to carry as patches, or its
  gzipped size exceeds any budget the owner would accept: the spike's NO-GO, and the fallback.
- A browser's OPFS storage proves unreliable across sessions in a way syncing at both ends cannot
  cover, which the spike's eviction measurement reads.

## More Information

- SPEC-334 (rows 1.1, 1.2 and 1.4; R3, R5).
- ADR-058 (the patched fork), ADR-022 (the engine's spike protocol and budgets), ADR-335 (the
  FFI transport), ADR-338 (the FSRS-7 crate).
