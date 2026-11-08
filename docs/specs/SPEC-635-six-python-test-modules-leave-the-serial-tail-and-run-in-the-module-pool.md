# SPEC-635: six python test modules leave the serial tail and run in the module pool

- **Issue:** none. **Context(s):** the gate script's python stage runner (`scripts/python_stage.py`)
  and its tests (`scripts/tests/`), with no workflow, crate, web or `scripts/check.sh` change.
- **Decided by:** ADR-659 (six test modules join the module pool, and the serial tail keeps the
  deploy pattern and `test_engine_pin.py`).
- **Schematic:** `docs/schematics/python-stage-module-pool.md`, as SPEC-633 left it: it draws the
  tail by its rule and names no entry, so this change leaves it as it is.
- **Status:** delivered by the pull request that adds this file, with its test, the tail list
  change and `docs/red-first/SPEC-635.md`. **Mutation band:** none: the change adds no mutation
  row. **Model:** none (section 8). Amends SPEC-633 without editing it: R1 here supersedes SPEC-633
  R3 (its section 2) and the first two bullets of SPEC-633 section 5, and every other requirement
  and criterion of SPEC-633 stands, its bound B included.

## 1. The problem, measured

`scripts/python_stage.py` runs each test module of each suite in a pool of module processes, then
the modules its `SERIAL_MODULES` list names, one after another (SPEC-633). At the dev this change
starts from, the list holds eight entries: five with the reason `spawns cargo` and three with the
reason `unmeasured on the box`, the reason text the runner prints for each tail module.

On dev's push run 42337100283, the hygiene job 128296375343 read python 605 s in its `timings.tsv`.
That is over SPEC-633's bound B of 600 s (set against job 127832581467).
That job's suites read `Ran` counts of 1969, 119 and 12 for `scripts/tests`, `tools/parity-oracle` and `agent/tests` (job 128296375343).

Six of the tail's modules are the subject of this change. Each ran alone in the tail, and its own
`Ran` line in that job's `python.log` reads:

| module | tests | its own time |
|---|---|---|
| `test_bin_kind_census.py` | 9 | 12.857 s (job 128296375343) |
| `test_check_gate.py` | 11 | 1.053 s (job 128296375343) |
| `test_ci_workflows.py` | 99 | 118.765 s (job 128296375343) |
| `test_dispatch_shards.py` | 44 | 67.574 s (job 128296375343) |
| `test_fork_lock_guard.py` | 7 | 0.570 s (job 128296375343) |
| `test_mutation_rows.py` | 69 | 56.513 s (job 128296375343) |

Together they held 257.332 s of the serial tail (job 128296375343).

**Why each may share a run.** Each module's calls were read at their source, at the dev this
change starts from, for a shared build directory, a write to the lock and state outside the
module's own process:

- `test_bin_kind_census.py`: its one cargo call is `cargo metadata --no-deps --offline`
  (`scripts/tests/test_bin_kind_census.py:652`), and its `rustc` calls write dep-info into each
  planted crate (`:1069`). It builds nothing and writes into no shared target directory.
- `test_check_gate.py`: cargo is a stub in every test but one, which runs the gate's `audit-rust`
  stage with the real cargo (`scripts/tests/test_check_gate.py:739`) over a two-package workspace of
  path dependencies that the test writes in its own temporary directory (`:703`).
- `test_ci_workflows.py`: it calls no cargo. Each shell call runs with an environment built from
  `PATH` alone (`scripts/tests/test_ci_workflows.py:1880`), or with a temporary `GITHUB_OUTPUT`
  beside it (`:1928`), or inside a temporary directory of its own (`:3615-3624`).
- `test_dispatch_shards.py`: cargo is a stub it plants under its own root
  (`scripts/tests/test_dispatch_shards.py:1582`), every write lands in a temporary directory, and
  its one `PATH` change is a `mock.patch.dict`, which restores it (`:1846`).
- `test_fork_lock_guard.py`: its one cargo call is `cargo metadata --locked` at the repository
  root (`scripts/tests/test_fork_lock_guard.py:115-117`), which resolves the lock without a build
  and may not write it.
- `test_mutation_rows.py`: its run helper drops `CARGO_TARGET_DIR` from each child's environment
  (`scripts/tests/test_mutation_rows.py:165`), so each fixture crate builds into a target folder of
  its own inside a temporary directory.

After the six moves no pooled module shares a build directory with another. What they still share
is cargo's package cache lock, which a cargo process holds while it resolves. Every resolution among
the six is offline, path-only or free of registry packages but one, `test_fork_lock_guard.py`'s
whole-workspace `cargo metadata --locked`. The three moved modules that run `cargo metadata` took
12.857 s, 1.053 s and 0.570 s as whole modules (job 128296375343), so a wait on that lock is bounded by those times.

