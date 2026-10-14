# Schematic: generated mutants for the repository's Python, on a pull request and weekly

Kind: data flow (the pull request's mutation jobs with the Python job added, and the weekly
battery's Python shards) and a state machine (one file's run inside a shard). Read at DeckStreak
`dev` 26263de (`.github/workflows/ci.yml`, `.github/workflows/mutation-weekly.yml`,
`scripts/mutation-verdict.py`, `scripts/mutation_rows.py`). Decided by ADR-073; built by SPEC-087.

This ADDS to `docs/schematics/mutation-testing.md` and redraws none of it: the Rust shards, the
rows, the web job and the verdict's Rust class are drawn there, and the equivalence record's path
from a fragment to a verdict in `docs/schematics/mutation-equivalence-record.md`. Here each of
those appears as one node, and only the Python run is drawn whole.

## 1. A pull request's diff: where the Python job sits

```mermaid
flowchart TD
  plan["mutation-plan: mutation-verdict.py plan, the one scope decision every class shares"] --> classes{"each changed path's class"}
  classes -->|"scripts/NAME.py, one segment under scripts"| scripts["scripts: new, never production"]
  classes -->|"tools/parity-oracle/generate.py"| oracle["oracle: production, as today"]
  classes -->|"crates, web, anything else"| existing(["drawn in mutation-testing.md"])
  scripts --> pylist["mutation_python.py list --plan: mutants whose span holds a changed code line, nothing run"]
  oracle --> pylist
  plan --> pywhole["mutation_python.py list --all: the whole population, for the record's binding"]
  pylist --> pyshards["mutation-verdict.py shards --python-listed: ceiling of listed over 40, clamped 1 to 32"]
  pyshards --> pymatrix["python_shards and python_matrix, and each shard's mutants in the plan"]

  subgraph mutation-python: one job per shard k of n
    pymatrix --> pyrun["mutation_python.py run --plan --shard k/n --failfast"]
    pyrun --> pyreport[("mutation-python-shard-k: python.json and the exit")]
  end

  plan --> rowsjob(["mutation-rows: the rows the diff selects, drawn in mutation-testing.md"])
  plan --> rustjob(["mutation-rust: drawn in mutation-testing.md"])

  subgraph verdict ["mutation-verdict: after every shard, if always"]
    pyreport --> promised{"a report from every shard 0 to n-1?"}
    promised -->|"one missing, unreadable, or not of the schema"| voidshard(["VOID, naming the shard"])
    promised -->|yes| judge["judge --class scripts, then judge --class oracle, each with --python and --rows"]
    rowsjob --> judge
    rustjob --> rustclass(["judge --class rust, drawn in mutation-testing.md"])
    pywhole --> judge
    records[("scripts/mutation-equivalent.d/python.json")] --> judge
    judge --> anyvoid{"a timeout, a void mutant, a VOID file, or an exit 4?"}
    anyvoid -->|yes| voidrun(["VOID, naming each"])
    anyvoid -->|no| examined{"examined: killed, survived and uncovered, plus the rows on changed lines"}
    examined -->|"0, and the class applies"| voidnothing(["VOID: a changed code line and nothing examined"])
    examined -->|"above 0"| survivors{"a survived or uncovered mutant that no record binds?"}
    survivors -->|yes| fail(["FAIL, naming each: kill it with a test, red first, or record it with its argument"])
    survivors -->|no| pass(["examined N, each unviable mutant and byte reader named"])
  end

  voidshard --> ci["ci: needs mutation-python and mutation-verdict beside every other job"]
  voidrun --> ci
  voidnothing --> ci
  fail --> ci
  pass --> ci
```

A class whose changed lines are all blank or comments reads `not-applicable` before any of this,
as SPEC-039 R4 reads every class; the `mutation-python` job still runs one shard, prints its case
and writes a report, because `ci` reads a skipped need as failed.

## 2. One file's run inside a shard

