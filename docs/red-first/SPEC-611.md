# Red-first record: SPEC-611

SPEC-611 (R1 to R5, A1 to A4). The SPEC, ADR-633, the schematic, the fragment and the one cite in
SPEC-404 were committed first. The three tests were committed next with a stub of the bounded rule
that reads every value as stated and refuses nothing, so A1 and A2 fail by their own assertions: a
value past its range, and a per-entry key that names no TLA+ entry, are read. The criteria tests
run in the `hygiene` job of CI, which runs `scripts/tests`.
