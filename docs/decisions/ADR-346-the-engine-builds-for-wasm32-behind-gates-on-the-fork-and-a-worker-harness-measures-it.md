---
status: accepted
decision-makers: "the DeckStreak architect, the browser engine spike's builder"
---

# The engine builds for wasm32 behind gates on the fork, and a Worker harness measures it

## Context and Problem Statement

ADR-336 proposes that the web client runs Anki's engine in the browser, in a Worker over the
origin-private file system (OPFS), and waits for a spike to decide GO or NO-GO. At the fork
commit this repository pins, the engine does not build for `wasm32-unknown-unknown`: an ungated
check fails in five dependency crates (`mio`, `getrandom` on two lines, `libsqlite3-sys` and
`zstd-sys`; SPEC-335 section 1). The spike had to choose how to gate the engine for the browser on
a branch of the fork, and how to measure the result in Chromium and WebKit. This ADR records those
choices; the measurements are SPEC-335's section 7.

## Decision Drivers

- ADR-336's outcome needs a measured build, a gzipped size, and an open, answer and undo in
  Chromium and WebKit.
- The native build must not change: the pin stays reproducible and its difference from the
  upstream tag stays checkable (ADR-058).
- Every gate is a `cfg` on the target that a later bump can re-apply or drop, so the fork's spike
  branch is a list of patches, as ADR-336 requires of a multi-patch fork.
- The harness uses a synthetic collection only, and the measurement covers the engine a web client
  would ship, not a cut-down part of it.

## Considered Options (the alternatives it was chosen against)

### The SQLite route

- Move rusqlite, for both targets, to the line whose `libsqlite3-sys` routes wasm32 to `sqlite-wasm-rs`, with its `fallible_uint` feature: chosen because one SQLite crate then serves both targets and `fallible_uint` keeps the `u64` and `usize` conversions the engine's queries use.
- A second `libsqlite3-sys` for wasm32 beside the native one: rejected because its `links = "sqlite3"` key admits one such crate in a dependency graph, and the engine's pinned rusqlite line has no wasm32 route.
- A hand-written FFI shim to a browser SQLite build: rejected because every engine call through rusqlite would be re-pointed through it, a second database layer for the fork to carry.

### The native-only dependencies

- Native-only and wasm32-only dependency tables in the engine's manifest: chosen because they keep the in-crate sync server's crates, tokio's multi-thread, file-system and signal features and zstd's multithreaded compression off the wasm32 graph, and leave the native graph as it was.
- Cargo features that switch those dependencies off: rejected because the engine inherits them from workspace entries that turn the features on, and an inherited dependency cannot drop a feature its workspace entry enables.

### The async runtime

- A current-thread tokio runtime on wasm32 and the multi-thread one natively, built in the backend's one constructor: chosen because the Worker has one thread and every caller keeps the same runtime handle.
- No runtime on wasm32: rejected because the backend hands its runtime handle to every async call it makes, so removing it would gate each caller instead of one constructor.

### The sync transport

- A wasm32 twin of the streamed sync request that refuses with "not implemented": chosen because the browser's HTTP backend has no streamed request body and its response streams are not `Send`, and the open, answer and undo the spike measures make no sync call.
- Porting the streamed transport to the browser's fetch: rejected because it is a rewrite of the transport, which the web sync screens decide (#631), and it is not on the measured path.

### The clock

- The wall clock read from the JavaScript `Date` on wasm32: chosen because it is the browser's own clock, and the engine's timestamps keep their type.
- The standard library's clock: rejected because `SystemTime::now` panics on `wasm32-unknown-unknown`.

### The storage

- The SAH-pool VFS over OPFS, in a dedicated Worker: chosen because it keeps the collection in OPFS, the store ADR-336 decides, through access handles a dedicated Worker holds.
- The in-memory VFS: rejected because it keeps nothing across a close, so a reopen could not be measured.
- The IndexedDB-backed VFS: rejected because ADR-336 decides OPFS as the collection's store, so the spike measures that store.

### What the module exports

- The backend's whole `run_method`, exported from the harness module: chosen because the linker then keeps the engine a web client would call, so the size covers the engine.
- Only the five calls the scenario makes: rejected because the linker would drop the rest of the engine, and the size would measure a fraction of what a web client ships.

### The harness

- Playwright with a persistent profile in both browsers: chosen because WebKit refuses the OPFS root in Playwright's default ephemeral context, and a persistent profile runs in both.
- Playwright's default ephemeral context: rejected because WebKit refuses OPFS there, so WebKit could not be measured.
- A WebKit run on a hosted CI runner from a workflow on the fork's branch: dropped: the harness runs on a persistent local profile instead, and the branch now carries no workflow.

## Decision Outcome

Chosen options: the first option under each heading above, because together they build the engine
for `wasm32-unknown-unknown` with the native check unchanged (SPEC-335 M1, M2) and measure it in a
Worker over OPFS in Chromium and WebKit (SPEC-335 M3 to M9). They sit on the fork's spike branch
as three commits: the gates `c67ef888175daf1e28b6cc20964c1d9e75cf50c6`, the build
`cbe60369e419a604e676da627e47e6148c34cdca` and the harness
`98d1455385dfea67cac832fa98f11a54004f8233`, on the pinned
`57382da085e6752738dc4bb617789be836a23300`. The gates are 27 sites: 23 code sites in the engine's
source and 4 manifest and config entries (SPEC-335 M10).

### Consequences

- Good, because every gate is a `cfg` on the target, so the native build is the one the pin
  already builds, and each gate can be re-applied or dropped on an Anki bump.
- Good, because the harness re-runs from the fork's branch alone: `wasm-spike/build.sh` prints
  the sizes, and `npx playwright test --repeat-each 5` repeats the scenario in each browser.
- Bad, because the rusqlite line moves for the native build too, which moves its bundled SQLite
  when the web engine adopts the branch.
- Bad, because the sync transport refuses on wasm32 until the web sync screens decide it (#631).
- Bad, because the FSRS crate keeps its parallel iterator dependency, so its optimiser would need
  threads at run time, and two engine call sites outside the scenario read the standard clock;
  neither is on the open, answer and undo path.
- Bad, because a browser context that refuses OPFS, as WebKit's ephemeral context does, cannot
  keep a collection this way; its fallback is the web engine's (#626).

### Confirmation

SPEC-335 section 7: each row names the fork commit and the command that measured it, from
`cargo build --release --target wasm32-unknown-unknown -p anki` to
`npx playwright test --project webkit --repeat-each 5`.

## What would make this wrong

- An Anki bump moves engine code across the gates so that the native check changes: the gates
  would then be changing the native build, which this ADR rules out.
- The web engine's phone-class measurement (#626) finds the module or its timings too large for a
  phone, which the desktop profile here cannot show.

## More Information

- SPEC-335 (the spike's record and its measurements), issue #614.
- ADR-336 (the web client runs the engine in the browser), ADR-058 (the patched fork), ADR-022
  (the engine's spike protocol).
