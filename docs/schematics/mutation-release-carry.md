# Schematic: the release run carries a Python outcome from one base run, and deals only the rest

Kind: data flow (the release run's Python path through `mutation-plan`, the Python legs and
`mutation-verdict`, before and after the carry), with one population file's carry decision drawn
as a flowchart. Read at DeckStreak `dev` `75d555c4` (`.github/workflows/ci.yml`,
`scripts/mutation-verdict.py`, `scripts/mutation_python.py`, `scripts/mutation_rows.py`). Decided
by ADR-511; built by SPEC-493.

This ADDS to `docs/schematics/mutation-testing-python.md` and redraws none of it: the Python legs,
the equivalence records and the verdict's Python class are drawn there, and here each appears as
one node. Rust, rows and web run as that schematic and `docs/schematics/mutation-testing.md` draw
them, and carry nothing.

## 1. Before: every listed Python mutant runs

At `75d555c4` the plan step (`ci.yml:399-409`) writes `plan.json` and `git.diff`, the runner lists
the diff's Python mutants and the whole population's (`ci.yml:435-438`), and `shards`
(`ci.yml:439-441`) deals every listed mutant into legs. The plan's artifact keeps the whole
`mutation/` directory (`ci.yml:449-455`).

```mermaid
flowchart TD
  plan["mutation-plan: mutation-verdict.py plan writes plan.json and git.diff"] --> pylist["mutation_python.py list --plan: python-listed.json"]
  plan --> pywhole["mutation_python.py list --all: python-whole.json"]
  pylist --> shards["mutation-verdict.py shards --python-listed python-listed.json"]
  shards --> legs["mutation-python and mutation-python-release: one leg per shard k"]
  legs --> reports[("mutation-python-shard-k: report.json")]
  reports --> judge["mutation-verdict: judge --class oracle, then --class scripts, each with --python"]
  pywhole --> judge
```

## 2. After: listing, carry, deal and judge

The listing steps, `shards`, `planned_leg` (`scripts/mutation_python.py:1252`) and the runner are
unchanged. `carry` runs on every event and writes `carry.json`; only on an admitted release run does
it find a base and carry anything. The union rule lives in `python_reports`
(`scripts/mutation-verdict.py:1970`), which `judge_python` (`:2139`) calls for each Python class.

```mermaid
flowchart TD
  plan["mutation-plan: plan writes plan.json and git.diff"] --> pylist["mutation_python.py list --plan: python-listed.json, the listing, unchanged"]
  plan --> pywhole["mutation_python.py list --all: python-whole.json"]
  plan --> admitted{"the release run of a pull request that CARRY_PULL_REQUESTS names?"}
  admitted -->|no| nothing["carry.json holds nothing and says why by name, and plan.json is unchanged"]
  admitted -->|yes| runs["the base step: this repository's ci runs from dev into main, listed by run id: runs.json"]
  runs --> pick["mutation_carry.py base: the newest qualifying run older than this one, no outcome read"]
  pick --> compare["the forge's compare of its head with this head, and its artifacts by name: compare.json, artifacts.json"]
  compare --> ancestor{"one candidate, its head an ancestor of this head, its plan artifact listed?"}
  ancestor -->|no| nothing
  ancestor -->|yes| fetch["downloads by its run id into carry-base: mutation-plan, and mutation-outcomes or else every mutation-python-shard report"]
  fetch --> carry["mutation-verdict.py carry: tree identity, key, read sets, each file all or none"]
  pylist --> carry
  carry --> carryjson[("carry.json: the carried names, their outcomes and killers, and the base run")]
  carry --> legsview[("plan.json: each carried file's code lines moved to carried_code")]
  nothing --> rerun
  legsview --> rerun["mutation_python.py list --plan: python-rerun.json"]
  rerun --> shards["mutation-verdict.py shards --python-listed python-rerun.json"]
  shards --> legs["mutation-python and mutation-python-release: one leg per shard k, unchanged"]
  legs --> reports[("mutation-python-shard-k: report.json")]

  subgraph verdict ["mutation-verdict: after every leg, if always"]
    reports --> union{"python_reports: the listed names equal the re-run names plus the carried ones, none in both?"}
    carryjson --> union
    pylist --> union
    union -->|no| voidunion(["VOID by name, with the three counts"])
    union -->|yes| judge["judge --class oracle, then --class scripts: a carried outcome judged as a re-run one, the records at the head"]
    pywhole --> judge
    judge --> athead["legs and the rows retirement check: at the head, never carried"]
    judge --> manifest["mutation-verdict.py manifest, if always"]
    manifest --> kept[("mutation-outcomes: the tree identity, each class's key, the whole and carried entries")]
  end

  kept -.->|"a later release run's base record"| fetch
```