```mermaid
stateDiagram-v2
  [*] --> Clean: run starts
  Clean --> Refused: a tracked change in the tree
  Refused --> [*]: exit 2, nothing mutated
  Clean --> Control: the file's modules from mutation-python.json
  Control --> Uncovered: the map names no module
  Uncovered --> [*]: every mutant uncovered and examined, no test run
  Control --> VoidFile: a test fails unmutated, or none is selected
  VoidFile --> [*]: VOID by name, no mutant run
  Control --> Sentinel: every test passes
  Sentinel --> VoidFile: every test fails on the appended comment
  Sentinel --> Mutants: failing tests named byte readers and left out
  Mutants --> Installed: next mutant, written in place
  Installed --> Unviable: ast.parse refuses it
  Installed --> Child: it parses
  Child --> Killed: a test fails, its ids recorded
  Child --> Survived: every test passes
  Child --> Timeout: the bound passes, the process group killed
  Child --> Void: no report printed
  Unviable --> Restore
  Killed --> Restore
  Survived --> Restore
  Timeout --> Restore
  Void --> Restore
  Restore --> Mutants: bytes written back, sha256 equal
  Restore --> Stopped: the write fails, or the digest differs
  Stopped --> [*]: exit 4, naming the file
  Mutants --> [*]: no mutant left, report written
```

The sentinel's run restores the file the same way before the first mutant. Every child runs with
`PYTHONDONTWRITEBYTECODE=1` and `PYTHONPYCACHEPREFIX` in a temporary directory, and loads each test
module as `unittest discover` does, so an import that fails is a failed test named for its module.
The child runs from a copy of the runner and of the `scripts/` modules it imports, taken outside the
tree before the first mutant, so no mutant of the runner runs as its own child, and drops the
copy's modules from `sys.modules` before it loads a test module, so a test that imports
`mutation_rows` by name reads the tree's file, the mutant installed.

## 3. The weekly battery: the Python shards added

```mermaid
flowchart LR
  checkout["checkout dev, or a dispatch whose package is empty, a crate, miniapp or python"] --> python["python, with no scope or python: mutation_python.py run --all --shard k/16, k = 0..15, every killer recorded"]
  checkout --> others(["rust, web, rows: drawn in mutation-testing.md, none of them under the python scope"])
  checkout --> listing["listing: adds mutation_python.py list --all"]
  python --> reports[("each shard's python.json")]
  others --> reports
  listing --> reports
  reports --> survivors["survivors: drafts one issue per file, Mutation survivors: path, unless an open issue holds that title"]
  survivors --> battery["battery: every one of the 16 Python reports, VOID by name for one missing"]
  battery --> table["table: python: listed N, killed K, equivalent E, unexplained U, unviable V"]
  table --> killers[("every killer of every killed mutant, which the killer map will read")]
```

## 4. What crosses each step

| from | to | what | shape |
|---|---|---|---|
| `mutation-plan` | `mutation-python` | the plan, the matrix, each shard's mutants | `plan.json`, `python_shards`, `python_matrix` |
| `mutation-plan` | `mutation-verdict` | the whole population's listing | the `list --all` output |
| `mutation-python` | `mutation-verdict` | one report per shard | `python.json`, schema `deckstreak.mutation-python.v1` |
| `mutation-rows` | `mutation-verdict` | the rows' outcomes, counted into each class's examined | `rows.json` |
| the tree | `mutation-verdict` | the Python records | `scripts/mutation-equivalent.d/python.json` |
| the weekly `python` job | `survivors`, `battery`, `table` | sixteen reports with every killer | `python.json` each |

## 5. Amendment (SPEC-415): a release's Python plan, before and after the refusal

Insert-only; every earlier byte of this schematic is kept. Kind: data flow (the plan, the Python
legs and the verdict) with one decision node, where the plan sizes or refuses. Read at DeckStreak
`dev` c74b6b9 (`scripts/mutation-verdict.py:120-121`, `:987-995`, `:1017-1023`, `:1072-1075` and
`:1603-1611`; `.github/workflows/ci.yml:376-446`, `:531-571`, `:646-702` and `:875-905`). Decided
by ADR-429; built by SPEC-415.

