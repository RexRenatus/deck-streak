# Red-first record: SPEC-319

The SPEC, its schematic, ADR-319 and the amendments were committed first, then the HeldFlush model
(A6, decided by the formal checker and recorded in the SPEC's section 3b, not here). The five tests
were committed alone at e884ccca, against the base's notifications and daemon code, and each ran by
its exact name before any implementation. A1 to A4 fail by assertion. A5, written against
`Router::holding`, which the base lacks, fails to compile, which the brief records as its red: the
build error is `E0599: no method named holding found for struct Router`, at the test's
`.holding()` call, and it is the only error. Each red run reads `running 1 test` and
`test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 3 filtered out` (A1 to A4).

```red-first
A1: red at e884ccca: assertion `left == right` failed: the owed badge was offered to the cycle's router and decided once; left: 0, right: 1
A2: red at e884ccca: assertion `left == right` failed: the badge is deferred, never withheld; left: [], right: ["defer"]
A3: red at e884ccca: assertion `left == right` failed: one push delivers the held badge (Ran { sends: 0 }): []; left: 0, right: 1
A4: red at e884ccca: assertion `left == right` failed: a fresh start seeds the celebrations' switch off; left: None, right: Some("0")
A5: red at e884ccca: error[E0599]: no method named `holding` found for struct `Router` in the current scope (crates/notifications/tests/router.rs, the `.holding()` call)
```
