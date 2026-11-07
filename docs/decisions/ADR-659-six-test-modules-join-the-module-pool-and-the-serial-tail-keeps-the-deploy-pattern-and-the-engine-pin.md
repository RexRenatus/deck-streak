---
status: "accepted"
---

# ADR-659: six test modules join the module pool, and the serial tail keeps the deploy pattern and the engine pin

Decides SPEC-635.

## Context and Problem Statement

`scripts/python_stage.py` runs the python stage's test modules in a pool of module processes and
then the modules its `SERIAL_MODULES` list names, one after another (SPEC-633, ADR-656). The list
held eight entries, and the stage stayed over its bound B: dev's push run 42337100283 read python 605 s in hygiene job 128296375343, against B's 600 s (job 127832581467).
SPEC-635 section 1 reads each tail module's calls for a shared build directory, a write to the
lock and state outside its own process, and finds six that can share a run.

## Decision Drivers

- A module leaves the tail only when its calls share no build directory and write no lock.
- A module whose cargo builds use the default target directory keeps its place in the tail.
- The runner's rule, its `serial:` lines, its verdict line and B stay as SPEC-633 states them.
- One setting still restores the serial stage without a revert.

## Considered Options (the alternatives each was chosen against)

- Chosen: delete six entries from `SERIAL_MODULES` (`test_bin_kind_census.py`,
  `test_check_gate.py`, `test_ci_workflows.py`, `test_dispatch_shards.py`,
  `test_fork_lock_guard.py` and `test_mutation_rows.py`) and keep two (`test_deploy_*.py`,
  `unmeasured on the box`, and `test_engine_pin.py`, `spawns cargo`), because each of the six
  builds nothing into a shared target directory and writes no lock.
- Keep `test_fork_lock_guard.py` in the tail as well: rejected because its one cargo call, a
  whole-workspace `cargo metadata --locked`, builds nothing and may not write the lock; the
  package cache lock it takes is measured by the pull request's own `hygiene` run instead.
- Give `test_engine_pin.py` a target directory of its own and pool it too: rejected because its
  tests check a build against the workspace's own target, so a target of its own would change
  what they check, and a cold build there is a cost this change has not measured.
- Pool the deploy pattern as well: rejected because the reason SPEC-633 R3 gives it still holds.
- Raise B instead: rejected because B is SPEC-633's bound, and this change moves no gate figure.

## Decision Outcome

Chosen option: six modules join the pool. `SERIAL_MODULES` holds `test_deploy_*.py` with the
reason `unmeasured on the box` and `test_engine_pin.py` with the reason `spawns cargo`, and every
other module runs in the pool (SPEC-635 R1). `test_engine_pin.py` keeps its place because its
builds use the default target directory, the one build directory a pooled module could share.

The six still share cargo's package cache lock, which a cargo process holds while it resolves.
`test_fork_lock_guard.py`'s whole-workspace `cargo metadata --locked` takes it, and the other moved
modules' resolutions are offline, path-only or free of registry packages. The pull request's own
`hygiene` run is the measurement of that lock: its `python.log` gives `test_fork_lock_guard.py`'s
own time in the pool beside the stage's time against B.

### Consequences

- Good: the six modules run at once with the rest of the pool, so the tail holds only the
  modules that cannot share a run.
- Good: the runner's rule, its output and SPEC-054 R7 hold as before; only the list changes.
- Neutral: the moved modules compete for the runner's CPUs with the other pooled modules.
- Bad: `test_engine_pin.py` still runs alone after the pool.

### Confirmation

SPEC-635 A1 is red against the runner that still lists eight entries, whose `serial:` lines name
the six moved modules, and green once the list keeps the two. SPEC-633 A2's test keeps every
assertion over the two kept entries. The pull request's `hygiene` run reads the stage's time
against B and `test_fork_lock_guard.py`'s own time in the pool.

## What would make this wrong

A moved module that waits on the package cache lock long enough to push the stage over B, or one
that gains a build into a shared target directory. The pull request's `hygiene` run reads the
first; a module with the second belongs back in the tail.

## More Information

SPEC-635 states the requirement and the criterion. SPEC-633 and ADR-656 are the runner's own
records and stay as they are. A revert of this change, or `PYTHON_STAGE_JOBS=1`, is the rollback.