Section 1's node `pyshards` still reads "ceiling of listed over 40, clamped 1 to 32". SPEC-362 R14
made the rule 20 listed mutants a leg, and SPEC-415 makes the clamp a refusal: the node is kept as
it was drawn, and this section is the plan as it now runs.

### 5.1 Before: the cap

```mermaid
flowchart TD
  blist["mutation_python.py list --plan: 5423 listed, plan job 114529617353, 24 seconds"] --> bsize["python_shards: ceil of 5423 over 20 is 272 legs, and min of 32 and 272 is 32"]
  bsize --> bmatrix["python_matrix 0 to 31, 169 or 170 mutants a leg; the plan exits 0 and prints no refusal"]
  bmatrix --> bleg["mutation-python leg k: run --shard k/32 --failfast, timeout-minutes 60"]
  bleg -->|"28 legs, cut at 60 minutes"| bcut(["cancelled at the bound, before the runner writes its report"])
  bleg -->|"4 legs, 33 to 54 minutes"| bshut(["runner shutdown, exit 143, no report"])
  bcut --> bverdict{"mutation-verdict: a report from every leg 0 to 31?"}
  bshut --> bverdict
  bverdict -->|"none of 32"| bvoid(["VOID, each leg by name; ci fails"])
```

The cap cuts at `python_shards`' `min`: 272 legs of work go into 32 jobs. The bound cuts at the
job's `timeout-minutes: 60`, and because the runner writes its report only when its run ends, a cut
leg leaves nothing for the verdict to read: no outcome, no examined count and no cost.

### 5.2 After: the refusal

```mermaid
flowchart TD
  alist["mutation_python.py list --plan: n listed"] --> aneed["python_needed: the larger of 1 and the ceiling of n over 20"]
  aneed --> afit{"legs needed at most 32?"}
  afit -->|"yes, n at most 640"| amatrix["python_shards: c legs dealt round robin; prints python legs c of ceiling 32"]
  amatrix --> aleg["mutation-python leg k: run --shard k/c --failfast, timeout-minutes 60, at most 20 mutants"]
  aleg --> areport[("mutation-python-shard-k: report.json")]
  areport --> averdict{"mutation-verdict: every leg 0 to c-1 reported whole, each examining the plan's mutants?"}
  averdict -->|"yes"| ajudge(["judged: each mutant killed, survived or recorded, by name"])
  averdict -->|"a leg cut or missing"| avoidleg(["VOID, the leg by name; ci fails"])
  afit -->|"no, n at least 641; 5423 needs 272"| arefuse(["REFUSED by name, exit 1: no plan byte and no step output written"])
  arefuse --> askip["mutation-plan fails: no Python leg and no Rust leg starts"]
  askip --> avoidplan(["mutation-verdict: VOID, the plan names no python shards; ci fails"])
```

### 5.3 What crosses each step, before and after

| step | before | after |
|---|---|---|
| the listing | 5423 listed, the runner's own listing | unchanged: nothing sampled, filtered or dropped |
| the legs | 272 needed, 32 run | 272 needed, refused whole at the plan |
| a leg's mutants | 169 or 170 | at most 20, or none run |
| a leg's bound | `timeout-minutes: 60` | unchanged |
| a mutant's bound | the larger of 60 seconds and five times its file's control run | unchanged |
| `--failfast` | on, for each mutant's run | unchanged |
| the plan's exit | 0, with no refusal | 1, with the refusal by name |
| the verdict | VOID, each of 32 legs named | VOID, the plan names no python shards |
| what stays examined | every listed mutant promised, none reported | in a sized plan, every listed mutant in exactly one leg, the examined sum equal to the listing |

### 5.4 How the Python and Rust legs share the run

The run holds 256 jobs. The Python legs keep a fixed share of at most 32, the Rust legs at most
209 (`LEG_CEILING["ci"]`, derived with the Python legs counted at 32), and 15 other jobs hold the
rest. Each plan refuses whole above its own share, by name, and neither borrows the other's.

## 6. The Python legs dealt by each file's cost (SPEC-418, ADR-432)

