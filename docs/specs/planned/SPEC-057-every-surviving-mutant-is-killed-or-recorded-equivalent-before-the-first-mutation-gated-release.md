# SPEC-057: every surviving mutant is killed or recorded equivalent before the first mutation-gated release

- **Wave:** W0. **Issue:** none of its own: it plans the owner's ruling (2026-09-28) that the first
  release judged by mutation testing waits until every surviving mutant is killed or recorded
  equivalent. Each delivery names the issue it closes, and the Mini App's delivery closes #240.
  **Context(s):** `repo` (`scripts/`, `.github/`, `.cargo/`, `docs/`), and every package the
  release's merge diff lists: `deck-streak-vault`, `deck-streak-ingest`, `deck-streak-kernel`,
  `deck-streak-identity`, `deck-streak-daemon`, `deck-streak-coordination`, `deck-streak-api`,
  `deck-streak-agent`, `deck-streak-progression`, `deck-streak-privacy` and `miniapp`.
- **Decided by:** ADR-070 (this SPEC's: the equivalence record excuses exactly a recorded mutant),
  ADR-057 (mutation testing on the diff and weekly; its D6 amended in part by ADR-070), ADR-016
  (planned SPECs), ADR-069 (a criterion retired insert-only), ADR-034 (`main` changes only by a
  release from `dev`) and ADR-059 (public text).
- **Status:** planned (in `docs/specs/planned/`). Each delivery fills its own row of section 7 and
  extends `docs/red-first/SPEC-057.md`; the last delivery, the Mini App's, moves this SPEC to
  `docs/specs/` (ADR-016).

## 1. The problem, measured

The first release pull request that mutation testing judges, `dev` into `main`, is judged on its
merge diff (SPEC-039 R3 and R18), and its `mutation-verdict` and `mutation-web` jobs fail on every
mutant that survives there. Every number below was recounted from the tools' own reports or listed
by the pinned tools (cargo-mutants 27.1.0, StrykerJS 10.0.0), never copied.

### 1.1 The run that counted the survivors

- **Run 36384080819**, a `workflow_dispatch` of `mutation-weekly.yml`, judged `4082551`: the head,
  at the time, of SPEC-039's delivery branch (#221), which carried `dev` at `9e8e53e` (#223) and
  that delivery's own commits. `dev` gained #221 afterwards, as one squash commit.
- **31 of its 32 shards reported whole**: exit 2 (16 shards) or 3 (15), with caught, missed, timed
  out and unviable summing to each report's `total_mutants`. Shard 21's runner received a shutdown
  signal and uploaded nothing (SPEC-039 section 8). Its shard jobs took 30 to 58 minutes on
  GitHub's runners.
- **Shard 21's mutants are known by name.** cargo-mutants lists 2,154 mutants at `4082551`, and each
  reported shard `k` holds exactly the listing's mutants `k`, `k + 32`, `k + 64` and so on. Shard 21
  held 67, and no report holds any of them: `vault` 29, `kernel` 12, `coordination` 10, `ingest` 9,
  `identity` 4, `daemon` 2 and `api` 1. Its job log records an outcome for 48 of them before the
  runner's shutdown signal, about 54 minutes in: 37 caught, 5 unviable and 6 missed, the 6 its
  annotations name (`vault` 4, `rails.rs` 2 with `note.rs` and `config.rs` 1 each; `ingest` 1,
  `sync.rs`; `identity` 1, `session.rs`). The other 19, all `vault`'s, from `rails.rs:594:18` on,
  were never reached. So at least 316 mutants survived at `4082551`, and 19 were never measured;
  the counts below are the 31 reports' and a floor.

| package | listed | reported | caught | timed out | missed | unviable | in shard 21 |
|---|---|---|---|---|---|---|---|
| `deck-streak-vault` | 939 | 910 | 619 | 8 | 218 | 65 | 29 |
| `deck-streak-ingest` | 297 | 288 | 180 | 2 | 43 | 63 | 9 |
| `deck-streak-kernel` | 367 | 355 | 284 | 8 | 17 | 46 | 12 |
| `deck-streak-identity` | 143 | 139 | 94 | 0 | 15 | 30 | 4 |
| `deck-streak-daemon` | 65 | 63 | 35 | 0 | 9 | 19 | 2 |
| `deck-streak-coordination` | 292 | 282 | 240 | 0 | 5 | 37 | 10 |
| `deck-streak-api` | 51 | 50 | 33 | 0 | 3 | 14 | 1 |
| total | 2,154 | 2,087 | 1,485 | 18 | 310 | 274 | 67 |

The 310 missed mutants lie in 33 files, counted from the same 31 reports:

| file | reported | caught | missed | timed out | unviable |
|---|---|---|---|---|---|
| `crates/vault/src/rails.rs` | 366 | 191 | 145 | 7 | 23 |
| `crates/vault/src/staged.rs` | 116 | 72 | 29 | 0 | 15 |
| `crates/vault/src/readings_tree.rs` | 184 | 157 | 15 | 1 | 11 |
| `crates/vault/src/note.rs` | 113 | 99 | 12 | 0 | 2 |
| `crates/vault/src/config.rs` | 28 | 10 | 9 | 0 | 9 |
| `crates/vault/src/fs.rs` | 20 | 12 | 4 | 0 | 4 |
| `crates/vault/src/sha256.rs` | 80 | 75 | 4 | 0 | 1 |
| `crates/ingest/src/sync.rs` | 76 | 45 | 17 | 2 | 12 |
| `crates/ingest/src/engine.rs` | 25 | 11 | 11 | 0 | 3 |
| `crates/ingest/src/sync_runs.rs` | 31 | 17 | 7 | 0 | 7 |
| `crates/ingest/src/settings.rs` | 45 | 36 | 3 | 0 | 6 |
| `crates/ingest/src/gate.rs` | 23 | 14 | 2 | 0 | 7 |
| `crates/ingest/src/lock.rs` | 6 | 0 | 2 | 0 | 4 |
| `crates/ingest/src/data_rights.rs` | 13 | 0 | 1 | 0 | 12 |
| `crates/kernel/src/redact.rs` | 78 | 64 | 5 | 6 | 3 |
| `crates/kernel/src/study_day.rs` | 182 | 173 | 4 | 0 | 5 |
| `crates/kernel/src/credentials.rs` | 12 | 9 | 2 | 0 | 1 |
| `crates/kernel/src/db.rs` | 17 | 6 | 2 | 0 | 9 |
| `crates/kernel/src/verdict.rs` | 5 | 2 | 2 | 0 | 1 |
| `crates/kernel/src/offload.rs` | 3 | 1 | 1 | 0 | 1 |
| `crates/kernel/src/settings.rs` | 22 | 11 | 1 | 2 | 8 |
| `crates/identity/src/session.rs` | 68 | 43 | 8 | 0 | 17 |
| `crates/identity/src/init_data.rs` | 49 | 40 | 4 | 0 | 5 |
| `crates/identity/src/owner.rs` | 13 | 5 | 3 | 0 | 5 |
| `crates/daemon/src/lifecycle.rs` | 37 | 18 | 8 | 0 | 11 |
| `crates/daemon/src/main.rs` | 15 | 13 | 1 | 0 | 1 |
| `crates/coordination/src/runner.rs` | 39 | 29 | 2 | 0 | 8 |
| `crates/coordination/src/delivery.rs` | 1 | 0 | 1 | 0 | 0 |
| `crates/coordination/src/liveness.rs` | 55 | 50 | 1 | 0 | 4 |
| `crates/coordination/src/obligations.rs` | 8 | 6 | 1 | 0 | 1 |
| `crates/api/src/health.rs` | 12 | 8 | 1 | 0 | 3 |
| `crates/api/src/session_routes.rs` | 20 | 16 | 1 | 0 | 3 |
| `crates/api/src/settings.rs` | 7 | 2 | 1 | 0 | 4 |

- **The Mini App.** The run's `web` job swept the whole app, 274 mutants: 192 killed, 1 timed out,
  53 survived and 28 uncovered. The release's merge diff holds 8 of its files and 266 of its
  mutants, among them all 53 survivors and 21 of the uncovered; the other 8 mutants, in the three
  `hooks` files, are 1 killed and 7 uncovered. #240 tracks all 81. No Mini App production file has
  changed since the run.

| file | in the release | killed | timed out | survived | uncovered |
|---|---|---|---|---|---|
| `web/app/src/lib/api.ts` | yes | 101 | 1 | 19 | 4 |
| `web/app/src/lib/telegram.svelte.ts` | yes | 45 | 0 | 18 | 0 |
| `web/app/src/lib/startapp.ts` | yes | 13 | 0 | 7 | 0 |
| `web/app/src/routes/+page.svelte` | yes | 21 | 0 | 5 | 1 |
| `web/app/src/routes/about/+page.svelte` | yes | 11 | 0 | 3 | 0 |
| `web/app/src/lib/routes.ts` | yes | 0 | 0 | 1 | 0 |
| `web/app/src/routes/+layout.ts` | yes | 0 | 0 | 0 | 15 |
| `web/app/src/routes/+layout.svelte` | yes | 0 | 0 | 0 | 1 |
| `web/app/src/hooks.server.ts` | no | 0 | 0 | 0 | 6 |
| `web/app/src/hooks.ts` | no | 0 | 0 | 0 | 1 |
| `web/app/src/hooks.client.ts` | no | 1 | 0 | 0 | 0 |

- **The rest of the run.** Its `rows` job proved all 27 rows KILLED. Its `survivors` job drafted 43
  issues, one per file with a survivor (the 33 Rust files and 10 of the Mini App's), and no survivor
  issue exists: the battery goes live only once a release carries it to `main`.

### 1.2 How the population moved: re-listed at `dev` 16ed8e2

- **The release, rehearsed.** `dev` 16ed8e2 (#247) is this plan's base. On a local synthetic merge
  of it into `main` 3d77726, never pushed, whose tree equals `dev`'s, SPEC-039's
  `plan --event pull_request --base-ref main` reads the release's case: 731 changed paths (66 Rust
  files, 8 Mini App files and the oracle's generator) and 48 selected rows. cargo-mutants lists
  2,207 mutants of that merge diff (`--list --json --in-diff`, which builds nothing), the same 2,207
  as its listing of the whole tree: the first release is judged on every mutant the tree holds.
  `shards` takes 27 shards, projected at 82,934 s serially and the slowest at 3,568 s of its
  3,600 s bound.
- **Per package.** A mutant at 16ed8e2 is the same as one at `4082551` when its file, function,
  genre, replacement and the source lines its mutation replaces match, counted as a multiset.

| package | listed at `4082551` | listed at 16ed8e2 | the same | new since the run | gone since the run |
|---|---|---|---|---|---|
| `deck-streak-vault` | 939 | 939 | 931 | 8 | 8 |
| `deck-streak-kernel` | 367 | 368 | 366 | 2 | 1 |
| `deck-streak-ingest` | 297 | 297 | 297 | 0 | 0 |
| `deck-streak-coordination` | 292 | 296 | 292 | 4 | 0 |
| `deck-streak-identity` | 143 | 143 | 143 | 0 | 0 |
| `deck-streak-daemon` | 65 | 84 | 60 | 24 | 5 |
| `deck-streak-api` | 51 | 51 | 51 | 0 | 0 |
| `deck-streak-privacy` | 0 | 29 | 0 | 29 | 0 |
| total | 2,154 | 2,207 | 2,140 | 67 | 14 |

- **What no sweep has measured.** The 67 new mutants, `deck-streak-privacy`'s 29 among them (#245
  added the crate after the run), and the 19 of shard 21's that its job never reached.
- **The survivors moved.** Every one of the 310 is still listed at 16ed8e2, and 157 of them under a
  different `file:line:column` name, counted by binding each to its mutant (section 1.4), because
  lines above them moved: `vault` 145, `ingest` 11 and `daemon` 1. Two old names now belong to other
  survivors: in `unquote`, the pair at `rails.rs:846:16` moved up three lines, and the pair from
  `849:16` took their name, so a count of old names missing from the new listing finds only 155.

### 1.3 How an equivalent is recorded today, and why the campaign cannot use it

- SPEC-039 R5 records an equivalent mutant as one anchored `exclude_re` entry in
  `.cargo/mutants.toml`, with `# EQUIVALENT: <reason> (#N)` above it, or as a
  `// Stryker disable next-line <mutator>: EQUIVALENT: <reason> (#N)` comment (ADR-057 D6).
- cargo-mutants filters every `exclude_re` match out of its listing, so an excluded mutant is never
  listed, sharded, run or reported: no later run tests the claim, and a count taken from the tool's
  reports cannot see it. StrykerJS reports a disabled mutant as `Ignored` and never runs it.
- The name an `exclude_re` anchors on carries the line and the column: 157 of the 310 names moved in
  one day's merges, and two of the old names now name a different survivor.
- The tree holds no exclusion today: `mutation-verdict.py exclusions` examines 0, and no file
  holds a `mutants::` attribute or a `Stryker disable` comment. Nothing needs migrating.

### 1.4 A record bound by an anchor, measured (ADR-070)

- **The rule.** The rows' anchor (SPEC-039 R8), applied to a mutant: a text that occurs exactly once
  in the file and inside whose occurrence the mutant's span starts, together with the tool's own
  description of the mutation, such as cargo-mutants' `replace + with * in civil_from_days`.
- **The Rust survivors.** A whole-line anchor binds 269 of the 310 to exactly one mutant, and a
  narrower window around the span's start binds the other 41, whose anchoring lines hold a second
  mutant of the same description (37 on the mutant's own line, 4 on a line added to make the text
  unique). All 310 bind at `4082551` and again at 16ed8e2, across the 157 moved names.
- **The Mini App's 81 survived and uncovered mutants.** Lines bind 71 and windows 4. The other 6
  also need the mutated text, because a `ConditionalExpression` over `a && b` and the one over `a`
  start at one position. All 81 bind.
- **Without a position** no record could say which mutant it excuses. Keyed by the file and the
  tool's description, the fields R5 gives a record besides its anchor, 667 of the 2,207 mutants at
  16ed8e2 share their key with another mutant, in 188 shared keys; adding the mutated span's text
  still leaves 665.

### 1.5 Two riders the first delivery carries

- **A squash merge's subject.** `scripts/mutation-verdict.py` reads a push as a merge only when its
  subject starts `Merge pull request #N from`, and reads such a push `not-applicable`. `dev` also
  takes squash merges, whose subject is the pull request's title followed by ` (#N)` (`0f68ed4` for
  #221, `32bf1e1` for #238). Such a push names no merge by that rule, so it is judged on its
  first-parent diff, the diff its pull request's jobs already judged at an up-to-date head
  (ADR-034).
- **A lock that only `--locked` accepts.** #246 put `Cargo.lock` back in cargo's canonical form
  after a text merge left a version qualifier that `--locked` accepts and an unlocked resolve
  rewrites. cargo-mutants resolves without `--locked`, so such a lock leaves the tree dirty, and the
  rows' runner refuses a dirty tree. Every gate stage passes `--locked`, so nothing in the gate
  refuses that lock; #246 left the guard to the next mutation delivery.

### 1.6 A third rider, measured on the vault's own diff

The vault's diff at dd734e5, against `dev` f5322b2, changes three files under `crates/vault/src`,
`note.rs`, `readings_tree.rs` and `staged.rs`, and only inside their `#[cfg(test)] mod tests`
items: 86 added lines, 71 of them code. SPEC-039's `plan` counts every such line as a production
code line (its R2 and R4), so the Rust class applied. cargo-mutants 27.1.0 never mutates an item
marked `#[cfg(test)]`, or one with an attribute whose path ends in `test` (its `visit.rs`), and
when no mutant overlaps a diff it exits 0 before it lists, so `--list --json --in-diff` printed
nothing at all, not `[]`. `shards` read that empty file as no listing and was VOID, and the
verdict was VOID because the plan named no shards (run 36463302615; run 36461579108, at 8b18276,
whose `crates/` are the same, read the same). Any pull request that adds a unit test in `src` and
changes no production line reads the same. The architect ruled it a defect of the plan rather than
of the delivery, and gave the vault's delivery a third rider (R22, A28).

## 2. Requirements

**The rule**

R1. Every mutant of the first mutation-gated release's merge diff that no test kills is recorded
    equivalent. Each missed Rust mutant, and each survived or uncovered Mini App mutant, is either:
    - KILLED by a test that asserts the behaviour the mutant breaks: red first where the behaviour
      is new, and where it already exists, proved by the mutant itself, which the delivery's
      closing sweep reports caught; or
    - recorded EQUIVALENT (R4 to R7).
    Nothing caps the survivors a run may hold, no mutant, function or file is skipped, and no
    uncovered Mini App mutant is recorded: it first gains a test that reaches it.
R2. A killing test lives in the mutated crate's own tests (`crates/<crate>/tests/`, or a test module
    in its `src`), because cargo-mutants tests a mutant with the tests of its own package only, its
    default, which DeckStreak keeps. A Mini App killer is a Vitest test in `web/app/src`.
R3. A delivery changes its crate's production code only to fix a defect that a killing test exposes,
    red first and named in its pull request. Code that no test can reach is recorded equivalent,
    with that as its evidence; removing such code is a change of its own.

**The record (ADR-070)**

R4. An equivalent mutant is recorded in `scripts/mutation-equivalent.d/<package>.json`: one
    fragment per Cargo package, named for it (`deck-streak-vault.json`), and `miniapp.json` for the
    Mini App, each `{"records": [...]}`. A delivery writes only its own crate's fragment.
    `scripts/mutation-verdict.py` is the one reader.
R5. A record holds:
    - `file`: the mutated file, from the repository's root;
    - `mutant`: the tool's own description of the mutation without its location, which is
      cargo-mutants' name after `<file>:<line>:<column>: ` (`replace + with * in civil_from_days`)
      or StrykerJS's `<mutatorName>: <replacement>`;
    - `anchor`: a text that occurs exactly once in `file`;
    - `span`: the mutated text, present only when two mutants of that description start inside the
      anchor at one position;
    - `reason`: one line, why no test can tell the mutant apart;
    - `evidence`: the code fact that makes it so, stated where a reviewer can check it;
    - `reached_by`, in a Rust record: a test of the mutant's own package that runs the mutated code,
      named as a row's killer is (`<target>::<test path>`);
    - `issue`: `#N`, the issue its delivery closes.
R6. A record binds the listed mutant of its `file` and `mutant` whose span starts inside the
    anchor's one occurrence and, when `span` is present, whose mutated text equals it. Lines and
    columns are 1-based, as both tools report them.
R7. The census runs in the gate's `python` stage and needs no mutation tool. It refuses, by name, a
    record missing a field, a `reason` of more than one line, `evidence` that is empty or repeats
    the `reason`, an `issue` that is not `#N`, an anchor that occurs other than once in its file, a
    `file` outside its fragment's package (for `miniapp.json`, outside SPEC-039 R2's web production
    code), a Rust record whose `reached_by` does not resolve to exactly one test of that package, a
    fragment named for no package, and a record held twice. It prints `examined N`. A tree may hold
    no record.