## 3. One population file's carry

All of a file's listed mutants carry, or none does (SPEC-493 R8). Every refusal prints the
file's reason by name, and the file runs whole.

```mermaid
flowchart TD
  file["a population file f with listed mutants"] --> base{"a base, whose plan is a release plan with this plan's base?"}
  base -->|no| whole(["every listed mutant of f runs, its reason printed by name"])
  base -->|yes| key{"the key of f's class equal: the leg jobs' text, the runner's bytes, the runner digest of mutation_rows.py?"}
  key -->|no| whole
  key -->|yes| entry{"f's map entry equal, and the census at the head clean?"}
  entry -->|no| whole
  entry -->|yes| tree{"is the read set S(f) the whole tree?"}
  tree -->|yes| anypath{"did any path at all change between the base and the head?"}
  tree -->|no| changed{"did a path of S(f), present or absent, or a listing of it, change?"}
  anypath -->|yes| whole
  changed -->|yes| whole
  anypath -->|no| judged{"was each listed mutant of f judged once at the base as killed, survived, uncovered or unviable, in a whole leg or by its manifest?"}
  changed -->|no| judged
  judged -->|no| whole
  judged -->|yes| carried(["every listed mutant of f carries, with its base outcome and killers"])
```

## 4. What crosses each step

| from | to | what | shape |
|---|---|---|---|
| the forge's runs listing | `mutation_carry.py base` | this repository's `ci` runs for pull requests from `dev` into `main` | `runs.json` |
| the forge's compare and artifacts listing | `mutation_carry.py base` | whether the candidate's head is an ancestor of this head, and which artifacts it kept | `compare.json`, `artifacts.json` |
| the base run | `carry` | its plan, its diff and its listing | `carry-base/mutation-plan/`: `plan.json`, `git.diff`, `python-listed.json` |
| the base run | `carry` | its manifest, or else its legs' reports | `carry-base/mutation-outcomes/mutation-outcomes.json`, or `carry-base/legs/mutation-python-shard-k/report.json` |
| `carry` | the re-run listing | the plan with each carried file's `code` lines in `carried_code` | `plan.json` |
| `carry` | `mutation-verdict` | the carried names, outcomes and killers, and the base's run id | `carry.json`, schema `deckstreak.mutation-carry.v1`, in the `mutation-plan` artifact |
| the runner | `shards` | the re-run set | `python-rerun.json` |
| `mutation-plan` | `mutation-verdict` | the listing the union rule holds the legs and the carry to | `python-listed.json` |
| the Python legs | `mutation-verdict` | one report per leg | `mutation-python-shard-k/report.json` |
| `mutation-verdict` | a later release run | this run's tree identity, keys and judged entries | `mutation-outcomes`, schema `deckstreak.mutation-outcomes.v1` |

## 5. What is never carried

- Rust, rows and web outcomes: they run as before.
- A `timeout` or `void` outcome, and any mutant of a partial or VOID leg.
- Any outcome on a run that is not admitted.
- The equivalence records, the rows retirement check and `legs`: each is evaluated at the head.

## 6. How `ci` reads the release run's jobs (SPEC-500, ADR-518)

Every step above runs on the release run as drawn, and its results reach the run as before: the legs' reports, the
verdict's judgement and `mutation-outcomes`, which `mutation-verdict` uploads whatever the legs returned, so a later
release run can carry from this one. What changes is `ci`'s reading on that run alone, drawn in
`docs/schematics/mutation-testing.md` section 13: `base-is-dev` names the release run by `release_run`'s predicate,
and `ci` prints `mutation-plan`'s, the Python legs' and `mutation-verdict`'s results, with the other mutation jobs',
as advisory lines and admits each by value. A carry refused by name, a VOID leg or a survivor still reads red in its
own job and its report on the release run; on every other run a red mutation job still fails `ci`.
