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
- The cost of Anki's engine: a cold build of `ingest` takes minutes, and its build script runs
  again on every cargo command.
- A local run's cost: a copy of the tree pays a cold build of the engine and its disk each time,
  where a targeted, in-place run reuses what the gate already built.
- One reader for the rows, in DeckStreak's own code, and no vendored file: the packs stay
  box-only (ADR-056).
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
- `--in-place`, locally and in CI: chosen. Measured on `clock.rs`, with `-j 1` for the copy and
  serial in place: 17 s warm and 34 s cold in place, against 41 s for the default copy, which paid
  a 21 s cold build in a temporary directory. A CI checkout is disposable and its `target/` is the
  restored cache; a local run reuses the target the gate already built.
- The default copy into a temporary directory: rejected for DeckStreak, because each run pays a
  cold build, and with the engine that is minutes and a whole target of temporary disk. Its one
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
- The mutation-rows pack's shape, read by DeckStreak's own code: chosen, because a SPEC's rows then
  live in one file only it writes, so concurrent deliveries do not collide. A header file declares
  each table's target spelling, band fragments hold `{"tables": {...}}`, one band per SPEC
  (`S<NNN>00` to `S<NNN>99`), and `scripts/mutation_rows.py` is the one reader, whose census
  refuses a row that can prove nothing.
- One flat rows file: rejected, because every delivery would append to one file, the collision the
  pack's fragments were made to end.
- A band per delivery: rejected, because a SPEC's rows would scatter across files named by
  delivery, and the band would not say which SPEC's invariant a row guards.
- A format of DeckStreak's own: rejected, because the pack's shape carries its measured lessons
  (a band per file, a header that declares how each table spells its target), and a new format
  would have to learn them again.

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
  engine's mutants over every shard, about five each, and keeps each shard's job between 31 and 58
  minutes (run 36384080819, 2,154 mutants) under a `timeout-minutes` of 120. `--timeout 300` bounds each mutant's tests, above the engine's 140 s,
  and `--build-timeout 600` each mutant's build: in place, cargo-mutants applies no multiplier and
  bounds no build unless told, and it never bounds the unmutated baseline's build, so a cold cache
  cannot time a shard out. The `survivors` job counts every report the jobs promise, and fails
  naming each one missing or partial.
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

**D9, the runner under each mutant.**
- cargo-nextest 0.9.146 (`test_tool = "nextest"`): chosen, because it is the gate's own runner, a
  process per test, so a mutant is judged by the tests exactly as the gate runs them. Measured: the
  unmutated baseline passed identity's tests under it (SPEC-039 section 8).
- `cargo test`, cargo-mutants' default: rejected by measurement, because one process per binary
  failed identity's `init_data_never_reaches_the_log` at the baseline on two of 32 CI runners (run
  36372763914), so those shards examined nothing (exit 4).

**D10, the pack's own probe.**
- DeckStreak's own checks: chosen, because the owner decided the packs stay box-only (ADR-056), so
  this delivery vendors no file of the mutation-rows pack. The census refuses a row that can prove
  nothing, and `mutation-verdict.py configs` judges each tool's configuration by what the tool
  itself refuses, read from each tool's own source.
- Vendoring the pack's probe and wiring its rows into the gate: rejected, by that decision. A check
  CI needs lives in this repository's own scripts, where its tests can hold it.

**D11, the release's run, and its shards.**
- A release pull request into `main` judged on its merge diff, in shards the plan sizes: chosen,
  by the owner's directive ("deckstreak will need per merge diff mutation testing", "dev to main").
  The plan lists the diff's mutants with cargo-mutants itself (`--list --json --in-diff`, which
  builds nothing), projects each round-robin shard's time from the unmutated baseline and each
  mutant's package cost, measured on the weekly battery's GitHub runners, and takes the fewest
  shards whose slowest is projected within an hour, half the shard job's timeout. The matrix is the
  plan's output, and the verdict counts every shard the plan promised, from `0` to `n-1`, as the
  battery does (D7), and checks that the shards' reports hold every listed mutant once. One path
  judges every diff: a pull request into `dev` whose diff fits one shard runs one.
- The release read `not-applicable`: rejected by the directive, though each of its changes was
  judged on its own pull request into `dev`. It is also the weaker proof: the diff that ships is
  the merge of every change, and the weekly battery sweeps `dev` only once a week.