**The verdict (ADR-070)**

R8. Whenever `mutation-plan` lists the diff's mutants, it also lists the whole tree's
    (`cargo mutants --list --json`, which builds nothing), and `mutation-verdict` binds every Rust
    record against that listing: a record that binds no listed mutant is STALE, and one that binds
    two is AMBIGUOUS. Each fails by name.
R9. In the Rust class, `judge` counts a missed mutant that exactly one record binds as
    `equivalent`, apart from `unexplained`, and names it; a missed mutant that no record binds is
    unexplained and fails by name, as today. A record whose mutant the run reports caught or timed
    out is REFUTED, and one whose mutant is unviable is UNNEEDED. Each fails by name.
R10. In the web class, `judge` counts a survived mutant that exactly one `miniapp.json` record binds
    as `equivalent`. A record whose mutant is killed or timed out is REFUTED, uncovered is
    UNCOVERED, and a compile or runtime error is UNNEEDED; a record of a file the run mutated that
    binds no mutant there is STALE; and any `Ignored` mutant fails. Each fails by name.
R11. No exclusion hides a mutant. `.cargo/mutants.toml` holds no `exclude_re`, `exclude_globs`,
    `examine_re`, `examine_globs` or `skip_calls` key; no file under `crates/*/src` holds a
    `mutants::skip` or `mutants::exclude_re` attribute; no file under `web/app/src` holds a
    `Stryker disable` comment; and `web/app/stryker.config.json` excludes no mutator.
    `mutation-verdict.py exclusions` refuses each by name.
