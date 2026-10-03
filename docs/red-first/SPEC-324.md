# Red-first record: SPEC-324

The SPEC, ADR-325 and the schematic were committed first (62d8180b). The tests and the generated
population (`tools/table-census/population.rs`) were then committed alone at af842373, before the
shared reader, its call sites and the owner rule existed, and each ran by its exact name with
`--no-fail-fast` across `-p deck-streak-progression -p deck-streak-economy` and the three census
targets. Every red is an assertion on the missing behaviour: the census refuses nothing it should.
The runs read `test result: FAILED. 0 passed; 1 failed` for `wallet_census`, `1 passed; 2 failed`
for `ledger_census` and `0 passed; 3 failed` for `xp_census`.

```red-first
A1: red at af842373: assertion `left == right` failed: s000 concat of x|p_ledger; left: [], right: ["crates/quests/src/s000_concat.rs spells xp_ledger from literals, joined or in another case, and only progression's code may"]
A2: red at af842373: assertion `left == right` failed; left: [], right: ["crates/quests/queries/s000_piece.sql spells xp_settlement from literals, joined or in another case, and only progression's code may", ...]
A3: red at af842373: assertion `left == right` failed: s000 concat of c|oin_ledger; left: [], right: ["crates/quests/src/s000_concat.rs spells coin_ledger from literals, joined or in another case, and only economy's code may"]
A4: not red: no reader existed at the base to refuse a near miss, so every near miss and control was already unrefused; MUTATION COVERAGE, which row S32415 holds against the reader
A5: red at af842373: assertion `left == right` failed: pub const JOINED: &str = concat!(env!("PREFIX"), "_ledger"); left: [], right: ["crates/quests/src/fail.rs joins a value the census cannot read beside part of xp_ledger, and only progression's code may"]
A6: red at af842373: assertion `left == right` failed; left: [], right: ["crates/progression/src/generic.rs calls settle inside progression's own code, and only coordination's code may unless progression admits it", "crates/progression/src/pointer.rs ...", "crates/progression/src/wrapper.rs ..."]
A7: red at af842373: assertion `left == right` failed; left: ["crates/streaks/src/caller.rs calls settle, and only coordination's code may"], right: ["crates/progression/src/lib.rs calls settle inside progression's own code, and only coordination's code may unless progression admits it", "crates/streaks/src/caller.rs calls settle, and only coordination's code may"]
```

A6's second test, `an_admitted_file_of_the_owner_is_accepted`, is ADDED AT GREEN: it calls the
owner rule's predicate and builds the use it judges, neither of which exists at the base, so it
cannot compile there. It lands with the predicate in the implementation commit (ruling 108 (4)),
and A6's red above is its first test's.
