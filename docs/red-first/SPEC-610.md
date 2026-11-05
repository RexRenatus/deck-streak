# Red-first record: SPEC-610

The SPEC, ADR-632 and the changelog fragment were committed alone. Then came the tests of A1 and A2
against the unchanged helpers, so each test ran and failed by its assertion on the directory of the
report. The helper change turned them green with the tests unchanged.

```red-first
A1: red at 607400ce: AssertionError: True is not false : the report was written into the parent of the scenario tree
A1: green at 5baef3b3
A2: red at 607400ce: AssertionError: True is not false : the report was written into the parent of the scenario tree
A2: green at 5baef3b3
A3: not red: the runner is unchanged, and the criterion pins that it still writes costs.jsonl beside the report
```

The two tests were committed with the helpers unchanged (607400ce), and each failed by its
assertion on the directory of the report, run by name on that tree. The helper change in 5baef3b3
turned both green with the tests unchanged, and the run at this delivery head reads them green.
The commit after 5baef3b3 changes only this record.
