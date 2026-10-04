---
status: accepted
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The web engine pins the fork's wasm32 patches by tag and crosses into a Worker through wasm-bindgen

## Context and Problem Statement

ADR-336 is `accepted` on the browser engine spike's GO (SPEC-335, ADR-346): Anki's engine runs in
the browser on `wasm32-unknown-unknown`, in a dedicated Worker over OPFS. The spike built it from
the fork's spike branch, whose 27 gate sites sit in one commit beside a build commit and a harness
commit, over ADR-058's one-fix commit `57382da085e6752738dc4bb617789be836a23300`. The web engine
(SPEC-338, #626) brings that engine into DeckStreak's workspace, and four questions need an answer
before any code does:

- how the fork carries the browser's patches, given ADR-336's rule that each is one commit with its
  own name, reason and removal condition;
- which SQLite route the engine takes, since the spike's branch does not resolve beside DeckStreak's
  `sqlx` (SPEC-338 section 1: `libsqlite3-sys` `^0.38.1` against `<0.38.0`, one `links = "sqlite3"`
  per graph);
- what the crate is and what crosses its JavaScript boundary, given ADR-337: the engine core reaches
  the engine's backend only through an allow-listed dispatcher;
- what a browser context that refuses OPFS gets, and what the Mini App's embedded context gets.

## Decision Drivers

- ADR-058's rules hold for every patch: the `[patch]` entry pinned by `rev`, a tag on the fork,
  nothing force-pushed or deleted while pinned, the upstream tag kept in the dependency line.
- Each patch can be dropped on its own when upstream carries it.
- The native build stays the build the pin already builds: every patch is a `cfg` on the target or a
  dependency line both targets share.
- No page script reaches an engine call the study client has not chosen to make (ADR-337).
- The size the gate reads is the size of the engine a web client ships (ADR-346).
- A refusal says why; nothing studies into storage that silently disappears.
- The stack takes Rust to WebAssembly only by measured exception (`rust-wasm`): the engine must
  schedule exactly as the engine of every other client does, it runs in a dedicated Worker, and
  ADR-336 states its gzip budget. `stack.json` cites this ADR for it.

## Considered Options (the alternatives it was chosen against)

### How the fork carries the patches

- A new fork branch `wasm32-26.09.3` from ADR-058's commit with one commit per patch, tagged `deckstreak-pin-26.09.3-wasm32` and pinned by `rev`: chosen because each patch then carries its own name, reason and removal condition in its commit, and an Anki bump re-applies or drops each one alone.
- Pin the spike branch as it stands: rejected because its 27 gates are one commit, its harness commit adds a workspace member the engine does not need, and its rusqlite line does not resolve beside DeckStreak's `sqlx` (SPEC-338 M1).
- Squash every patch into one commit on ADR-058's: rejected because a single commit cannot be dropped in part when upstream carries one of its patches, which ADR-336 requires.
- A direct `git` dependency on the fork: rejected for ADR-058's reasons, the two readers of the dependency line.

### The SQLite route

- rusqlite 0.39 with `fallible_uint`, for both targets: chosen because 0.39 routes `wasm32-unknown-unknown` to `sqlite-wasm-rs` (its own `links` key) and takes `libsqlite3-sys` `^0.37.0` natively, inside the range `sqlx` 0.9 accepts, so one `libsqlite3-sys` serves the engine and the kernel.
- rusqlite 0.40, the spike's line: rejected because its `libsqlite3-sys` `^0.38.1` conflicts with `sqlx-sqlite` 0.9's `<0.38.0`, and no `sqlx-sqlite` release accepts 0.38.
- Move `sqlx` instead: rejected because no release exists to move to, and the kernel's repository base is not this delivery's to change.

### getrandom's browser backend in this workspace

- The feature alone, which the lockfile's getrandom lines read: chosen because getrandom 0.3.4 and later and 0.4 select `wasm_js` by the feature, so the `wasm32` check of this workspace builds with no flag and no file (SPEC-338 section 7, M2).
- A `.cargo/config.toml` setting `--cfg getrandom_backend="wasm_js"` for the target: rejected because the settle census compiles with cargo's own defaults and refuses a `.cargo/config.toml` at the root (SPEC-072 A34), and the flag selects nothing the feature does not already.
- The flag in `RUSTFLAGS` on the build script's command line: rejected because no getrandom line this lockfile resolves needs it, and a flag nobody needs is one more setting to keep equal in two places.

### The crate and its JavaScript boundary

- A workspace member `crates/web-engine` whose dependencies are all wasm32-only, exporting named study calls and an allow-listed `run_method` through `wasm-bindgen`: chosen because the native build is the target-independent study rule alone, the page reaches only the calls the study client makes, and the dynamic dispatch keeps the whole engine in the module the size gate reads.
- Export the backend's whole `run_method` and a raw SQL call, as the spike's harness did: rejected because any script on the page could then reach every engine call, the exempt writes of ADR-337 included, and write the collection through SQL around the engine.
- Export only the named calls: rejected because the linker would drop the rest of the engine, and the gate would measure a fraction of what the study screens will ship (ADR-346's reason).
- wasm-pack over `wasm-bindgen`: rejected because it adds a tool that wraps the same CLI, and the gate pins `wasm-bindgen` and `wasm-opt` directly.

### Who each message listener hears

- Each listener, the Worker's on its scope and the client's on its port, admits a message whose origin is empty or equal to its own origin, and ignores any other with no reply: chosen because ASVS 5.0.0 3.5.5 asks every message receiver to check its sender, a dedicated Worker's channel delivers its messages with an empty origin, so the page and its Worker always pass, and the check is one comparison in each listener.
- Rely on the channel alone: a dedicated Worker's scope hears only the page that started it, and the client's port is that Worker, so no other sender reaches either. Rejected as the only control: it holds today, but a per-listener origin rule read both listeners as checking nothing, and a later change that handed the scope or the port to another sender would carry no check with it. It is recorded here as the reason a stricter check is not needed.
- Admit only the empty origin: rejected because it would refuse every message in a browser that labelled a Worker's messages with the page's origin, and the receiver's own origin is never a foreign sender.

### How the page reads the Worker's peak memory

- An export of the module's linear memory in pages (`memory_pages`, from `core::arch::wasm32::memory_size`), answered by a protocol `memory` operation in bytes: chosen because the module's own export is the reading, the loader keeps returning the bindings exactly as `wasm-bindgen` writes them, and linear memory never shrinks, so a reading after each step is the high-water so far.
- Keep the object `wasm-bindgen`'s init resolves to, and read its `memory.buffer.byteLength`: rejected because the loader would then hand the session a wrapper in place of the bindings, and the reading would rest on what the generated init happens to resolve to, which its typed surface does not promise.
- `performance.measureUserAgentSpecificMemory()`: rejected because it needs a cross-origin isolated page, which the Mini App's page is not, and one engine of the two the tests drive implements it.

### One collection per origin

- A Web Lock, `deck-streak-collection`, requested with `ifAvailable` before the pool is installed: chosen because the browser grants it to one Worker per origin and answers a second at once, so a second tab refuses by name before it touches OPFS.
- Leave it to the SAH pool's exclusive access handles: rejected because the second install fails or contends inside the VFS, after the module has loaded, and says nothing a screen can show.
- An election over `BroadcastChannel`: rejected because it is racy across tabs that start together, and the Web Locks API exists for exactly this.

### A context that refuses OPFS

- Refuse with `storage-refused` and load no engine: chosen because a study session must survive its tab, and the refusal names why.
- An in-memory collection in its place: rejected because every answer would vanish when the Worker ends, which is the silent loss the drivers rule out.
- ADR-336's server-side collection as a fallback: rejected because it is ADR-336's answer to a NO-GO, it is not built, and it would be a third copy of the collection.

## Decision Outcome

Chosen options: the first under each heading above.

- **The fork.** Branch `wasm32-26.09.3` on `RexRenatus/anki`, over ADR-058's
  `57382da085e6752738dc4bb617789be836a23300`, tagged `deckstreak-pin-26.09.3-wasm32`, carries
  ten patches, one commit each. Each commit's message holds its name, its reason and its removal
  condition, and ADR-058's appended note records each commit's id once the branch and the tag are
  pushed:

  | patch | what it does | removal condition |
  |---|---|---|
  | `sqlite-route` | rusqlite 0.39 with `fallible_uint`, both targets | the upstream tag's rusqlite routes wasm32 and resolves beside `sqlx` |
  | `native-only-sync-server` | the in-crate sync server and its crates on native targets only | upstream gates its server by feature or target |
  | `current-thread-runtime` | a current-thread tokio runtime on wasm32; the updater refuses there | upstream builds its runtime per target |
  | `single-thread-zstd` | zstd without `zstdmt` on wasm32 | upstream gates `zstdmt` per target |
  | `sequential-rayon` | the two desired-retention sweeps run in sequence on wasm32 | upstream gates rayon per target |
  | `js-date-clock` | the wall clock from the JS `Date` on wasm32 | the standard clock works on wasm32 |
  | `browser-fetch` | the HTTP paths fit the browser's fetch; the streamed sync request refuses | a browser sync transport replaces the refusal (#631) |
  | `getrandom-wasm-js` | getrandom's `wasm_js` backend on wasm32 | getrandom picks a browser backend by default |
  | `native-only-log-file` | `tracing-appender` on native targets only; no log file on wasm32 | `tracing-appender` builds for wasm32 again |
  | `browser-tls` | the rustls custom-certificate path is not built on wasm32 | upstream gates that path per target |

- **The pin.** The root manifest's `[patch."https://github.com/ankitects/anki.git"]` entry takes
  `anki` and `anki_proto` from the fork at the tag's commit by `rev`, and the dependency lines keep
  the upstream tag `26.09.3`. No configuration file or flag selects getrandom's browser backend: the
  lockfile's getrandom 0.3.4 and 0.4.3 select `wasm_js` by the feature the fork's
  `getrandom-wasm-js` patch turns on, and the `wasm32` build's only settings are the C compiler and
  archiver on the command line of `scripts/web-engine-build.sh`, which CI's job and a builder's
  measurement both call.
- **The crate.** `deck-streak-web-engine`, a `cdylib` and an `rlib`. Natively it holds the study
  rule (`study.rs`): a wire rating 1 to 4 to Anki's answer and the next state it selects, and the
  table of study calls `run_method` admits, by service and method index. On wasm32, `wasm.rs`
  exports: install the storage, init, open, close, seed (into an empty collection only, for the
  tests and measurements; each note two fields of 200 characters, ADR-022's shape, from the
  target-independent `synthetic.rs`), the next card, answer, undo, a read-only snapshot of one
  card, the module's linear memory in pages, and `run_method`, which refuses any call outside the
  table by name. No exempt write of ADR-337 is in
  the table; the engine core and its `OwnerGesture` own them (#623).
- **The Worker.** A dedicated module Worker owns the module. It takes the Web Lock first, then
  installs the SyncAccessHandle pool in OPFS directory `deck-streak`, then loads the module and opens
  `/deck-streak/collection.anki2`. The page asks for persistent storage and shows the answer.
- **The message boundary.** The Worker's listener and the client's each admit a message whose
  origin is empty or their own, and ignore any other: the Worker answers nothing and the client
  settles nothing (SPEC-338 R13).
- **A context that refuses OPFS** answers `storage-refused` and loads no engine.
- **The peak memory** is the module's linear memory, read by the protocol's `memory` operation
  after each step; the Worker's JavaScript heap is not in it.
- **The Mini App's embedded context** takes the same path in a frame under another site, where the
  browser partitions OPFS and the lock by the top-level site. SPEC-338 section 7 measured it framed
  across sites (M11): in both browsers the frame opens a collection of its own, answers and undoes.

### Consequences

- Good, because each patch names its own end, and an Anki bump drops the ones upstream has taken.
- Good, because the page reaches the engine through a short table of calls, and every new call joins
  the table with a test.
- Good, because a second tab and a refused storage each say why, before anything is opened.
- Bad, because the fork now carries eleven commits, each re-applied on every Anki bump.
- Bad, because rusqlite 0.39 moves the bundled SQLite of the native engine and of `sqlx` together
  (`libsqlite3-sys` 0.34 to 0.37), which the `rust`, `engine` and `release` jobs and
  `engine-measure.yml` re-read.
- Bad, because a browser that refuses OPFS cannot study offline at all until ADR-336's fallback is
  built.

### Confirmation

SPEC-338's A1, A2, A8 to A11, A14, A15, A20 and A21, the browser tests, and its section 7: the
fork's commits and the `[patch]` rev, the native and wasm32 checks at the tag, the size, and each
measured item.

## What would make this wrong

- An Anki bump moves engine code so that a patch no longer applies as one commit, or changes the
  native build: the patch is re-derived, and the native check is the test.
- A `sqlx` release accepts `libsqlite3-sys` 0.38: the SQLite route may then move to rusqlite 0.40.
- The study screens need a call the table lacks for every screen: the table grows by call, never by
  admitting `run_method` whole.
- A browser grants OPFS but not the Web Locks API: the Worker then refuses as if storage were
  refused, and SPEC-338's measurements would show it.

## More Information

ADR-058 (the pinned fork; this ADR appends a note there), ADR-336, ADR-337, ADR-346, ADR-022;
SPEC-335, SPEC-338; #623, #626, #631, #637. The Web Locks API
(https://developer.mozilla.org/en-US/docs/Web/API/Web_Locks_API), the SyncAccessHandle pool VFS
(https://sqlite.org/wasm/doc/trunk/persistence.md#vfs-opfs-sahpool) and the Cargo Book on `[patch]`
(https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html).