Added after section 5; every earlier byte is kept. Kind: data flow. Read at DeckStreak `dev`
c74b6b9, whose line numbers these are, with SPEC-415's change on top (`scripts/mutation-verdict.py`
`python_shards` at 987-995 and `shards` at 1008-1093, `scripts/mutation_python.py` `select` at
768-796 and its report at 901-902, `.github/workflows/ci.yml` `mutation-python` at 531-570).
Decided by ADR-432; built by SPEC-418.

Before: the legs are sized by a count, and a leg runs its own `k::n`. Nothing is written until a
run ends, so a leg cut at its 60 minutes leaves no cost behind.

```mermaid
flowchart LR
  listing["listing: N Python mutants, in listing order"] --> count["python_shards: N over 20 legs, rounded up"]
  count --> refused["more than 32 legs: the plan refused whole, by name"]
  count --> dealt["shard k holds the names k::n"]
  dealt --> leg["leg k of n: runs chosen k::n of its own listing, never checked against the plan"]
  leg --> cut["a leg cut at its 60 minutes: no report and no cost, nothing uploaded"]
  leg --> report[("report.json, written only when the run ends")]
  report --> verdict["mutation-verdict: the python class"]
  cut --> verdict
```

After: a priced file's mutants are dealt by what they cost, within half the job, and every leg
records each cost as it is paid, so a cut leg still measures the next plan.

```mermaid
flowchart LR
  listing["listing: N Python mutants, in listing order"] --> bycost["python_by_cost"]
  table[("PYTHON_CONTROL_SECONDS: a price per file, shipped empty")] --> bycost
  bycost -->|no listed file priced| count["the count rule of section 5, byte for byte"]
  bycost -->|a priced file| runs["each priced file cut into runs of at most k, k = floor of 1800 over its price, less 2"]
  runs --> dealt["runs dealt heaviest first onto the lightest leg, in the fewest legs whose slowest is within 1800 s"]
  dealt --> unpriced["unpriced mutants: 20 a leg, round robin, after the priced legs"]
  unpriced --> refused["more than 32 legs, or one mutant's leg past 1800 s: refused whole by name, exit 1, the plan unwritten"]
  unpriced --> plan[("plan.json python: count, shards, projected_seconds")]
  count --> plan
  plan --> leg["leg k of n: planned_leg holds the plan's count and partition to its own listing"]
  leg -->|a name missing or extra| usage["refused before any mutant: exit 2, no report"]
  leg -->|the partition holds| ran["runs exactly the names shard k lists"]
  ran --> costs[("costs.jsonl beside report.json: a control line, then one line per mutant, each flushed")]
  ran --> report[("report.json, when the run ends")]
  costs --> prices["mutation-verdict.py prices --costs DIR: each file's price, with its run, attempt and shard"]
  prices -.->|a later delivery commits the table| table
  report --> verdict["mutation-verdict: the python class, every planned leg counted"]
  usage --> verdict
```

A priced file's leg projects (2 + m) x price for its m mutants there: its control, its sentinel
and each mutant at the control's cost. The bound is 1800 seconds, half the `mutation-python` job's
60 minutes. The run's 225 legs (256 jobs less the 31 others) stay in fixed shares, 32 Python and
193 Rust, so the plan's joint line reads its legs and projected runner-minutes against
32 x 30 + 193 x 180 = 35700, and the leg ceiling is unchanged.

| from | to | what | shape |
|---|---|---|---|
| `mutation-plan` | `mutation-python` | each leg's names and its projection | `plan.json` `python`: `count`, `shards[].mutants`, `shards[].projected_seconds` |
| `mutation-python` | its uploaded artifact | each file's control seconds and each mutant's seconds, as paid | `costs.jsonl`, one JSON line each: `control`, `uncovered`, `mutant` |
| a run's downloaded artifacts | `mutation-verdict.py prices` | every leg's `costs.jsonl` | one table line per file, with its run, attempt and shard |
| `prices`' output | `PYTHON_CONTROL_SECONDS` | a file's price, committed by a later delivery | a whole number of seconds per path |

## 7. The share, the measured prices and the scoped test set (SPEC-420, ADR-434)

