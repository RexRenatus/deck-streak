# Red-first record: SPEC-333

The tests were committed alone first (b14c7363). At that commit `deploy/slo.json` held one SLO and
the module failed by assertion on both MCP tests of A1; A2 passed, as a guard over the API entry.

```red-first
A1: red at b14c7363: AssertionError: 0 SLO(s) over deck-streak-mcp.service, not one
A1: green at 79c272e0
A2: not red: green at the base for the API entry, a guard over every SLO that now also holds the MCP entry
```

Correction (fix round 2): CI's `hygiene` read `test_slo_evaluator`'s journal-read count red at
`f3fbd24e` (`AssertionError: 8 != 4`, run 37179950148), because the evaluator reads each declared
SLO's journal once per run and the test counted one SLO. It is green at `cfb12fe4`, where the count
covers every declared SLO.