R12. The weekly battery binds every record of both classes against its reports and the whole tree's
    listing (R8 to R10), and its `survivors` job drafts no issue for an equivalent mutant.

**The table**

R13. `mutation-verdict.py table --reports DIR [--package P] [--listed FILE]` reads a battery's
    reports and the record, and prints one line per package, `miniapp` for the Mini App:
    `table: <package>: listed N, killed K, equivalent E, unexplained U, unviable V`.
    - Killed is caught or timed out (Stryker: killed or timed out); unexplained is missed with no
      record (Stryker: survived with no record, or uncovered); and N is K + E + U + V.
    - It exits 1 on an unexplained mutant or on a record that fails (R8 to R10), and 3 (VOID) on a
      report that is missing or partial.
    - With `--listed`, a plan's listing, each listed mutant that no report tested is VOID by name.
R14. `mutation-weekly.yml`'s `workflow_dispatch` takes a `package` input: a package of the
    workspace, or `miniapp`. Its `rust` shards then sweep only that package's mutants
    (`--package`), or none for `miniapp`; `battery` counts only the reports that scope promises (a
    shard the scope gave no mutant owes none, as SPEC-039 A38 holds for a pull request's shards);
    and the `survivors` job ends with `table` over that scope. Without the input the battery sweeps
    everything, as it does today.

**The deliveries**

R15. One delivery per crate, in the owner's order: `deck-streak-vault` first, then
    `deck-streak-ingest`, `deck-streak-kernel`, `deck-streak-identity`, `deck-streak-daemon`,
    `deck-streak-coordination` and `deck-streak-api`; then `deck-streak-agent` and
    `deck-streak-progression`, which gained their mutants after this plan's base (section 9); then
    `deck-streak-privacy`, which no sweep has measured; then the Mini App, which closes #240. Each
    delivery owns one row of section 7, its crate's fragment and its ids in the band (R20). A mutant
    of its crate that the population gains while the delivery runs is its own.
R16. Each delivery, before its first kill, re-lists its crate at its base
    (`cargo mutants --list --json --package <crate>`) and dispatches the battery scoped to it
    (R14); the vault's dispatches once R14 exists on its branch. That run's `table` line becomes
    its opening row, committed with its row's test, which then reads red; a crate whose opening
    sweep already reads unexplained 0 records its criterion `not red`, naming that run. It kills or
    records every unexplained mutant of its crate, dispatches the scoped battery again at its head,
    and fills its row from that run: listed, killed, equivalent, unexplained 0, unviable, its pull
    request and both runs. It changes its own row of section 7 and nothing else of this SPEC,
    unless its measurements prove the plan wrong, which it records in section 9 with the reason.
R17. The first delivery, the vault's, builds R4 to R14 before its first kill, each with its test red
    first. It also:
    - sets ADR-070 `accepted`;
    - retires SPEC-039's A20 insert-only (ADR-069's form), because A20 plants a justified
      `exclude_re` and a justified `Stryker disable` that R11 now refuses, and appends a dated
      amendment to SPEC-039 naming ADR-070;
    - moves `docs/BUILDER-BRIEF.md`'s mutation section, the comments in `.cargo/mutants.toml` and
      `web/app/stryker.config.json`, and the survivors' draft text from the old form to the record.
