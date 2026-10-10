# Red-first record: SPEC-400

SPEC-400 (R1 to R10, A1 to A13), decided by ADR-414. The SPEC, ADR-414 and the browser checks were
committed first, with no change to the app. A1 and A2 are browser checks, so their red is read in
CI's `web` check run on that commit, and each line below quotes the failure from that run.

```red-first
A1: red at <C1 sha>: <the failure line, quoted from CI>
A2: red at <C1 sha>: <the failure line, quoted from CI>
```
