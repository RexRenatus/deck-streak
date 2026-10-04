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
A1: green at b713e00b
A2: green at b713e00b
A3: green at b713e00b
A5: green at b713e00b
A6: green at b713e00b
A7: green at b713e00b
```

A6's second test, `an_admitted_file_of_the_owner_is_accepted`, is ADDED AT GREEN: it calls the
owner rule's predicate and builds the use it judges, neither of which exists at the base, so it
cannot compile there. It lands with the predicate in the implementation commit (ruling 108 (4)),
and A6's red above is its first test's.

Three commits between the red af84237 and the green b713e00 edit a test file, and none changes an
assertion of A1 to A7. The censuses live in their test files, so the reader's wiring is a test-file
edit.

- 80d333a, the implementation, includes the shared reader in `ledger_census.rs`,
  `wallet_census.rs` and `xp_census.rs` and adds its refusals to each census's own. It also adds
  the owner rule to `xp_census.rs`, with A6's second test (above) and, in the existing round-6
  test `the_census_reads_progressions_own_reexports_as_it_reads_the_other_crates`, one expected
  refusal of its planted wrapper `crates/progression/src/inner.rs` in its sorted place (ruling 99).
- ede1fcd changes one expected verdict in the killer population of `xp_census.rs`. The control
  `S2 B build scripts` with `include!(env!("CALL_FILE"))` is now refused (ruling 115, under ruling
  108 (1)), and the pinned digest of the 2218 trees changes with it.
- b713e00 cures clippy in code. It moves the round-6 test's planting of progression's aliases into
  a helper it calls, with the same files and text, and writes A4's planted `health_body` control as
  `r#"..."#` instead of `r##"..."##`, with the same value.

The greens ran at b713e00b across `-p deck-streak-progression -p deck-streak-economy
-p deck-streak-kernel` and the four targets with `--no-fail-fast`: `wallet_census` read `2 passed;
0 failed`, `log_capture_class` `8 passed; 0 failed`, `ledger_census` `4 passed; 0 failed` and
`xp_census` `39 passed; 0 failed; 1 ignored` (the ignored test is ignored at the base too). The
killer census read `killer examined 2218 tree(s)`, `members escaping: 0; controls judged wrongly:
0`. A4 has no green line because it was never red.

The 29 rows S32401-S32429 were proved at b713e00b by full id with `python3 scripts/mutation_rows.py
prove --row <id>`: `rows: examined 29: killed 29, survived 0, void 0`. Every row selected exactly
one test for its control and one for its mutant, and each read `KILLED: its killer passed without
the mutant and failed with it`. S32426's killer is A6's
`the_owners_own_operation_is_refused_unless_it_admits_it`.
