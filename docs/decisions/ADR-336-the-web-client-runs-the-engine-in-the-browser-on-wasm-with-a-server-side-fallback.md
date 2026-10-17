---
status: accepted
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

Chosen option: the engine compiled to WASM, in a Worker over OPFS storage. **The decision
outcome comes from the browser engine spike**, which is time-boxed and ends in GO or NO-GO. It
builds the engine for `wasm32` from a branch of the fork, records the gzipped size, and opens,
answers and undoes on a synthetic collection in a Worker over OPFS, in Chromium and WebKit under
Playwright. On GO, the delivery that records it sets this ADR to `accepted` with the measurements
and the size budget. On NO-GO, this ADR records the fallback as its outcome, and the web engine
delivery builds the server-side study collection instead; SPEC-334's row 1.4 holds either way.

**GO, from the browser engine spike (SPEC-335, ADR-346).** The engine builds for
`wasm32-unknown-unknown` from the fork's spike branch. In a dedicated Worker over OPFS, through the
SAH-pool VFS, it opens a synthetic collection of 300 notes, builds the queue, answers a card and
undoes the answer correctly in 5 runs of 5 in Chromium and in WebKit under Playwright, and it ships
at 7529787 bytes gzip -9. The measurements, from SPEC-335 section 7:

- **Size.** The module a browser loads, after `wasm-bindgen` and `wasm-opt -Oz`: 23845690 bytes
  raw, 7521156 bytes gzip -9. Its JS bindings: 42833 bytes raw, 8631 bytes gzip -9. Shipped total:
  7529787 bytes gzip -9. Before the size pass, the cargo cdylib: 27394535 bytes raw, 7750461 bytes
  gzip -9. The embedded translation table is the largest single part of the module's data: 8357373
  bytes of generated source.
- **Chromium**, median (min-max) of 5 runs, in milliseconds: open 1.1 (1.0-1.2), queue build 17.2
  (13.7-18.1), answer 6.0 (5.9-9.6), undo 3.6 (3.1-7.9); module load 107.5. Undo correct 5 of 5.
- **WebKit** (Playwright's WebKit build, headless; its clock resolves to 1 ms), the same: open 1.0,
  queue build 7.0 (6.0-8.0), answer 3.0 (2.0-4.0), undo 2.0 (1.0-3.0); module load 261.0. Undo
  correct 5 of 5.
- **Undo correct** means one review-log row for the card after the answer and none after the undo,
  with the cards table changed by the answer and restored byte-equal by the undo. The journal mode
  reads `wal`.
- **OPFS.** WebKit refuses the OPFS root in Playwright's default ephemeral context; a persistent
  profile runs in both browsers.
- **The gates.** 27 sites on the fork: 23 code sites in the engine's source and 4 manifest and
  config entries. The engine's native check reads the same before and after them.

**The size budget is 8000000 bytes gzip -9 for the shipped module plus its JS bindings.**

**Not measured, and the web engine's acceptance (#626):** a phone-class device, peak memory,
persistence across a page reload and under storage eviction, a second tab on one collection, the
brotli size, and the module's size with the translation table loaded apart from the engine. The
sync transport on `wasm32` is the web sync screens' (#631), and real Safari on a device is the
owner's acceptance session's (#637).

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