Every `path:line` in this section was read at dev `e64a778e`. Section 6 stays as written. This
section shows what changes around it: the share, one value with a cap; the price table, filled by
the `prices` verb from one recorded run; and each mutant, run against the tests its line's trace
names.

### 7.1 Before: a Python leg at dev `e64a778e`

```mermaid
flowchart TD
  bList["plan job: mutation_python.py list over the diff"] --> bShards["mutation-verdict.py shards"]
  bTable["PYTHON_CONTROL_SECONDS, empty"] --> bShards
  bShards -->|"no file priced: the count rule, 20 a leg"| bShare{"legs above PYTHON_MAX_SHARDS?"}
  bShare -->|"yes"| bRefuse["REFUSED by name; no leg starts"]
  bShare -->|"no"| bMatrix["python_matrix output"]
  bMatrix --> bLeg["each leg: run --plan --shard k/n --failfast"]
  bLeg --> bFile["per file in turn: control, bound, sentinel, then that file's mutants"]
  bFile --> bMut["each mutant: every mapped test module, less the byte readers"]
  bMut --> bReport["report.json and costs.jsonl"]
  bReport --> bVerdict["mutation-verdict.py: every listed mutant examined; each void read by name"]
```

The share is `PYTHON_MAX_SHARDS`, on line 122, beside the count rule's 20, and nothing caps it. No
leg runs a control mutant.

### 7.2 After: a Python leg under SPEC-420

```mermaid
flowchart TD
  aList["plan job: mutation_python.py list over the diff"] --> aCap{"share outside 1 to PYTHON_SHARE_CAP?"}
  aCap -->|"yes"| aCapRefuse["REFUSED by name before any deal"]
  aCap -->|"no"| aShards["mutation-verdict.py shards: priced files by cost, unpriced files by the count rule"]
  aTable["PYTHON_CONTROL_SECONDS: the prices verb over one recorded run, re-derived by a test"] --> aShards
  aShards --> aShare{"legs above the share?"}
  aShare -->|"yes"| aRefuse["REFUSED by name, unchanged"]
  aShare -->|"no"| aMatrix["python_matrix output, unchanged"]
  aMatrix --> aLeg["each leg: run --plan --shard k/n --failfast"]
  aLeg --> aControl["leg control: three named mutants, expected KILLED, KILLED, SURVIVED"]
  aControl -->|"any other outcome"| aVoid["every file entry void: control, by name"]
  aControl -->|"as expected"| aPrep["every file first: control, bound, sentinel with its trace"]
  aPrep --> aSets["scoped set per line, or the whole suite"]
  aSets --> aMut["each mutant: its scoped tests in suite order, or the whole suite"]
  aMut -->|"scoped pass"| aConfirm["whole suite confirms before SURVIVED"]
  aMut -->|"kill, or a whole run"| aReport["report.json and costs.jsonl with scope, why and tests"]
  aConfirm --> aReport
  aVoid --> aReport
  aReport --> aVerdict["mutation-verdict.py: unchanged readers"]
```

In `run --all`, the scheduled battery's mode, the leg control runs and every file is prepared
first, but no trace is taken and every mutant runs the whole suite.

### 7.3 One mutant's tests

```mermaid
flowchart TD
  sStart["mutant on line L of file F"] --> sTrace{"F's trace whole?"}
  sTrace -->|"no: failed or partial"| sWhole["whole suite"]
  sTrace -->|"yes"| sModule{"name says in module, or L ran at import or outside a test?"}
  sModule -->|"yes"| sWhole
  sModule -->|"no"| sSet{"any test executed L?"}
  sSet -->|"no"| sWhole
  sSet -->|"yes"| sScoped["tests that executed L, plus every test that joins all sets"]
  sScoped --> sRan{"ran as many tests as the set names?"}
  sRan -->|"no"| sWhole
  sRan -->|"yes, a test failed"| sKilled["KILLED"]
  sRan -->|"yes, all passed"| sWhole
  sWhole --> sOutcome["KILLED or SURVIVED, by the whole suite"]
```

A test joins every scoped set of F when a process it started left no record, when its process image
was replaced by `os.exec`, or when it left a thread running past its end. Byte readers, the tests the
sentinel turns red, stay out of the scoped and the whole runs alike.

