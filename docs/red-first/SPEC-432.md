# Red-first record: SPEC-432

The SPEC, ADR-446, the schematic and the three test targets were committed alone at 06d8a0da, with
`tools/table-census/table_census.rs` untouched and every classifier stubbed while keeping its
inputs: the float rules found no float, the rewrite matcher found no rewrite, the sync-write class
was none and each judge refused nothing. Every walk still read its whole population, so no zero
refusal fired ahead of an assertion. Each of the thirteen criteria was then run alone by name,
`cargo test -p <package> --test <target> -- --exact <name>`, and each run read `running 1 test`.
Eleven failed, each on an assertion about its criterion's missing behaviour, and A3 and A7, the
negative controls, passed. The three packages' suites at 06d8a0da failed on exactly those eleven
tests.

```red-first
A1: red at 06d8a0da: assertion `left == right` failed: the economy's float sites are not the listed ones (SPEC-432 R2), left: {}, against the five listed items
A1: green at 5cfdc843
A2: red at 06d8a0da: a planted float is not refused by its rule: all 5 planted rules missing, F1 to F5
A2: green at 5cfdc843
A3: not red: a negative control; its seven whole-number shapes must be counted as no float, and a stub that counts no float refuses none of them, so it passed at 06d8a0da
A4: red at 06d8a0da: a moved site is not refused by name: all 4 moved sites missing (an unlisted item, a grown count, a shrunk count, a listed item gone)
A4: green at 5cfdc843
A5: red at 06d8a0da: assertion `left == right` failed: the rewrites of the append-only tables are not the sanctioned ones (SPEC-432 R6), left: {}, against the seven sanctioned writes
A5: green at 5cfdc843
A6: red at 06d8a0da: a planted rewrite is not refused by file, item, kind and table: all 72 planted rewrites missing
A6: green at 5cfdc843
A7: not red: a negative control; its ten planted inserts, reads, other tables and commented rewrites must not be refused, and a stub that matches nothing refuses none of them, so it passed at 06d8a0da
A8: red at 06d8a0da: an unreadable write is not refused: all 7 unreadable writes missing
A8: green at 5cfdc843
A9: red at 06d8a0da: a moved sanction is not refused by name: all 3 moved sanctions missing
A9: green at 5cfdc843
A10: red at 06d8a0da: a sync write has no class: all 12 sync writes unclassed
A10: green at 5cfdc843
A11: red at 06d8a0da: a planted write that can apply twice is not refused by name and class: all 5 planted writes missing
A11: green at 5cfdc843
A12: red at 06d8a0da: an idempotent write is not classed by its shape: all 5 shapes unclassed, the first `begin: None, expected Keyed`
A12: green at 5cfdc843
A13: red at 06d8a0da: a module case is not refused by name: all 3 module cases missing
A13: green at 5cfdc843
```

The classifiers live in the three test files, so the tests are edited between red and green: the
green commit, 5cfdc843, replaces the stubs' bodies with ADR-446's rules (D1's five float rules, D2's
four kinds, spellings and fail-closed arms, and D3's five classes, INCREMENT outside `excluded.` and
module classification), and changes no test function, assertion, planted case, register or list.
The thirteen then passed by name. A1 read `examined 6 source file(s)` and `examined 4085 token(s)`
and found the five listed items. A5 read `examined 340 source file(s)`, `examined 5401 literal(s)`,
`examined 33 migration(s)` and `examined 5360 statement(s)` and found the seven sanctioned writes.
A10 read `examined 22 module(s)`, `examined 404 literal(s)` and `examined 12 sync write(s)`, classed
KEYED 1, UPSERT 4, SET 4, DELETE 0, PLAIN 3 and INCREMENT 0. The three packages' suites at
5cfdc843 failed on nothing.

## The join-prefix arm reads capitals

R5 refuses a literal that ends in a write keyword, a prefix joined to a table at run time. That arm
reads the keyword in capitals only, as the SQL in `crates/*/src/` writes it; the placeholder arm
and every other keyword read are case-insensitive. Measured at 5cfdc843 by installing a
case-insensitive arm and running A5: it refuses three literals that are labels, not statements,
each a lone `"update"`: `crates/vault/src/staged.rs` `parse_op` (`Some("update")`),
`crates/bot/src/commands.rs` `handle` (`kind = "update"`) and `crates/bot/src/gate.rs` `admit`
(`kind: "update"`). A lower-case join prefix is therefore not refused: a known false negative of
this check.

## The weakening table

Across `git diff b25ed04a...HEAD`, no test is deleted, renamed or skipped, no planted case is
removed, and no filter, exclusion, skip list, sample or bound is added on the test side. The diff
adds SPEC-432's nine files and changes no other path. The table is empty.
