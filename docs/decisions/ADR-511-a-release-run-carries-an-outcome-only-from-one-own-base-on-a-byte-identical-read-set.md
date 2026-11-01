---
status: "accepted"
---

# ADR-511: a release run carries an outcome only from one own base, on a byte-identical read set

Decides SPEC-493. An admitted release run carries a Python mutant's outcome from one recorded
release run of this repository, only for a file whose whole read set is byte-identical at that run
and here, and runs every other listed mutant as it does today.

## Context and Problem Statement

The release run lists every mutant `dev` gained since the last release and runs each one:
`scope_of` (`scripts/mutation-verdict.py:203-212`) names the release case, the listing steps
(`.github/workflows/ci.yml:424-438`) list the diff's and the whole population's mutants, and
`shards` (`ci.yml:441`) deals every listed Python mutant into legs, each read at `dev` `75d555c4`.
A release pull request whose run is repeated after a small change therefore re-runs every listed
mutant, though a recorded run already judged most of them on the same bytes.

Carrying a mutant's outcome when its own file is unchanged is unsound in three ways, each read
at `75d555c4`:

- A test reads data its closure never imports. `scripts/tests/test_threat_model.py:212` runs
  `threat_model.judge(ROOT)` on the real tree, which reads every `docs/schematics/*.md`
  (`scripts/threat_model.py:18`, `:86`), so a new schematic can change a verdict while every
  Python file is unchanged.
- A test probes paths that are absent. A module added where an import looks first, or a file
  added to a directory a test walks, changes an outcome while every file the base run opened is
  equal.
- A base chosen by its outcomes is not one draw. Choosing, among several recorded runs, one in
  which a mutant was killed passes more often than running the mutant once.

A static census of the 16 Python population files (each file, its mapped test modules and their
in-repo imports, read by `ast`) finds, in 14 of them, a member whose reads no static read set can
bound: another process, a directory walk, a read of the working directory, the repository root
passed to a call, or a dynamic import. Rust tests read outside their crates
(`crates/notifications/tests/request_allow_list.rs:564-589`,
`crates/progression/tests/xp_census.rs:396-404`).

## Decision Drivers

- No VOID and no FAIL is waived: a carried survivor fails until it is killed or recorded.
- Every bound, timeout, ceiling and per-mutant budget stays as `dev` holds it. The scoped price
  table and its recorded-run fixture are refreshed whole from one dispatched price run at this
  delivery's head, because the carry module joins the Python population unpriced.
- Over the merged outcomes the verdict is the one a run of every listed mutant gives, whenever
  each carried outcome equals the one a re-run would give.
- Nothing that picks a base reads an outcome.
- No covered span of a formal entry is edited: `judge_python`, `shards`, `whole_reports`,
  `partition`, `planned_leg`, `child_trace` and `scoped_sets` stay as they are.

## Decision

- D1, one base of this repository, by run id. The carry applies only to the release run of a
  release pull request that `CARRY_PULL_REQUESTS`, `frozenset({822})`, names. The base is the
  newest run, older than this one, of this repository's `ci` workflow at
  `.github/workflows/ci.yml` for a `pull_request` event from `dev` into `main`, completed and
  neither cancelled nor skipped, whose head is an ancestor of this head by the forge's compare.
  Exactly one candidate is judged, and a fork's run never qualifies. `mutation-plan` holds
  job-level permissions of `contents: read` and `actions: read`, and downloads the base's
  `mutation-plan` and `mutation-outcomes` (else its `mutation-python-shard-*` reports) by the
  pinned download action, each step bounded and continuing on error.
- D2, tree identity by the diff's sections. A base qualifies only when its plan is a release
  plan whose `base`, the merge ref's first parent, equals this plan's. A path is changed
  when its section of the base's `git.diff` and its section of this run's differ byte for
  byte; a path in neither diff is equal, every binary section is changed, and a directory's
  listing is changed when a changed path below it is present on one side only. A base file rebuilt
  from the merge-base blob and its section must have a blob id that begins with the section's
  `index` id, or there is no base.
- D3, the key. A Python class carries only when the text of the jobs `mutation-python` and
  `mutation-python-release` (comment and blank lines aside) is equal, `scripts/mutation_python.py`
  is byte-identical, and the runner digest of `scripts/mutation_rows.py` is equal: the syntax
  trees of its top-level statements that are not a `def` or `class`, and of each function or
  class the runner names as `mutation_rows.<name>`, closed over the module-level names their
  bodies reference.
- D4, a default-whole census. Each population file's read set is its closure (the file, its
  mapped test modules and every in-repo module they import or load by a constant path), with the
  probes each import and each constant path implies, present or absent, and every listing up to
  the root. A member with any construct outside SPEC-493 R7's allowlist makes the read set the
  whole tree. The census reads every file it reaches, parked files included, and names paths by
  rule or glob.