### 7.4 The trace records

The trace is taken in the sentinel run: the unmutated lines, with one comment appended. It is taken
inside the leg, on every run.
- **The runner's process** records each test's lines of F through `sys.monitoring` `LINE` events,
  restarted at each test's start. It counts each test's spawns from the process-creation audit
  events.
- **Each child process** gets the same tracer and audit hook from a scratch `sitecustomize`, and
  writes one record when it exits: its pid, its parent's pid, its test, its lines of F, and how
  many processes it started.
- **The judge** reads the directory once, after the runner exits. For each process, it counts the
  records that name it as their parent. A process with fewer records naming it than processes it
  started makes its test join every set.

`formal/tla/TracedSetCovers` checks that a dropped test never executed L, in every order of spawn,
exit, write and read.

### 7.5 What crosses each step

| step | before (dev `e64a778e`) | after (SPEC-420) |
|---|---|---|
| listing | `mutation_python.py list` over the diff | unchanged |
| trace | none | the sentinel run's, per file, per leg, in `run --plan` only |
| scoped set | every mapped module, less the byte readers | the tests that executed the line, or the whole suite on each fallback |
| price | `PYTHON_CONTROL_SECONDS`, empty | the `prices` verb's seven lines over one recorded run's 1017 records, re-derived by a test |
| deal | SPEC-418's cost deal; the count rule for unpriced files | unchanged |
| share | `PYTHON_MAX_SHARDS` on line 122, uncapped | `PYTHON_MAX_SHARDS` alone on its line; `PYTHON_SHARE_CAP = 128` |
| refusal | by name above the share | by name outside 1 to the cap, before any deal; by name above the share, unchanged |
| control | none | three named mutants first in every leg, read through each file entry's `void` |
| verdict | every listed mutant examined; each `void` read by name | unchanged readers; a mutant record names `scope`, `why` and `tests` |

## 8. Scoped prices from a dispatched run (SPEC-421, ADR-435)

Every `path:line` in this section was read at dev `c726af67`. Sections 6 and 7 stay as written.
This section shows what changes around them: a dispatched price run measures each population file
scoped, a verb turns its records into a second table of triples, and the plan reads that table
when its legs run scoped.

### 8.1 Before: a Python leg's price at dev `c726af67`

```mermaid
flowchart TD
  bList["plan job: mutation_python.py list over the diff"] --> bShards["mutation-verdict.py shards"]
  bWhole["PYTHON_CONTROL_SECONDS: seven whole-suite prices from one battery run"] --> bShards
  bShards -->|"priced file: k = 1800 // price - 2; a run weighs (2 + size) x price"| bDeal["python_deal"]
  bShards -->|"unpriced file: the count rule, 20 a leg"| bDeal
  bDeal --> bShare{"legs above PYTHON_MAX_SHARDS?"}
  bShare -->|"yes"| bRefuse["REFUSED by name; no leg starts"]
  bShare -->|"no"| bLeg["each leg: run --plan --shard k/n --failfast"]
  bLeg --> bScoped["each mutant: its scoped tests, or the whole suite"]
  bScoped --> bCosts["costs.jsonl: control and mutant records; no sentinel record; no job"]
  bCosts --> bVerdict["mutation-verdict.py: every listed mutant examined"]
```

The leg runs each mutant scoped, but the plan charged it the whole-suite control:
`scripts/mutation-verdict.py` at 224 s a mutant, six a leg (`scripts/mutation-verdict.py:135-143`,
:1071-1135). Nine listed files had no price, and no run measured a scoped one.

### 8.2 After: the price run and the fill

