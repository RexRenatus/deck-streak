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

## Rebuild on current dev

This branch reaches dev unchanged through two merge commits on current dev: 2895915b merges it, and
32a0c9d3 merges the data integrity branch of SPEC-435 beside it, so every commit above, red and green,
is carried as it was measured. The checks were re-run there, and the green commit of the rebuild
is 1e29f006. At 1e29f006, A1 read `examined 6 source file(s)` and `examined 4085 token(s)` and found the
five listed items. A5 read `examined 410 source file(s)`, `examined 6609 literal(s)`, `examined
33 migration(s)` and `examined 6531 statement(s)` and found the seven sanctioned writes. A10 read
`examined 23 module(s)`, `examined 393 literal(s)` and `examined 12 sync write(s)`, classed KEYED
1, UPSERT 4, SET 4, DELETE 0, PLAIN 3 and INCREMENT 0. No list of this SPEC changed. 1e29f006 makes
one cure, in the escape-hatch census of SPEC-435: it re-pins that census's examined count to its
population at current dev, and it edits no file of this SPEC.

## Amendment (#890): the two shares in whole coins

SPEC-432 section 10 (R11 to R13, A14 to A17). The amendment, ADR-446's D10, the tests, the Lean
entry with its vectors file, the formal ledger row and the three whole percent constants were
committed first (ef036c87), beside the float constants, with `rules.rs` unchanged. A16, A17 and A1
were then run alone by name at that commit, `cargo test -p deck-streak-economy --test <target> --
--exact <name>`, and each read `running 1 test` and failed on an assertion about its criterion.
The two rules in whole coins were committed next (1baa7440), with the float constants removed and
no test changed, and the three then passed by name. The commit after 1baa7440 changes only this
record, so the run at the pull request head reads the same code.

```red-first
A16: red at ef036c87: the float fine answers 180143985094820 at wallet 9007199254741001, the vector 180143985094821
A17: red at ef036c87: the float fine answers 184467440737095520 at the largest wallet, the proof 184467440737095517
A1: red at ef036c87: the head's five float sites stand against an empty listing
A16: green at 1baa7440
A17: green at 1baa7440
A1: green at 1baa7440
A14: not red: the goldens held the float form too, so it guards the rewrite, and S08204 and S08205 are killed by it
A15: not red: the percents equal the fractions by construction, so it guards the rewrite, and S43237 is killed by it
```

SPEC-435's register tree test, `every_unchecked_operator_in_the_money_and_counter_paths_is_listed_with_its_bound`, is red at
ef036c87 too, for its own reason: the register edit lands before the rules change, so it lists
3 and 4 integer sites while `rules.rs` still holds one float site in each rule. It is green at
1baa7440.
