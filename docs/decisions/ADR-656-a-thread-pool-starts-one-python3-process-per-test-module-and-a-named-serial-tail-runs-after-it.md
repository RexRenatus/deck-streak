---
status: "accepted"
---

# ADR-656: a thread pool starts one python3 process per test module, and a named serial tail runs after it

Decides SPEC-633.

## Context and Problem Statement

The python stage of `scripts/check.sh` ran its three suites as three `unittest discover` calls,
one after another, and it is the floor of the `hygiene` job: dev's push run 42199982313 read python 1121 s in hygiene job 127832581467.
Most test modules share nothing, so they can run at once; a few spawn cargo, and some a local run
may not run, so nothing shows they can. SPEC-633 section 1 gives the census that sorts them. The
stage must keep SPEC-054 R7 (a suite that runs no test fails) and its verdict line.

## Decision Drivers

- Each test module runs alone in its process, so no module sees another's working directory,
  environment or imported state.
- The modules that cannot share a run are named in one list, each with its reason.
- The stage's verdict line, its exit and SPEC-054 R7 stay as they are, and a log reads the same at
  any degree of concurrency.
- A stand-in `python3` on PATH still answers every module run, as the gate's own test needs.
- One setting restores the serial stage without a revert.

## Considered Options (the alternatives each was chosen against)

- Chosen: `concurrent.futures.ThreadPoolExecutor` with `PYTHON_STAGE_JOBS` workers (default
  `os.cpu_count()`), each worker starting one `python3 -m unittest discover -s <suite> -p
  <module file>` by `subprocess.run` and holding its output, then the `SERIAL_MODULES` tail one by
  one, because each module gets a process of its own while the pool's threads only wait.
- `ProcessPoolExecutor`: rejected because its workers are long-lived processes that would import
  and run many modules each, so modules would share a process again, unless each worker started a
  child anyway, which a thread does as well with no pickling.
- One `unittest` process running test cases on threads: rejected because the modules share module
  state, the working directory and `os.environ` in one process.
- A third-party parallel runner: rejected because the gate scripts use the standard library only,
  and it would change how the suites are found.
- Splitting the suites over more CI jobs: rejected because it adds a job to the workflow, and each
  job pays its own setup.

## Decision Outcome

Chosen option: a thread pool of module processes and a serial tail. `scripts/python_stage.py` lists
each suite's `test_*.py` modules in name order, names each tail module on one `serial:` line, runs
the other modules in the pool and then the tail modules one by one, prints each module's output in
module-name order, then the verdict line. `stage_python` calls it with the three suites.
`SERIAL_MODULES` lists five modules that spawn cargo (`test_bin_kind_census.py`,
`test_check_gate.py`, `test_engine_pin.py`, `test_fork_lock_guard.py` and `test_mutation_rows.py`)
and three entries a local run may not measure (`test_ci_workflows.py`, `test_dispatch_shards.py`
and the deploy class as the pattern `test_deploy_*.py`), each with its reason. A CI census of the
tail cleared one module, `test_memory_scope.py`, which left the tail for the pool; of the other
entries, four read hits not yet resolved, three run cargo under a timeout, and the deploy class
stays whole. Each module process starts with `PYTHONDONTWRITEBYTECODE=1` added to its inherited
environment, so no pooled module leaves bytecode under the tree for a tail module or the secrets
stage to read.

### Consequences

- Good: the pooled modules run at once, up to the runner's CPU count.
- Good: the verdict line, the stage's exit and SPEC-054 R7 hold as before, and a red module still
  shows its whole output.
- Neutral: a module's output appears when the module ends, in module-name order, never
  interleaved.
- Bad: the tail still runs serially, and its modules include some of the slowest.

### Confirmation

SPEC-633 A1 is red against a runner that keeps every input and runs serially, and green once the
pool runs modules at once. A2 pins the verdict line and the tail lines. A3, SPEC-054 A13's test,
is red once the stage calls the runner, until its planted tree carries the runner. A4 is red while
module processes write bytecode, and green once each starts with bytecode writes off. The pull
request's `hygiene` run reads the stage's time against B, 600 s (job 128188919143).

## What would make this wrong

A pooled module that changes state outside its own process, such as a file at a fixed path that
another module reads. The census found none; a module that adds one belongs in the tail.

## More Information

SPEC-633 states the requirements and the criteria. SPEC-054 R7 and A13 are the stage's rule and the
gate's own test of it. `PYTHON_STAGE_JOBS=1` is the rollback.
