# SPEC-335: the browser engine spike: Anki's engine builds for wasm32 and opens, answers and undoes in a Worker over OPFS in Chromium and WebKit

- **Wave:** the app campaign, Phase 0 (SPEC-334 rows 1.1 and 1.2). **Issue:** #614 (the browser
  engine spike). **Context(s):** none in this repository: the spike's code and its harness live on
  the engine fork's spike branch, and this repository gains documents only.
- **Decided by:** ADR-346 (this SPEC's own: the fork's wasm32 gates and the measuring harness).
  It measures the outcome of ADR-336 (the web client runs the engine in the browser, with a
  server-side study collection as the fallback), which this delivery sets to `accepted` with GO,
  and it works under ADR-058 (the patched fork) and ADR-022 (the engine's spike protocol).
- **Status:** judged: a spike record. It adds no code to this repository and states no acceptance
  command (section 3). What was measured is in section 7, each row naming the fork commit and the
  harness command that measured it.

## 1. The problem, measured

- **The decision this settles.** ADR-336 proposes that a web study session runs Anki's engine (the
  `anki` crate, at the fork commit this repository pins) in the browser, in a Worker over the
  origin-private file system (OPFS), and that the server-side study collection is the fallback.
  Its decision outcome waits for this spike: GO or NO-GO, with a gzipped size and a measured open,
  answer and undo in Chromium and WebKit.
- **The engine did not build for the browser.** At the pinned fork commit
  `57382da085e6752738dc4bb617789be836a23300`, an ungated
  `cargo check --target wasm32-unknown-unknown -p anki` fails in five dependency crates: `mio`
  (the async runtime's network feature, which has no wasm32 support), `getrandom` on two lines (no
  browser randomness backend selected), `libsqlite3-sys` (the engine's rusqlite line has no wasm32
  route) and `zstd-sys` (multithreaded compression needs a threads header the target lacks).
- **Nothing about the browser was measured before this spike:** not the module's size, not an
  open, an answer or an undo, and not whether a browser lets the engine keep its collection in
  OPFS.

## 2. Requirements

Each requirement is judged by the measurement in section 7: met, not met, or not measured.

R1. The engine builds for `wasm32-unknown-unknown` in release from the fork's spike branch, both
    alone (`-p anki`) and inside a harness module that exports the backend's `run_method`.
    **Judged: met** (M1).
R2. The gates leave the native build as it was: the engine's native check reads the same before
    and after them. **Judged: met** (M2).
R3. The module a browser loads plus its JS bindings, compressed with `gzip -9`, fits the size
    budget: 8000000 bytes gzip -9 for the shipped module plus its JS bindings, set by the DeckStreak
    architect from this measurement and recorded in ADR-336's decision outcome.
    **Judged: met**, at 7529787 bytes (M3).
R4. In a dedicated Worker, over OPFS through the SAH-pool VFS, the engine opens a synthetic
    collection of 300 notes, builds the study queue, answers a card and undoes the answer, in
    Chromium and in WebKit under Playwright, five runs each, with no error and no console error.
    **Judged: met**, 5 of 5 in each browser (M5, M6).
R5. The undo is correct: after the answer the card has one review-log row and the cards table has
    changed; after the undo the card has no review-log row and the cards table is byte-equal to
    its state before the answer. **Judged: met**, 5 of 5 in each browser (M7).
R6. The collection persists in OPFS across a close and a reopen inside one page, with all 300
    notes read back, and the database runs in `wal` journal mode. **Judged: met** (M8).
R7. The engine opens its collection in Playwright's default ephemeral browser context as well as
    in a persistent profile. **Judged: not met in WebKit**, which refuses the OPFS root in the
    ephemeral context; Chromium ran in both (M9).
