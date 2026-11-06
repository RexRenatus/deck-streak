# Red-first record: SPEC-633

The SPEC, ADR-656, the schematic and the changelog fragment were committed alone. Then came the
tests of A1 and A2 with a module runner that keeps every input (the suites, `PYTHON_STAGE_JOBS`, the
serial tail, the output order and the verdict line) but runs every module one after another, so the
A1 test ran and failed at its barrier.

```red-first
A2: not red: the serial runner already keeps the verdict line, the output order and the tail lines, and the criterion pins them
```
