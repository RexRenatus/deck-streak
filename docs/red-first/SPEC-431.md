# Red-first record: SPEC-431

SPEC-431 (R1 to R17, A1 to A10). The SPEC, ADR-445 and the schematic were committed first
(5a387deb31813a9c6309940502faf5ce69030370). The three censuses were committed next, alone, with
finders that return no finding (2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e), and each of the ten
tests read red for its own criterion, on the planted defects its finder did not refuse; the real
tree passed and every population printed its count. A1 to A5 were run at that commit. A6 to A10
were read in CI only, in the pull request's run at that commit, because their census reads every
production Rust file. The finders and the three lists were committed next
(0d1b9392b6eddf4bf51ba730963a959054e06833): the finders were replaced whole and given their
helpers, and no test, fixture or planted defect changed. A1 to A5 read green at that commit; A6
to A10 read green in CI at the pull request's head, which holds it. Each red below is quoted from
the run at its commit.

```red-first
A1: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['bot -> kernel [dependencies]: an edge with no row in config/crate-edges.json'] (9 of 9 planted defects unrefused)
A2: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['api -> kernel [dev-dependencies]: an edge with no row in config/crate-edges.json'] (10 of 12 planted defects unrefused; the two that add no edge pass)
A3: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['api -> kernel [dependencies]: ADR-2 is held by 0 files in docs/decisions, not one'] (4 of 4 planted defects unrefused)
A4: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['lean/Chest: no vectors test names formal/vectors/chest.jsonl, and no owed row'] (9 of 9 planted defects unrefused)
A5: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/economy/tests/formal_vectors_ledger.rs: names formal/vectors/ledger.jsonl, which no Lean entry declares'] (7 of 7 planted defects unrefused)
A6: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/app/src/lib.rs: 1 clock read(s) and no row: crates/app/src/lib.rs:2: SystemTime::now'] (8 of 8 planted defects unrefused)
A7: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/app/src/lib.rs: 1 clock read(s) and no row: crates/app/src/lib.rs:2: SystemTime::now'] (10 of 13 planted defects unrefused; the three spellings that are no read pass)
A8: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/app/src/lib.rs: 1 clock read(s) and no row: crates/app/src/lib.rs:3: Instant::now'] (2 of 10 planted defects unrefused; the eight test-code places, which hold no production read, pass)
A9: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/app/src/lib.rs: 1 clock read(s) and no row: crates/app/src/lib.rs:2: Instant::now'] (2 of 10 planted defects unrefused; the eight comments and literals, which hold no read, pass)
A10: red at 2a36fff6d6afc4b769a519b36d8c1f4d7eedb02e: AssertionError: Lists differ: [] != ['crates/app/src/lib.rs: ADR-2 is held by 0 files in docs/decisions, not one'] (2 of 2 planted defects unrefused)
A1: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A2: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A3: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A4: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A5: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A6: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A7: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A8: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A9: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
A10: green at 0d1b9392b6eddf4bf51ba730963a959054e06833
```
