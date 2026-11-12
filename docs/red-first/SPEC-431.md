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

## Rebuild on current dev

This delivery was rebuilt on a later `dev`: its commits were merged onto it unchanged, the
documents were amended (SPEC-431 section 10, ADR-445 D10), and the as-found test was committed
next, alone. A11 is red first: at that commit the census still reads `as_found` as an unknown key,
keeps the four vectors findings an `as_found` row would admit and reports no stale row, so A11
fails by assertion on the real tree, before any planted defect. At the same commit the censuses
read the base population at the rebuild's cut red by name, each by its first assertion over the
real tree, with these causes:

- A1 and A2: the ten edges the cut adds have no row.
- A4: the eight Lean entries the cut adds have no row.
- A5: the four vectors findings that no row kind could hold.
- A6 to A9: the two production files with clock reads that the cut adds have no row, and
  `crates/ffi/src/engine.rs` reads the clock twice where its row counts one.

A3 and A10 read green there, since each judges the admission of the existing rows only.

The as-found code and the three re-cut lists were committed next (d672785a): it changed the census code of scripts/tests/test_formal_parity.py and no test method, fixture or planted defect.
The rows that dev adds over the rebuild's cut were listed next (2e355c1f), over the tree the pull request merges; no test or census code changed.

A1: red at dfb89b3f (the base population at the rebuild's cut)
A1: green at d672785a
A2: red at dfb89b3f (the base population at the rebuild's cut)
A2: green at d672785a
A4: red at dfb89b3f (the base population at the rebuild's cut)
A4: green at 2e355c1f
A5: red at dfb89b3f (the base population at the rebuild's cut)
A5: green at d672785a
A6 to A9: red at dfb89b3f (the base population at the rebuild's cut)
A6 to A9: green at d672785a

```red-first
A11: red at dfb89b3f: AssertionError: Lists differ: ['crates/companion/tests/formal_vectors_de[447 chars]res'] != [] First list contains 4 additional elements.
A11: green at d672785a
```

## The two owed vectors tests (#878)

SPEC-431 section 11 was committed first. The two `owed` rows were then removed
alone (84e28c0f), with row S43115 re-anchored and no test added: A4 read red at that commit by its first
assertion over the real tree, which named both entries as implemented entries with no vectors
test and no owed row. The two vectors tests and rows S43137 to S43141 were committed next (9c5ea821): A4
read green there, and A12 and A13 passed over the code as it is.

A4: red at 84e28c0f
A4: green at 9c5ea821

```red-first
A12: not red: the kept-away rule already answers every vector the port wrote, so the test reads green over the code as it is; its reds are hand rows S43137, S43138 and S43139, killed by A12
A13: not red: the shared reader already refuses none of the port's trees and refuses each control, so the test reads green over the code as it is; its reds are hand rows S43140 and S43141, killed by A13
```
