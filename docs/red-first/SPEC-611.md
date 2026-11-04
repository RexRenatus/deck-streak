# Red-first record: SPEC-611

SPEC-611 (R1 to R5, A1 to A4). The SPEC, ADR-633, the schematic, the fragment and the one cite in
SPEC-404 were committed first. The three tests were committed next with a stub of the bounded rule
that reads every value as stated and refuses nothing, so A1 and A2 fail by their own assertions: a
value past its range, and a per-entry key that names no TLA+ entry, are read. The criteria tests
run in the `hygiene` job of CI, which runs `scripts/tests`.

```red-first
A1: red at 080b947: AssertionError: Refused not raised : k = 4294967296 was read (job 126552576579)
A1: green at 72acf62
A2: red at 080b947: AssertionError: Refused not raised : budgets.entries.OpLog was read as a budget (job 126552576579)
A2: green at 72acf62
A3: not red: the stub reads every value as stated; rows S61108, S61109 and S61110 prove it can fail
A4: not red: the presence harness of SPEC-295, green at the base, runs A3 under every planted refusal and holds each call of the reader in the three new tests to its rule
```

The refusals were committed in 72acf62 with every test unchanged, and the commits after it change only documents.