R8. The collection persists across a page reload and survives the browser's storage eviction.
    **Judged: not measured** (section 5, #626).
R9. The timings hold on a phone-class device. **Judged: not measured** (section 5, #626).

## 3. Acceptance criteria

This SPEC adds no code to this repository, so it states no acceptance criterion here and its fence
is empty. The spike's code and its harness live on the engine fork's spike branch, and the
measurements that judge R1 to R9 are in section 7, each row naming the fork commit and the harness
command. No test is added here: a spike record has no later builder, and a test that read this file
back would prove only that the file was written.

```acceptance
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-335-the-engine-builds-for-wasm32-and-studies-in-a-browser-worker-over-opfs.md` | none: the spike record | added |
| `docs/decisions/ADR-346-the-engine-builds-for-wasm32-behind-gates-on-the-fork-and-a-worker-harness-measures-it.md` | none: the spike record | added |
| `docs/decisions/ADR-336-the-web-client-runs-the-engine-in-the-browser-on-wasm-with-a-server-side-fallback.md` | none: the spike record | changed: `status: accepted`, and GO, the measurements and the size budget in its decision outcome |
| `docs/schematics/app-clients-engine-and-sync.md` | none: the spike record | changed: one edge label states REL-01 (dev builds always sync to the staging sync user) |
| `changelog.d/spike-wasm-engine-335.md` | none: the spike record | added |

## 5. What this does NOT do

- It measures no phone-class device: every timing here comes from a headless desktop browser
  profile under Playwright; the web engine measures a phone-class device (#626).
- It measures no peak memory of the Worker and its module (#626).
- It measures no persistence across a page reload and no storage eviction: the collection was
  reopened inside one page only (#626).
- It opens no second tab on one collection, which the SAH-pool VFS holds exclusively (#626).
- It measures no brotli size: only `gzip -9` (#626).
- It measures no module size with the translation table loaded apart from the engine: only the
  table's generated source was measured (#626).
- It measures no other build profile: no link-time optimisation and no size-first optimisation
  level, only the release profile followed by `wasm-opt -Oz` (#626).
- It measures no collection of realistic size: the scenario seeds 300 synthetic notes only (#626).
- It runs no media and no import or export on wasm32: the scenario opens, answers and undoes only
  (#626).
- It builds no fallback for a browser context that refuses OPFS, as WebKit's ephemeral context does
  (#626).
- It adds no CI size-budget gate, pins none of the spike's fork commits in `Cargo.toml` and appends
  no note to ADR-058: the web engine delivery does each when it adopts the branch (#626).
- It ports no sync transport to wasm32: the streamed transport refuses there behind a gate, and the
  web sync screens decide the browser's transport (#631).
- It measures no real Safari: the WebKit figures are Playwright's WebKit build, headless, and the
  owner's acceptance session reads Safari on iPhone, iPad and the web (#637).

## 6. Risks

- **The phone is slower than the desktop profile.** Every timing in section 7 is a headless desktop
  browser profile. The web engine's phone-class measurement detects it before the branch is
  adopted.
- **WebKit's clock is coarse.** Its `performance.now()` resolves to 1 ms, so its figures under
  10 ms carry that step. A finer reading needs a longer scenario, not more runs.
- **The rusqlite line moves for the native build too.** One SQLite crate serves both targets
  (ADR-346), so adopting the branch moves the native build's bundled SQLite with it. The engine's
  native tests and this repository's gate read that when the web engine pins the branch.
- **Paths the scenario did not reach.** The FSRS crate depends on a parallel iterator crate
  without an option to drop it, so its optimiser would need threads at run time; two engine call
  sites outside open, answer and undo (media file times and backups) read the standard clock,
  which panics on wasm32. The web engine's own tests detect either when they reach those paths.
- **The module grows past its budget.** Every engine change can add bytes. The CI size-budget gate
  the web engine adds reads the shipped total on every change.
- **The spike branch stops resolving.** The fork commits in section 7 are cited by full sha; the
  web engine delivery checks each still resolves before it pins the branch.

## 7. Measured in the fork's harness

Fork commits: the gates `c67ef888175daf1e28b6cc20964c1d9e75cf50c6`, the build
`cbe60369e419a604e676da627e47e6148c34cdca` and the harness
`98d1455385dfea67cac832fa98f11a54004f8233`, on the pinned
`57382da085e6752738dc4bb617789be836a23300`. The harness is the fork's `wasm-spike/` directory:
`build.sh` builds the module and prints its sizes; `tests/spike.spec.mjs` drives one scenario per
run in a module Worker and prints one report line. Each run installs the SAH-pool VFS over OPFS,
creates the collection and seeds 300 Basic notes in one add, closes it, then times the open, the
queue build, the answer (Good) and the undo, reads the card's review-log rows and the cards table
around them, closes, reopens and counts the notes.

| id | criterion, as measured | measured by |
|---|---|---|
| M1 | The engine and the harness module both build for `wasm32-unknown-unknown` in release. | `cargo build --release --target wasm32-unknown-unknown -p anki`, then `-p anki-wasm-spike`, at `cbe60369e419a604e676da627e47e6148c34cdca` |
| M2 | The engine's native check reads the same before and after the gates, diffed error line by error line: alone, the same seven errors at both commits, from an async-runtime feature (`tokio/io-util`) the engine uses and the workspace manifest does not name, which a consumer's build unifies in; with that feature named, clean at both. | `cargo check -p anki`, and again with `--features tokio/io-util`, at `57382da085e6752738dc4bb617789be836a23300` and at `c67ef888175daf1e28b6cc20964c1d9e75cf50c6` |
| M3 | The module a browser loads, after `wasm-bindgen` and `wasm-opt -Oz`: 23845690 bytes raw, 7521156 bytes gzip -9. Its JS bindings: 42833 bytes raw, 8631 bytes gzip -9. Shipped total: 7529787 bytes gzip -9. Before the size pass, the cargo cdylib: 27394535 bytes raw, 7750461 bytes gzip -9. | `wasm-spike/build.sh` at `cbe60369e419a604e676da627e47e6148c34cdca` |
| M4 | The raw cdylib's sections: CODE 7895989 bytes, DATA 17393569 bytes, name 1594432 bytes. The embedded translation table is the largest single part of DATA: 8357373 bytes of generated source. | `llvm-objdump -h` of the cargo cdylib at `cbe60369e419a604e676da627e47e6148c34cdca` |
| M5 | Chromium: open, queue build, answer and undo, 5 runs of 5, no error and no console error; timings below. | `npx playwright test --project chromium --repeat-each 5` in `wasm-spike/` at `98d1455385dfea67cac832fa98f11a54004f8233` |
| M6 | WebKit: the same scenario, 5 runs of 5, no error and no console error; timings below. | `npx playwright test --project webkit --repeat-each 5` in `wasm-spike/` at `98d1455385dfea67cac832fa98f11a54004f8233` |
| M7 | Undo correct, 5 of 5 in each browser: one review-log row for the card after the answer and none after the undo; the answer changed the cards table and the undo restored it byte-equal. | the assertions of `wasm-spike/tests/spike.spec.mjs` in M5 and M6 |
| M8 | The collection reopens in the same page with its 300 notes; `journal_mode` reads `wal`. | the assertions of `wasm-spike/tests/spike.spec.mjs` in M5 and M6 |
| M9 | WebKit refuses the OPFS root in Playwright's default ephemeral context (`An error occurred while getting the directory handle`); a persistent profile runs, so the harness launches a persistent profile in both browsers. Chromium passed 5 of 5 in the ephemeral context. | the harness of M5 and M6 with Playwright's default context in place of the persistent one |
| M10 | The gates: 27 sites, which are 23 code sites in the engine's source, each a `cfg` on `target_arch = "wasm32"` or its negation, and 4 manifest and config entries. | `git diff 57382da085e6752738dc4bb617789be836a23300 c67ef888175daf1e28b6cc20964c1d9e75cf50c6` in the fork |

Timings in milliseconds, median (min-max) of 5 runs, persistent profile, headless:

| step | Chromium | WebKit |
|---|---|---|
| module load (fetch, compile, instantiate) | 107.5 (99.9-110.9) | 261.0 (241.0-292.0) |
| OPFS pool install | 31.3 (28.1-35.2) | 16.0 (14.0-19.0) |
| seeding 300 notes | 72.0 (68.0-79.9) | 35.0 (32.0-40.0) |
| open | 1.1 (1.0-1.2) | 1.0 (1.0-1.0) |
| queue build | 17.2 (13.7-18.1) | 7.0 (6.0-8.0) |
| answer | 6.0 (5.9-9.6) | 3.0 (2.0-4.0) |
| undo | 3.6 (3.1-7.9) | 2.0 (1.0-3.0) |
| undo correct | 5 of 5 | 5 of 5 |

WebKit's clock resolves to 1 ms, so its figures are whole milliseconds.