```mermaid
flowchart TD
  pDispatch["one dispatch of mutation-prices.yml at the delivery's branch"] --> pPop["population job: the files of scripts/mutation-python.json"]
  pRehearsal["pull request rehearsal: one named file; registers the workflow"] --> pPop
  pPop --> pLeg["one leg per file, on ci.yml's mutation-python runner class"]
  pLeg --> pRun["run --file F --scoped --sample 20 --failfast; MUTATION_JOB_ID from job.check_run_id"]
  pRun --> pRecords["costs.jsonl: control, sentinel, uncovered and mutant records; each setup record names run, attempt and job"]
  pRecords --> pUpload["artifact mutation-prices-job-index, uploaded under if: always()"]
  pUpload --> pCopy["the fill copies only the legs whose artifact holds report.json"]
  pCopy --> pFixture["scripts/tests/fixtures/mutation-scoped-prices-run: committed apart from the table"]
  pFixture --> pVerb["mutation-verdict.py prices --scoped --costs DIR"]
  pVerb -->|"no record, no run, no job, a why of all, two jobs or two attempts"| pVoid["VOID; EXIT_FAIL; no table line"]
  pVerb -->|"records read"| pLines["one triple line per population file, each naming run, attempt and job; unpriced lines; examined line"]
  pLines --> pTable["PYTHON_SCOPED_SECONDS, re-derived byte for byte by a test"]
```

The workflow files nothing and judges nothing. A leg cut at its 120-minute limit leaves no
`report.json`, so its file reads `unpriced` by name and keeps its fallback.

### 8.3 After: the plan's choice of price

```mermaid
flowchart TD
  aList["plan job: mutation_python.py list over the diff"] --> aCap{"share outside 1 to PYTHON_SHARE_CAP?"}
  aCap -->|"yes"| aCapRefuse["REFUSED by name before any deal, unchanged"]
  aCap -->|"no"| aSel{"plan in mutation_python.SCOPED_SELECTIONS?"}
  aSel -->|"yes"| aScoped["scoped = PYTHON_SCOPED_SECONDS"]
  aSel -->|"no"| aNone["scoped = None"]
  aScoped --> aPrice["python_price per file: the scoped triple, else the whole price thrice, else None"]
  aNone --> aPrice
  aPrice -->|"a triple"| aCost["setup = control + sentinel; k = (1800 - setup) // each; a run weighs setup + size x each"]
  aPrice -->|"None"| aCount["the count rule, 20 a leg, fail-closed"]
  aCost --> aDeal["python_deal: a leg holding the file adds size x each"]
  aCount --> aDeal
  aDeal --> aShare{"legs above the share of 32?"}
  aShare -->|"yes"| aRefuse["REFUSED by name, unchanged"]
  aShare -->|"no"| aMatrix["python_matrix output; every listed mutant in exactly one leg"]
  aMatrix --> aLine["when a file is priced scoped: python legs priced scoped line"]
```

`prepare` traces on the same value the plan reads, `selection(args) in SCOPED_SELECTIONS`, or on
`--scoped`, so the plan prices a leg scoped exactly when the leg runs scoped. For a whole price the
triple is (p, p, p), so every deal and every printed line is byte-identical to section 6's.

### 8.4 What crosses each step

| step | before (dev `c726af67`) | after (SPEC-421) |
|---|---|---|
| listing | `mutation_python.py list` over the diff | unchanged |
| scoped set | the tests that executed the line, in `run --plan` only | unchanged in `run --plan`; also in a `--file --scoped` price run |
| kept costs | control, uncovered and mutant records naming run and attempt | the setup records also name their job; a traced sentinel writes one sentinel record; a whole run writes none |
| price record | `PYTHON_CONTROL_SECONDS`, seven whole-suite prices | unchanged, as the fallback; `PYTHON_SCOPED_SECONDS`, triples from one dispatched run's committed records, re-derived by a test |
| scoped or whole | always whole | the scoped triple when `plan` is in `SCOPED_SELECTIONS` and the file has one; else the whole price thrice; else the count rule |
| deal | a run weighs (2 + size) x price | a run weighs setup + size x each; byte-identical for a whole price |
| share | `PYTHON_MAX_SHARDS` 32 under `PYTHON_SHARE_CAP` 128 | unchanged |
| refusal | by name above the share; by name at k < 1 | unchanged in text; k read as (1800 - setup) // each |
| verdict | every listed mutant examined; each `void` read by name | unchanged; no mutant leaves the plan, and every shard is counted |
