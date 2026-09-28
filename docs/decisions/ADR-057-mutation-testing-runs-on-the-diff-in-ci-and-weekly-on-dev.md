---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# Mutation testing runs on the diff in CI and weekly on dev

## Context and Problem Statement

Nothing in DeckStreak's CI proves that a test can fail: a test that passes whatever the code does
passes the gate. The maintainer's rule is that every behaviour change proves its tests kill
mutants of the changed code, that a run which examined nothing is VOID rather than green, that
invariants carry hand-proved rows, that nothing is weakened silently, and that the whole
repository is swept once a week (issue #217). Which tools, which scope, which run shapes, and
which record of rows make that true on a Rust workspace that embeds Anki's engine, a SvelteKit
front end and a Python parity oracle, on GitHub-hosted runners (SPEC-039)?

## Decision Drivers

- A verdict read from the tool's own report, never from an exit code alone, because each tool
  measured here exits 0 having examined nothing (SPEC-039 §1).
- The cost of Anki's engine: a cold build of `ingest` is about 5 minutes and 15 to 18 GB.
- The box's limits: two cargo slots, and a disk that must stay above 25 GB.
- One reader for the rows, and the vendored pack's own classes able to judge them.
- A pull request never saves a cache, and nothing reaches an issue unscrubbed.

## Considered Options (the alternatives it was chosen against)

**D1, the tools.**
- Each ecosystem's tool: chosen, because each generator is its ecosystem's maintained tool:
  cargo-mutants 27.1.0 for Rust, StrykerJS 10.0.0 with its Vitest runner for the Mini App, and
  hand-proved rows for the oracle's Python, each pinned exactly and installed by a SHA-pinned
  action or the lockfile.
- mutmut 3.8.0 for the oracle's Python: rejected by measurement. It refused the oracle's layout,
  and with the test's module name changed it reported 415 of 415 mutants as "no tests" and exited
  0. Fitting it would mean moving the oracle into an importable package, which changes the
  generator's pinned bytes and every golden's recorded digest.
- cosmic-ray for the oracle's Python: not chosen here, because it is unmeasured on this tree; it
  mutates the file in place and runs any test command, so it may fit, and #219 measures it.
- No generated mutants, rows only: rejected, because a row is one mutant a person chose, and the
  rule asks for the mutants of every changed line.

**D2, where cargo-mutants builds.**
- `--in-place`, locally and in CI: chosen. Measured on `clock.rs`, `-j 1`: 17 s warm and 34 s cold
  in place, against 41 s for the default copy, which paid a 21 s cold build in a temporary
  directory. A CI checkout is disposable and its `target/` is the restored cache; a local run
  reuses the target the gate already built.
- The default copy into a temporary directory: rejected for DeckStreak, because each run pays a
  cold build, and with the engine that is minutes and up to 18 GB of temporary disk. Its one
  advantage, never touching the working tree, is kept another way: runs are on a committed tree
  only, and cargo-mutants restores each file.
- The copy with `copy_target`: rejected, because it copies the whole `target/`, the engine
  included, on every run.

**D3, what a pull request mutates.**
- Rust: `--in-diff` over the merge ref's diff (the mutants whose span overlaps a changed line):
  chosen. It is the tool's own change-only mode, and its cost follows the diff.
- Rust, every mutant of every touched file: rejected for pull requests, because `rails.rs` alone
  has 378 mutants, and `--in-place` runs one at a time, so one touched file could cost an hour.
  The weekly battery sweeps whole files, and files each one's survivors as an issue.
- Web, every mutant of every touched file: chosen, because a whole file costs seconds (20 mutants
  of `startapp.ts` in 16 s), and a survivor already on a touched file is the pull request's own
  (the maintainer's rule 5).
- Web, the changed lines only (`--mutate file:start-end`): rejected, because it gains little over
  a whole file and leaves a touched file's old survivors to the weekly run.

**D4, what VOID means.**
- Per class: a class whose production files changed a code line and whose examined count is zero
  is VOID and fails; a row the diff selects on a changed line counts as examined; blank and
  comment lines, and deletions, read `not-applicable` by name and count: chosen. It is the
  maintainer's rule 3 at the granularity each tool reports, with the edge case (a diff of
  comments, attributes or constants gives zero mutants) decided in the open.
- Pass a zero-mutant diff silently: rejected; that is the vacuous green rule 3 exists to refuse.
- VOID per file: rejected for now, because a file whose change is a type, an enum variant or a
  `use` line gives the tool nothing, and no row can mutate a type; the verdict names such a file
  `unexamined` instead.
- Treat any zero as VOID, comments included: rejected, because a documentation change could then
  never pass, with no test or row able to make it pass.

**D5, the rows.**
- The mutation-rows pack's shape: a header file, band fragments of
  `{"tables": {...}}`, and a reader with the pack's interface (`load_tree`, `PopulationRefused`,
  `row_target`), one band per SPEC (`S<NNN>00` to `S<NNN>99`): chosen. The vendored probe's
  `find-differs`, `band-ids` and `mutants-distinct` then judge the rows, and a SPEC's rows live in
  one file only it writes, so concurrent deliveries do not collide.
