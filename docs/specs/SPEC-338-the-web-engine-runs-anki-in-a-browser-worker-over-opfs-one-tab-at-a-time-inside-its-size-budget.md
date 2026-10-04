# SPEC-338: the web engine: Anki's engine runs in a browser Worker over OPFS, one tab at a time, inside its size budget

- **Wave:** the app campaign, Phase 1 (SPEC-334 row 1.4, R5). **Issue:** #626. **Context(s):**
  `deck-streak-web-engine` (added: the engine for the browser's Worker) and `miniapp`
  (`web/app/src/lib/engine/`, the Worker and its client).
- **Decided by:** ADR-348 (the fork's patches and their pin, the crate's shape and its JS boundary,
  the SQLite route, the context that refuses OPFS, and the Mini App's embedded context) and ADR-349
  (the size gate's form and the content security policy). It builds the outcome ADR-336 accepted on
  the browser engine spike's GO (SPEC-335, ADR-346), under ADR-058 (the patched fork), to which
  this delivery appends a note.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-338.md`. **Mutation band:** `S33800-S33899`.

## 1. The problem, measured

- **Nothing in this repository builds for the browser.** The browser engine spike built Anki's
  engine for `wasm32-unknown-unknown` on the fork's spike branch and measured it in a harness on that
  branch (SPEC-335 section 7); this repository gained documents only. No workspace member targets
  `wasm32`, no Worker owns an engine, and no CI job reads the module's size.
- **The spike's branch does not resolve in this workspace.** With the root manifest's `[patch]` rev
  moved to the spike branch's tip `98d1455385dfea67cac832fa98f11a54004f8233`,
  `cargo metadata --format-version 1` exits 101: the branch's SQLite route, `rusqlite` 0.40, needs
  `libsqlite3-sys` `^0.38.1`, while this workspace's `sqlx-sqlite` 0.9.0 (the kernel's repository
  base) accepts `>=0.30.1, <0.38.0`, and `links = "sqlite3"` admits one `libsqlite3-sys` in a graph.
  No `sqlx-sqlite` release accepts 0.38 (the crates.io index reads 0.9.0 as the newest). `rusqlite`
  0.39 routes `wasm32-unknown-unknown` to `sqlite-wasm-rs` (whose `links` key is `wsqlite3`) and
  takes `libsqlite3-sys` `^0.37.0` natively, inside `sqlx`'s range (ADR-348).
- **The spike's engine does not build for `wasm32` on this workspace's lockfile.** With the engine
  patched in on rusqlite 0.39, `cargo check --target wasm32-unknown-unknown -p
  deck-streak-web-engine` exits 101: this lockfile holds `tracing-appender` 0.2.5, whose rolling
  module calls two `symlink` functions that crate builds only for Unix, Windows and Redox (E0425,
  twice). The fork's own lockfile holds 0.2.3, which is why the spike never met it.
- **The page's policy refuses WebAssembly compilation.** `web/app/svelte.config.js`'s `kit.csp`
  sets `script-src` to `self` and Telegram only, and a policy whose `script-src` lacks
  `'wasm-unsafe-eval'` blocks `WebAssembly.compile` and `instantiate` in every context it governs.
- **SPEC-335 section 5 left ten items to this delivery (#626)**, each unmeasured at the spike's GO:
  a phone-class device; the Worker's peak memory; persistence across a page reload and under
  storage eviction; a second tab on one collection; the brotli size; the module's size with the
  translation table loaded apart from the engine; a fallback for a context that refuses OPFS; the
  CI size-budget gate; the fork commits pinned in `Cargo.toml`; the note appended to ADR-058. The
  spike's own list of what it did not measure adds three: the build variants (`lto` and
  `opt-level = "z"`) against the budget; a collection of realistic size, where the spike seeded 300
  notes; and media and import or export on `wasm32`. Section 3's criteria and section 7's
  measurements answer each.

## 2. Requirements

R1. A workspace member `deck-streak-web-engine` (`crates/web-engine`) builds for
    `wasm32-unknown-unknown` in release as a `cdylib`, and natively as a crate whose only code is
    target-independent: its study rule (a wire rating to Anki's answer and to the next state it
    selects, and the table of study calls `run_method` admits) and the synthetic notes' fields,
    with native tests. Its `wasm32` build exports, through `wasm-bindgen`, the study calls the
    Worker makes (install the storage, open, close, seed a synthetic collection into an empty one,
    the next card, answer, undo, and a read-only snapshot of one card), the module's linear memory
    in pages, and the backend's `run_method`, which refuses by name any service and method outside
    the table, so no exempt write of ADR-337 is reachable from the page.
R2. The engine comes from the fork's branch `wasm32-26.09.3` at its tag
    `deckstreak-pin-26.09.3-wasm32`: ADR-058's one commit plus one commit per `wasm32` patch, ten,
    each naming its patch, its reason and its removal condition (ADR-336, ADR-348). The root
    manifest's `[patch]` entry pins `anki` and `anki_proto` by `rev` to the tag's commit, and the
    dependency line keeps the upstream tag `26.09.3`. The lockfile resolves one `libsqlite3-sys`,
    shared by the engine and `sqlx`.
R3. A dedicated module Worker owns the engine. The page reaches it only through `EngineClient`,
    whose requests carry an id and whose every reply carries that id with a value or an error code.
    The Worker accepts only the protocol's operations, refuses a malformed request with
    `bad-request` before it reaches the engine, and loads the engine only after the tab lock and
    the storage are held. Its `memory` operation answers the module's linear memory in bytes
    (pages of 65536 bytes); that memory never shrinks, so a reading taken after each step is the
    Worker's high-water so far, which section 7's peak memory reads.
R4. The collection lives in OPFS through the SyncAccessHandle pool VFS, one pool directory and one
    collection path per origin. The page requests persistent storage with
    `navigator.storage.persist()` and surfaces the answer as `persisted`, `not-persisted` or
    `unsupported`.
R5. One collection per origin across tabs: the Worker takes the Web Lock `deck-streak-collection`
    with `ifAvailable` before it installs the pool. A second tab's Worker answers `collection-busy`
    and installs no pool and opens no collection.
R6. A context that refuses OPFS, as WebKit's ephemeral context does, gets `storage-refused` with a
    message naming why, and the Worker loads no engine: no in-memory collection is offered in its
    place (ADR-348).
R7. The page's policy admits `'wasm-unsafe-eval'` in `script-src` and nothing else new; every
    other directive is unchanged.
R8. CI's `web-engine` job builds the module (`cargo build --release --target
    wasm32-unknown-unknown`, `wasm-bindgen --target web`, `wasm-opt -Oz`) and
    `scripts/web-engine-size.py` fails it when the module plus its JS bindings exceed 8000000 bytes
    `gzip -9` (ADR-336's budget), records their brotli size beside it, and reads VOID, never a
    pass, when either file is missing or empty or a compressor is absent. The `ci` aggregate needs
    the job (ADR-349).
R9. In Chromium and WebKit under Playwright, with a persistent profile, the built module opens a
    synthetic collection, answers a card and undoes the answer correctly, keeps the collection
    across a page reload, and refuses a second tab; WebKit's ephemeral context reads
    `storage-refused`.
R10. ADR-058 gains an appended note naming ADR-336, ADR-348 and the patch list; its body stays.
R11. Every SPEC-335 `(#626)` item, and each of the three the spike's own list adds, is measured
    in section 7 or decided here, each with its method, and a measurement taken by an
    approximation says so.
R12. The synthetic collection `seed` writes has the shape of ADR-022's measured collection: each
    note carries two fields of 200 characters, and no two notes share a front, so a measurement
    at a realistic size stores and indexes what a real collection would.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | The study rule maps each wire rating, 1 to 4, to Anki's answer (again, hard, good, easy) and selects that answer's next state | `cargo test -p deck-streak-web-engine --test study -- --exact a_rating_on_the_wire_picks_its_answer_and_its_next_state` |
| A2 | A wire rating outside 1 to 4 is refused by name | `cargo test -p deck-streak-web-engine --test study -- --exact a_rating_outside_one_to_four_is_refused` |
| A17 | `run_method` admits exactly the study calls' services and methods, and refuses every other pair by name, the exempt writes included | `cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls` |
| A3 | The size gate fails a module plus bindings over 8000000 bytes `gzip -9` (a planted over-budget control) | `python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_module_over_the_budget_fails_the_gate` |
| A4 | The size gate passes a module plus bindings under the budget, printing each file's `gzip -9` and brotli sizes and the total against 8000000 | `python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_module_under_the_budget_passes_and_records_brotli` |
| A5 | The size gate reads VOID, not a pass, when the module or its bindings is missing or empty | `python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_missing_or_empty_file_is_void_not_a_pass` |
| A6 | The page's policy admits `'wasm-unsafe-eval'` and nothing else new | `pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"` |
| A7 | The client pairs each reply with its request by id, and an error reply rejects with its code | `pnpm exec vitest run web/app/src/lib/engine/client.test.ts -t "the client pairs each reply with its request and rejects an error with its code"` |
| A8 | The Worker refuses a malformed or unknown request with `bad-request` before the engine loads | `pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a malformed or unknown request is refused before the engine loads"` |
| A9 | A second tab is refused with `collection-busy` while the first holds the lock: no pool, no open; in the browsers, `web/app/tests-engine/engine.spec.ts` "a second tab is refused" | `pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a second tab is refused while the first holds the collection"` |
| A10 | A context that refuses OPFS is refused with `storage-refused` and loads no engine; in WebKit's ephemeral context, `engine.spec.ts` "a context that refuses OPFS is refused" | `pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a context that refuses OPFS is refused with storage-refused"` |
| A11 | The session opens, answers and undoes through the engine, and the undo restores the card; in the browsers, `engine.spec.ts` "opens, answers and undoes over OPFS" and "the collection survives a page reload" | `pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "the session opens, answers and undoes through the engine"` |
| A12 | The page's persistence request is surfaced as `persisted`, `not-persisted` or `unsupported` | `pnpm exec vitest run web/app/src/lib/engine/persistence.test.ts -t "the persistence answer is persisted, not-persisted or unsupported"` |
| A13 | The Worker's entry serves the session on the Worker's own scope and loads the module the build ships | `pnpm exec vitest run web/app/src/lib/engine/worker.test.ts -t "the worker serves the session on its own scope"` |
| A14 | The root manifest patches `anki` and `anki_proto` by `rev` to one commit of the fork, and every engine package in the lockfile comes from it | `python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork` |
| A15 | ADR-058's appended note records the pinned commit, its difference from the upstream tag and each patch | `python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_adr_058_records_the_pinned_commit_and_what_it_saves` |
| A16 | CI's `web-engine` job builds the module, runs the size gate and the browser tests, and the `ci` aggregate needs it | `python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_web_engine_job_builds_the_module_and_holds_it_to_its_budget` |
| A18 | The session answers `memory` with the module's linear memory in bytes, its pages times 65536, once the collection is open, and `not-open` before | `pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "the session reports the module's memory in bytes"` |
| A19 | Each synthetic note carries two fields of 200 characters, and no two notes share a front | `cargo test -p deck-streak-web-engine --test synthetic -- --exact each_synthetic_note_carries_two_fields_of_two_hundred_characters` |

```acceptance
A1: cargo test -p deck-streak-web-engine --test study -- --exact a_rating_on_the_wire_picks_its_answer_and_its_next_state
A2: cargo test -p deck-streak-web-engine --test study -- --exact a_rating_outside_one_to_four_is_refused
A17: cargo test -p deck-streak-web-engine --test study -- --exact run_method_admits_only_the_study_calls
A3: python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_module_over_the_budget_fails_the_gate
A4: python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_module_under_the_budget_passes_and_records_brotli
A5: python3 -m unittest discover -s scripts/tests -p test_web_engine_size.py -k a_missing_or_empty_file_is_void_not_a_pass
A6: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"
A7: pnpm exec vitest run web/app/src/lib/engine/client.test.ts -t "the client pairs each reply with its request and rejects an error with its code"
A8: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a malformed or unknown request is refused before the engine loads"
A9: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a second tab is refused while the first holds the collection"
A10: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "a context that refuses OPFS is refused with storage-refused"
A11: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "the session opens, answers and undoes through the engine"
A12: pnpm exec vitest run web/app/src/lib/engine/persistence.test.ts -t "the persistence answer is persisted, not-persisted or unsupported"
A13: pnpm exec vitest run web/app/src/lib/engine/worker.test.ts -t "the worker serves the session on its own scope"
A14: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_the_engine_is_patched_by_rev_to_a_commit_of_the_fork
A15: python3 -m unittest discover -s scripts/tests -p test_engine_pin.py -k test_adr_058_records_the_pinned_commit_and_what_it_saves
A16: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_web_engine_job_builds_the_module_and_holds_it_to_its_budget
A18: pnpm exec vitest run web/app/src/lib/engine/session.test.ts -t "the session reports the module's memory in bytes"
A19: cargo test -p deck-streak-web-engine --test synthetic -- --exact each_synthetic_note_carries_two_fields_of_two_hundred_characters
```

The browser criteria (A9, A10, A11) name their Playwright tests in the table, and their fence lines
run the Vitest test of the same rule, because a fence line resolves to a Vitest, cargo or unittest
test only. CI's `web-engine` job runs the Playwright tests over the module it builds (A16).

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/web-engine/Cargo.toml` | `deck-streak-web-engine` | added |
| `crates/web-engine/src/lib.rs` | `deck-streak-web-engine` | added: the crate root |
| `crates/web-engine/src/study.rs` | `deck-streak-web-engine` | added: the study rule and the call table, both targets |
| `crates/web-engine/src/wasm.rs` | `deck-streak-web-engine` | added: the `wasm-bindgen` exports, `wasm32` only |
| `crates/web-engine/src/synthetic.rs` | `deck-streak-web-engine` | added: the synthetic notes' fields, both targets |
| `crates/web-engine/tests/study.rs` | `deck-streak-web-engine` | added: A1, A2, A17 |
| `crates/web-engine/tests/synthetic.rs` | `deck-streak-web-engine` | added: A19 |
| `Cargo.toml` | workspace | the crate's dependencies, and the `[patch]` entry's `rev` and `anki_proto` |
| `Cargo.lock` | workspace | changed by `cargo` only |
| `deny.toml` | workspace | the licences of the crates the module adds, if the audit names one |
| `stack.json` | workspace | the `rust-wasm` element, citing ADR-348 |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | the doc comment names the pinned commit's ADRs, not one fix |
| `docs/CONTEXT-MAP.md` | docs | the crate's line in the map |
| `docs/schematics/app-clients-engine-and-sync.md` | docs | one line naming this delivery's schematic |
| `docs/TESTING.md` | docs | the `web-engine` job |
| `scripts/web-engine-build.sh` | gate | added: builds the module and its bindings |
| `scripts/web-engine-size.py` | gate | added: the size gate |
| `scripts/tests/test_web_engine_size.py` | gate | added: A3, A4, A5 |
| `scripts/tests/test_engine_pin.py` | gate | A14, A15: the patch takes `anki` and `anki_proto`; ADR-058's appended note |
| `scripts/tests/test_ci_workflows.py` | gate | A16, and the `ci` aggregate's needs |
| `.github/workflows/ci.yml` | gate | the `web-engine` job and the `ci` job's need of it |
| `scripts/mutation-rows.d/S33800-S33899.json` | gate | added: this SPEC's rows |
| `web/app/svelte.config.js` | `miniapp` | `'wasm-unsafe-eval'` in `script-src` |
| `web/app/src/lib/csp.test.ts` | `miniapp` | A6 |
| `web/app/src/lib/engine/protocol.ts` | `miniapp` | added: the Worker's requests, replies and error codes |
| `web/app/src/lib/engine/client.ts` | `miniapp` | added: `EngineClient` |
| `web/app/src/lib/engine/client.test.ts` | `miniapp` | added: A7 |
| `web/app/src/lib/engine/session.ts` | `miniapp` | added: the Worker's session: lock, storage, engine |
| `web/app/src/lib/engine/session.test.ts` | `miniapp` | added: A8 to A11, A18 |
| `web/app/src/lib/engine/persistence.ts` | `miniapp` | added: the persistence request |
| `web/app/src/lib/engine/persistence.test.ts` | `miniapp` | added: A12 |
| `web/app/src/lib/engine/worker.ts` | `miniapp` | added: the Worker's entry |
| `web/app/src/lib/engine/worker.test.ts` | `miniapp` | added: A13 |
| `web/app/engine-harness/index.html` | `miniapp` | added: the page the browser tests drive |
| `web/app/engine-harness/main.ts` | `miniapp` | added: the page's script |
| `web/app/vite.engine.config.ts` | `miniapp` | added: builds and serves the harness under the page's policy |
| `web/app/playwright.engine.config.ts` | `miniapp` | added: Chromium and WebKit, persistent profiles |
| `web/app/tests-engine/engine.spec.ts` | `miniapp` | added: the browser tests and the measurements |
| `web/app/package.json` | `miniapp` | the `test:engine` script |
| `docs/specs/SPEC-338-the-web-engine-runs-anki-in-a-browser-worker-over-opfs-one-tab-at-a-time-inside-its-size-budget.md` | docs | added |
| `docs/schematics/web-engine-worker-and-opfs.md` | docs | added |
| `docs/decisions/ADR-348-the-web-engine-pins-the-forks-wasm32-patches-by-tag-and-crosses-into-a-worker-through-wasm-bindgen.md` | docs | added |
| `docs/decisions/ADR-349-ci-builds-the-module-and-holds-it-to-8000000-bytes-and-the-page-admits-wasm-compilation.md` | docs | added |
| `docs/decisions/ADR-058-the-engine-pins-a-patched-fork-of-26-09-3-until-upstream-carries-the-fix.md` | docs | a note appended; its body stays |
| `docs/red-first/SPEC-338.md` | docs | added |
| `changelog.d/web-engine-338.md` | docs | added |

## 5. What this does NOT do

- It ports no sync transport to the browser: the streamed transport refuses on `wasm32` behind the
  fork's gate, and the web sync screens decide the browser's transport and the syncs at session
  start and end (#631).
- It measures no real Safari on a device: the WebKit figures are Playwright's WebKit build, and the
  owner's acceptance session reads Safari on iPhone, iPad and the web (#637).
- It builds no study screen: the Worker's client is the screens' boundary, and the review screen,
  the deck list and the card frame are the web study screens' (#626 row 1.4, built by the study
  screens' delivery).
- It loads no translation table apart from the engine: section 7 measures the table's share of the
  module and the shipped total stays inside the budget with it embedded, so a split, which would be
  one more engine patch, is not made (#626).
- It runs no FSRS optimiser and no backup in the browser: those engine paths need threads or read
  the standard clock on `wasm32` (SPEC-335 section 6), and the open, answer and undo path reaches
  neither (#626).
- It puts no FSRS-7 beside the engine in the Worker: the isolated FSRS-7 crate is its own delivery,
  and compiles to the same target there (#641).
- It builds no `OwnerGesture` and admits no exempt write: `run_method`'s table holds the study calls
  alone, and the engine core with its allow-listed dispatcher and the gesture token owns the rest
  (#623).
- It stores and syncs no media in the browser: the engine keeps media in a folder through the
  standard file system, which `wasm32-unknown-unknown` refuses, so media reach the Worker through
  the web sync screens' transport (#631) and show in the card frame (#619).
- It imports and exports no package in the browser: in-app import is a later parity phase (#611),
  and the backup at a full sync is the web sync screens' (#631).
- It adds no stage to `scripts/check.sh`: the module's build needs a `wasm32` target, `wasm-bindgen`
  and `wasm-opt`, which the local gate does not require, so CI's `web-engine` job runs it (#626,
  ADR-349).

## 6. Risks

- **The SQLite line moves for the native build.** `rusqlite` 0.39 moves the engine's bundled SQLite
  and `sqlx`'s `libsqlite3-sys` from 0.34 to 0.37. The `rust`, `engine` and `release` jobs run the
  workspace's and the engine set's tests on it, and `engine-measure.yml` re-runs ADR-022's budgets.
- **The module grows past its budget.** Each engine change can add bytes; the `web-engine` job fails
  by name above 8000000 bytes `gzip -9`.
- **A second tab corrupts the collection.** The SyncAccessHandle pool holds its files exclusively;
  the Web Lock taken first turns the second open into `collection-busy` (A9, and the browser test).
- **The browser evicts the collection.** Persistent storage is requested and its answer surfaced
  (A12); a collection lost to eviction reopens empty and says so (`existed: false`), and the sync at
  session start restores it (#631).
- **A phone is slower than the desktop.** Section 7 measured that a throttled desktop CPU does not
  slow the Worker (M12), so it claims no phone-class figure; the owner's acceptance session reads a
  real phone (#637).
- **The fork's tag stops resolving.** The tag is never moved or deleted while it is pinned
  (ADR-058); a fresh fetch in CI would fail by name.

## 7. Measured

Each row names its method. Figures are appended as they are taken and never edited.

| id | the SPEC-335 `(#626)` item, or this delivery's own | figure | method |
|---|---|---|---|
| M1 | The spike's branch in this workspace | does not resolve: `libsqlite3-sys` `^0.38.1` against `sqlx-sqlite` 0.9.0's `<0.38.0` | `cargo metadata --format-version 1` with the `[patch]` rev at `98d1455385dfea67cac832fa98f11a54004f8233` |
| M2 | getrandom's browser backend in this workspace, with no flag and no `.cargo/config.toml` | builds: `cargo check --target wasm32-unknown-unknown -p deck-streak-web-engine` exits 0 on this lockfile's getrandom 0.3.4 and 0.4.3, each selecting `wasm_js` by its feature | the check, with only the C compiler and archiver for `wasm32` set on its command line |
| M3 | The brotli size, beside the budget's `gzip -9` | module plus bindings: `gzip -9` 7534582 bytes, 465418 under 8000000; brotli quality 11, 3490230 bytes (module 7525719 and 3482547, bindings 8863 and 7683) | `scripts/web-engine-size.py` over the module `scripts/web-engine-build.sh` wrote from this branch |
| M4 | The module's size with the translation table loaded apart from the engine | the module holds the table twice: each of the 2132 string literals of the i18n crate's generated table occurs twice, because the table is a `const` read at two generic sites, so each emits a copy. With every copy zeroed the module reads `gzip -9` 3247939 (the table's share 4277780, 56.8%) and brotli 2240869 (share 1241678, 35.7%); raw, the copies are 15730960 of 23901294 bytes. The literals alone compress to 2137818 `gzip -9` and 1233167 brotli | every occurrence of each literal in the i18n crate's generated `strings.rs` zeroed in a copy of the module, then `scripts/web-engine-size.py`; zeroing one copy only cuts `gzip -9` by the literals' own size and brotli by nothing, since brotli's window reaches the other copy |
| M5 | The Worker's peak memory | a study session over 250,000 notes: 37683200 bytes of linear memory after open, the queue, an answer and its undo, in both browsers; over 300 notes, 20643840. Seeding 250,000 notes in one request reaches 804126720 (Chromium) and 804061184 (WebKit), the seed's own high-water | the `memory` operation after each step (A18), in `engine.spec.ts`'s measurement test |
| M6 | A collection of ADR-022's size: 250,000 notes of two 200-character fields | Chromium: open 256.3 ms (the module's load, the pool, the engine and the open), the queue 82.9, an answer 8.9, its undo 4.0, open after a reload 262.2; the seed 14352.9. WebKit: 412, 112, 7, 4 and 385; the seed 13762 | `ENGINE_MEASURE_NOTES=250000`, `engine.spec.ts`'s measurement test, headless desktop-class browsers, one run each |
| M7 | Persistence across a page reload | kept in both browsers: five notes and the answered card's one review, and 250,000 notes | `engine.spec.ts` "the collection survives a page reload", and the measurement test's reloads |
| M8 | Persistence under storage eviction | Chromium: an origin cleared of all its storage reopens as `{existed: false, notes: 0}`, by name. WebKit: NOT MEASURED, no protocol call clears an origin (#637) | `engine.spec.ts`'s eviction test, the protocol's `Storage.clearDataForOrigin` |
| M9 | A second tab on one collection | refused in both browsers with `collection-busy`, "another tab holds the collection", while the first tab answers on | `engine.spec.ts` "a second tab is refused" |
| M10 | A context that refuses OPFS | WebKit's ephemeral context reads `storage-refused`, and no engine loads | `engine.spec.ts` "a context that refuses OPFS is refused" |
| M11 | The Mini App's framed context | a cross-site frame opens its own collection, seeds, answers and undoes in both browsers, over storage partitioned to the frame | `engine.spec.ts` "measures the engine in a cross-site frame": one loopback name frames the harness from the other |
| M12 | A phone-class device | NOT MEASURED: a 4x throttle of the page's CPU left the Worker at 0.93x to 1.55x of its unthrottled figures over 250,000 notes (the seed 1.09x, the queue 1.55x, an answer 0.93x, the undo 1.00x), so the throttle does not stand for a slower CPU in the Worker; a real phone is read in the owner's acceptance session (#637) | `ENGINE_CPU_THROTTLE=4`, the protocol's `Emulation.setCPUThrottlingRate` on the page, against the same run unthrottled |
| M13 | The page's policy governs the Worker | without `'wasm-unsafe-eval'` the module does not load: the Worker answers `engine-failed`, quoting the `script-src` directive it violates | a plant of `svelte.config.js`'s `script-src`, `engine.spec.ts` "opens, answers and undoes over OPFS" in Chromium, then the file restored |