R18. The vault's delivery carries two riders, each with its own criterion:
    - a push whose subject's first line ends with a squash merge's ` (#N)` reads `not-applicable`,
      naming `#N`, as `Merge pull request #N from` does. When the title itself ends with an issue's
      `(#M)`, the last number names the pull request. A subject that names no pull request is still
      judged on its first-parent diff;
    - the gate's `audit-rust` stage runs an unlocked resolve (`cargo metadata --format-version 1`,
      without `--locked`) and then `git diff --exit-code -- Cargo.lock`, and fails by name when the
      resolve rewrote the lock.
R19. The last delivery, the Mini App's, judges the whole release at its head before it moves this
    SPEC:
    - it dispatches the battery with no scope;
    - on a local synthetic merge of its head into `main`, never pushed, it runs SPEC-039's
      `plan --event pull_request --base-ref main`, cargo-mutants' `--list --json --in-diff` of that
      merge diff, and `table --listed` over that listing and the dispatch's reports;
    - every package reads unexplained 0, and every listed mutant was tested.
    It refills every row from that run, keeping each row's pull request; adds a row for any package
    the listing holds and the table lacks; records the rehearsal in section 8; and moves this SPEC
    to `docs/specs/` with `docs/red-first/SPEC-057.md` complete (ADR-016).
R20. A row that pins an invariant the tool cannot mutate (SPEC-039 R8) goes in
    `scripts/mutation-rows.d/S05700-S05799.json`, the one band this SPEC's deliveries share, with
    ids allotted so that no two deliveries collide: vault S05701 to S05719, ingest S05720 to S05729,
    kernel S05730 to S05739, identity S05740 to S05749, daemon S05750 to S05759, coordination
    S05760 to S05764, agent S05765 to S05769, api S05770 to S05774, progression S05775 to S05779,
    privacy S05780 to S05789, and the Mini App S05790 to S05799.
