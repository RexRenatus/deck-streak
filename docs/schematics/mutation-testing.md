# Schematic: mutation testing, from a pull request's diff to a verdict, and the weekly battery

Kind: data flow (the pull request's mutation jobs, the weekly battery) and a state machine (one
row's proof). Read at DeckStreak `dev` 32bf1e1 (`.github/workflows/ci.yml`, `scripts/check.sh`,
the mutation-rows pack's row shape), with cargo-mutants 27.1.0 and StrykerJS 10.0.0 as SPEC-039 §1
measured them, and its shards as R18 sizes them. Decided by ADR-057; built by SPEC-039. The
equivalence record (ADR-070, SPEC-057) joins the verdict's step and the battery; its own path, from
a fragment to a verdict, is drawn in `docs/schematics/mutation-equivalence-record.md`.

## 1. A pull request's diff, through the mutation jobs

```mermaid
flowchart TD
  pr[a pull request into dev, or a release pull request into main] --> merge[the merge ref: HEAD^1 is the base's tip]
  push[a push to dev or main] --> names{does its subject name the pull request it merges?}
  names -->|"Merge pull request #N, or a squash's title ending (#N)"| na([every job: not-applicable, naming #N, whose jobs judged this tree])
  names -->|no| first[its first-parent diff, HEAD^1...HEAD]
  merge --> plan[mutation-plan: mutation-verdict.py plan: git diff HEAD^1...HEAD, once, to git.diff]
  first --> plan
  plan --> classes{each changed path's class}
  classes -->|crates/*/src/**/*.rs| rust[rust]
  classes -->|web/app/src/** less tests, paraglide, d.ts| web[web]
  classes -->|tools/parity-oracle/generate.py| oracle[oracle]
  classes -->|anything else| other([not production: counted, never mutated])
  plan --> lines{each production file's changed lines}
  lines -->|all blank or comments, or only deletions| nap([not-applicable: its count, by file])
  lines -->|Rust: every code line inside an item marked cfg-test or with a test attribute| testonly([not-applicable, test-only: its count, by file, and no tool installed])
  lines -->|a production code line| applies[the class applies]
  plan --> rows[the rows it selects: on its paths, added or changed, on a killer's file]
  rust --> list[cargo mutants --list --json --in-diff git.diff: the diff's mutants, nothing built]
  rust --> wholelist[cargo mutants --list --json: the whole tree's mutants, whole.json, nothing built]
  list --> shards[mutation-verdict.py shards: the fewest round-robin shards, each projected within an hour]
  list -->|no mutant overlaps the diff: it prints nothing, read as an empty listing| shards
  shards -->|more than 256 shards| refused([REFUSED with its projection, never capped])
  shards --> matrix[the matrix 0 to n-1, and the plan artifact every job reads]

  subgraph mutation-rust: one job per shard k of n
    matrix --> cm[cargo mutants --in-place --in-diff git.diff --sharding round-robin --shard k/n --timeout 300 --build-timeout 600]
    cm --> outcomes[(mutation-rust-shard-k: outcomes.json and the exit)]
  end

  subgraph mutation-rows
    rows --> prove[mutation_rows.py prove]
    oracle --> prove
    prove --> rowsreport[(rows.json)]
    prove --> retired[mutation_rows.py retired --base HEAD^1]
  end

  subgraph mutation-verdict: if always
    outcomes --> count[every shard from 0 to n-1: one missing or partial is VOID, by name]
    count --> partition[the reports hold every listed mutant once: in two shards fails, in none is VOID]
    partition --> judgeR[mutation-verdict.py judge --class rust, then --class oracle]
    rowsreport --> judgeR
    wholelist --> bindR[every Rust record bound against whole.json: one listed mutant each, else STALE or AMBIGUOUS]
    record[(scripts/mutation-equivalent.d)] --> bindR
    bindR --> judgeR
  end

  subgraph mutation-web
    web --> sync[paraglide-js compile; svelte-kit sync]
    sync --> st[stryker run --mutate each changed file, whole]
    st --> mjson[(reports/mutation/mutation.json)]
    mjson --> judgeW[mutation-verdict.py judge --class web]
    record --> judgeW
  end

  judgeR --> verdict{the verdict}
  judgeW --> verdict
  verdict -->|a missed or survived mutant no record excuses; an uncovered or ignored mutant; a record STALE, AMBIGUOUS, REFUTED, UNNEEDED or UNCOVERED; a mutant in two shards; a row not KILLED; a retirement without approval| fail([FAIL: named])
  verdict -->|a class that applies examined 0; a report or a shard missing or partial; a listed mutant in no shard| void([VOID: fails the job])
  verdict -->|every examined mutant caught or recorded EQUIVALENT, examined above 0| ok([ok: counts printed, equivalent apart])
  fail --> ci[ci: needs all five jobs]
  void --> ci
  ok --> ci
  na --> ci
  refused --> ci
```

Each job that writes a report uploads it under `if: always()`, restores the Rust cache or the pnpm
store, and saves nothing. `cargo mutants --in-place` and the runner both rewrite a tracked file and
restore it, so each runs in a job of its own checkout, never beside another reader of the tree.
Every job reads the plan's one `git.diff`, and the verdict counts the shards the plan promised, not
the ones that happened to report.

## 2. What crosses each step

| step | what crosses | guard |
|---|---|---|
| plan | the diff, `git diff HEAD^1...HEAD`, written to `git.diff` | the new side of the diff is the checked-out tree, so cargo-mutants never exits 5 on a mismatch |
| classes | each changed path | R2's globs, exactly; a test, a script or a document is never production |
| lines | each production file's new-side changed lines | blank and comment lines are counted apart; a file whose hunks only delete reads not-applicable with its count; in Rust, a line whose every token lies inside an item cargo-mutants never mutates for a test attribute (`#[cfg(test)]`, `#[test]`, `#[tokio::test]`) is test-only, counted apart and never a code line (SPEC-057 R22) |
| shards | cargo-mutants' own listing of the diff's mutants, `--list --json` | each shard's projected time, the baseline's 346 s and each of its mutants' package cost, within an hour; the fewest shards that fit; more than 256 refused, never capped |
| cargo-mutants | the diff, the tree, the shard `k/n` | `--in-place` on the checkout, round-robin as the plan projected, `--timeout 300` on each mutant's tests and `--build-timeout 600` on its build, the shard job's `timeout-minutes` of 120; its exit is recorded, never trusted alone |
| Stryker | every changed web production file, whole | the whole file, so a survivor already there is the pull request's (rule 5); `thresholds.break` 100 |
| rows | the selected rows | the runner's own refusals (section 3); the report lists every row with its verdict |
| retired | the rows at `HEAD^1` against the rows at `HEAD` | a row gone while its target stays needs an entry in `scripts/mutation-rows.retired.json` |
| judge | every shard's `outcomes.json`, `mutation.json`, `rows.json` | examined is caught, missed and timed out; unviable is never a kill; zero examined on an applying class is VOID; a missing report or shard is VOID, by name, and so is a partial one (an exit other than 0, 2 or 3, or counts short of `total_mutants`); the shards' reports hold each listed mutant once |

## 3. One row's proof

```mermaid
stateDiagram-v2
  [*] --> Clean: a tree with no tracked change
  [*] --> Refused: a tracked change (exit 2)
  Clean --> Anchored: the find occurs exactly once; sha256 recorded
  Clean --> Void: the find occurs 0 or 2+ times
  Anchored --> Control: run the killer, no mutant
  Control --> Void: red, or not exactly one test selected
  Control --> Installed: green, one test selected
  Installed --> Built: cargo test --no-run passes, or the Python parses
  Installed --> Void: does not build or parse (never a kill)
  Built --> Killed: the killer fails, one test selected
  Built --> Survived: the killer passes
  Built --> Void: not exactly one test selected, or it timed out
  Killed --> Restored
  Survived --> Restored
  Void --> Restored
  Restored --> [*]: the saved bytes written back and the sha256 equal again
```

The restore runs whatever happened after the install, and the next row starts only when the
target's sha256 is the one recorded. A Python killer runs with its own `PYTHONPYCACHEPREFIX` and
no bytecode written, so no run reads another's compiled mutant.

## 4. The weekly battery

```mermaid
flowchart LR
  sched[schedule, weekly: live once the file is on main] --> checkout[checkout dev]
  dispatch[workflow_dispatch at any ref that holds the file: package empty, a crate, or miniapp] --> checkout
  prtrig[a pull request that changes the workflow] --> rehearsal[rehearsal: one file, one row, one Stryker file, the drafts; files nothing]
  checkout --> shards[rust, unless the scope is miniapp: cargo mutants --package the scope's crate, if any, --sharding round-robin --shard k/32, k = 0..31, --in-place --timeout 300 --build-timeout 600]
  checkout --> webfull[web, with no scope or miniapp: stryker run, whole]
  checkout --> rowsall[rows, with no scope: prove every row]
  checkout --> listing[listing: cargo mutants --list --json, the whole tree, nothing built]
  listing --> reports
  shards --> reports[(each shard's mutants.out)]
  webfull --> reports
  rowsall --> reports
  reports --> survivors[survivors: issues: write, this job alone]
  survivors --> drafts[mutation-verdict.py survivors: one draft per file, none for a mutant one record excuses]
  drafts --> scrub[public-scrub.py --no-tree --subject drafts]
  scrub --> dedupe{an open issue with that title?}
  dedupe -->|yes| skip([not filed twice])
  dedupe -->|no| file([gh issue create])
  file --> count[battery --shards 32 --listed whole.json: every report the scope promises, whole]
  skip --> count
  count --> table[mutation-verdict.py table --listed whole.json: one line per package in the scope, every record bound]
  count -->|one missing or partial| red([the job fails, naming each])
  table -->|a listed mutant no report tested| red
  table -->|an unexplained mutant, or a record that fails| red
  table -->|every promised report whole, unexplained 0| green([the job passes, its last line the table])
```

Every shard and job uploads its report under `if: always()`, so a shard that fails on its
survivors still hands them to the `survivors` job, which runs `if: always()` and never on a pull
request. A shard whose runner was shut down uploads nothing, and one stopped early leaves a partial
report: the job's last step counts every report the battery promises, and fails naming each one
missing or partial, so neither reads as a shard with no survivor.

A dispatch scoped to one crate (SPEC-057 R14) promises the shards its scope gave a mutant, which
`battery` counts from the listing: cargo-mutants deals the scope's mutants round-robin, so shard
`k` holds one when the scope lists more than `k`, and a shard of none exits 0 and writes no
report. The scope promises no rows, and `miniapp` promises the Stryker sweep alone. The step then
runs `table` over the same scope, whose line is the step's last.

## 5. The rows file

| file | holds |
|---|---|
| `scripts/mutation-rows.json` | the header alone: `_` (the rules, and one `target spelling:` line per table), `arities`, and empty `tables` |
| `scripts/mutation-rows.d/S<NNN>00-S<NNN>99.json` | SPEC-NNN's rows, `{"tables": {...}}` and nothing else |
| `scripts/mutation-rows.retired.json` | each row retired while its target stayed: its id, the reason and the maintainer's approval |
| `scripts/mutation_rows.py` | the one reader (`load_tree`, `PopulationRefused`) and a row's target by its table's declared spelling, the census, `prove` and `retired` |

| table | 1 | 2 | 3 | 4 | 5 | 6 |
|---|---|---|---|---|---|---|
| `MUTATIONS` | crate directory | file in the crate | find | replace | killer `<target>::<test path>` | description |
| `CARGO_KILLED_SCRIPT_MUTATIONS` | path from the root | find | replace | killer's crate | killer `<target>::<test path>` | description |
| `SCRIPT_MUTATIONS` | path from the root | find | replace | description | killer `<module>.<Class>.<method>` | |
