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
    matrix --> cm[cargo mutants --in-place --in-diff git.diff --sharding round-robin --shard k/n --timeout 1200 --build-timeout 600]
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
| shards | cargo-mutants' own listing of the diff's mutants, `--list --json` | each shard's projected time, the baseline's 346 s and each of its mutants' package cost, plus the settle census's 788 s for each mutant of a package that holds it and once in the baseline (SPEC-327), within an hour; the fewest shards that fit; more than 256 refused, never capped |
| cargo-mutants | the diff, the tree, the shard `k/n` | `--in-place` on the checkout, round-robin as the plan projected, `--timeout 1200` on each mutant's tests and the baseline's (SPEC-327) and `--build-timeout 600` on its build, the shard job's `timeout-minutes` of 120; its exit is recorded, never trusted alone |
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
  checkout --> shards[rust, unless the scope is miniapp: cargo mutants --package the scope's crate, if any, --sharding round-robin --shard k/32, k = 0..31, --in-place --timeout 1200 --build-timeout 600]
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

## 6. A leg with nothing to examine is not started (SPEC-290, ADR-290)

Section 1 draws every leg as started. SPEC-290 lets two of them not start, and the decision is the
plan's listing, read in each leg's job-level condition before its matrix is expanded; a leg's
absence decides nothing. The verdict and `ci` keep every refusal: a shard the listing promises and
that reported nothing is VOID by name, and a skipped leg reads as correct only where the listing
says it had nothing to examine.

```mermaid
flowchart TD
  plan[mutation-plan: the listing, then shards] --> outs[its outputs: listed, rows, scope, and the matrix, unchanged]
  outs --> rustcond{mutation-rust: is listed not 0?}
  rustcond -->|yes| rustlegs[one leg per shard of the matrix, as before]
  rustcond -->|no: no shard lists a mutant, whether or not the Rust class applies| rustskip([mutation-rust not started: skipped])
  outs --> rowscond{mutation-rows: a row selected, or the scope is diff?}
  rowscond -->|yes| rowsleg[the selected rows proved, then the retirement check on a diff]
  rowscond -->|no: a push that merges a pull request| rowsskip([mutation-rows not started: skipped])
  rustlegs --> judge
  rustskip --> judge
  rowsleg --> judge
  rowsskip --> judge
  judge[mutation-verdict, if always: judge each shard 0 to n-1] --> shard{the shard's artifact}
  shard -->|whole| counted[its report counted]
  shard -->|absent, and the listing gives the shard no mutant| notstarted[not started: nothing examined, the rows carry the count]
  shard -->|absent or partial, and the listing gives the shard mutants| voidshard([VOID: the shard, by name])
  counted --> total{the reports' mutants equal the listing's count?}
  total -->|no| voidsum([VOID: both counts named])
  total -->|yes| verdictok[the class judged as before]
  notstarted --> verdictok
  verdictok --> legcheck[legs: each skipped leg against the plan]
  legcheck -->|skipped while the listing owed it work| voidleg([VOID: the leg and what it owed, by name])
  legcheck -->|skipped with nothing to examine, or started| legok[the legs read correct]
  voidshard --> gate
  voidsum --> gate
  voidleg --> gate
  legok --> gate
  gate{ci, if always: every need success, a skipped mutation-rust or mutation-rows admitted once each}
  gate -->|a need failed or was cancelled, the verdict skipped or failed, another job skipped| red([ci fails])
  gate -->|otherwise| green([ci passes])
```

| step | what crosses | guard |
|---|---|---|
| `shards` | the listing, into the plan and the step outputs | `listed` is the count of mutants the shards hold, `0` when the Rust class does not apply; the shard count and the matrix are unchanged |
| a leg's `if:` | the plan's outputs alone | `listed != '0'` for the rust leg; `rows == 'true' \|\| scope == 'diff'` for the rows leg, so the retirement check runs on every diff |
| the verdict | each shard's artifact, the plan's listing | a shard with no artifact reads `not started` only when the listing gives it no mutant; the reports' mutants must equal the listing's count |
| `legs` | each leg's result, the plan | a skip the listing owed is VOID by name; a result that is not a job's is VOID |
| `ci` | every need's result | `skipped` admitted from the two legs alone, once each; `mutation-verdict` must succeed |

## 7. The sizer charges the settle census (SPEC-327, ADR-328)

Read at DeckStreak `dev` bb1fab8. One `--timeout` bounds the unmutated baseline's whole test run
and each mutant's, so it must cover the settle census's two tests in `deck-streak-progression`,
measured at 788 s with their start offset (SPEC-327 section 1). The bound is `--timeout 1200` on
every command, and the sizer charges the census where it is paid: once per mutant of the package,
and once in the baseline of every shard whose plan lists that package. A package the census table
does not name is sized exactly as before.

```mermaid
flowchart TD
  listing[the plan's listing: one package per mutant] --> costs[mutant_costs: the table cost, plus the census term of the mutant's package]
  listing --> base[baseline_seconds: 371, plus the census term of each named package, counted once]
  costs --> fewest[fewest_shards: the least count whose slowest projection fits 3600 s]
  base --> fewest
  fewest --> projected[projected: each shard starts at the baseline and takes its round-robin mutants]
  projected --> plan[the plan: shard count, projected seconds, baseline seconds]
  plan --> legs[one mutation-rust leg per shard, each mutant's tests bounded at 1200 s]
  legs --> mutant{a mutant's tests, the census included}
  mutant -->|a test fails| caught([caught])
  mutant -->|every test passes| missed([missed])
  mutant -->|past 1200 s| timeout([timeout, scored killed as before])
```

| step | what crosses | guard |
|---|---|---|
| `CENSUS_SECONDS` | `deck-streak-progression` to 788 s, a table apart from `SECONDS_PER_MUTANT` | a package it does not name pays 0, so no unnamed package is charged the table's highest twice |
| `mutant_costs` | each listed package | table cost plus census term; rows S32700 and S32701 |
| `baseline_seconds` | the listing's packages, as a set | 371 plus each named package's term once; rows S32702 and S32703 |
| `shards` and `size` | the computed baseline, into `fewest_shards` and `projected` | the pull request's plan and the package dispatch both pay it; row S32704 |
| every `cargo mutants` command | `--timeout 1200 --build-timeout 600` | `BOUNDS` and the byte pins hold each command to it; rows S12904 to S12909 and S32705 |

## 8. A release's mutants in one run of legs (SPEC-362, ADR-373)

Schematic for SPEC-362 and ADR-373 (rows band S36200-S36299). It draws the data flow of a pull
request's Rust mutants, the release pull request's included, and of the scheduled battery, as the
build leaves them. Names are the jobs, artifacts and files the workflows write; section 8.4 cites
each step by `path:line`. Where sections 1, 2, 4 and 7 name the bound of an hour, the 120-minute
leg, the 32 whole-tree legs, `--timeout 1200` or a census of 788 s, this section supersedes them.

### 8.1 A pull request's run, the release's included

```mermaid
flowchart TD
    EV["pull_request into dev or main<br/>(the merge ref; a release's head is dev)"] --> PLAN

    subgraph PLAN["job mutation-plan"]
        D["git diff HEAD^1...HEAD<br/>-> git.diff"] --> L["cargo mutants --list --json --in-diff git.diff<br/>-> listed.json (the population)"]
        L --> S["mutation-verdict.py shards<br/>fewest round-robin legs whose slowest is projected<br/>within SHARD_BOUND_SECONDS (half the leg's timeout)"]
        S --> C{"legs N within<br/>LEG_CEILING for ci.yml?"}
        C -- "no" --> R["REFUSED, whole: count, ceiling, projection<br/>(never capped)"]
        C -- "yes" --> P["plan.json: legs 0..N-1, each its mutants<br/>prints: legs N of ceiling C"]
    end

    P --> ART1[("artifact mutation-plan<br/>git.diff, listed.json, plan.json")]
    R --> ART1

    ART1 --> M

    subgraph M["job mutation-rust, matrix leg = 0..N-1 (one run)"]
        LEG["leg k: cargo mutants --in-place --in-diff git.diff<br/>--sharding round-robin --shard k/N<br/>--timeout from the census; nextest profile mutants"]
        LEG --> B["baseline: the leg's tests, unmutated<br/>(log: its slowest test)"]
        B --> MU["each mutant: build, then tests until the first failure<br/>caught / missed / timeout / unviable"]
        MU --> O["mutants.out: outcomes.json, logs"]
    end

    O --> ART2[("artifact mutation-rust-shard-k<br/>one per leg, latest attempt")]

    ART1 --> V
    ART2 --> V

    subgraph V["job mutation-verdict (if: always())"]
        V1["the population is listed.json:<br/>plan's legs hold each listed mutant once, else VOID"]
        V2["every planned leg 0..N-1:<br/>no report or a partial one is VOID by name"]
        V3["each leg: 1.5 x its slowest baseline test<br/>within the per-mutant timeout, else VOID"]
        V4["reports hold each listed mutant once:<br/>in two legs FAIL, in none VOID"]
        V5["judge: a missed mutant FAILS; caught, timeout<br/>and unviable are examined"]
        V1 --> V2 --> V3 --> V4 --> V5
    end

    V5 --> CI{"job ci (if: always())<br/>needs every job, the verdict included"}
    CI -- "every need success" --> OK["required check ci: success"]
    CI -- "any other result" --> NO["required check ci: failure"]

    NO -. "a leg VOID from a runner shutdown:<br/>Re-run failed jobs" .-> M
```

Reading it:

- One run holds every leg. `LEG_CEILING` is 256 less the most jobs the run's other jobs can
  generate, so the plan never asks for a matrix the run cannot start.
- The plan's legs come from the tool's listing, and the verdict checks them against that listing,
  so a sizing defect cannot shrink the population the verdict judges.
- A leg that never reports is VOID because the verdict walks the plan's legs, never the artifacts
  that arrived.
- A push to `dev` while a release pull request's run is in flight starts a new run on the new head
  and cancels this one (the workflow's concurrency group): the cancelled run decides nothing.

### 8.2 The scheduled battery

```mermaid
flowchart TD
    T["schedule (the default branch's workflow) or dispatch<br/>checks out dev"] --> SZ

    subgraph SZ["job size"]
        WL["cargo mutants --list --json (no diff)<br/>-> the whole tree's listing"] --> WS["the same sizing as shards,<br/>at mutation-weekly.yml's ceiling"]
        WS --> WC{"legs within the battery's ceiling?"}
        WC -- "no" --> WR["REFUSED, whole, by name"]
        WC -- "yes" --> WP["plan: legs 0..N-1"]
    end

    WP --> WM["job rust legs 0..N-1<br/>the same leg command, no --in-diff"]
    WM --> WA[("artifact per leg")]
    WA --> BV["BATTERY: every planned leg counted;<br/>the listing is the population"]
    BV --> SV["survivors: each surviving mutant filed once"]
```

The battery's legs were a fixed 32. They are sized from the listing like any diff's, so the
battery judges the whole tree at the bound every leg can hold, and refuses by name when the tree
outgrows the ceiling.

### 8.3 A leg's report, as the verdict reads it

```mermaid
stateDiagram-v2
    [*] --> Running: the matrix starts leg k
    Running --> Whole: every dealt mutant has an outcome
    Running --> Partial: timeout or shutdown after some outcomes
    Running --> None: shutdown before any upload
    Whole --> Counted: the verdict reads it
    Partial --> Void: VOID by name
    None --> Void: VOID by name
    Void --> Running: Re-run failed jobs (a new attempt of leg k)
    Counted --> [*]
```

An artifact's name is unique in its run, and the leg's upload sets no `overwrite`, whose default
refuses a name the run already holds; the verdict downloads `mutation-rust-shard-*` with
`merge-multiple: true`. So a re-run attempt's upload does not replace what an earlier attempt of
leg k stored, and the verdict reads whichever report the store holds for each leg. The TLA+ entry
`formal/tla/EveryLegCounted` lets the verdict read any stored attempt, so its two properties hold
under either reading of a re-run's upload.

### 8.4 Where each step lives

Read at this delivery's `039b003c`; the documents committed after it change none of the files
cited. A line moves with any edit above it, so search by the item named beside it after that.

| step | `path:line` |
|---|---|
| the run's concurrency group: a push to `dev` cancels a release pull request's run in flight | `.github/workflows/ci.yml:25-27` |
| job `mutation-plan` | `.github/workflows/ci.yml:374` |
| the diff's listing, `listed.json`, the population | `.github/workflows/ci.yml:417` |
| the whole tree's listing, `whole.json` | `.github/workflows/ci.yml:423` |
| `mutation-verdict.py shards` | `.github/workflows/ci.yml:432` |
| artifact `mutation-plan` | `.github/workflows/ci.yml:437` |
| `SECONDS_PER_MUTANT`, `BASELINE_SECONDS`, `CENSUS_SECONDS` | `scripts/mutation-verdict.py:849`, `:866`, `:872` |
| `SHARD_BOUND_SECONDS`, `LEG_CEILING` | `scripts/mutation-verdict.py:876`, `:879` |
| `MUTANT_TIMEOUT_SECONDS`, `TIMEOUT_MARGIN` | `scripts/mutation-verdict.py:882`, `:883` |
| `fewest_shards`: the fewest legs within the bound, up to the ceiling | `scripts/mutation-verdict.py:911` |
| `shards`: sized at the `ci` ceiling, its refusal, its headroom line | `scripts/mutation-verdict.py:1008`, `:1044`, `:1045`, `:1077-1081` |
| job `mutation-rust`, its `timeout-minutes: 360`, its matrix of legs | `.github/workflows/ci.yml:445`, `:449`, `:453` |
| leg k's `cargo mutants` command | `.github/workflows/ci.yml:514` |
| nextest's `mutants` profile, which stops at the first failure | `.config/nextest.toml:8-9` |
| cargo-mutants runs nextest under that profile | `.cargo/mutants.toml:17`, `:23` |
| artifact `mutation-rust-shard-k`, no `overwrite` | `.github/workflows/ci.yml:521` |
| job `mutation-verdict`, `if: always()` | `.github/workflows/ci.yml:644-646` |
| the legs' download, `merge-multiple` | `.github/workflows/ci.yml:672-673` |
| `judge --class rust` | `.github/workflows/ci.yml:687` |
| `judge_rust` | `scripts/mutation-verdict.py:1389` |
| `whole_reports`, walking the plan's legs 0..N-1 | `scripts/mutation-verdict.py:1218`, `:1234` |
| R7: `baseline_void`, read for each whole leg | `scripts/mutation-verdict.py:1315`, `:1419-1422` |
| R8: `population_gaps`, the listing as the population | `scripts/mutation-verdict.py:1368`, `:1423` |
| `partition`: each listed mutant tested once | `scripts/mutation-verdict.py:1264`, `:1465` |
| `examined_sum` | `scripts/mutation-verdict.py:1293`, `:1468` |
| job `ci`, `if: always()`, needing every job | `.github/workflows/ci.yml:873-876` |
| the battery's job `size`, and its `mutation-verdict.py size` | `.github/workflows/mutation-weekly.yml:61`, `:99` |
| `size`: the battery's ceiling, its refusal, its headroom line | `scripts/mutation-verdict.py:925`, `:930`, `:943`, `:956` |
| the battery's job `rust`, its `timeout-minutes: 360`, its matrix, its two leg commands | `.github/workflows/mutation-weekly.yml:108`, `:112`, `:116`, `:172`, `:174` |
| the battery's verdict | `.github/workflows/mutation-weekly.yml:395`; `scripts/mutation-verdict.py:2109` |
| the model of the legs and the verdict | `formal/tla/EveryLegCounted/EveryLegCounted.tla` |
| the rows that pin each new constant and check | `scripts/mutation-rows.d/S36200-S36299.json` |

## 9. The rows legs: dealt by measured cost, merged before the verdict (SPEC-417, ADR-431)

Kind: data flow. Read at DeckStreak `dev` 8870392 (`.github/workflows/ci.yml:376-441`, `:578`,
`:582`, `:615`, `:628-644`, `:668-672`, `:685-702`; `scripts/mutation_rows.py:1169-1188`,
`:1191-1222`; `scripts/mutation-verdict.py:879`, `:1195`). Decided by ADR-431; built by SPEC-417.

Schematic for SPEC-417 and ADR-431 (rows band S41700-S41799). It draws how a pull request's
hand-written mutation rows reach the verdict, before and after the rows are dealt into legs. Names
are the jobs, artifacts and files the workflow writes; section 9.4 cites each step by `path:line`.
The rows job keeps its bound of 90 minutes: what changes is how many jobs share the rows.

### 9.1 Before: one rows job, cut at its bound

```mermaid
flowchart TD
  plan["job mutation-plan<br/>artifact mutation-plan, plan.json"] -->|"needs"| rows["job mutation-rows, one job<br/>timeout-minutes: 90"]
  rows --> prove["mutation_rows.py prove<br/>every chosen row, one at a time"]
  prove --> bound{"every row proved<br/>within 90 minutes?"}
  bound -->|"yes"| report["rows.json, written after the last row<br/>artifact mutation-rows"]
  bound -->|"no: cut at the bound"| noreport["no report written<br/>nothing uploaded"]
  report --> verdict["job mutation-verdict<br/>downloads mutation-rows by name"]
  noreport --> verdict
  verdict --> judge["judge_rows"]
  judge -->|"a report"| graded["each row KILLED or SURVIVED"]
  judge -->|"no report"| void["every selected row VOID<br/>selected and never proved"]
```

A release's diff selects all 2512 rows. At the measured mean of 16.2 s a row they need about 678
minutes, more than the 360 minutes a hosted job may run, so one job cannot hold them at any bound.
The release run's rows job proved 331 of them in 5357 s and was cut at its 90 minutes, with no
report.

### 9.2 After: legs dealt by measured cost, merged before the verdict

```mermaid
flowchart TD
  plan["job mutation-plan<br/>mutation_rows.py legs"] --> fit{"the fewest legs n whose largest,<br/>ceil(R / n) rows x ROW_SECONDS,<br/>is within ROWS_BOUND_SECONDS,<br/>with n at most ROWS_MAX_LEGS?"}
  fit -->|"none"| refused["REFUSED, whole, by name:<br/>the count and the most legs<br/>exit 2, no output written"]
  refused -->|"the plan job fails"| cifail["job ci fails"]
  fit -->|"n"| outputs["outputs rows_legs = n<br/>rows_matrix = 0 to n-1"]
  outputs -->|"needs"| leg["job mutation-rows, matrix leg k = 0 to n-1<br/>timeout-minutes: 90, fail-fast: false"]
  leg --> deal["mutation_rows.py prove --leg k/n<br/>the chosen rows k, k+n, k+2n, in plan order"]
  leg --> retired["leg 0 only<br/>mutation_rows.py retired"]
  deal --> legreport["rows-leg-k.json<br/>artifact mutation-rows-leg-k"]
  legreport -->|"needs: every leg has ended"| download["job mutation-verdict<br/>downloads every mutation-rows-leg-k,<br/>merged into reports/mutation-rows"]
  download --> merge["mutation_rows.py merge<br/>legs 0 to n-1, in leg order"]
  merge --> dealcheck{"every id in the leg its deal gave it,<br/>and in one leg only?"}
  dealcheck -->|"no"| refusedmerge["REFUSED by name<br/>exit 2, no report written"]
  dealcheck -->|"yes"| merged["rows.json, the one report<br/>each absent leg named on its own line"]
  merged --> judge["judge_rows, unchanged"]
  judge --> graded["each proved row KILLED or SURVIVED"]
  judge --> void["an absent leg's rows VOID by id"]
```

A leg cut at its bound uploads nothing, so `merge` names it and the judge reads its rows as VOID
by id, never as a pass. The legs share no written object: each uploads its own artifact, they read
the plan's artifact and a restore-only cache, and the verdict's job reads the legs only after
`needs` has ended every one (ADR-431 D8).

### 9.3 The arithmetic, and the run's ceiling

- price: 5357 s over 331 rows is 16.2 s a row; `ROW_SECONDS = 17`, rounded up to the second;
- bound: the rows job keeps 90 minutes, 5400 s, now each leg's; `ROWS_BOUND_SECONDS = 2700`, half
  of it; a leg holds at most 158 rows (158 x 17 = 2686 s; 159 x 17 = 2703 s);
- a release's 2512 rows: 2512 x 17 = 42,704 s in all; 15 legs hold 2370 rows, too few, so 16 legs
  of at most 157 rows, each projected at 157 x 17 = 2669 s, within 2700 s;
- room: `ROWS_MAX_LEGS = 17` legs hold 2686 rows; 2687 rows are refused whole, by name;
- the release's re-run: at least 2539 rows, the 2512 with SPEC-415's 8, SPEC-416's 3 and this delivery's 16;
  16 legs hold 2528, too few, so 17 legs of at most 150 rows, each projected at 2550 s;
- the run's ceiling for Rust legs: every `ci.yml` job but `mutation-rust` can generate 47 jobs at
  the base, the rows job among them as one; counted at its most legs, they generate 63, so
  `LEG_CEILING["ci"]` falls from 209 to 193, the run's 256 less 63. A release listing of 200 Rust
  legs is then refused whole by name, and a listing of 192 fits.

### 9.4 Where each step lives

Read at `8870392`, before this delivery's change. A line moves with any edit above it, so search
by the item named beside it after that.

| step | `path:line` |
|---|---|
| job `mutation-plan`, its outputs and its steps | `.github/workflows/ci.yml:376-441` |
| job `mutation-rows`, and its `timeout-minutes: 90` | `.github/workflows/ci.yml:578`, `:582` |
| the rows job's dependency cache, restored only | `.github/workflows/ci.yml:615` |
| `mutation_rows.py prove`, the rows the diff selects | `.github/workflows/ci.yml:628-634` |
| `mutation_rows.py retired` | `.github/workflows/ci.yml:635-637` |
| artifact `mutation-rows` | `.github/workflows/ci.yml:638-644` |
| the verdict's download of the rows report, by name | `.github/workflows/ci.yml:668-672` |
| the judge step | `.github/workflows/ci.yml:685-702` |
| `chosen`: the population the runner proves | `scripts/mutation_rows.py:1169-1188` |
| `prove`, its report written once, after the last row | `scripts/mutation_rows.py:1191-1222`, `:1217-1218` |
| `LEG_CEILING` | `scripts/mutation-verdict.py:879` |
| `judge_rows`: a selected row with no proof is VOID | `scripts/mutation-verdict.py:1195` |
| the rows that pin each new constant, verb and step | `scripts/mutation-rows.d/S41700-S41799.json` |

## 10. The pull request's web sweep in three shards, read by name (SPEC-419, ADR-433)

Kind: data flow. Read at DeckStreak `dev` e64a778 (`.github/workflows/ci.yml:736-794`, the one
`mutation-web` job before this change; `scripts/mutation-verdict.py:1993-2051`, `judge_web`, which
reads only the report's `files` and is unchanged). Decided by ADR-433; built by SPEC-419 (rows
band S41900-S41999). It supersedes the `mutation-web` subgraph of section 1: names are the jobs,
artifacts and files the workflow and `scripts/web_mutation_shards.py` write.

### 10.1 Before: one job, cut at its bound

```mermaid
flowchart TD
  plan1["job mutation-web, one job<br/>timeout-minutes: 100<br/>mutation-verdict.py plan"] --> run1["stryker run --mutate every changed web file"]
  run1 --> fits{"done within 100 minutes,<br/>upload included?"}
  fits -->|"yes"| report1["reports/mutation/mutation.json"]
  fits -->|"no: the job is cut"| cut1["the job reads cancelled<br/>even when StrykerJS finished"]
  report1 --> judge1["mutation-verdict.py judge --class web"]
```

### 10.2 After: three shards dealt by StrykerJS's own count, one job reads them by name

```mermaid
flowchart TD
  shard["job mutation-web-shard, matrix shard k = 0, 1, 2<br/>timeout-minutes: 100, fail-fast: false"] --> planS["mutation-verdict.py plan<br/>stryker_mutate: the changed web files"]
  planS --> count["web_mutation_shards.py deal --shard k<br/>stryker serve stdio: configure, discover<br/>each planned file's mutant count"]
  count -->|"the count fails"| novoid["VOID, no shard.json written"]
  count --> deal["whole files, largest count first,<br/>each onto the lightest shard;<br/>ties by path, then the lowest index"]
  deal --> record["shard.json: counts, the whole deal,<br/>this shard's files and mutants, empty"]
  record -->|"mutants above 0"| sweep["stryker run --mutate this shard's files<br/>stryker.exit and mutation.json"]
  record -->|"no mutant: an explicit empty record"| keep["artifact mutation-web-shard-k<br/>uploaded if: always()"]
  sweep --> keep
  keep -->|"needs: every shard has ended"| verdictjob["job mutation-web, one job, if: always()<br/>timeout-minutes: 100<br/>downloads mutation-web-shard-*"]
  verdictjob --> union["web_mutation_shards.py union<br/>reads mutation-web-shard-0, -1 and -2 by name"]
  union --> whole{"every record present and its shard's,<br/>its files the deal of its counts,<br/>every dealt mutant in its report,<br/>every planned file in one shard,<br/>the shards' result success?"}
  whole -->|"no"| void2["VOID by name, exit 3<br/>no union written"]
  whole -->|"yes"| merged["the shards' files merged<br/>stryker/mutation.json"]
  merged --> judge2["mutation-verdict.py judge --class web, unchanged"]
```

A shard cut at its bound reads `cancelled`, so the union reads VOID by name and judges no mutant;
a shard the deal gave nothing still writes and uploads its record, so a missing record is always a
defect, never an empty shard. The shards share no written object: each writes its own directory
and artifact, and the verdict's job reads them only after every shard has ended.

## 11. The release run's Rust legs, up to one signed ceiling in one matrix (SPEC-423, ADR-437)

Schematic for SPEC-423 and ADR-437 (rows band S42300-S42399). It draws the Rust half of the plan
before and after the delivery, read at dev `a3a052b2`: the release mark the plan already carries,
the ceiling constant, the two refusals, the one `mutation-rust` matrix of at most 256 legs, and
the verdict's count of every leg. Names are the jobs, outputs, constants and texts the scripts
write. Where section 8.1 says one run holds every leg within `LEG_CEILING` for `ci.yml`, this
section narrows it for the release pull request's run alone.

### 11.1 Before: every run's Rust legs are held to the pull request ceiling

```mermaid
flowchart TD
    MARK["plan.json: release true only when release_run holds<br/>(SPEC-422 R1, R2)"] --> RS
    LIST["listed.json: the diff's Rust mutants"] --> RS
    RS["shards: fewest_shards up to LEG_CEILING ci"] --> RC{"fits within 190 legs?"}
    RC -- "no" --> RR["REFUSED, whole: the pull request run's ceiling<br/>exit 1, no shards, no outputs"]
    RC -- "yes" --> RP["plan.json shards 0..N-1<br/>outputs shards and matrix<br/>prints legs N of ceiling 190"]
    RP --> MR["job mutation-rust, matrix shard = 0..N-1"]
```

The release mark reaches the Python half only (`python_release_ceiling`, SPEC-422): on the release
run a Rust listing that needs more than 190 legs reads the pull request refusal.

### 11.2 After: the release run's Rust legs are held to RUST_RELEASE_CEILING

```mermaid
flowchart TD
    MAIN["shards verb: python_share_refused, python_ceiling_refused,<br/>then rust_ceiling_refused, before any deal"] --> RANGE{"RUST_RELEASE_CEILING an int<br/>within LEG_CEILING ci .. RUST_MATRIX_LEGS?"}
    RANGE -- "no" --> RANGEOUT["REFUSED by name: the release ceiling is outside its range<br/>exit 1, no deal, on every run"]
    RANGE -- "yes" --> HELP{"rust_release_ceiling: release mark true<br/>and the constant above LEG_CEILING ci?"}
    HELP -- "no: None" --> PRPATH["ceiling = LEG_CEILING ci<br/>sized, refused and printed as before, byte for byte"]
    HELP -- "yes" --> RELPATH["ceiling = RUST_RELEASE_CEILING"]
    PRPATH --> FIT{"fewest_shards within the ceiling?"}
    RELPATH --> FIT
    FIT -- "no, release" --> RELOUT["REFUSED, whole: the release run's ceiling (RUST_RELEASE_CEILING)<br/>a larger figure needs a new measurement and a new owner-signed ruling"]
    FIT -- "no, pull request" --> PROUT["REFUSED, whole: the pull request run's ceiling<br/>(lines not edited)"]
    FIT -- "yes" --> DEALT["plan.json shards 0..N-1, round robin i mod N<br/>outputs shards and matrix = 0..N-1"]
    DEALT --> LINES["release run only: the Rust release line beside the Python one<br/>legs N of ceiling C, and the run-legs line, read the ceiling"]
    LINES --> ONE["job mutation-rust, ONE matrix, shard = 0..N-1<br/>N at most RUST_MATRIX_LEGS"]
```

- The release mark is the one SPEC-422 writes; no second predicate reads the event.
- At the placeholder, `RUST_RELEASE_CEILING` equals `LEG_CEILING["ci"]`, the helper returns None,
  and the release run takes the pull request path byte for byte.
- A figure above `RUST_MATRIX_LEGS` never reaches a deal: the range check refuses it on every run,
  so a second matrix is a new decision, never a cap.
- No job, output or check-run name is added: `mutation-rust` expands the plan's `matrix` output
  whatever its count, and `LEG_CEILING`'s derivation sets that one job apart as before.

### 11.3 The verdict counts every leg, whatever the count

```mermaid
flowchart LR
    PLANS["plan.json shards 0..N-1"] --> WALK["whole_reports: one promised report per leg 0..N-1"]
    ARTS[("mutation-rust-shard-k, merged download")] --> WALK
    WALK -- "a leg with mutants and no report" --> VOIDLEG["VOID by the leg's name"]
    WALK --> PART["partition: each listed mutant tested once"]
    PART --> SUM["examined_sum: the reports count the listing exactly"]
    SUM --> CIJOB{"job ci needs mutation-verdict"}
```

The walk reads the plan's own legs, never the artifacts that arrived, so a count past 190 is walked
whole exactly as any count is (SPEC-290, SPEC-362 R8; SPEC-423 A9).

### 11.4 Where each step lives

Read at dev `a3a052b2`; the new lines are cited by name, and the build re-reads every line at its
green commit.

| step | where |
|---|---|
| the release mark: written by the plan, read as `is True` | `scripts/mutation-verdict.py:825-832`, `:1100` |
| `LEG_CEILING`, the pull request and battery ceilings | `scripts/mutation-verdict.py:961` |
| `RUST_RELEASE_CEILING` and `RUST_MATRIX_LEGS`, each with its `#:` line | directly after `:961` (new) |
| `rust_ceiling_refused` and `rust_release_ceiling` | after `python_release_ceiling`, `:1095` (new) |
| `fewest_shards`, the scan up to a ceiling | `scripts/mutation-verdict.py:993` |
| the Rust sizing, its pull request refusal and its headroom line in `shards` | `scripts/mutation-verdict.py:1322`, `:1323-1329`, `:1382-1386` |
| the run-legs line | `scripts/mutation-verdict.py:1387-1399` |
| `announce`, the `shards` and `matrix` outputs | `scripts/mutation-verdict.py:1256` |
| the range check in the `shards` verb | `scripts/mutation-verdict.py:3713-3721` |
| `mutation-rust`'s axis on the plan's `matrix` output | `.github/workflows/ci.yml:468-469` |
| the verdict's walk of every leg | `scripts/mutation-verdict.py:1542`, `:1588`, `:1617` |
| `RUN_JOBS`, the limit one matrix holds | `scripts/tests/test_dispatch_shards.py:219` |

## 12. Each Rust leg's own bounded baseline, then cargo-mutants without its own (SPEC-480 section 9, ADR-509)

Schematic for SPEC-480 section 9 and ADR-509 (rows S48008 to S48038 of band S48000-S48099). It
draws one leg of the `mutation-rust` matrix before and after the delivery, read at dev `3bdfccd6`,
and the verdict job's reads of every leg's artifact. Names are the steps, files, constants and
texts the workflow and the scripts write. Where section 11 draws how many legs one run holds, this
section draws what each leg runs and how the verdict reads it.

### 12.1 Before: cargo-mutants' one timeout bounds the leg's own baseline too

```mermaid
flowchart TD
    LISTB["listed.json and git.diff, from the plan job"] --> CMB["cargo-mutants, one shard, in the memory scope<br/>--timeout 2300 --build-timeout 600"]
    CMB --> BASEB{"its own baseline: the shard's packages,<br/>each test within the 2300 s timeout?"}
    BASEB -- "no: Timeout" --> NONEB["no mutant of the leg is run<br/>outcomes.json holds the Baseline outcome alone"]
    BASEB -- "yes" --> MUTB["each mutant within --timeout 2300"]
    NONEB --> VERB["verdict: baseline_void reads the report's Baseline outcome<br/>VOID by the leg's name"]
    MUTB --> VERB
```

A leg whose tests together take longer than the per-mutant timeout can judge no mutant: its
baseline is cut at the same timeout that bounds each mutant (ADR-328).

### 12.2 After: the leg's own baseline under its budget, then cargo-mutants with --baseline=skip

```mermaid
flowchart TD
    LISTA["listed.json, from the plan job"] --> STEP["step id baseline: mutation-baseline.sh in the memory scope<br/>the shard's packages, mutant i to leg i mod n, under timeout 3845"]
    STEP --> REC["baseline/baseline.exit and baseline/baseline.log<br/>under the leg's report directory"]
    REC --> GREEN{"exit 0?"}
    GREEN -- "yes: green=true" --> CMA["cargo-mutants --baseline=skip --timeout 2300<br/>--build-timeout 600, in the memory scope"]
    GREEN -- "no" --> NOTRUN["cargo-mutants does not run<br/>no mutant of the leg is run"]
    CMA --> OUTA["mutants.out/outcomes.json, with no Baseline outcome"]
    OUTA --> ARTA[("artifact mutation-rust-shard-k")]
    REC --> ARTA
    NOTRUN --> ARTA
    ARTA --> BVA["verdict: baseline_void for every leg the plan deals a mutant"]
    BVA -- "no record" --> VNONE["VOID: it left no record of its unmutated baseline"]
    BVA -- "exit 124" --> VBUDGET["VOID: its unmutated baseline reached its 3845 s budget"]
    BVA -- "another non-zero exit" --> VFAIL["VOID: its unmutated baseline failed, with its exit"]
    BVA -- "exit 0" --> SLOW{"its log's slowest test, times 1.5,<br/>within the 2300 s timeout?"}
    SLOW -- "no, or no test timed" --> VSLOW["VOID by the leg's name, as SPEC-362 R7 reads it"]
    SLOW -- "yes" --> JUDGED["the leg's mutants are judged"]
    ARTA --> TMO["step after the verdict's, if always: mutation-timeouts.sh<br/>each package's Timeouts beside its baseline tests' seconds"]
    TMO --> NAMED["READ BY NAME: each Timeout of a package over the margin, or untimed"]
```

- A leg's mutants are judged only against that leg's own baseline record; the plan's charge of a
  baseline, `BASELINE_SECONDS`, does not change (SPEC-480 R14).
- `--timeout 2300 --build-timeout 600` and every other word of the cargo-mutants command are
  unchanged: no mutant is filtered or skipped, and a leg whose baseline is not green runs none.
- A VOID is never a pass; a report that holds no `Baseline` outcome, as `--baseline=skip` writes
  it, is read whole.

### 12.3 Where each step lives

Read at dev `3bdfccd6`; the new lines are cited by name, and the build re-reads every line at its
green commit.

| step | where |
|---|---|
| the plan job's listing, `listed.json` | `.github/workflows/ci.yml:426` |
| the leg's plan download | `.github/workflows/ci.yml:476-479` |
| the step `id: baseline` and `scripts/mutation-baseline.sh` | before the cargo-mutants step, `.github/workflows/ci.yml:524` (new) |
| the cargo-mutants command, its `if:` and `--baseline=skip` | `.github/workflows/ci.yml:533` |
| the leg's upload of its report directory | `.github/workflows/ci.yml:536-542` |
| `BASELINE_BUDGET_SECONDS` and `BASELINE_BUDGET_EXIT` | after `TIMEOUT_MARGIN`, `scripts/mutation-verdict.py:971` (new) |
| `baseline_void`, over the leg's own record | `scripts/mutation-verdict.py:1684` |
| the shard loop of `judge_rust`, every dealt leg | `scripts/mutation-verdict.py:1779` |
| `read_exit` | `scripts/mutation-verdict.py:2501` |
| the verdict step | `.github/workflows/ci.yml:767` |
| the Timeouts step and `scripts/mutation-timeouts.sh` | after the verdict step, `.github/workflows/ci.yml:785` (new) |
| the derive of the budget | `tools/rust-baseline-budget/size_budget.py` (new) |
