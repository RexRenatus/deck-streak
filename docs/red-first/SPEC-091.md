# Red-first record: SPEC-091

This record is the horizon's, #91's pull request, the first of SPEC-091's two (section 3c). It
delivers A8, A9 and A19; CU3 (#90) adds its criteria's lines when it moves them back into the
acceptance fence.

The order of work: the SPEC moved out of `docs/specs/planned/` with its section 3c and ADR-317
(d5d88a3a); the registry and the three goldens (f2b3b805); the tests beside an inert horizon module
that compiled and answered nothing, every constant 0, a zero scan and empty texts (a0243e4e); the
port (ecaffe96); the SPEC's mutation rows, each proved KILLED by its full id (487d2897); and one more
golden case, a borrowed card at due 0 under a negative day number, which kills the one mutant of the
borrowed-position guard that cargo-mutants found alive, with the dead arm recorded (b1cb39a6).

Each criterion was run at a0243e4e, selecting its own test, and failed by assertion, not by a compile
error, a missing fixture or an empty selection: the whole file ran 3 tests, 0 passed and 3 failed.

```red-first
A8: red at a0243e4e: assertion `left == right` failed: the curve of {"cards":[],"today":100}; left: [], right: [0, 0, 0, 0, ...] (365 zeros)
A8: green at ecaffe96
A9: red at a0243e4e: assertion `left == right` failed: the window, {"cards":[],"desired_retention_pct":90.0,"today":100}; left: 0, right: 30
A9: green at ecaffe96
A19: red at a0243e4e: assertion `left == right` failed: horizon.HORIZON_DAYS: ours 0, the predecessor's 365
A19: green at ecaffe96
```

CU3 adds its criteria's lines when it moves them back into the acceptance fence.