**The two kept entries.** `test_engine_pin.py` (`spawns cargo`) builds with the default target
directory, the one build directory a pooled module could share, so it stays in the tail. The deploy
pattern `test_deploy_*.py` keeps the reason SPEC-633 R3 gives it.

**Load.** In the pool the moved modules compete for the runner's CPUs: `test_dispatch_shards.py`
starts several bash workers at once, and `test_mutation_rows.py` compiles fixture crates. Each of
their calls that could slow under load runs under a timeout of its own, and the pull request's
`hygiene` run measures them under that load.

## 2. Requirements

- **R1.** `SERIAL_MODULES` in `scripts/python_stage.py` holds exactly two entries:
  `test_deploy_*.py` with the reason `unmeasured on the box`, and `test_engine_pin.py` with the
  reason `spawns cargo`. Every other module, the six of section 1 among them, runs in the pool. R1
  supersedes SPEC-633 R3 (SPEC-633 section 2) and the first two bullets of SPEC-633 section 5 ("What
  this does NOT cover"); SPEC-633 R1, R2 and R4 to R7 stand as they are.

The rollback is a revert of this change, or `PYTHON_STAGE_JOBS=1` (SPEC-633 R2), which runs every
module one after another.

## 3. Acceptance criteria of SPEC-635

| id | criterion | decided by |
|---|---|---|
| A1 | one suite planted with the six modules of section 1, a module for each kept entry and one other module runs every planted module and prints each one's mark in module-name order; its verdict line counts 9 tests with exit 0, and its `serial:` lines name the two kept entries alone, in name order (R1) | `python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_only_the_two_kept_entries_run_in_the_tail` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_only_the_two_kept_entries_run_in_the_tail
```

SPEC-633 A2's test keeps every assertion. The tail it plants is the two kept entries, and its first
case plants `test_engine_pin.py` where it planted `test_ci_workflows.py`. That case is not red,
since the runner already kept `test_engine_pin.py` in the tail.

The pull request's own `hygiene` run accepts the stage as a whole. Its `python.log` reads the base
run 42337100283's Ran count per suite (1969, 119 and 12) plus the one test this change adds, its
`serial:` lines name no module of section 1, and its `timings.tsv` reads python at 600 s or less (B, against job 127832581467).

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/python_stage.py` | gate scripts | changed: `SERIAL_MODULES` keeps two entries, and its comment cites R1 |
| `scripts/tests/test_python_stage.py` | tests | changed: `TAIL` keeps the two entries, A2's first case plants `test_engine_pin.py`, and the A1 test is added |
| `docs/specs/SPEC-635-six-python-test-modules-leave-the-serial-tail-and-run-in-the-module-pool.md` | records | added: this SPEC |
| `docs/decisions/ADR-659-six-test-modules-join-the-module-pool-and-the-serial-tail-keeps-the-deploy-pattern-and-the-engine-pin.md` | records | added: its decision |
| `docs/red-first/SPEC-635.md` | records | added: the red-first record |
| `changelog.d/python-stage-tail-635.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- `test_engine_pin.py` in the pool: it would need a target directory of its own, which would change
  what its tests check. It stays in the tail.
- The deploy pattern: its entry and its reason stay as SPEC-633 states them.
- The workflow and the gate script: no job, step, leg, ceiling or timeout of `.github/workflows/`
  changes, `scripts/check.sh` is not edited, and B stays SPEC-633's.

## 6. Risks

- A moved module that changes state outside its own process. Answer: section 1 read each module's
  calls; a module that adds such a call belongs back in the tail.
- The package cache lock: `test_fork_lock_guard.py`'s `cargo metadata --locked` may wait while
  another pooled cargo process resolves. Answer: the other resolutions are short (section 1), and
  the pull request's `hygiene` run reads that module's own time in the pool.
- A stage over B under load. Answer: the pull request's `hygiene` run reads the stage's time
  against B; a revert restores the eight-entry tail, and `PYTHON_STAGE_JOBS=1` restores the serial
  stage without a revert.

## 7. What only CI proves

The pooled runs of `test_ci_workflows.py` and `test_dispatch_shards.py`, which a local run may not
run, the stage's time against B and the Ran counts: the pull request's `hygiene` run reads them in
its `check-stage-logs-hygiene` artifact. The `mutation-python` legs prove the tests of
`test_python_stage` against the runner's mutants.

## 8. Formal model

None. The change edits one list of the runner, which has no protocol or state machine of its own,
and no formal entry covers `scripts/python_stage.py`.

## 9. Mutation rows

None: the runner stays mapped to `test_python_stage` in `scripts/mutation-python.json`, and its
mutants are dealt to the `mutation-python` legs.
