# Red-first record: SPEC-635

The SPEC, ADR-659 and the changelog fragment were committed alone. Then came the A1 test, with
`TAIL` cut to the two kept entries and SPEC-633 A2's first case planting `test_engine_pin.py`,
against the runner whose `SERIAL_MODULES` still lists eight entries, so the A1 test ran every
planted module and failed on its `serial:` lines.

```red-first
```