R21. The first release into `main` that mutation testing judges waits until every row of section 7
    reads unexplained 0 (the owner's ruling), so that its `mutation-verdict` and `mutation-web` jobs
    find no unexplained mutant in its merge diff.

**The plan's test-only lines (the third rider, section 1.6)**

R22. In the Rust class, a changed line is **test-only** when every token of code on it lies inside
    an item that cargo-mutants 27.1.0 never mutates for an attribute: a `fn`, `mod`, `impl` or
    `trait` whose outer attributes hold `#[cfg(test)]`, or an attribute whose path ends in `test`
    (`#[test]`, `#[tokio::test]`), from its first attribute to the `}` or `;` that ends it.
    SPEC-039's `plan` finds those items with the lexer that reads SPEC-039 R4's literals, so a
    brace, `#[cfg(test)]` or `#[test]` inside a string, raw string, character literal or comment
    opens and closes nothing, and it records each file's test-only lines apart from its code
    lines and its blank or comment lines.
    - A diff whose changed Rust code lines are all test-only reads the class `not-applicable` by
      name, naming its test-only lines, and never VOID: it lists no mutant and plans no Rust work.
    - A diff that also changes a production code line applies as before, and cargo-mutants' own
      listing names that line's mutants: the tool lists none inside a test item.
    - `shards` reads the listing step's empty output as an empty listing, because cargo-mutants
      exits 0 before it prints when no mutant overlaps the diff; a missing listing stays VOID. So a
      production line that no tool can mutate, a constant with a literal value, still applies and
      reads VOID without a covering row (SPEC-039 R8): an empty listing never makes a class
      not-applicable. (cargo-mutants does mutate an operator in a constant's initializer.)
    - Every other shape stays production code: a `cfg` that joins `test` to another predicate
      (`not(test)`, `any(test, ...)`), which the tool mutates; an inner `#![cfg(test)]`; a
      `#[cfg(test)]` statement or expression; a `#[cfg(test)]` item of another kind; and a module
      file that `#[cfg(test)] mod name;` declares. Each errs toward applying, which reads VOID
      without a row, never toward passing.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every record carries its file, mutant, anchor, reason, evidence and issue, and a Rust record's `reached_by` resolves to exactly one test of its own package; the census refuses a planted record lacking each, and reports its examined count | `test_mutation_equivalent.py` |
| A2 | the census refuses a record whose anchor occurs other than once, whose file lies outside its fragment's package, whose reason spans two lines, or that is held twice | `test_mutation_equivalent.py` |
| A3 | a missed mutant that exactly one record binds is counted equivalent and named, and a missed mutant no record binds fails as unexplained, by name | `test_mutation_verdict.py` |
| A4 | a record that binds no mutant of the whole tree's listing is STALE, and one that binds two is AMBIGUOUS, each failing by name | `test_mutation_verdict.py` |
| A5 | a record whose mutant was caught or timed out is REFUTED, and one whose mutant was unviable is UNNEEDED, each failing by name | `test_mutation_verdict.py` |
| A6 | a record still binds its mutant when lines are added above it, and binds nothing once its anchored text changes | `test_mutation_verdict.py` |
| A7 | in the Mini App a record excuses a survived mutant only: an uncovered one fails as UNCOVERED, a killed one as REFUTED, and any ignored mutant fails | `test_mutation_verdict.py` |
| A8 | a planted `exclude_re`, `exclude_globs`, `examine_re`, `examine_globs` or `skip_calls` key, `mutants::skip` or `mutants::exclude_re` attribute, `Stryker disable` comment and excluded Stryker mutator are each refused by name, and the tree holds none | `test_mutation_workflows.py` |
| A9 | the table prints one line per package in section 7's columns, whose listed equals killed, equivalent, unexplained and unviable summed, and fails on an unexplained mutant or a failed record | `test_mutation_verdict.py` |
| A10 | given a plan's listing, the table names each listed mutant that no report tested, VOID | `test_mutation_verdict.py` |
| A11 | a dispatch scoped to one package sweeps only its mutants, owes a report only from a shard its scope gave a mutant, and ends with that package's table line | `test_mutation_workflows.py` |
| A12 | whenever the plan lists the diff's mutants it lists the whole tree's too, and the verdict's job binds every record against that listing | `test_mutation_workflows.py` |
| A13 | a push whose subject ends with a squash merge's `(#N)` reads not-applicable naming that pull request, the last number when the title carries another | `test_mutation_verdict.py` |
| A14 | a lock that an unlocked resolve rewrites fails the `audit-rust` stage by name, and a canonical lock passes | `test_check_gate.py` |
| A15 | the builder brief teaches the equivalence record, and names no exclusion as a way to record an equivalent | `test_mutation_workflows.py` |
| A16 | the vault's row reads unexplained 0 from its closing sweep, its counts sum to its listed, its equivalent equals its fragment's records, and it names its pull request and both runs | `test_mutation_campaign.py` |
| A17 | the same, for the ingest's row | `test_mutation_campaign.py` |
| A18 | the same, for the kernel's row | `test_mutation_campaign.py` |
| A19 | the same, for the identity's row | `test_mutation_campaign.py` |
| A20 | the same, for the daemon's row | `test_mutation_campaign.py` |
| A21 | the same, for the coordination's row | `test_mutation_campaign.py` |
| A22 | the same, for the api's row | `test_mutation_campaign.py` |
| A23 | the same, for the privacy's row | `test_mutation_campaign.py` |
| A24 | the same, for the Mini App's row, whose sweep is the whole app (#240) | `test_mutation_campaign.py` |
| A25 | every row reads unexplained 0 from one unscoped dispatch at the last delivery's head, and section 8 records the release rehearsal: that head, the merge's base, the run, and a listing whose every mutant was tested | `test_mutation_campaign.py` |
| A26 | the same as A16, for the agent's row | `test_mutation_campaign.py` |
| A27 | the same as A16, for the progression's row | `test_mutation_campaign.py` |
| A28 | four planted fixtures, each red against the plan before R22 for its own reason: a test-only diff reads the Rust class not-applicable by name and never VOID; a mixed diff applies on its production line alone, and cargo-mutants' own listing of it names only that line's mutants; a production-only diff applies and the plan names its production lines, a brace, `#[cfg(test)]` and `#[test]` inside literals and comments, a `#[cfg(not(test))]` function and a constant on the line of the brace that closes a test module all read as production; and cargo-mutants' empty `--in-diff` output is an empty listing, whose constant-only diff still reads VOID without a row | `test_mutation_verdict.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_mutation_equivalent.py -k every_record_carries_its_mutant_anchor_reason_evidence_and_issue
A2: python3 -m unittest discover -s scripts/tests -p test_mutation_equivalent.py -k a_record_the_census_cannot_bind_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_recorded_missed_mutant_is_equivalent_and_an_unrecorded_one_fails
A4: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_record_binding_no_listed_mutant_is_stale_and_two_is_ambiguous
A5: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_record_whose_mutant_was_caught_is_refuted
A6: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_record_follows_its_mutant_when_lines_move_above_it
A7: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_web_record_excuses_a_survivor_and_never_an_uncovered_mutant
A8: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k no_exclusion_hides_a_mutant_from_the_listing
A9: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k the_table_counts_each_package_in_the_campaigns_columns
A10: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_listed_mutant_no_report_tested_is_void_by_name
A11: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k a_dispatch_scoped_to_one_package_sweeps_only_its_mutants
A12: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_verdict_binds_every_record_against_the_whole_listing
A13: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_squash_merge_subject_names_the_pull_request_it_merges
A14: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k a_lock_an_unlocked_resolve_rewrites_fails_the_audit_stage
A15: python3 -m unittest discover -s scripts/tests -p test_mutation_workflows.py -k the_builder_brief_teaches_the_equivalence_record
A16: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_vault_row_reads_no_unexplained_mutant
A17: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_ingest_row_reads_no_unexplained_mutant
A18: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_kernel_row_reads_no_unexplained_mutant
A19: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_identity_row_reads_no_unexplained_mutant
A20: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_daemon_row_reads_no_unexplained_mutant
A21: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_coordination_row_reads_no_unexplained_mutant
A22: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_api_row_reads_no_unexplained_mutant
A23: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_privacy_row_reads_no_unexplained_mutant
A24: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_miniapp_row_reads_no_unexplained_mutant
A25: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k every_row_and_the_release_rehearsal_read_no_unexplained_mutant
A26: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_agent_row_reads_no_unexplained_mutant
A27: python3 -m unittest discover -s scripts/tests -p test_mutation_campaign.py -k the_progression_row_reads_no_unexplained_mutant
A28: python3 -m unittest discover -s scripts/tests -p test_mutation_verdict.py -k a_test_only_src_diff_reads_not_applicable_and_a_production_line_still_applies
```

- **A1 to A15** are the vault's machinery and riders. Each is committed red against a stub that
  keeps its entry point and does nothing (the census examines nothing, the verdict binds no record,
  `table` prints no line, the subject rule names no squash, the audit stage runs no resolve), so
  each fails by assertion for its own reason, as SPEC-039 section 3 did. A1, A2 and A9 plant
  fixtures in a temporary directory; A3 to A7, A10 and A13 run the verdict over synthetic listings
  and reports in cargo-mutants' and StrykerJS's own shapes; A8, A11, A12 and A15 read the tree and
  the workflows; A14 runs the stage against a planted lock.
- **A16 to A24, A26 and A27** read section 7 of this SPEC, wherever it then lives
  (`docs/specs/planned/` or `docs/specs/`), and the row's fragment. Each is written by its row's
  delivery. It is red at the commit that carries the row's opening sweep, whose unexplained count
  is above 0, and green at the commit that carries its closing sweep; a row whose opening sweep
  already reads 0 is disclosed `not red`, naming that run (R16). Its evidence is the pair of
  battery dispatches its red-first lines name, and a verifier reads those runs.
- **A25** is the last delivery's. Its evidence is the rehearsal it records in section 8 and the
  dispatch that rehearsal names.
- **A28** is the vault's third rider (R22). Its one test plants four fixtures, each in a subtest of
  its own, and each committed red against the plan as it stood at dd734e5, failing by assertion
  for its own reason. The listings it plants are cargo-mutants 27.1.0's own `--list --json
  --in-diff` output over the fixtures' diffs, which builds nothing: nothing at all for the
  test-only and the constant-only diffs. The production-only fixture's line 54, a constant on the
  line of the brace that closes the test module, came in the delivery's fix round with row S05709.
  cargo-mutants' listing of that diff is unchanged by it, since the constant lists no mutant, and
  the plan at dd734e5 read every line as production, so the line adds no red of its own; the row
  proves that the test fails when the plan reads a line that holds a test item's closing brace and
  production code as test-only.

## 4. File manifest

| file | delivery | change |
|---|---|---|
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | this plan | added; each delivery fills its own row of section 7 (R16), and the last moves it to `docs/specs/` (R19) |
| `docs/decisions/ADR-070-the-equivalence-record-excuses-exactly-a-recorded-mutant.md` | this plan | added, `proposed`; the vault's delivery sets it `accepted` (R17) and appends a dated note on its third rider (R22) |
| `docs/decisions/ADR-057-mutation-testing-runs-on-the-diff-in-ci-and-weekly-on-dev.md` | this plan | changed: a note appended that points at ADR-070 |
| `docs/schematics/mutation-equivalence-record.md` | this plan | added: the record's path to the verdict, one record across runs, the deliveries and the rehearsal |
| `changelog.d/docs-mutant-campaign-057.md` | this plan | added |
| `scripts/mutation-verdict.py` | vault | changed: the reader, the census, the binding and its failures (R4 to R10), the exclusions (R11), the battery's records and drafts (R12), `table` (R13), the squash subject (R18), and the plan's test-only lines and the listing's empty output (R22) |
| `scripts/tests/test_mutation_equivalent.py` | vault | added: A1, A2 |
| `scripts/tests/test_mutation_verdict.py` | vault | changed: A3 to A7, A9, A10, A13, A28 |
| `scripts/tests/test_mutation_workflows.py` | vault | changed: A8, A11, A12, A15; SPEC-039's A20 retired |
| `scripts/tests/test_check_gate.py` | vault | changed: A14 |
| `scripts/tests/test_mutation_campaign.py` | vault, then each delivery | added by the vault's delivery, with the reader of section 7 and A16; each later delivery adds its own row's test (A17 to A25) |
| `scripts/check.sh` | vault | changed: the unlocked resolve in `audit-rust` (R18) |
| `.github/workflows/ci.yml` | vault | changed: the whole tree's listing beside the diff's (R8) |
| `.github/workflows/mutation-weekly.yml` | vault | changed: the `package` input, the scoped count and `table` (R12, R14) |
| `.cargo/mutants.toml` | vault | changed: its comment names the record; it holds no exclusion key (R11, R17) |
| `web/app/stryker.config.json` | vault | changed: its `_comment` names the record (R17) |
| `docs/BUILDER-BRIEF.md` | vault | changed: the mutation section teaches the record (R17, A15) |
| `docs/schematics/mutation-testing.md` | vault | changed: the verdict's step binds the record, and points at `docs/schematics/mutation-equivalence-record.md`; the plan's test-only arm and the listing's empty output (R22) |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | vault | changed insert-only: A20 struck and set apart in a `retired` fence, and a dated amendment naming ADR-070 (R17); a second dated amendment, its section 11, for the plan's test-only lines and the listing's empty output (R22) |
| `docs/red-first/SPEC-039.md` | vault | changed insert-only: A20's lines set apart in a `retired` fence |
| `docs/specs/SPEC-056-every-pack-is-judged-on-the-box-and-nothing-of-the-hub-is-published.md` | vault | changed insert-only: section 7 lists SPEC-039's A20, since SPEC-056 A18 holds that table equal to every retirement the delivered SPECs hold (section 9) |
| `crates/vault/tests/`, `crates/vault/src/` | vault | the killing tests (R2); the source only by a red-first fix (R3) |
| `crates/ingest/tests/`, `crates/ingest/src/` | ingest | the same |
| `crates/kernel/tests/`, `crates/kernel/src/` | kernel | the same |
| `crates/identity/tests/`, `crates/identity/src/` | identity | the same |
| `crates/daemon/tests/`, `crates/daemon/src/` | daemon | the same |
| `crates/coordination/tests/`, `crates/coordination/src/` | coordination | the same |
| `crates/api/tests/`, `crates/api/src/` | api | the same |
| `crates/agent/tests/`, `crates/agent/src/` | agent | the same |
| `crates/progression/tests/`, `crates/progression/src/` | progression | the same |
| `crates/privacy/tests/`, `crates/privacy/src/` | privacy | the same |
| `web/app/src/` | the Mini App | its killing and covering Vitest tests, `*.test.ts` beside the code (R1, R2); production files only by a red-first fix (R3) |
| `scripts/mutation-equivalent.d/deck-streak-vault.json` and one fragment per package, `miniapp.json` for the Mini App | each delivery | added when the delivery records its first equivalent (R4) |
| `scripts/mutation-rows.d/S05700-S05799.json` | each delivery | added by the first delivery that pins a row; each writes only its allotted ids (R20) |
| `docs/red-first/SPEC-057.md` | each delivery | added by the vault's delivery; each appends its own lines |
| `changelog.d/` | each delivery | added: one fragment, named for its branch |
| `docs/specs/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | the Mini App | moved from `docs/specs/planned/`, with every row refilled from one run (R19) |

## 5. What this does NOT do

- It generates no mutants for the gate's own Python in `scripts/`; those guards keep their tests and
  their rows (#218).
- It generates no mutants for the parity oracle's Python, whose invariants stay hand-proved rows
  (#219).
- It keeps no map of which ordinary test kills which mutant, so a later pull request that deletes
  one of this campaign's killing tests is refused only when a row names that test; otherwise the
  weekly battery finds the mutant alive again and files it (#220).
- It adds no row for the two bounds no test pins on its own, `Hour::new`'s upper bound and
  identity's future skew (#222).
- It makes no `ingest` mutant cheaper: each costs about two minutes on GitHub's runners while Anki's
  engine rebuilds on every cargo command (#228, #233).

## 6. Risks

- **An equivalence claim is wrong.** Detected three ways. Each record carries its `evidence` and,
  for Rust, a `reached_by` test the census resolves (R5, R7), which the verifier reads; a mutant no
  test reaches is untested, not equivalent, and an uncovered Mini App mutant is never excused
  (R10). The mutant keeps running, so the first test that kills it turns the record REFUTED (R9).
  And a claim whose anchored code changes goes STALE in the gate's census (R7).
- **The population moves under the campaign as W1 lands.** Every delivery re-lists its crate at its
  base and sweeps it at its head (R16), so a mutant the crate gains meanwhile is its own. The last
  delivery judges the whole release at one head (R19), and adds a row for a package the table
  lacks. A W1 pull request is judged on its own diff by the pull-request mutation jobs, so a new
  crate arrives with its survivors already killed or recorded.
- **A hosted runner is shut down mid-shard** (SPEC-039 section 8), as shard 21 of run 36384080819
  was. The battery names such a shard MISSING, and `table` reads VOID rather than a lower count
  (R13); "Re-run failed jobs" runs that shard and the count again. A row is filled only from a run
  whose every promised report is whole.
- **The release's shards sit close to their bound.** At 16ed8e2 the slowest of 27 shards is
  projected at 3,568 s of its 3,600 s bound, from SPEC-039 R18's costs, which name no cost for
  `deck-streak-privacy` and charge it the highest. Tests the campaign adds make each mutant slower,
  so the plan may take a 28th shard, as R18 intends; a shard that still outruns its timeout is VOID
  by name, and the costs are measured again from the weekly battery's reports.
- **A flaky test reads a recorded mutant as caught.** The record then fails as REFUTED, loudly, and
  the run is repeated; a mutant that is killed only sometimes is a flaky test to fix, not a record.
- **A pinned tool renames its mutants.** A cargo-mutants or StrykerJS upgrade that changes the
  descriptions turns every record STALE at once; the records are bound again in the same change as
  the pin (ADR-070).
- **A killing test is later deleted.** Nothing in a pull request names which ordinary test killed a
  mutant (#220); the weekly battery finds the mutant alive again.
- **Deliveries in flight together share one band file.** `scripts/mutation_rows.py` refuses a
  fragment that is not a whole SPEC's band, so every delivery that pins a row writes
  `scripts/mutation-rows.d/S05700-S05799.json`, and two open at once conflict in that file even with
  disjoint ids (R20), the collision ADR-070 D1 cites for one shared file. A delivery merges `dev`
  before its push, and a conflict resolves by keeping both sides' rows, which
  `mutation_rows.py prove` then checks again.
- **A test item of a shape the plan does not read.** R22 reads four item kinds under two
  attribute forms, the ones cargo-mutants' own visitor skips. A change inside any other shape,
  such as a module file that `#[cfg(test)] mod name;` declares, applies, lists no mutant and reads
  VOID without a row. That is loud, never a pass, and the reader is widened in the change that
  first meets the shape, with a fixture of its own.

## 7. The campaign plan and its table

One delivery per crate, in the owner's order (R15). The first delivery builds the record, the
verdict's reading, the table and the battery's scope, and carries the two riders (R17, R18); the
last, the Mini App's, judges the whole release and moves this SPEC (R19).

The opening rows are run 36384080819's whole shards, in which listed counts what they reported, so
shard 21's 67 mutants, 6 of them known survivors and 19 never reached (section 1.1), are in no row;
`deck-streak-privacy`, which no sweep has measured, shows its listing at `dev` 16ed8e2, and
`deck-streak-agent` and `deck-streak-progression`, which listed no mutant there, show theirs at
`dev` 53184dd (section 9). Each
delivery replaces its row with its own sweeps (R16): listed, killed (caught or timed out),
equivalent, unexplained, which must read 0, unviable, and its pull request with its opening and
closing runs.

| order | crate | listed | killed | equivalent | unexplained | unviable | PR |
|---|---|---|---|---|---|---|---|
| 1 | `deck-streak-vault` | 943 | 861 | 15 | 0 | 67 | #277 (runs 36438243392, 36483219611) |
| 2 | `deck-streak-ingest` | 309 | 239 | 3 | 0 | 67 | #313 (runs 36502008965, 36505515113) |
| 3 | `deck-streak-kernel` | 779 | 714 | 2 | 0 | 63 | #546 (runs 36965665452, 36986911724) |
| 4 | `deck-streak-identity` | 338 | 244 | 5 | 0 | 89 | #754 (runs 37952349493, 37955658270) |
| 5 | `deck-streak-daemon` | 101 | 71 | 1 | 0 | 29 | #318 (runs 36515002230, 36517001675) |
| 6 | `deck-streak-coordination` | 409 | 324 | 2 | 0 | 83 | #329 (runs 36526822999, 36528662545) |
| 7 | `deck-streak-api` | 70 | 50 | 0 | 0 | 20 | #328 (runs 36528558184, 36529229929) |
| 8 | `deck-streak-agent` | 239 | 174 | 0 | 0 | 65 | #338 (runs 36543074674, 36545641689) |
| 9 | `deck-streak-progression` | 57 | 39 | 0 | 0 | 18 | #333 (runs 36533129255, 36534197772) |
| 10 | `deck-streak-privacy` | 29 | 26 | 0 | 0 | 3 | #332 (runs 36533127814, 36533533659) |
| 11 | `miniapp` | 7193 | 7165 | 26 | 0 | 2 | #785 (runs 38057671668, 38057667627) |

A delivered row's PR cell reads `#<pull request> (runs <opening>, <closing>)`.

2026-09-29 note: the `deck-streak-vault` row's counts and its runs are #277's. Pull request #412 (SPEC-110, the law
drills) has since added vault mutation rows in `S11000-S11099` and edited `drills.rs` and `drill_notes.rs`, so
the row is not re-measured for it: its counts read the vault at #277.

What each delivery meets first, from section 1:

- **vault**: 218 missed, 145 of them in `rails.rs`, the rails that block executable content in the
  vault (SPEC-042); then `staged.rs` 29, `readings_tree.rs` 15, `note.rs` 12, `config.rs` 9, and
  `fs.rs` and `sha256.rs` 4 each; 29 in shard 21, 4 of them missed and 19 never reached, and 8 new
  at 16ed8e2. Its mutants cost about 8 s each on GitHub's runners (SPEC-039 R18).
- **ingest**: 43 missed, 17 in `sync.rs` and 11 in `engine.rs`; each mutant costs about two minutes,
  so its 297 mutants are the campaign's costliest sweep, projected at 37,422 s serially.
- **kernel**: 17 missed across seven files, 5 in `redact.rs`.
- **identity**: 15 missed, 8 in `session.rs`.
- **daemon**: 9 missed, 8 in `lifecycle.rs`; 24 mutants new since the run.
- **coordination**: 5 missed in four files.
- **api**: 3 missed, one each in `health.rs`, `session_routes.rs` and `settings.rs`.
- **agent**: 146 mutants, never swept.
- **progression**: 57 mutants, never swept.
- **privacy**: 29 mutants, never swept.
- **the Mini App**: #240 tracks its 53 survived and 28 uncovered mutants, each to be killed or
  recorded equivalent, with no mutant skipped. By this SPEC's rule (R1, R10), an uncovered mutant
  first gains a test that reaches it in the Vitest run that StrykerJS measures, stubbing the
  browser where it must, since the Playwright suite under `web/app/tests/` is not part of that run:
  `hooks.server.ts`, `hooks.ts`, `+layout.ts` and `+layout.svelte`, which no Vitest test reaches,
  each gain one that imports and runs them, as the route tests beside them already do.

## 8. The release rehearsal

At this plan's base, `dev` 16ed8e2, the rehearsal of section 1.2 lists the release: 2,207 mutants of
its merge diff, the whole tree's, in 27 shards projected at 82,934 s serially, with the Mini App's
8 changed files and their 266 mutants for `mutation-web`. No sweep has yet judged that population
whole. The last delivery records its own rehearsal here (R19): its head, the merge's base, the
dispatch's run, the listing per package, and `table --listed`'s lines.

## 9. Amendments

A delivery whose measurements prove this plan wrong records it here, dated, with the reason
(R16).

- **2026-09-28, the vault's delivery.** Section 1.3 read the tree as holding no exclusion, since
  `mutation-verdict.py exclusions` then examined no entry, but `.cargo/mutants.toml` held an empty
  `exclude_re = []` key, which R11 refuses as a key: the delivery removed it, and nothing else
  needed migrating. The manifest lacked SPEC-056, whose A18 holds its section 7 equal to every
  criterion the delivered SPECs retire, so R17's retirement of SPEC-039's A20 adds a row there,
  insert-only with a dated amendment; section 4 now names it.
- **2026-09-28, the vault's delivery: two crates that no delivery owned.** At the vault's base,
  `dev` 53184dd, cargo-mutants lists 2,410 mutants where this plan's base, `dev` 16ed8e2, listed
  2,207: `deck-streak-agent` gained 146 and `deck-streak-progression` 57, two crates that listed
  none at 16ed8e2, so neither R15 nor section 7 named them, no delivery owned their survivors, and
  the band's ids were fully allotted. The architect placed them after the api: they are section
  7's rows 8 and 9, `deck-streak-privacy` becomes row 10, and the Mini App stays last at row 11,
  because the last delivery judges the whole release (R19). Their ids come from the upper halves
  of two allotments: the agent takes S05765 to S05769 and the coordination keeps S05760 to S05764;
  the progression takes S05775 to S05779 and the api keeps S05770 to S05774. R15, R20, section 7,
  the context list and section 4's manifest are edited in place. The two rows read their listing
  at 53184dd, equivalent 0 since no record exists, and `unmeasured` for what only a sweep counts,
  until their deliveries sweep them.
- **2026-09-28, the vault's delivery: the two crates' criteria.** Rows 8 and 9, added by the
  amendment above, had no criterion of their own, so no red-first line could close them. The
  architect gave them A26 (`deck-streak-agent`) and A27 (`deck-streak-progression`), in the form
  A17 to A24 give the other crates' rows: each reads its row and its crate's fragment, is written by
  its row's delivery, and has no red-first line until that delivery's opening sweep. Section 3's
  table, its `acceptance` fence and its note on A16 to A24 name them; A25 already reads every row.
- **2026-09-28, the vault's delivery: a third rider (R22, A28).** Section 1.6 records the
  measurement: at dd734e5 the plan read 71 changed lines inside three `#[cfg(test)]` modules as
  production code, and the pull request's `mutation-plan` and `mutation-verdict` jobs read VOID on
  a diff that changes no production line. SPEC-039 R2 and R4 count every changed code line of a
  `crates/*/src` file as production code, and nothing in this plan changed that. The architect
  ruled the plan wrong rather than the delivery, and chose to fix the plan in this delivery
  rather than move the three test modules into `crates/vault/tests/` or make the private items
  they test public. ADR-070's note of this date records the decision and what it was chosen
  against, and SPEC-039 section 11 records the amendment to that SPEC's plan. Section 1.6, R22,
  A28, section 3's table, fence and notes, section 4's manifest, section 6's risks and section
  10's references are edited in place.

## 10. References

- SPEC-039 (sections 1, 3, 8 and 11; R2, R3, R4, R5, R8, R12, R18), ADR-057, ADR-070, ADR-069,
  ADR-016, ADR-034; #221, #240, #245, #246.
- cargo-mutants: filtering by name (https://mutants.rs/filter_mutants.html), skipping code
  (https://mutants.rs/skip.html, https://mutants.rs/attrs.html), the tests it runs for a mutant
  in a workspace (https://mutants.rs/workspaces.html), the functions it never mutates
  (https://mutants.rs/mutants.html) and testing a diff (https://mutants.rs/in-diff.html); for
  R22, 27.1.0's own `src/visit.rs` (`attrs_excluded`), `src/in_diff.rs` and `src/main.rs`.
- StrykerJS: disabling mutants
  (https://github.com/stryker-mutator/stryker-js/blob/master/docs/disable-mutants.md) and its
  per-test coverage, which tells `Survived` from `NoCoverage`
  (https://github.com/stryker-mutator/stryker-js/blob/master/docs/configuration.md).

## 11. Amendment, 2026-09-29: a package dispatch is sharded by its projected weight

Made by issue #368's delivery, insert-only under ruling (i) of SPEC-038 section 8: every earlier
byte is kept in order. It inserts this section only.

- **R14's dispatch no longer fans a package out to a fixed 32 legs.** A dispatch naming a package
  sizes its legs from that package's listing with R18's projection, and the battery reads the same
  count; a scheduled run and a dispatch with no package keep 32. SPEC-129 decides it and ADR-129
  records it.
