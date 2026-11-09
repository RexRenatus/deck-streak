# Red-first record: SPEC-635

The SPEC, ADR-659 and the changelog fragment were committed alone. Then came the A1 test, with
`TAIL` cut to the two kept entries and SPEC-633 A2's first case planting `test_engine_pin.py`,
against the runner whose `SERIAL_MODULES` still lists eight entries, so the A1 test ran every
planted module and failed on its `serial:` lines.

```red-first
A1: red at a354ad57: scripts/tests/test_python_stage.py:175: AssertionError: Lists differ: ['serial: one/test_bin_kind_census.py (spawns cargo)', 'seri[378 chars]go)'] != ['serial: one/test_deploy_planted.py (unmeasured on the box)[46 chars]go)'] (box run)
A1: green at 8061d970
```

A1 was committed with the runner whose `SERIAL_MODULES` still lists eight entries (a354ad57); it
read red on a local run, on its `serial:` lines, with every other test of the module green. The
tail list lost its six entries in 8061d970, which changes no test file, so no commit changes a
test file between A1's red and its green, and the A1 test read green on a local run at that
commit. SPEC-633 A2's first case, changed in a354ad57 to plant `test_engine_pin.py`, is not red:
the runner already kept `test_engine_pin.py` in the tail, and the A2 test read green at a354ad57
and at 8061d970. The commit after 8061d970 changes only this record.
