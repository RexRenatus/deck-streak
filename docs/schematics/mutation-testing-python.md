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
  pylist --> pyshards["mutation-verdict.py shards --python-listed: ceiling of listed over 40, clamped 1 to 8"]
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
