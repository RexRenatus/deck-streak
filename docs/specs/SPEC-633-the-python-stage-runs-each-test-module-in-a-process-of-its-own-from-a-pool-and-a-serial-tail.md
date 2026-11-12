# SPEC-633: the python stage runs each test module in a process of its own, from a pool and a serial tail

- **Issue:** none. **Context(s):** the gate script's python stage (`scripts/check.sh`,
  `stage_python`) and the tests of the gate scripts (`scripts/tests/`), with no workflow, crate or
  web change.
- **Decided by:** ADR-656 (a thread pool starts one python3 process per test module, and a named
  serial tail runs after it).
- **Schematic:** `docs/schematics/python-stage-module-pool.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests, the runner and
  `docs/red-first/SPEC-633.md`. **Mutation band:** none: the change adds no mutation row.
  **Model:** none (section 8). Builds on SPEC-054 R7 and A13 and keeps both: A13's test keeps
  every assertion, and its planted tree gains the runner (R7 here).

## 1. The problem, measured

At the dev this change starts from, `stage_python` in `scripts/check.sh` runs three
`python3 -m unittest discover` calls one after another, one per suite, and the stage is the floor
of the `hygiene` job. On dev's push run 42199982313, the hygiene job 127832581467 read python 1121 s in its `timings.tsv`.
That job's suites read `Ran 1902`, `Ran 119` and `Ran 12` for `scripts/tests`, `tools/parity-oracle` and `agent/tests` (job 127832581467).
The suites hold 128, 6 and 1 `test_*.py` modules, none of them in a subfolder.

Before any runner change, a local run of that dev tree measured which modules can share a run.
It gives counts only here: a local run has no run or job id, so none of its durations is quoted.

**(i) The static census.** One `git grep` over the three suites, each hit opened at its line:

| class | `scripts/tests` | `tools/parity-oracle` | `agent/tests` | what the hits do |
|---|---|---|---|---|
| `os.chdir` | 12 hits in 3 modules | 0 | 0 | change the working directory of their own process only |
| writes to `os.environ` | 56 hits in 16 modules | 8 hits in 3 modules | 0 | in-process, most of them by `mock.patch.dict`, which restores the environment |
| fixed paths in the system temporary directory | 10 hits in 5 modules | 0 | 0 | fixture text and assertion patterns; no module writes a fixed path |
| socket binds | 6 hits in 3 modules | 2 hits in 2 modules | 0 | every bind asks for port 0, so the system picks a free port |
| writes under the repository root | 1 hit in 1 module | 1 hit in 1 module | 0 | each copies a file from the repository into a temporary root |

A module that runs in a process of its own keeps its working directory and its environment to
itself, so no module needs a temporary-directory change.

**The cargo census.** Each module the local run may run (122, 6 and 1 of them, the deploy class and
three modules held to CI left out) ran alone, with a `cargo` and a `rustup` stand-in first on PATH
that each record their arguments and exit 3. Two planted controls ran first and both fired: a module
that calls `cargo` was recorded, and one that calls `rustup run stable cargo` was recorded and built
nothing. Four modules called the stand-in: `test_bin_kind_census.py`, `test_check_gate.py`,
`test_fork_lock_guard.py` and `test_mutation_rows.py`. A fifth, `test_engine_pin.py`, builds the
ingest crate with `cargo build --locked` by its source; on the local run its two tests failed first
for want of `protoc` and `cargo-deny`, so the stand-in never saw it. `test_setting_shapes.py` runs
`rustc` with its output inside temporary directories of its own, shares nothing, and stays in the
pool. `git status --porcelain` read empty after the census.

**(ii) Three runs.** The 125 modules left (the 129 less the four that called the stand-in) ran
three times under the stand-ins: one process at a time, four processes at once, and four at once in
reverse module order. All three read the same Ran count per suite (1529, 119 and 12) and the same
16 failing test ids, every one in `test_engine_pin.py` or `test_setting_shapes.py`, whose tests need
tools the local run lacks. `git status --porcelain` read empty after each run, and no run called a
stand-in.

**(iii) Per-module time.** The one-at-a-time run timed each module, and the bound B below follows
from the slowest of them.
The slowest module's time plus 60 s stays below 600 s, so B is 600 s (set against the serial figure of job 127832581467).

**The serial tail.** These modules never share a run, and run one after another after the pool:

| entry | reason |
|---|---|
| `test_bin_kind_census.py` | spawns cargo |
| `test_check_gate.py` | spawns cargo |
| `test_engine_pin.py` | spawns cargo |
| `test_fork_lock_guard.py` | spawns cargo |
| `test_mutation_rows.py` | spawns cargo |
| `test_ci_workflows.py` | unmeasured on the box |
| `test_deploy_*.py` | unmeasured on the box |
| `test_dispatch_shards.py` | unmeasured on the box |

Run at once, the modules that spawn cargo would contend for one target directory and for the
runner's disk. The three "unmeasured" entries are modules a local run may not run (the deploy class
by its pattern); they stay in the tail until a CI run of the stand-in census clears them. The
reason text is what the runner prints.

**The tail census.** This pull request's first pooled `hygiene` run timed every tail module.
Its stage read 576 s, 348.980 s of it in the tail and 227.020 s in the pool (job 128188919143).
Each tail entry but the deploy class was then read at its source for the static census's classes,
plus `CARGO_TARGET_DIR`, `cargo clean` and a timeout on a call that runs cargo. Of the nine entries
the tail then held, one read cleared: `test_memory_scope.py`, whose one environment write is a
restored `mock.patch.dict` and whose timeouts are workflow text, so it now runs in the pool. Four
read hits not yet resolved (`test_bin_kind_census.py`, `test_ci_workflows.py`,
`test_dispatch_shards.py` and `test_mutation_rows.py`), three run cargo under a timeout
(`test_check_gate.py`, `test_engine_pin.py` and `test_fork_lock_guard.py`), and the deploy class
stays whole in the tail.

**Bytecode.** The same run read two reds that the pool caused. A module that imports a sibling
module left its bytecode under the tree, so a tail module that scrubs the deploy templates found
five `.pyc` files, and the secrets stage after the python stage read one finding, which a local
scan of a local run's bytecode traced to a test module's `.pyc` file. Each module process
therefore starts with bytecode writes off.

**The bound B** is the larger of 600 s and the slowest module's own time plus 60 s, since one module alone above 540 s would put 600 s out of reach; it is 600 s here, against dev's serial 1121 s (job 127832581467).
The pooled stage read 576 s, its tail 348.980 s and its pool 227.020 s (job 128188919143).
The one move the census clears projects 570.47 s (job 128188919143), so the maintainers set B at 600 s.

## 2. Requirements

- **R1.** `scripts/python_stage.py <suite>...` lists each suite's top-level `test_*.py` modules in name order and runs each one as `python3 -m unittest discover -s <suite> -p <module file>`, in a process of its own. `python3` is the one PATH finds, never `sys.executable`, so a stand-in on PATH answers every module run. Each module process inherits the stage's environment with `PYTHONDONTWRITEBYTECODE=1` added, so no module writes bytecode under the tree.
- **R2.** The modules outside the serial tail run in a pool of at most `PYTHON_STAGE_JOBS` processes at once, by default `os.cpu_count()`. `PYTHON_STAGE_JOBS=1` runs them one after another, which is the rollback.
- **R3.** `SERIAL_MODULES` holds each tail entry with its reason, and an entry may be an `fnmatch` pattern: the eight entries of section 1's table, five that spawn cargo and three unmeasured. The tail's modules run one after another after the pool, and the runner first prints one `serial: <suite>/<module> (<reason>)` line per tail module.
- **R4.** The runner prints each module's output in module-name order, suite by suite in argument order, the tail's modules among them, then the stage's verdict line `python: <suite> ran N test(s), exit c; ...`. N is the sum of each module's last `Ran N` line (0 for a module that printed none), and c is the suite's first non-zero module exit, else 0.
- **R5.** A suite with no module, or one that ran 0 tests, fails the stage, and so does a non-zero module exit: the runner then exits 1, else 0 (SPEC-054 R7).
- **R6.** `stage_python` keeps `need_python && need_cargo` and its comments, keeps its suite loop over `scripts/tests tools/parity-oracle agent/tests`, and calls `python3 scripts/python_stage.py "${suites[@]}"` with the suites it collected; no other line of `scripts/check.sh` changes.
- **R7.** SPEC-054 A13's test, `ThePythonStageRunsEverySuite`, plants the runner beside its copy of `scripts/check.sh`. Its python3 stand-in answers a `-m unittest` call with `Ran 0 tests` and hands every other call to the real python3, found before the test narrows PATH. Every assertion keeps its text.
- **R8.** After each module's output, the runner prints one line that names the module, `module: <suite>/<module> ran N test(s), OK`, where N is that module's own last `Ran N` (0 for a module that printed none), with `FAILED (exit c)` in place of `OK` when the module exits c, not 0. So one module's result is read by name in `python.log`, and the verdict line stays the last line.

R1 to R5 and R8 are the runner; R6 is the stage's one call; R7 keeps the gate's own test of the
stage true to the stage it now runs.

## 3. Acceptance criteria of SPEC-633

| id | criterion | decided by |
|---|---|---|
| A1 | two planted modules that each wait up to ten seconds for the other both pass when the runner runs two modules at once, and the first times out when `PYTHON_STAGE_JOBS=1` runs them one after another (R1, R2) | `python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_two_modules_run_at_once` |
| A2 | the verdict line gives each suite's tests and exit in argument order; a red module, a suite with no module and a module that ran no test each fail the stage; each module's output comes in module-name order; every tail entry, the pattern too, runs and is named with its reason (R3, R4, R5) | `python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_the_verdict_line_sums_each_suite` |
| A3 | SPEC-054 A13's four cases hold over the runner: a red suite leaves the others run and named, a suite that ran no test fails by name, so does one whose unittest exits 0 having run nothing, and green suites pass (R6, R7) | `python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k ThePythonStageRunsEverySuite` |
| A4 | a planted test module that imports a planted sibling module leaves no `.pyc` file and no `__pycache__` folder under its suite, even when the stage's own environment allows bytecode writes (R1) | `python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_module_processes_write_no_bytecode` |
| A5 | after each module's output, one line names the module with its own tests and its `OK`, or `FAILED` with its exit, pooled and tail modules alike, in module-name order, each after its own module's `Ran` line (R8) | `python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_each_module_is_named_beside_its_ran_line` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_two_modules_run_at_once
A2: python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_the_verdict_line_sums_each_suite
A3: python3 -m unittest discover -s scripts/tests -p test_check_gate.py -k ThePythonStageRunsEverySuite
A4: python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_module_processes_write_no_bytecode
A5: python3 -m unittest discover -s scripts/tests -p test_python_stage.py -k test_each_module_is_named_beside_its_ran_line
```

