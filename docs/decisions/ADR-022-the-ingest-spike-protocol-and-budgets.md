---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The ingest engine spike runs a fixed protocol against budgets fixed before it measures

## Context and Problem Statement

ADR-009 proposes Anki's own Rust engine for ingest and names a Python sidecar as the fallback, and
leaves the choice to a W0 spike: build time and binary size in CI, resident memory while opening a
large collection and resolving its new-card queue, judged against "the ingest budget" and "the CI
time budget". Neither budget has a number, and no synthetic collection is defined. A measurement
whose pass mark is set after the numbers are known decides nothing. What exactly is measured, on
what, and against which numbers?

## Decision Drivers

- The host is small and shared with other services (CHARTER 3); DeckStreak's share is SPEC-032's
  host budget.
- The host never compiles (CHARTER 3); CI builds on GitHub-hosted runners whose gate job has a
  60-minute timeout (SPEC-002's `ci.yml`).
- Releases are kept side by side on the host's disk (ADR-010).
- The measurement must be repeatable by anyone from the public repository: synthetic data only.

## Considered Options (the alternatives it was chosen against)

- Fix the protocol and the budgets now, measure once in the delivery, and let the numbers select the engine — chosen: the pass mark cannot bend toward the measurement, and a failure selects the fallback ADR-009 already names.
- Measure first and set the budgets from what was measured — rejected because a budget read off the measurement accepts whatever was measured.
- Accept the engine if it builds — rejected because memory, not compilation, is what the host cannot give back.
- Measure on the host — rejected because the host never compiles and is in live use; the host's own view arrives with the memory watch in W2.
- Measure against a copy of the owner's collection — rejected because the repository is public and the measurement must be reproducible by anyone; the synthetic collection is sized to exceed a large personal collection.

## Decision Outcome

Chosen option: this protocol and these budgets.

The synthetic collection, built by `crates/ingest/tests/support/synthetic.rs` from a fixed seed with
bulk inserts into a collection the engine created:

| parameter | value |
|---|---|
| cards | 250,000, one per note, two text fields of 200 characters each |
| decks | 20, under two roots (one law, one language) |
| reviews | 200,000 study reviews spread over the last 400 days |
| new cards per day | 20 per deck in the deck configuration |

The budgets:

| measure | how | budget |
|---|---|---|
| cold build | `engine-measure.yml` on `ubuntu-24.04`, no cache restored, `cargo build --release --locked --example engine_probe -p deck-streak-ingest`, including any tool the engine's build needs | at most 20 minutes |
| binary size | the stripped `engine_probe` | at most 100 MiB |
| open and queue | peak `VmHWM` of the `engine_budget` test that opens the collection and resolves today's new-card queue | at most 256 MiB |
| full download | peak `VmHWM` of the test that full-downloads the collection from a local sync server | at most 256 MiB |
| incremental sync | wall time to pull 100 new reviews from a local sync server | at most 60 seconds |
| licences | `cargo deny check licenses` with the engine | pass; a licence added to the allow list must be compatible with AGPL-3.0-or-later |

Why these numbers: the job unit that runs a sync gets a `MemoryHigh` of 320 MiB in SPEC-032's host
budget, so 256 MiB leaves a fifth of headroom below throttling; 100 MiB per binary keeps two
releases on disk well inside the host's free space; 20 minutes of engine build keeps the whole gate
job under 40 of its 60 minutes.

The engine is admitted as a git dependency of `ingest` pinned to a release tag, with `deny.toml`'s
`allow-git` naming Anki's repository only. If any budget fails, the delivery records the numbers,
ADR-009 is superseded by a new ADR choosing the sidecar, and SPEC-022 is amended before its sync
criteria are built.

### What the engine brought, recorded at the spike

The measured numbers are ADR-009's Confirmation. What admitting the engine took:

- **The tag and its feature.** The engine's tag is `26.05`. The feature is `rustls`, the TLS
  backend Anki's own build selects on Linux.
- **Two git sources.** Anki's repository, and `ankitects/rust-url`, the fork the engine's own
  manifest pins by revision for `percent-encoding-iri`. `allow-git` names exactly these two
  (SPEC-022 §7).
- **One licence added: `Unlicense`**, for `systemstat` (under the engine's `fsrs`, `burn` and
  `burn-train`). It is a public-domain dedication with a permissive fallback, and the FSF's licence
  list says "both public domain works and the lax license provided by the Unlicense are compatible
  with the GNU GPL" (https://www.gnu.org/licenses/license-list.html#Unlicense, read 2026-09-27),
  so it is compatible with AGPL-3.0-or-later (ADR-018). No other
  licence was needed. Anki's own crates are AGPL-3.0-or-later and unpublished, so cargo-deny reads
  them as private, like this workspace's crates.
- **Eight advisories, all "unmaintained", none a vulnerability**, each a named exception in
  `deny.toml`: `paste` (RUSTSEC-2024-0436), `unic-char-range` (RUSTSEC-2025-0075), `unic-common`
  (RUSTSEC-2025-0080), `unic-char-property` (RUSTSEC-2025-0081), `unic-ucd-category`
  (RUSTSEC-2025-0094), `unic-ucd-version` (RUSTSEC-2025-0098), `rustls-pemfile`
  (RUSTSEC-2025-0134) and `bincode` (RUSTSEC-2025-0141). The sidecar would run the same engine
  with the same crates, so they do not separate the two options; an engine upgrade re-reads them.
- **One bundled SQLite for the workspace.** A dependency graph may hold one crate that links the
  native `sqlite3`. The engine's `rusqlite` 0.36 accepts only `libsqlite3-sys` 0.34 and the kernel's
  `sqlx` 0.9 accepts 0.30.1 up to 0.37, so the lockfile holds 0.34.0, which both accept. An upgrade
  of either keeps one version both accept, or it does not resolve.
- **One build tool: `protoc` 31.1**, the version and archive digest Anki's own build pins, because
  the engine's build scripts compile its protobuf definitions. The measurement installs it inside
  the clock, and the gate job installs it too, because the gate builds the engine.
- **A finding for every later delivery: the engine rebuilds on every cargo invocation.** Its
  `anki_proto` build script registers each file it generates as a rerun input
  (`anki_io::write_file_if_changed`) and rewrites those files after cargo's invocation stamp, so
  cargo always finds them newer and recompiles the engine. The cold build is unaffected; each
  cargo command of the gate pays one engine compile.
- **The engine's sync server reads its users only from its process environment** (`SYNC_USER1`).
  Setting an environment variable is `unsafe` in edition 2024, which this workspace forbids, so the
  tests run the server in a child process whose environment `Command::env` sets.

### Consequences

- Good, because the choice is made by numbers anyone can reproduce.
- Good, because the same synthetic collection later sizes the readings' day-set tests.
- Bad, because the budget tests add about a minute to the gate; they are the price of a memory
  budget that cannot silently erode.

### Confirmation

SPEC-022's A1 to A3 and `engine-measure.yml`'s report; the numbers are written into ADR-009's
Confirmation section, and ADR-009's status changes with them.

## What would make this wrong

- The host is resized or DeckStreak's host budget changes, changing the memory the job unit can
  have (then SPEC-032's budget and these numbers move together, by a new ADR).
- GitHub's hosted runners change size, making the build budget measure a different machine.
- Anki's engine grows past a budget in a later release; the pinned tag holds until an upgrade
  delivery re-runs this protocol.

## More Information

ADR-009; ADR-010; ADR-018; SPEC-022; SPEC-032; the maintainer's survey (private input).

Amended in part on 2026-09-28: the finding above is the only rebuild cause that fires for this
repository's pinned git dependency. `rslib/build.rs:13` in the pinned tag also emits
`cargo:rerun-if-changed=../out/buildhash`, a file that only Anki's own build runner creates, and
cargo re-runs a build script whose watched path is missing. For a git dependency that watch is
inert: since 1.69 (rust-lang/cargo#11613), cargo skips mtime checks for paths under
`$CARGO_HOME/git` and `$CARGO_HOME/registry`, where the checkout lives. It fires only for a consumer
that takes the engine by path. Measured on the maintainer's machine with `cargo build -p
deck-streak-ingest`: a no-op build took 38.7 to 40.5 s and about 72 s of CPU, and the `anki` library
took 34.4 s of that wall time. With the first cause alone neutralised in a scratch target, the same
no-op was fresh throughout in 0.48 to 0.63 s. Both are in the latest upstream release. How to fix
the first cause, through an upstream change or a patched dependency carried until upstream releases
one, is the maintainer's decision (#228).

Amended in part on 2026-09-28, by ADR-058 (proposed with SPEC-055): the maintainer decided both
routes the paragraph above leaves open. The fix goes upstream as a pull request, and until an
upstream release carries it the engine's dependency keeps naming an upstream release tag while a
`[patch]` entry in the root manifest replaces it with a commit of the maintainer's fork, that tag
plus the one fix. `allow-git` then names that fork and `ankitects/rust-url`, and no longer Anki's
repository, from which no package comes while the patch holds. This takes effect when SPEC-055's
delivery accepts ADR-058; until then the pin rule above stands. The protocol and the budgets do not
change. They were run again at 26.09.3, where every budget held (`engine-measure.yml` run
36374499584, SPEC-055 §7). The fork is removed when an upstream release carries the fix (#233).

Amendment (2026-09-28): passages describing the host's capacity and its other services, and another
service's software version, in the decision drivers, a considered option, the record of what the
engine brought and a condition that would make this wrong, were redacted under the public-prose rule
(ADR-059).

Amendment (2026-09-28): the name of the maintainer's private tooling was replaced with a neutral one
under the public-text rule (ADR-059).
