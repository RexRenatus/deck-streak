# Red-first record: SPEC-360

SPEC-360 (R1 to R10, A1 to A12). The SPEC, ADR-371 and the schematic were committed first
(b54b6803). The census of A3 to A7 was committed alone (2ab35d4c), before the crate existed, and
the crate's shape next (a86bd751): a `review_xp` that returns 0, an `is_study_event` that returns
false, and the real table. The crate's golden tests of A1 and A2 and the ease-5 pin of A12 were
then committed alone (ee252155) and run over that shape. The rule moved into the crate at
c96de99c; progression became the translation over it at 8b83d18c, which left the statics census
of A11 red until its line was written at 22a5a8c7. Each red below is quoted from the run at its
commit.

```red-first
A1: red at ee252155: the XP of {"ease":1,"ivl":0,"rtype":0,"tier":null} left: 0 right: 4
A1: green at c96de99c
A2: red at ee252155: {"ease":1,"rtype":0} left: false right: true
A2: green at c96de99c
A3: red at 2ab35d4c: Lists differ: ['deck-streak-xp has no manifest'] != []
A3: green at a86bd751
A4: red at 2ab35d4c: Lists differ: ['Cargo.lock holds no deck-streak-xp package'] != []
A4: green at a86bd751
A5: red at 2ab35d4c: Lists differ: ['crates/xp/src embeds economy.json 0 times, not once'] != []
A5: green at a86bd751
A6: red at 2ab35d4c: Lists differ: ["the map's fence holds no line for deck-streak-xp"] != []
A6: green at 8b83d18c
A7: red at 2ab35d4c: First extra element 0: 'crates/xp/src/review_xp.rs rounds half to even 0 times, not once'
A7: green at 8b83d18c
A8: not red: it pins the server's XP over the 265 cases, which the base already has; it guards the move
A9: not red: it holds the 24 constants the base already holds, eight of them re-read from the crate's table; it guards the move
A10: not red: it pins the two callers' pricing, which the base already has; it guards the move
A11: red at 8b83d18c: no count of a day's failed routes outlives one route left: ["static: crates/xp/src/table.rs holds `static TABLE: LazyLock<ReviewXpTable> = LazyLock::new(parse);`, which is not written out"] right: []
A11: green at 22a5a8c7
A12: red at ee252155: the XP of ease 5, interval 0, type 0 and no tier left: 0 right: 8
A12: green at c96de99c
```
