---
status: proposed
decision-makers: "the owner, the DeckStreak architect"
---

# The FSRS-7 crate is a context of its own at a pinned revision, its replay is timed natively and under V8 through WASI, and the undo and full-sync probes run in ingest's tests against the engine's own server

## Context and Problem Statement

SPEC-334 row 1.2 owes three measurements: FSRS-7's replay time, whether the engine's undo deletes
the review-log row, and the full-sync choice path (#620, SPEC-342). Two ADRs settle what is
built:
- ADR-338: FSRS-7 sits in an isolated crate at a pinned git revision, admitted by its own
  `allow-git` entry. It never links into the engine, and its state is replayed from the review
  history when the collection opens.
- ADR-337: undo after sync and the one-way sync are never-list entries 1 and 4, reached only
  through `OwnerGesture`.

Neither settles how the measurements are taken. Five questions are left:
1. where the crate sits in the context map, and how its revision enters the workspace;
2. how a wasm timing is taken in CI;
3. which sync server the probes run against, and how it is started and stopped;
4. where the probes live;
5. how the two scheduler packages are shown to coexist.

A sixth follows from the first: where the measurement harness's code lives.

## Decision Drivers

- ADR-338's Confirmation:
  - the replay measured natively and on WASM;
  - `cargo deny` with exactly the FSRS-7 crate's `allow-git` entry added.
- The context map is binding, and a member it does not name is refused.
- The figures must predict a client:
  - the web client runs the engine in a single-threaded Worker under a browser's JavaScript
    engine (ADR-336);
  - the iPhone and iPad client runs it natively (ADR-335).
- Nothing runs on a host. CI builds and runs everything (SPEC-334 section 6: "the full-sync
  measurement runs against a scratch sync server in CI").
- A measurement that examined nothing, or that a test cannot fail, decides nothing.

## Considered Options (the alternatives it was chosen against)

D1, where the crate sits and how its revision is admitted:

- A new context crate, `crates/fsrs7` (`deck-streak-fsrs7`), whose map line reads `depends on: nothing`:
  chosen because ADR-338 makes FSRS-7 a scheduler of its own, which the stretch row's delivery
  extends (#641). The pinned crate enters `[workspace.dependencies]` by git URL and a full 40-hex
  revision, under the key `fsrs7` with `package = "fsrs"`.
- A spike workspace outside the root, with its own lockfile: rejected because the revision is
  then pinned twice. DeckStreak's lockfile would never prove the two packages coexist, and
  `cargo deny` would never read the entry ADR-338 confirms by.
- A module of `deck-streak-ffi`: rejected because the adapter links the engine, and ADR-338 says
  the FSRS-7 crate never does. An adapter would also hold a scheduler.
- A module of `deck-streak-ingest`: rejected because ingest is the engine's anti-corruption layer
  and depends on it. It is the same link.
- Pinning by branch name: rejected because a branch moves under the build, and an update would
  change the scheduler silently.
- Vendoring the upstream source into the tree: rejected because the copy is re-synced by hand,
  and the lockfile records no provenance for it.

D2, how a wasm timing is taken in CI:

- The timing example built for `wasm32-wasip1` and run under Node's V8 through `node:wasi`, with
  Node from `.nvmrc` through the setup action the `web` job already pins; native timed on one
  thread: chosen for three reasons:
  - V8 is the JavaScript engine of the browser that ADR-336's Worker runs in most often;
  - WASI gives the module a monotonic clock and an exit status without a bindings crate;
  - one thread is what the Worker has.
- Chromium and WebKit through Playwright, with a `wasm-bindgen` harness: rejected because it
  admits a bindings crate and a JavaScript harness before the web engine decides them (#626).
  Browser figures are #626's.
- wasmtime: rejected because its compiler is no browser's, so its figure predicts no client.
- `wasm-bindgen` under Node: rejected because it admits the bindings crate early, for no figure
  that WASI does not give.
- Native only, scaled by an assumed factor: rejected because no measured factor exists.
- Native with every thread: rejected because the Worker has one thread, so the two figures would
  not compare.

D3, the scratch sync server:

- The engine's own sync server at the workspace's pin, started by the test in a child process
  (`SyncServer::start`, already in ingest's test support):
  - it is stopped by closing its standard input, and killed on drop if it hangs;
  - it serves a temporary directory on a loopback port;
  - chosen because it is the same server code at the same commit as the server the release builds
    (ADR-347). It runs wherever the test runs, and the `rust` job already starts it for
    `recorder_control`.
- The release's recipe in a job, `cargo install --git <fork> --rev <pin> anki-sync-server`, started
  in the background and stopped by SIGINT: rejected for two reasons:
  - it builds the engine a second time on every run;
  - a test that needs the server either passes vacuously where the server is absent, or exists
    only in that job.

  The protocol code is the same crate at the same commit.
- A stock container image of the server: rejected because it is not the workspace's pin and
  carries no provenance.
- A scripted fake server: rejected because the probe would then measure the fake.

D4, where the probes live:

- `crates/ingest/tests/undo_and_full_sync.rs`, a new test binary of ingest's, run in the `test`
  stage outside the engine set: chosen for three reasons:
  - ingest owns the engine's sync-protocol knowledge;
  - its test support already holds the server, the recording relay that decodes each request and
    the synthetic collections;
  - the probes' collections are a few cards each.
- `deck-streak-ffi`'s tests: rejected because they would copy that support, and the adapter's
  allow-list holds no sync call.
- A shared test-support crate: rejected because it adds a workspace member, and a map line, for
  test code.
- The engine set: rejected because that set is split out by duration (`scripts/check.sh`), and a
  probe of a few cards is not slow.

D5, how the two packages are shown to coexist:

- A dev-dependency of `deck-streak-fsrs7` on the released crate by its exact version, under the
  key `fsrs6`, and one test binary that links both: chosen because it proves the link, not only
  the resolution. Dev-dependencies are not context edges.
- The lockfile alone: rejected because it proves that cargo resolved two packages, not that one
  binary links both.
- Linking the engine into the crate's tests: rejected because it puts the engine in the
  scheduler's test graph and an engine build in every test run of the crate.

D6, where the measurement harness lives:

- In the crate's library, as a `measure` module (the synthetic history, the fixed line and the
  report), with two examples that only call it: chosen because the harness is then tested by A7
  and A10 to A13, and mutated with the crate.
- Inline Python in the workflow, as `engine-measure.yml` does: rejected because nothing tests it,
  and a report that drops a cell would pass unseen.
- In the examples alone: rejected because the mutation tool mutates no example.
- A script under `scripts/`: rejected because it adds a second language reading Rust's output,
  and rows for it.

## Decision Outcome

The chosen option of each of D1 to D6. The development line's package carries the released
crate's name and version string, so cargo tells the two apart by source. ADR-338's "per-version
disambiguation" is in fact disambiguation by source, and A1 and A4 hold it.

`<FSRS7-REV>` is `4bc0a0979f95dd01cc653031b6f2a01429e4c32b`, the head of fsrs-rs's `main` when the
build started (`git ls-remote https://github.com/open-spaced-repetition/fsrs-rs.git
refs/heads/main`). Read at that revision:

| fact | where |
|---|---|
| FSRS-7 is selected for 0 or 34 parameters | `src/model.rs:21-27` (`ModelVersion::from_param_count`) |
| a review is `FSRSReview { rating: u32, delta_t: f32 }`, and the first review's `delta_t` is 0 | `src/dataset.rs:25-32` |
| the memory state carries `stability_fast` | `src/inference.rs:45-52` |
| a batch call exists, `memory_state_batch(items, starting_states)`, so R6 keeps `batch` | `src/inference.rs:212` |
| the initial stability for Good is the third default parameter | `src/model.rs:173-175`, `src/inference_v7.rs:1-5` |
| the package is `fsrs`, with the released crate's version string, and declares no features | `Cargo.toml:2-3` |

`deny.toml`'s comment above `allow-git` says "Nothing else", and ADR-058 and SPEC-055 name
exactly two sources. This delivery appends an amendment to ADR-058 naming the third, by ADR-338
and this ADR, and points SPEC-055's R3 and A1 to it.

### Consequences

- Good, because:
  - the replay figure comes from the same crate, revision and lockfile that the client will link;
  - the wasm figure comes from a browser's engine with no bindings admitted;
  - the probes run on every pull request, in the job that already runs the server.
- Good, because the two packages' coexistence is a test, not a belief.
- Bad, because the workspace carries an unreleased revision that is re-pinned by hand
  (ADR-338's own consequence).
- Bad, because the crate's library carries measurement code that the client never calls.
- Bad, because a WASI run under Node is not a browser Worker, so #626 still measures in Chromium
  and WebKit.
- Bad, because the containment census that #623 builds counts code callers only, never tests:
  ingest's test support already calls a one-way sync (`upload_from_another_client`), and this
  delivery's probes call both one-way syncs and undo. A census that counted test callers would
  refuse them.

### Confirmation

SPEC-342's A1 to A22 and its section 7, measured by `fsrs7-measure.yml`'s run and CI's `rust`
job, and its rows in S34200-S34299.

## What would make this wrong

- An upstream release carries FSRS-7. The git entry and the dev-dependency then retire, and
  ADR-338's own "What would make this wrong" applies.
- A browser's figure in #626 disagrees with the WASI figure by more than its run-to-run spread.
  The browser figure then rules for the web client, and this method stays for regressions only.
- The release-built server behaves differently from the in-test server on the full-sync path.
  The release's own smoke test, and the staging rehearsal ADR-347 names, would read it.
- The probes push the `test` stage past its split. They then move into the engine set by a
  change to `scripts/check.sh`.

## More Information

ADR-338, ADR-337, ADR-336, ADR-335, ADR-347, ADR-346, ADR-058, ADR-022; SPEC-334 (R8, R10, row
1.2, section 9), SPEC-335 (M7), SPEC-055; `docs/schematics/fsrs-7-replay-undo-probe-and-full-sync-choice.md`;
#620, #641, #626, #623, #630, #631, #633.
