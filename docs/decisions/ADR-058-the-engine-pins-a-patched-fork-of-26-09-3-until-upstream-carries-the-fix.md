---
status: proposed
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

The owner decided to move the engine to 26.09.3 together with the predecessor (SPEC-055), and to fix
the rebuild both upstream and in DeckStreak: the maintainer submits the fix to Anki, and DeckStreak
carries it on the maintainer's fork until an upstream release holds it. How does DeckStreak pin a
patched engine without loosening ADR-022's supply-chain rule, and when does the carry end?

## Decision Drivers

- The pin is reproducible from the public repository, and `cargo deny` passes with exactly the
  sources the graph uses.
- The patched engine differs from an upstream release by exactly the fix, and that is checkable.
- The carry has one end condition, which a measurement decides.
- Any engine change re-runs ADR-022's protocol.
- The gate never edits files in a dependency's checkout or in cargo's build directory.

## Considered Options (the alternatives it was chosen against)

- A direct `git` dependency on the fork pinned by `rev`, the upstream tag plus the fix — chosen: measured in a scratch resolution, it gives the same `Cargo.lock` as a `[patch]` entry, cargo fetches only the fork, and `cargo deny` passes with `allow-git` naming exactly the fork and `rust-url`, with no unmatched source.
- A `[patch."https://github.com/ankitects/anki.git"]` entry pointing at the fork, the dependency still naming the upstream tag — rejected because, measured, cargo still fetches the upstream repository although nothing in the graph comes from it; keeping upstream in `allow-git` then raises cargo-deny's unmatched-source warning, and dropping it leaves a fetched source that the policy does not name, while the lockfile is byte-identical to the direct pin's.
- Stay on 26.05 unpatched — rejected because it keeps paying 38 to 51 s of engine recompilation on every cargo command (#228) and leaves DeckStreak on a different release from the one the owner moves the predecessor to.
- Upgrade to 26.09.3 unpatched and wait for upstream — rejected because both causes are still in 26.09.3 and on upstream `main`, so the bump saves nothing measurable (31 to 35 s per no-op), and when upstream releases a fix is not DeckStreak's to decide.
- Route C, a gate-side workaround that resets the generated files' mtimes after every cargo command — rejected by the maintainer because it edits cargo's build directory, depends on cargo's internal layout, must follow every ad-hoc cargo command, and a gate that edits mtimes is a new way for a gate to lie.
- Vendor the engine's source into this repository — rejected because it copies a large AGPL tree into a public repository to change one hunk, hides the engine from the lockfile's view of dependencies, and turns every Anki bump into a re-vendoring.

## Decision Outcome

Chosen option: a direct dependency on the fork, pinned by revision.

- **The dependency.** `[workspace.dependencies]` holds
  `anki = { git = "https://github.com/RexRenatus/anki.git", rev = "<the pinned commit>", features = ["rustls"] }`.
  Its comment names this ADR, the upstream tag the commit is based on, and #233.
- **The fork.** `RexRenatus/anki`, the same fork the upstream pull request comes from. It carries a
  branch at upstream tag `26.09.3` plus exactly one commit: the `rslib/io/src/lib.rs` hunk that
  stops `write_file_if_changed` registering a path under the running build script's `OUT_DIR`. The
  other hunks drafted for upstream matter only to a consumer that takes the engine by path, and are
  not carried. The pinned commit carries a tag on the fork, and neither the branch nor the tag is
  force-pushed or deleted while DeckStreak pins it. The maintainer creates and pushes both (#233).
- **The sources.** `allow-git` names exactly `https://github.com/RexRenatus/anki.git` and
  `https://github.com/ankitects/rust-url.git`, and `unknown-git = "deny"` still refuses any other.
  The two advisory exceptions whose crates leave the graph with `burn` (RUSTSEC-2024-0436 for
  `paste`, RUSTSEC-2025-0141 for `bincode`) are removed, and so is the `Unlicense` allowance
  ADR-022 added for `systemstat`: nothing at 26.09.3 needs it (SPEC-055 §1).
- **ADR-022's pin rule is amended while this holds.** The engine is pinned to the fork's commit by
  revision rather than to an upstream tag. Everything else in ADR-022 stands: the protocol, the
  synthetic collection, the budgets, and a re-run of both on any engine change.
- **The work per Anki bump.** While the fork is carried, each upgrade adds three steps to the ones
  any upgrade takes (reading the notes and the tag diff, re-running ADR-022's protocol and SPEC-022's
  criteria):
  1. check first whether the new tag carries the fix, and if it does, remove the fork instead;
  2. the maintainer applies the one commit to the new tag (`git apply --check`, then a cherry-pick),
     then pushes a new branch and tags it. `write_file_if_changed` has not changed since upstream
     #4439, which added the registration, so the hunk has applied unchanged so far;
  3. the delivery moves `rev` to the new commit.
- **The removal condition.** The pinned upstream tag carries the fix, or an equivalent that stops
  registering `OUT_DIR` outputs. The check: with the engine at that tag, unpatched and consumed as a
  git dependency, a second `cargo build -p deck-streak-ingest` finishes with every unit Fresh, in
  cargo's own time (SPEC-055 A2). The removal delivery points the dependency back at
  `https://github.com/ankitects/anki.git` at that tag, puts upstream back in `allow-git` and drops
  the fork, lets `Cargo.lock` follow, supersedes this ADR, and re-runs ADR-022's protocol (#233).
  A2 stays, and it then guards against a regression upstream.

### Consequences

- Good, because no cargo command recompiles the engine any more: a no-op build falls from 31 to 35 s
  to under a second, in the gate and in each of CI's Rust jobs.
- Good, because 26.09.3's graph is smaller: 685 packages become 477, and two exceptions for
  unmaintained crates leave `deny.toml`.
- Good, because the pin, the fork's single commit and the end condition are each checked by a
  test or a recorded measurement, rather than by memory.
- Bad, because DeckStreak trusts a second repository, the maintainer's fork, for the engine's
  source. The pin by revision and the one-file difference from the upstream tag bound that trust.
- Bad, because each Anki bump costs a cherry-pick and a tag until upstream releases the fix.

### Confirmation

SPEC-055's A1 (the pin by revision and the exact `allow-git`), A2 (a second build recompiles
nothing), A3 (every advisory exception and allowed source is live) and A5. The delivery records here,
for A5 to judge with the rules SPEC-022's A1 applies to ADR-009:

- the pinned commit, and `git diff --stat 26.09.3 <rev>` on the fork: one file, `rslib/io/src/lib.rs`;
- ADR-022's measurements at that commit, in `engine-measure.yml` with the run named, as a table of
  measure, budget, measured and verdict;
- the no-op build before and after the pin, in cargo's own time;
- CI's warm path after the first push to `dev` that saves a cache, handed to SPEC-038's amendment
  (#207);
- the status: `accepted` when every budget holds.

## What would make this wrong

- An upstream release carries the fix. The removal delivery then supersedes this ADR (#233).
- Upstream declines the fix, or reshapes the build scripts so that the hunk no longer applies. The
  fix is re-derived against the new code, and A2 is still the check.
- Cargo stops judging build-script inputs by mtime alone, for example if checksum-based freshness
  covers them. The fix may then become unnecessary, and A2 on the unpatched tag would show it.
- The fork's host drops the pinned commit. The tag on it exists to prevent that, and a fresh fetch
  in CI would fail by name.

## More Information

ADR-022 (whose pin rule this amends), ADR-009, ADR-018, ADR-037, ADR-055; SPEC-022, SPEC-038,
SPEC-055; #228, #233, #234, #235. The Cargo Book on `[patch]` and git dependencies
(https://doc.rust-lang.org/cargo/reference/overriding-dependencies.html), cargo-deny's sources check
(https://embarkstudios.github.io/cargo-deny/checks/sources/cfg.html) and rust-lang/cargo#11613
(https://github.com/rust-lang/cargo/pull/11613), each read on 2026-09-28.
