---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# The engine pins a patched fork of 26.09.3 until upstream carries the fix

## Context and Problem Statement

ADR-022 admits Anki's engine as a git dependency of `ingest` pinned to a release tag, with
`deny.toml`'s `allow-git` naming Anki's repository and `ankitects/rust-url`. Its finding stands: the
engine recompiles on every cargo command, because the `anki_proto` build script registers the files
it writes under `OUT_DIR` as rerun inputs (`rslib/io/src/lib.rs:353-355`) and writes the prost
output a second time (`rslib/proto_gen/src/lib.rs:253`), so cargo always finds them newer than the
run (#228). Measured on the maintainer's machine, a no-op `cargo build -p deck-streak-ingest` takes
31 to 35 s at 26.09.3 and 38 to 51 s at 26.05, and every unit is Fresh in 0.33 s once that cause is
neutralised (SPEC-055 §7). The engine's second watch, `rslib/build.rs:13`, is inert for a git
dependency, because cargo skips mtime checks for paths under `$CARGO_HOME` (rust-lang/cargo#11613).
Both lines are unchanged at 26.09.3 and on upstream `main`, so a pin bump alone fixes nothing.

The owner decided to move the engine to 26.09.3 (SPEC-055), and to fix the rebuild both upstream
and in DeckStreak: the maintainer submits the fix to Anki, and DeckStreak carries it on the
maintainer's fork until an upstream release holds it. How does DeckStreak pin a patched engine
without loosening ADR-022's supply-chain rule, and when does the carry end?

## Decision Drivers

- The pin is reproducible from the public repository, and `cargo deny` passes with exactly the
  sources the graph uses.
- The patched engine differs from an upstream release by exactly the fix, and that is checkable.
- The upstream release DeckStreak runs stays readable where the gate and the engine's measurement
  read it: the dependency line in `Cargo.toml`.
- The carry has one end condition, which a measurement decides.
- Any engine change re-runs ADR-022's protocol.
- The gate never edits files in a dependency's checkout or in cargo's build directory.

## Considered Options (the alternatives it was chosen against)

- A `[patch."https://github.com/ankitects/anki.git"]` entry that replaces the engine with a commit of the fork, pinned by `rev`, while the dependency keeps naming the upstream tag — chosen: measured in a scratch resolution, it re-locks exactly the engine's five packages to the fork, and the dependency line keeps the upstream tag that SPEC-022's A1 check and `engine-measure.yml`'s report both read.
- A direct `git` dependency on the fork pinned by `rev` — rejected because it breaks both readers of the dependency line (`test_engine_spike_record.py`'s engine-tag pattern and `engine-measure.yml:104`) and hides which upstream release DeckStreak runs, for a `Cargo.lock` byte-identical to the patch's.
- Stay on 26.05 unpatched — rejected because it keeps paying 38 to 51 s of engine recompilation on every cargo command (#228), and the owner decided to move the engine to 26.09.3.
- Upgrade to 26.09.3 unpatched and wait for upstream — rejected because the cause #228 names is still in 26.09.3 and on upstream `main`, so the bump saves nothing measurable (31 to 35 s per no-op), and when upstream releases a fix is not DeckStreak's to decide.
- Route C, a gate-side workaround that resets the generated files' mtimes after every cargo command — rejected by the maintainer because it edits cargo's build directory, depends on cargo's internal layout, must follow every ad-hoc cargo command, and a gate that edits mtimes is a new way for a gate to lie.
- Vendor the engine's source into this repository — rejected because it copies a large AGPL tree into a public repository to change one file, hides the engine from the lockfile's view of dependencies, and turns every Anki bump into a re-vendoring.

## Decision Outcome

Chosen option: a `[patch]` entry on the upstream source, pointing at a commit of the fork.

- **The manifest.** `[workspace.dependencies]` keeps
  `anki = { git = "https://github.com/ankitects/anki.git", tag = "26.09.3", features = ["rustls"] }`.
  The root manifest, the only one whose patches cargo reads, gains
  `[patch."https://github.com/ankitects/anki.git"]` with
  `anki = { git = "https://github.com/RexRenatus/anki.git", rev = "<the pinned commit>" }`, and a
  comment naming this ADR and #233. The engine's own path dependencies (`anki_proto`, `anki_io`,
  `anki_i18n`, `anki_proto_gen`) come from the same commit of the fork: measured, those five
  packages are all the lockfile moves.
- **The fork.** `RexRenatus/anki`, the same fork the upstream pull request comes from. It carries a
  branch at upstream tag `26.09.3` plus exactly one commit, which changes only `rslib/io/src/lib.rs`
  so that `write_file_if_changed` stops registering a path under the running build script's
  `OUT_DIR`. Changes that matter only to a consumer that takes the engine by path are not carried.
  The pinned commit carries a tag on the fork, and neither the branch nor the tag is force-pushed or
  deleted while DeckStreak pins it. The maintainer creates and pushes both (#233).
- **The sources.** `allow-git` names exactly `https://github.com/RexRenatus/anki.git` and
  `https://github.com/ankitects/rust-url.git`, and `unknown-git = "deny"` still refuses any other.
  Measured, cargo loads the upstream repository only when it resolves the patch anew, never with a
  committed `Cargo.lock`, and no package in the graph comes from it. `cargo deny` checks the graph,
  so an upstream entry left in `allow-git` raises its unmatched-source warning. If the patch were
  ever dropped by mistake, the audit would then refuse the unpatched upstream source by name.
- **What the tag brings to `deny.toml`.** The two advisory exceptions whose crates leave the graph
  with `burn` (RUSTSEC-2024-0436 for `paste`, RUSTSEC-2025-0141 for `bincode`) are removed. So is
  the `Unlicense` allowance ADR-022 added for `systemstat`: the crates left under that licence all
  offer MIT, and the audit passes without it (SPEC-055 §1). `CC0-1.0` stays: it predates the engine,
  and it is not this change's to remove.
- **ADR-022's pin rule is amended while this holds.** The engine still names an upstream tag, and a
  `[patch]` entry replaces it with the fork's commit. Everything else in ADR-022 stands: the
  protocol, the synthetic collection, the budgets, and a re-run of both on any engine change.
- **The work per Anki bump.** While the fork is carried, each upgrade adds three steps to the ones
  any upgrade takes (reading the notes and the tag diff, re-running ADR-022's protocol and SPEC-022's
  criteria):
  1. check first whether the new tag carries the fix, and if it does, remove the fork instead;
  2. the maintainer applies the one commit to the new tag (`git apply --check`, then a cherry-pick),
     then pushes a new branch and tags it. `write_file_if_changed` has not changed since
     ankitects/anki#4439, which added the registration, so the change has applied unchanged so far;
  3. the delivery moves the dependency's `tag` and the patch's `rev` together.
- **The removal condition.** The pinned upstream tag carries the fix, or an equivalent that stops
  registering `OUT_DIR` outputs. The check: with the engine at that tag, unpatched and consumed as a
  git dependency, a second `cargo build -p deck-streak-ingest` finishes with every unit Fresh, in
  cargo's own time (SPEC-055 A2). The removal delivery deletes the `[patch]` entry, moves the tag,
  puts upstream back in `allow-git` and drops the fork, lets `Cargo.lock` follow, supersedes this ADR,
  and re-runs ADR-022's protocol (#233). A2 stays, and it then guards against a regression upstream.

### Consequences

- Good, because no cargo command recompiles the engine any more: a no-op build falls from 31 to 35 s
  to under a second, in the gate and in each of CI's Rust jobs.
- Good, because 26.09.3's graph is smaller: 685 packages become 477, and two exceptions for
  unmaintained crates leave `deny.toml`.
- Good, because the dependency line still says which upstream release runs, and removing the carry
  is deleting the `[patch]` entry.
- Bad, because DeckStreak trusts a second repository, the maintainer's fork, for the engine's
  source. The pin by revision and the one-file difference from the upstream tag bound that trust.
- Bad, because moving the pin (a tag or `rev` bump, or `cargo update`) needs the upstream
  repository as well as the fork while the patch holds, although a committed `Cargo.lock` never
  loads upstream.
- Bad, because each Anki bump costs a cherry-pick and a tag until upstream releases the fix.

### Confirmation

SPEC-055's A1 (the tag, the patch by revision and the exact `allow-git`), A2 (a second build
recompiles nothing), A3 (every advisory exception and allowed source is live) and A5. ADR-022's
numbers at the pinned commit go below the spike's in ADR-009's Confirmation, which SPEC-022's A1
judges (SPEC-055 R11). SPEC-055's delivery recorded, for A5:

| record | measured |
|---|---|
| the pinned commit | `57382da085e6752738dc4bb617789be836a23300` on `RexRenatus/anki`, the head of branch `fix-proto-out-dir-rerun-26.09.3`, tagged `deckstreak-pin-26.09.3` (`git ls-remote`, read on 2026-09-28) |
| its parent | `29bb700b951e`, upstream tag `26.09.3`, its only parent |
| `git diff --stat 26.09.3 57382da` on the fork | `rslib/io/src/lib.rs`, 1 file changed, 47 insertions(+), 2 deletions(-) |

| measure | before the pin | after the pin |
|---|---|---|
| a no-op `cargo build -p deck-streak-ingest`, in cargo's own `Finished` time on the maintainer's machine | 29.3 to 38.2 s at `26.09.3` unpatched (four runs), each recompiling `anki_proto`, `anki` and the ingest crate; 35.5 s at `26.05` (A2 at the base) | 0.33 to 0.35 s at the pinned commit (five runs), every unit Fresh |

- **CI on SPEC-055's pull request.** The pull request changes `Cargo.lock`, so in
  `ci.yml` run 36386849369 at `4df16b0` every Rust job restored `dev`'s entry (1,005,229,631 bytes)
  by its fallback key, in 11 to 21 s, and compiled the fork's engine, which that entry lacks. By
  SPEC-038 section 8's metric, the slower gate job (an `engine` leg, 3m23s) plus `ci` (4 s) took
  3m27s. The fix shows in the `rust` job: `doctest`, its third cargo command, found every unit
  Fresh and took 1 s, against 31 s on `dev` (run 36383672922), where each cargo command recompiled
  the engine. Its `hygiene` job ran A2 in the python stage, 373 units Fresh and the second build
  0.2 s of wall time, and was red only on A5 and A7, the two records not yet written at that
  commit.
- **ADR-022's protocol.** `engine-measure.yml` run 36386849300 at `4df16b0`: every budget holds
  (ADR-009's Confirmation).
- **CI's warm path after the first push to `dev` that saves a cache.** Appended here by the
  orchestrator after the merge, because only such a push saves a cache (ADR-055; SPEC-055 R6 and
  R10).
- **The status.** `accepted`: every budget holds at the pinned commit.

## What would make this wrong

- An upstream release carries the fix. The removal delivery then supersedes this ADR (#233).
- Upstream declines the fix, or reshapes the build scripts so that the change no longer applies. The
  fix is re-derived against the new code, and A2 is still the check.
- Cargo stops judging build-script inputs by mtime alone, for example if checksum-based freshness
  covers them. The fix may then become unnecessary, and A2 on the unpatched tag would show it.
- The fork's host drops the pinned commit. The tag on it exists to prevent that, and a fresh fetch
  in CI would fail by name.

## More Information

ADR-022 (whose pin rule this amends), ADR-009, ADR-018, ADR-037, ADR-055; SPEC-022, SPEC-038,
SPEC-055; #228, #233, #235. The Cargo Book on `[patch]` and git dependencies
(https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html), cargo-deny's sources check
(https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html) and rust-lang/cargo#11613
(https://github.com/rust-lang/cargo/pull/11613), each read on 2026-09-28.

## Confirmation, appended 2026-09-28: CI's warm path after the pin

This section fills the Confirmation's reserved bullet, "CI's warm path after the first push to
`dev` that saves a cache". Measured from `dev`'s push runs on GitHub-hosted runners, by SPEC-038
section 8's metric (the slowest gate job plus `ci`, excluding the wait for a runner):

- **The merge's own push, run 36391248783 at 65d7649 (#242).** The lockfile changed, so every Rust
  job restored the 26.05-era entry by its fallback key (1,005,229,631 bytes) and compiled the fork's
  engine. `rust` then saved the new entry under the 26.09.3 lockfile's key: 1,331,055,956 bytes.
  The jobs: engine (1) 192 s, engine (2) 182 s, rust 171 s, hygiene 156 s. The gate: 195 s.
- **The first warm push, run 36392549996 at 5f3f2d5 (#243, docs only).** Every Rust job restored
  that entry by its exact key, and nothing was saved. The jobs: engine (2) 143 s, engine (1) 127 s,
  rust 96 s, hygiene 139 s. The gate: 147 s, against 176 s at 9e8e53e before the fork (run
  36382447015).
- **Later warm pushes, the gate:** 868aaa5 (#245, run 36395345580) 155 s; 0f68ed4 (#221, run
  36396463661) 161 s, excluding the new mutation jobs, which SPEC-039 bounds itself; d4af48e (#246,
  run 36397385939) 152 s; d9ffe44 (#244, run 36398486069) 161 s; 16ed8e2 (#247, run 36410369748)
  149 s.
- **The engine's recompile on every cargo command is gone.** The `rust` job now takes 93 to 139 s
  warm, against 147 s before the fork (run 36382447015), with its three forced recompiles.

## Note, appended by SPEC-338: the pin moves to the fork's wasm32 patches

This note leaves the body above as it was decided. ADR-336 accepted Anki's engine in the browser on
`wasm32-unknown-unknown`, and ADR-348 decided how the fork carries the patches that build it: one
commit per patch over this ADR's fix commit `57382da085e6752738dc4bb617789be836a23300`, on branch
`wasm32-26.09.3` of `RexRenatus/anki`, tagged `deckstreak-pin-26.09.3-wasm32`. The root manifest's
`[patch]` entry takes `anki` and `anki_proto` by `rev` from the tag's commit, the dependency line
keeps the upstream tag `26.09.3`, and the fix above stays the range's first commit. Every patch is a
`cfg` on the target or a dependency line both targets share, so the native engine is the one this
ADR pinned, on rusqlite 0.39's SQLite line (ADR-348).

| record | measured |
|---|---|
| the pinned commit | `c538de55a23e695234e794029fce0dafff2d36a9`, the head of branch `wasm32-26.09.3`, tagged `deckstreak-pin-26.09.3-wasm32` (`git ls-remote`) |
| its range | ten commits over `57382da`, one per patch; none adds a workflow file (`git diff --stat 57382da c538de5 -- .github` is empty) |
| `git diff --stat 26.09.3 c538de5` on the fork | 22 files changed, 338 insertions(+), 69 deletions(-) |

| patch | commit | reason | removal condition |
|---|---|---|---|
| `sqlite-route` | `15732ac4825cbe3f45fe0f30d276957badd0a0ca` | rusqlite 0.39 routes wasm32 to `sqlite-wasm-rs` and keeps `libsqlite3-sys` inside the range `sqlx` accepts | the upstream tag's rusqlite routes wasm32 and resolves beside `sqlx` |
| `native-only-sync-server` | `e777006206e1cf8a612c5523b42ea6d1e721f14b` | the in-crate sync server needs a socket runtime wasm32 lacks | upstream gates its server by feature or target |
| `current-thread-runtime` | `e24fe01727006c12e534e336de8dc41a932f838d` | tokio on wasm32 has no multi-thread runtime, fs, net or signal | upstream builds its runtime per target |
| `single-thread-zstd` | `25189dc8e4e2d7c3af3c755db269fb2bb1992e86` | zstd's `zstdmt` needs threads wasm32 lacks | upstream gates `zstdmt` per target |
| `sequential-rayon` | `bf7b4a732f4e43543b26d9b7fc40517151566bd1` | rayon has no thread pool on wasm32 | upstream gates rayon per target |
| `js-date-clock` | `d123ca755da9c90a8a75b3222d0d94f6488d096e` | the standard clock panics on wasm32 | the standard clock works on wasm32 |
| `browser-fetch` | `cebf678517c65530baca83f63f4838771073f9b2` | the browser's fetch has no `http1_only`, client timeout or streamed body | a browser sync transport replaces the refusal (#631) |
| `getrandom-wasm-js` | `a75d4c146d724e5e9fcd18aefae0e9bb77f016d0` | getrandom draws randomness from `crypto.getRandomValues` on wasm32 | getrandom picks a browser backend by default |
| `native-only-log-file` | `42539da45e22fdf9c764740e020c3db621989038` | `tracing-appender` does not build for wasm32, and a browser has no log file | `tracing-appender` builds for wasm32 again |
| `browser-tls` | `c538de55a23e695234e794029fce0dafff2d36a9` | the rustls custom-certificate path uses reqwest calls wasm32 lacks | upstream gates that path per target |

## Amendment: the FSRS-7 crate's source (SPEC-342)

`allow-git` names three sources, no longer exactly two. Beside the fork and `rust-url`, it admits
the upstream scheduler's repository, `open-spaced-repetition/fsrs-rs`: its development line is the
only source of FSRS-7, by ADR-338 and ADR-353 D1. `unknown-git = "deny"` still refuses any other
source. The engine's own scheduler stays the released crate from the registry, as this ADR left
it: only the isolated scheduler crate, `crates/fsrs7`, takes the git package, at the one full
revision ADR-353 records.

This amends the list in "The sources" above and nothing else: the fork, its patch and the pin rule
stand. Vendoring the scheduler's source into the tree, which would have kept `allow-git` at two
sources, was rejected in ADR-353 D1, because the copy is re-synced by hand and the lockfile records
no provenance for it. SPEC-055's R3 and A1 point here.

## Note, appended by SPEC-364: the pin moves to the browser's sync transport

SPEC-364's part b2 adds three patches to the fork's branch `wasm32-26.09.3`, each its own commit over
`c538de55a23e695234e794029fce0dafff2d36a9`, and a new tag, `deckstreak-pin-26.09.3-wasm32-sync`,
names the third; the earlier tag stays where it is. The root manifest's `[patch]` entry takes
`anki` and `anki_proto` by `rev` from the new tag's commit. `browser-xhr` replaces `browser-fetch`'s
refusal of the sync with a synchronous request from the dedicated Worker; `browser-fetch`'s other
gates stay, so its row is amended here, not removed. Every patch is still a `cfg` on the target or a
dependency line both targets share, so the native engine is the one this ADR pinned. Read with
ADR-336 and ADR-348, which this note leaves as they stand.

| record | measured |
|---|---|
| the pinned commit | `2cfa70478a1174f49cf98fc71a8b8c47fc54b3fb`, the head of branch `wasm32-26.09.3`, tagged `deckstreak-pin-26.09.3-wasm32-sync` (`git ls-remote`) |
| its range | thirteen commits over `57382da`, one per patch; none adds a workflow file (`git diff --stat 57382da 2cfa704 -- .github` is empty) |
| `git diff --stat 26.09.3 2cfa704` on the fork | 24 files changed, 500 insertions(+), 71 deletions(-) |

| patch | commit | reason | removal condition |
|---|---|---|---|
| `browser-fetch` (amended) | `cebf678517c65530baca83f63f4838771073f9b2` | its refusal of the sync is replaced by `browser-xhr`; its other gates stay | upstream gates those per target |
| `browser-xhr` | `5dbd27443794cc0575a54506b93882f5fcbee295` | the browser's asynchronous fetch never answers while the engine blocks the Worker, so the sync request is a synchronous request from the dedicated Worker | upstream builds its sync transport for the browser |
| `wasm-clock-threads` | `6f7acd1887eb02f106699413d963523e9b0102f9` | tokio has no time driver on `wasm32`, where a timer panics, and the browser gives the engine no thread | tokio drives timers on `wasm32`, and upstream starts no thread for a sync's abort or a background media sync |
| `wasm-collection-size` | `2cfa70478a1174f49cf98fc71a8b8c47fc54b3fb` | the browser gives the engine no file system on `wasm32`, so a normal sync could not read the collection file's size before its first request; the sync meta reads the open collection's SQLite page count times its page size instead, and the upload size check keeps the real size | upstream reads the collection's size without the file system on `wasm32` |

## Note, appended by SPEC-377: the pin moves to the browser's full sync files

SPEC-377's part c1 adds one patch to the fork's branch `wasm32-26.09.3`, its own commit over
`2cfa70478a1174f49cf98fc71a8b8c47fc54b3fb`, and a new tag, `deckstreak-pin-26.09.3-wasm32-full-sync`,
names it; the earlier tags stay where they are. The root manifest's `[patch]` entry takes `anki` and
`anki_proto` by `rev` from the new tag's commit. `browser-full-sync-files` lets a full upload and a
full download move the collection through SQLite where the browser gives the engine no file system,
and keeps the full sync's progress monitor off tokio's timer. The patch is a `cfg` on the target and
a dependency line for `wasm32` alone, so the native engine is the one this ADR pinned. Read with
ADR-336, ADR-348 and ADR-388, which this note leaves as they stand.

| record | measured |
|---|---|
| the pinned commit | `bd4fe2f4e9aa6e37434678e7770e6934b6071fcb`, the head of branch `wasm32-26.09.3`, tagged `deckstreak-pin-26.09.3-wasm32-full-sync` (`git ls-remote`) |
| its range | fourteen commits over `57382da`, one per patch; none adds a workflow file (`git diff --stat 57382da bd4fe2f -- .github` is empty) |
| `git diff --stat 26.09.3 bd4fe2f` on the fork | 26 files changed, 576 insertions(+), 71 deletions(-) |

| patch | commit | reason | removal condition |
|---|---|---|---|
| `browser-full-sync-files` | `bd4fe2f4e9aa6e37434678e7770e6934b6071fcb` | the browser gives the engine no file system on `wasm32`, so a full upload could not read the closed collection and a full download could not write a temporary file and rename it over the collection; an upload reads the collection through SQLite's serialize, a download is checked in memory and written by SQLite's backup in one step, and the full sync's progress monitor never ticks, because tokio has no time driver there | upstream reads and writes a full sync's collection without the file system on `wasm32`, and tokio drives timers there |
