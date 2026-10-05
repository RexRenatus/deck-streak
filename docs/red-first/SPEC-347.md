# Red-first record: SPEC-347

Part 1 of SPEC-347 (R1-R3, A1-A5). The SPEC, its schematic, ADR-358 and the fragment were
committed first (97dbd20a). Each criterion's test was then committed alone, before the code that
turns it green, and each red below is quoted from the run of its red commit's tests.

```red-first
A1: red at 901c9a05: assertion `left == right` failed: a login through the adapter answers with the server's host key and the endpoint it was sent; left: Err(NotAllowed { service: 1, method: 3 })
A2: red at 901c9a05: panicked at crates/ffi/tests/login.rs:111:9: a wrong password reaches the engine and the engine refuses it, not NotAllowed { service: 1, method: 3 }
A1: green at d2bb9d4c
A2: green at d2bb9d4c
A3: red at 42153266: assertion `left == right` failed: each endpoint is admitted or refused by its rule; left: [("an absent endpoint", None), ("an empty endpoint", None), ...], right: [("an absent endpoint", Some((InvalidInput, "the sync login names no endpoint"))), ("an empty endpoint", Some((InvalidInput, "the sync login names no endpoint"))), ...]
A4: red at 42153266: assertion `left == right` failed: the guard refuses the login before the engine sees it, by its rule; left: (NetworkError, "A network error occurred.\n\nError details: builder error for url ()"), right: (InvalidInput, "the sync login's endpoint is neither https nor plain http to a loopback address")
A3: green at f17ccef9
A4: green at f17ccef9
A5: not red: it is #623's parity test run unchanged, and rows S34700 and S34701 each break it
```

- A1 and A2 were read on R1's tree before rustfmt rewrapped two statements of `login.rs` below
  both failing lines; 901c9a05 differs from the tree that ran by formatting alone.
- d2bb9d4c, A1's and A2's green commit, also grows the core's table census (`tests/table.rs`): its
  native set gains the pair (1,3), an exact-set change tied to R1 and no filter. No assertion of A1
  or A2 changed between their red and their green.
- A3 and A4 ran at 42153266 over the stub guard that admitted every endpoint: A3 read the absent
  endpoint admitted, and A4 read the engine's own client refusing the scheme with its own kind, in
  the process, with no request sent. No test file changed between 42153266 and f17ccef9.
