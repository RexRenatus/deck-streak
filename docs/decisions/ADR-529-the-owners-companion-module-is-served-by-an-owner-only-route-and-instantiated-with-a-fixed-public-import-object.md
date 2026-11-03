# ADR-529: The owner's companion module is served by an owner-only route and instantiated with a fixed public import object

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-511 |
| builds on | ADR-336, ADR-348, ADR-349, ADR-519 (ADR-348's dependency rule is narrowed for one crate: D4) |
| measured at | `92755ef0` |

## Context

SPEC-501 and ADR-519 made the scheduling companion's interface: integer-only frames, a decision in public code that
fails closed, and a public body that answers `Absent`. The owner's build may carry a companion as a WebAssembly module
of its own, beside the public engine's module in the web client's Worker, and public builds carry none. The stack takes
Rust to WebAssembly only by measured exception (`rust-wasm`, ADR-348): logic that must be bit-identical across clients,
run in a dedicated Worker, under ADR-336's gzip budget. The companion's state must hash the same on every client, which
is that exception's clause. This ADR decides where the owner's module is served from, how the Worker loads it, what
the Worker keeps of it, how the web engine decides over its frames, and what the budget counts. Every fact below was
read at `92755ef0`.

## D1. An owner-only route with three literal names, and what it was chosen against

- **Chosen: `GET /api/companion/v1/manifest.json`, `/module.wasm` and `/weights.bin`, three axum routes with no path
  parameter, behind identity's `OwnerSession`.** The session is judged first: 401 `no_session` for no session and for
  a `link` session. Then the directory: 404 `companion_absent` when `DECKSTREAK_COMPANION_DIR` is unset, a file is
  missing, or a file is not a regular file, which is every public deployment's answer. A present file answers 200 with
  its exact bytes, its content type and `nosniff`; every answer of the three routes, a refusal included, carries
  `Cache-Control: private, no-store`. A file over 16 MiB is refused 404 `companion_over_cap`, and no read takes more
  than the cap plus one byte. The directory is a setting the `api` role reads before it binds; a relative path refuses
  start by the setting's name.
- **A static path under the web root:** lost. Every file under the web root is public (`deploy/caddy/deck-streak.caddy`
  serves it to anyone), so the owner's module would be a download for every visitor, and no gate would stand between
  them.
- **Caddy forward-auth in front of a static directory:** lost. The session check belongs in Rust, where identity's
  sessions live and where a test can drive it; a proxy rule that copies it is a second check nobody tests.
- **One route with a `{name}` parameter:** lost. A name read from the request would reach the file system, and
  traversal would be a property of a filter rather than of the route's shape. Three literal routes give no request text
  a path to reach.
- **A server-side companion that answers over the API:** lost. The server is a relay for the owner's collection and
  computes no companion state; a companion on the server would read every review on the server's side.

## D2. The Worker digest-checks the module and instantiates it with a fixed public import object

- **Chosen: the loader fetches the manifest, then the module and the weights, checks each one's SHA-256 with Web
  Crypto against the manifest, and calls `WebAssembly.instantiate(bytes, IMPORTS)`, where `IMPORTS` is a frozen empty
  object in public code.** A 404, a malformed manifest, a digest mismatch, a file over 16 MiB, a failure to link (any
  import the module asks for) and a missing export each read `Absent`. The module crosses a raw byte-frame ABI:
  `memory`, `alloc(len) -> ptr`, `free(ptr, len)`, `open(ptr, len) -> u64` and `call(ptr, len) -> u64`, the `u64`
  packing the reply's pointer (high 32 bits) and length (low 32 bits). The Worker copies bytes in and out and builds
  no frame: the web engine builds every frame it sends (D4).
- **Why the digest and the import object both:** the manifest comes from the same route as the module, so its digest
  detects corruption, not substitution. Substitution is bounded by the import object: a module given nothing can read
  no storage, no network and no clock, so a substituted module can only compute wrong replies, which `decide` range
  checks (ADR-519 D4).
- **Importing the module's bindings script from the owner route:** lost. A script runs with the Worker's whole scope,
  its origin private file system, its network and its clock; its digest cannot be checked before the import runs it.
  The raw ABI needs no script.
- **One relinked module holding the public engine and the companion:** lost. It would put the closed companion and the
  AGPL engine in one artifact, make the owner's public module differ from the public build, and move the size gate's
  subject.

## D3. The Worker keeps the checkpoint verbatim, and loads it only through the web load check

- **Chosen: the checkpoint frame the companion answers is stored verbatim as `state.bin` in the origin private file
  system's `deck-streak-companion` directory, and read back byte-equal; purge sends `Purge` and removes the whole
  directory.** A stored checkpoint is handed back only as the `Restore` frame the web engine builds through
  `Restore::from(CheckpointReply)` (ADR-519 D2), never from raw bytes in the Worker.
- **The web load check, in public Rust:** the checkpoint is kept only when the hello negotiates, the stored frame is an
  `Ok` checkpoint of the negotiated model and the hello's adapter mode, the companion's reply to `Restore` is an `Ok`
  `Ack`, its head names a review in the store unless nothing was folded, and a fold index is held whose count and head
  equal the `Ack`'s. Each failure is one named discard, and the Worker purges and folds from genesis. A torn write
  fails to decode and is discarded the same way.
- **What waits:** the fold index and the engine's answer whether a review id names a review come with the press path
  (#997); until then no fold index is held, so every checkpoint with reviews folded is discarded. That direction is
  safe: a discarded checkpoint costs a re-fold, a wrongly kept one would schedule from a state the log does not hold.
- **No device category yet:** no product path writes a checkpoint in this change, so the privacy inventory is
  unchanged; the change that first writes one declares the category with its erase (ADR-519 D7).

## D4. The web engine links the interface crate on every target and decides as the native build does

- **Chosen: `deck-streak-web-engine` depends on `deck-streak-companion` under `[dependencies]`, and
  `src/companion.rs`, compiled on every target, holds `decide`, `restore`, `load_check` and `purge`.** `wasm.rs`
  exports each as one call. The interface crate is `no_std`, depends on nothing and does no I/O, so the native graph
  gains no engine and no I/O through this edge: ADR-348's rule that every dependency of the web engine is `wasm32`-only
  is narrowed for this one crate, because its decision must be tested natively over the same vectors the native
  decision answers.
- **`decide` answers bytes:** 17 for an admitted due date and 3 for a refusal, in the layout SPEC-511 R10 states, so the
  Worker moves bytes and the comparison with the native decision is byte for byte over every decision vector of
  `formal/vectors/companion-decide.jsonl`.
- **A copy of the decision in TypeScript:** lost. Two implementations of one decision drift; the bit-identical clause
  of `rust-wasm` exists to forbid that.
- The context map's fence gains `companion` on the web engine's row, and the schematic its arrow.

## D5. The budget counts every owner file, and the public site holds none of them

- **Chosen: `scripts/web-engine-size.py --owner-file PATH`, any number of times, sums every file it is given against
  ADR-336's 8,000,000 bytes `gzip -9 -n`.** The owner's page downloads the public module and its bindings plus the
  companion's module and weights; a budget that counted two of the four would pass a page over it. The owner's lane
  runs the gate over its four files; the public job keeps its two.
- **A census of the built site:** `scripts/companion-site-census.py` refuses a file named `module.wasm` or
  `weights.bin`, any file under `api/`, a `manifest.json` that names either file, and a WebAssembly module outside
  `engine/`. CI's `web-engine` job runs it over the staged build, so a companion file copied into the site fails CI
  before it can be deployed.

## Consequences

- Good: every public deployment answers 401 or 404 on the route and loads `Absent`; the owner's module reaches the
  owner's session alone, uncached, and runs with no capability but its own memory.
- Good: the decision that lets a due date be written stays one Rust function on both clients, checked byte for byte.
- Bad: the module's digest proves integrity against the same route, not provenance; provenance stays with the owner's
  own deployment of the directory (#998).
- Bad: until the press path lands, a stored checkpoint is never kept past a fold, so the load sequence is exercised
  only by tests.

## Confirmation

SPEC-511 C1 to C19: the route's tests in `crates/api/tests/companion_route.rs`, the loader's in
`web/app/src/lib/engine/companion.test.ts`, the web engine's in `crates/web-engine/tests/companion_decide.rs` and
`companion_load.rs`, the gate's and the census's in `scripts/tests`, and the context map's test.