- One flat rows file: rejected, because every delivery would append to one file, the collision the
  pack's fragments were made to end.
- A band per delivery: rejected, because a SPEC's rows would scatter across files named by
  delivery, and the band would not say which SPEC's invariant a row guards.
- A format of DeckStreak's own: rejected, because the vendored probe's row classes could not read
  it, and a second reader is a second place for a rule to drift.

**D6, recording an equivalent mutant.**
- One named mutant per exclusion: chosen, because each names one mutant, and a test holds the
  form: one anchored `exclude_re` entry in `.cargo/mutants.toml`, or one `Stryker disable
  next-line` comment, each with its reason and an issue.
- `#[mutants::skip]`: rejected, because it skips every mutant of the item, and a `cfg_attr(test,
  ...)` form needs the `mutants` crate as a dependency of every crate that uses it.

**D7, the weekly battery.**
- Thirty-two round-robin shards: chosen, because the engine's crates cost minutes a mutant. Anki's
  engine re-runs its build script on every cargo command, about 26 to 28 s even warm, and
  `ingest`'s 102 tests take about 140 s (SPEC-038's measurements), so each of `ingest`'s 168
  mutants costs about 3 minutes, and the 1,721 mutants on `dev` about 14 hours serially.
  Round-robin (`--sharding round-robin --shard k/32`, the matrix naming 0 to 31) spreads the
  engine's mutants over every shard, about five each, and keeps each shard near 40 minutes under a
  `timeout-minutes` of 120; `--timeout 300` bounds every cargo command, above the engine's 140 s.
- Sixteen slice shards: rejected, because a slice keeps a crate's mutants together, and the shard
  that holds `ingest`'s would run about five hours, past any job timeout.
- `--baseline=skip` behind a test job: rejected, because it adds a job whose only purpose is the
  baseline each shard can run itself, and each shard would then trust a baseline it never saw.
- One issue per mutant: rejected as noise; one issue per file, deduplicated by title against the
  open issues, keeps a survivor's context together.

**D8, proving the battery before it reaches main.**
- A rehearsal on the pull request: chosen, because `schedule` and `workflow_dispatch` run only
  for a workflow on `main`. A `pull_request` trigger filtered to the workflow's own file runs a
  `rehearsal` job (one file, one row, one Stryker file, and the survivors' drafting and scrub,
  filing nothing), and one dispatch is measured on the delivery's branch.
- Wait for a release: rejected, because the battery would first run unproved, on a schedule
  nobody watches.

## Decision Outcome

Chosen: D1 to D8's first options. SPEC-039 R1 to R17 state them as requirements.

### Consequences

- Good, because every pull request that changes production code reports how many mutants it
  examined, and a zero, a missing report or an unviable-only run fails it.
- Good, because a constant, a method named `new` or a guard that the tool cannot mutate is held
  by a row, and a row cannot leave while its target stays without the maintainer's approval.
- Good, because the vendored probe judges the tools' configurations and the rows.
- Bad, because a Rust pull request's CI time grows with its diff, most in the engine's crates
  (about 3 minutes a mutant in `ingest`); `timeout-minutes` bounds it, and a diff that reaches the
  bound shards the job the way the battery is sharded.
- Bad, because StrykerJS 10.0.0 has two open defects that read a kill as a survivor
  (stryker-js #6144, #6150). A false survivor fails loudly and is read by a person.
- Bad, because the weekly battery starts only after a release carries it to `main`.

### Confirmation

SPEC-039's A1 to A26; the `mutation-rust` job's red run and green run on the delivery's pull
request (R17); the pack's `practice` rows and `find-differs`, `band-ids` and `mutants-distinct`
in the gate's `packs` stage.

## What would make this wrong

- A cargo-mutants release that mutates methods named `new` or constants: the rows that exist only
  because it did not would then duplicate generated mutants, and `mutants-distinct` would say so.
- A pull request whose diff reaches the job's timeout: the job then shards.

## More Information

SPEC-039; issue #217; #218, #219 and #220 (the follow-ups); ADR-004 and ADR-039 (vendoring),
ADR-012 (testing), ADR-017 (CI), ADR-029 (the golden reader). cargo-mutants: mutants.rs
(in-diff, in-place, shards, exit codes); StrykerJS: stryker-mutator.io and the stryker-js
repository; GitHub: "Events that trigger workflows".
