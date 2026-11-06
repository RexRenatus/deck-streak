# Schematic: the python stage's module pool (SPEC-633, ADR-656)

Kind: data flow. It draws how `stage_python` in `scripts/check.sh` runs the three python suites
through `scripts/python_stage.py` once SPEC-633 lands, from the call to the verdict line.

```mermaid
flowchart TD
    stage["check.sh stage_python: need_python and need_cargo"] --> runner["python3 scripts/python_stage.py scripts/tests tools/parity-oracle agent/tests"]
    runner --> list["each suite's top-level test_*.py modules, in name order"]
    list --> split{"module matches a SERIAL_MODULES entry"}
    split -- "yes" --> named["one line per tail module: serial: suite/module (reason)"]
    split -- "no" --> pool["pool of PYTHON_STAGE_JOBS threads, default os.cpu_count(); 1 runs serially"]
    pool --> proc["one process per module: python3 -m unittest discover -s suite -p module, python3 from PATH"]
    named --> tail["the tail, one module at a time, after the pool"]
    proc --> held["each module's output, its exit and its last Ran N, held until it ends"]
    tail --> held
    held --> print["each module's output, suite by suite in argument order, modules in name order"]
    print --> sum["per suite: N the sum of its modules' Ran counts, c its first non-zero exit"]
    sum --> verdict["last line: python: suite ran N test(s), exit c; ..."]
    verdict --> exit{"any suite with exit c not 0, or N of 0"}
    exit -- "yes" --> red["exit 1: the stage fails (SPEC-054 R7)"]
    exit -- "no" --> green["exit 0: the stage passes"]
```

A suite with no module has N of 0 and fails. `check.sh` writes the stage's output to `python.log`
and its time to `timings.tsv`, as before.