- A fixed shard count, such as the battery's 32: rejected, because a release's diff and a pull
  request's range from a handful of mutants to the whole repository's (the release diff of `dev`
  09b60d2 holds all 2,153 of its mutants). A fixed count wastes runners on a small diff and can
  overrun on a large one; the projection sizes each run to its own diff.
- A separate release-only job pair, leaving `mutation-rust` unsharded for `dev`: rejected, because
  two paths would judge one rule, the release's rows and retirement check would need a second
  home, and a pull request into `dev` meets the same bound: at 124 s a mutant, about fifty of
  `ingest`'s mutants fill the job's 120 minutes.
- A shard count from the changed lines, without the tool: rejected, because mutants per line vary
  by kind of code (a method named `new` gives none), and only the tool's own listing is exact.
- Capping the shards, or the mutants a run examines: rejected; a run that needs more than the
  256 jobs a matrix holds is refused with its projection, never capped, and the change is split.

## Decision Outcome

Chosen: D1 to D11's first options. SPEC-039 R1 to R18 state them as requirements.

### Consequences

- Good, because every pull request that changes production code reports how many mutants it
  examined, and a zero, a missing report or an unviable-only run fails it.
- Good, because a constant, a method named `new` or a guard that the tool cannot mutate is held
  by a row, and a row cannot leave while its target stays without the maintainer's approval.
- Good, because DeckStreak's own census and configuration check judge the rows and the tools'
  configurations, and no vendored file is needed.
- Good, because the release is judged on the diff that ships, and a diff of any size runs in
  shards sized to it, each projected within half its job's timeout.
- Bad, because a release's run costs runner time: the whole repository's mutants, about as many
  runner-hours as the weekly battery. A newer push to the release pull request cancels the run it
  supersedes, so only the head that merges pays in full.
- Bad, because the projection's costs are means measured on GitHub's runners, and they go stale
  as the tests grow. A shard that outruns its bound leaves a partial report or none, which the
  verdict names VOID; the table is then measured again from the weekly battery's reports.
- Bad, because StrykerJS 10.0.0 has two open defects that read a kill as a survivor
  (stryker-js #6144, #6150). A false survivor fails loudly and is read by a person.
- Bad, because the weekly battery starts only after a release carries it to `main`.

### Confirmation

SPEC-039's A1 to A37; the `mutation-rust` job's red run and green run on the delivery's pull
request (R17); the census and the configuration check in the gate's `python` stage; the release's
plan measured on a synthetic merge of `dev` into `main` (SPEC-039 section 8).

## What would make this wrong

- A cargo-mutants release that mutates methods named `new` or constants: the rows that exist only
  because it did not would then duplicate generated mutants, and retire with the maintainer's
  approval (R11).
- A shard whose measured time nears its bound: the cost table is stale, or a crate's tests
  outgrew it, and the table is measured again.
- A cargo-mutants release that assigns round-robin shards differently: the verdict's partition
  check then names a mutant in two shards or in none, and the plan follows the tool.

## More Information

SPEC-039; issue #217; #218, #219, #220, #222 and #240 (the follow-ups); ADR-056 (the packs stay
box-only),
ADR-012 (testing), ADR-017 (CI), ADR-029 (the golden reader). cargo-mutants: mutants.rs
(in-diff, in-place, shards, exit codes); StrykerJS: stryker-mutator.io and the stryker-js
repository; GitHub: "Events that trigger workflows".

Amended in part on 2026-09-28, by ADR-070 (proposed with SPEC-057): D6's form, an anchored
`exclude_re` entry or a `Stryker disable` comment, hides the mutant it excuses from the listing,
since cargo-mutants filters an excluded mutant out and StrykerJS never runs an ignored one, so no
later run tests the claim and no count from the tools' reports sees it. An equivalent mutant is
instead recorded in `scripts/mutation-equivalent.d/`, one fragment per package, bound by an anchor
to exactly one listed mutant, and excused only while the tool keeps reporting it missed. This takes
effect when SPEC-057's first delivery accepts ADR-070; until then D6 stands. The rest of this
decision does not change.

Note (2026-09-28): the Confirmation's criteria are SPEC-039's A1 to A40, not A1 to A37.
