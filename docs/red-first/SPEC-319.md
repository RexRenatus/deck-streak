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
A1: green at 00f814d8
A2: green at 00f814d8
A3: green at 00f814d8
A4: green at 00f814d8
A5: green at 00f814d8
```

Two assertions were narrowed at the green commit 00f814d, to the criterion's own wording: the cycle's
badge also earns the first level, so the queue holds a second row for the level-up. A2 reads the
rows held for the badge's key, and A3 reads the pushes that name the badge: exactly one. Neither
was widened to pass; A1, A4 and A5 are unchanged.

Mutation rows, proved at 82b22151 (`rows: examined 6: killed 6, survived 0, void 0`):

- S31900: KILLED
- S31901: KILLED
- S31902: KILLED
- S31903: KILLED
- S31904: KILLED (in the cargo-killed script table, killer crate `daemon`)
- S31905: KILLED