A2 pins rules the serial runner already keeps, so it is recorded `not red` with its reason.

The pull request's own `hygiene` run accepts the stage as a whole. Its `python.log` reads the same Ran
count per suite as the base's run 42199982313 (1902, 119 and 12), plus the tests this change adds and
any that dev adds before the run, and its `timings.tsv` reads python at 600 s or less (B, against job 127832581467).

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/python_stage.py` | gate scripts | added: the module runner, its pool, its serial tail and its module lines |
| `scripts/check.sh` | gate scripts | changed: `stage_python` hands the suites of its loop to the runner |
| `scripts/mutation-python.json` | mutation scripts | changed: one entry maps the runner to `test_python_stage` |
| `scripts/tests/test_python_stage.py` | tests | added: the A1, A2, A4 and A5 tests |
| `scripts/tests/fixtures/python_stage/barrier_a.py` | tests | added: a planted module for A1 |
| `scripts/tests/fixtures/python_stage/barrier_b.py` | tests | added: the other planted module for A1 |
| `scripts/tests/test_check_gate.py` | tests | changed: `python_tree` copies the runner, and `EMPTY_UNITTEST` hands every call but `-m unittest` to the real python3 |
| `docs/specs/SPEC-633-the-python-stage-runs-each-test-module-in-a-process-of-its-own-from-a-pool-and-a-serial-tail.md` | records | added: this SPEC |
| `docs/decisions/ADR-656-a-thread-pool-starts-one-python3-process-per-test-module-and-a-named-serial-tail-runs-after-it.md` | records | added: its decision |
| `docs/schematics/python-stage-module-pool.md` | records | added: the stage's schematic |
| `docs/red-first/SPEC-633.md` | records | added: the red-first record |
| `changelog.d/python-stage-module-pool-633.md` | records | added: the changelog fragment |

## 5. What this does NOT cover

- Clearing the "unmeasured" entries: a CI run of the stand-in census over the three entries a local
  run may not run would show which of them can join the pool. Until then they stay in the tail.
- Pooling the modules that spawn cargo: each would need a target directory of its own, and the
  runner's disk would need room for them at once. They stay in the tail.
- The workflow: no job, step, leg, ceiling or timeout of `.github/workflows/` changes, and the
  `hygiene` job runs `scripts/check.sh` as before.

## 6. Risks

- A module that changes shared state outside its own process. Answer: the static census found none
  (section 1), and the three runs read equal failure sets and Ran counts in either order.
- A tail long enough that the stage misses B. Answer: the pull request's `hygiene` run measures it,
  and `PYTHON_STAGE_JOBS=1` restores the serial stage without a revert.
- A `PYTHON_STAGE_JOBS` that is not a positive integer. Answer: the runner fails with Python's own
  error, and the stage reads red.
- Output order. Answer: each module's output is held until the module ends and is printed in
  module-name order, so a log reads the same at any `PYTHON_STAGE_JOBS`.

## 7. What only CI proves

The stage's time, B and the Ran counts of the modules a local run may not run: the pull request's
`hygiene` run reads them in its `check-stage-logs-hygiene` artifact. The `mutation-python` legs
prove the tests of `test_python_stage` against the runner's mutants.

## 8. Formal model

None. The runner has no protocol or state machine of its own: it starts processes, waits for them
and sums their counts. No formal entry covers `scripts/check.sh` or the new runner.

## 9. Mutation rows

None: the runner is mapped to `test_python_stage` in `scripts/mutation-python.json`, and its mutants
are dealt to the `mutation-python` legs.