- D5, per file, all or none, through the legs' view. A file carries when no path or listing of
  its read set changed and every mutant of it listed here was judged once at the base as
  `killed`, `survived`, `uncovered` or `unviable`, in a whole leg or by the base's manifest.
  `carry` writes `carry.json`, then moves each carried file's `code` lines to `carried_code` in
  `plan.json`; the runner lists `python-rerun.json` from that plan, and `shards` deals it. The
  listing steps, `shards`, `planned_leg` and the runner are unchanged.
- D6, the union rule in the Python reader, and the manifest. `python_reports` reads `carry.json`
  and `python-listed.json`: for each class the listed names equal, as multisets, the names the
  whole re-run legs examined plus the carried names, with none in both, or the class is VOID by
  name with its three counts. A carried outcome is judged at the head as a re-run one is, and the
  equivalence records, the rows retirement check and `legs` are evaluated at the head and never
  carried. The verdict job uploads `mutation-outcomes` under `if: always()`: the tree identity,
  each class's key, and one entry per Python mutant judged in a whole leg or carried, never a
  `timeout` or a `void` one.

## What it was chosen AGAINST

- Carrying a mutant when its own file is unchanged: rejected because a root reader such as `test_threat_model.py` and an absent probe each change an outcome while the file is unchanged, so the rule admits a change that a run of every mutant fails.
- A declared read set per class (the file, its tests, their imports and fixtures, the map and the runner): rejected because it counts only files that exist and are imported, and misses a data file read by code, a module added where an import looks first and a file added to a walked directory.
- Per-mutant carry by filtering inside `planned_leg` or `shards`: rejected because both are covered spans of the `EveryLegCounted.tla` entry, so the change stales it and needs its model amended, where the legs' view moves only lines of `plan.json`.
- A file-level key over `scripts/mutation_rows.py`: rejected because open changes to that file, #954 among them, would make every release run carry nothing while they land, though the runner names only two of its functions.
- Leaving the runner out of the key: rejected because it narrows the gate: a runner change that alters what a mutant is or how a leg judges it would carry old outcomes.
- A traced read set per mutant now: rejected because no recorded run holds a trace to compare with, so the first traced run carries nothing, and it changes the runner's child; it is the later refinement SPEC-493 section 5 names.
- The best of several bases, or any base in which the mutant was judged whole: rejected because a base chosen by its outcomes passes more often than one draw of the same tests.
- The newest base per Python class: rejected because both Python classes share one key, so it picks the same run whenever either class qualifies, and adds a second provenance to every verdict line.
- Carrying `timeout` and `void` outcomes: rejected because a timeout follows the runner's load and the clock rather than the read set, and a void outcome measured nothing.
- Carrying Rust outcomes on the crates, the manifests, the lock and the toolchain: rejected because Rust tests walk the repository root, and 203 `.rs` files in 26 of 31 packages hold a token that can reach outside their crate.
- Admitting every release pull request now: rejected because a later release pull request's run is bound by SPEC-422 R4 and the release-legs ruling, which only the owner amends; the amendment is drafted for the owner's signature.

## Decision Outcome

D1 to D6 are built by SPEC-493 in one pull request into `dev`, with a Lean entry,
`formal/lean/Formal/ReleaseCarry.lean`, that ports the union rule and the per-file carry
predicate and enters at `ramp=report`.

### Consequences

- An admitted release run re-runs every listed Python mutant of a file whose read set changed,
  carries the rest from one base, and prints per class `carried N from run R` apart from its
  examined count; a class that applies with nothing re-run and nothing carried is VOID.
- A file whose read set is the whole tree carries only when no path at all changed between the
  base and the head; under R7's allowlist most population files are such files at `75d555c4`.
- Rust, rows and web outcomes run as before, and no listing, bound or budget changes.
- `mutation-plan` gains `actions: read` and its base steps; the workflow's default token stays
  `contents: read`.
- Each release run's manifest is a later run's base record, so a carried outcome can chain
  from run to run, each entry naming the run that examined it.
- SPEC-039 R3 and ADR-057 gain one insert-only amendment each. SPEC-422 and the release-legs
  ruling are unchanged, so a later release pull request is admitted only by the owner's
  amendment of both.

### Confirmation

SPEC-493's criteria A1 to A5 in the pull request's checks; its rows S49301 to S49304 read
KILLED by id; the Lean entry's two theorems read by the formal checker; and a release run's
own logs: `mutation-plan` names the base it chose or why there was none, and the verdict prints,
per Python class, the listed, re-run and carried counts.

## What would make this wrong

- A carried outcome that a re-run on the same read set and key would not give: an outcome is then
  not a function of its read set, as with a flaky test, a test near its timeout, or a runner image
  that moved under its label.
- A path a member probes that the census does not hold, through a construct R7 admits: the
  allowlist then bounds a member it should have made whole-tree.
- A base whose `mutation-plan` or manifest its own run did not write.
- A verdict whose listed names are not exactly the re-run names plus the carried ones, with no
  VOID printed.
- Release runs that carry next to nothing because most files are whole-tree: the census then
  costs more than it saves, and the traced read set of SPEC-493 section 5 is due.

## More Information

Built by SPEC-493; drawn in `docs/schematics/mutation-release-carry.md`. Refers #822.
