# Red-first record: SPEC-331

The SPEC, ADR-331, SPEC-324's appended section and the schematic were committed first (33ff49e8).
The tests were then committed alone at 103e483f, with `tools/table-census/table_census.rs`
untouched, and run by name with `--no-fail-fast` across `-p deck-streak-progression
-p deck-streak-economy` and the three census targets. Every red is an assertion on the missing
behaviour: dev's reader refuses a lone character it should not, because its pool holds every
character. The runs read `test result: FAILED. 0 passed; 1 failed` for `wallet_census` and for
`ledger_census`, and `1 passed; 2 failed` for `xp_census`, whose passing test is A2's.

```red-first
A1: red at 103e483f: assertion `refused.is_empty()` failed in each tree: ledger_census `235 file(s) refused in 85 of 85 tree(s)`, xp_census `355 file(s) refused in 125 of 125 tree(s)`, wallet_census `295 file(s) refused in 105 of 105 tree(s)`; the first, `xp_settlement@0 'x' method argument`, refuses `crates/quests/src/a1_after.rs` and `crates/streaks/src/a1_char.rs`
A1: green at 38d14d2b
A2: not red: it pins the joins dev's reader already refuses, each by exactly its planted files; it passed at 103e483f over its 18 join plants
A3: red at 103e483f: assertion `refused.is_empty()` failed: `7 file(s) refused in 3 of 3 disclosed plant(s)`, B1 2 files, B2 2 files and B8 3 files
A3: green at 38d14d2b
A4: not red: the real tree's three censuses refuse nothing at the base, and the reader change keeps them so
A5: not red: SPEC-324's population, near misses and fail-closed cases are judged by unchanged tests, as at the base
```

The design measured A1 at the earlier dev `e2eb20d7`: 235, 355 and 295 files refused. dev moved to
`cff914e3` before this delivery, and the counts at 103e483f are the same.

The reader change was committed at 38d14d2b, and the same targets then passed by name: A1 read
`examined 85`, `125` and `105 lone-character tree(s)` with nothing refused, and A3 read
`examined 3 disclosed plant(s)` with nothing refused. A2's 18 join plants stayed refused, A4's three
real-tree censuses still refuse nothing, and A5's tests passed unchanged.

A2 gained a nineteenth plant at 852a0656, P14: `"XP_"` returned by a function in one crate and
joined with `"Settlement"` in another, so that only the pool's case folding refuses it. It was added
after S32409's mutant (the pool not folded) survived its SPEC-324 killer at this reader, which
folds each reach's own pieces. Measured at 852a0656 by swapping the reader file: A2 passes with
dev's reader from `cff914e3` and with this one, so it stays not red; with S32409's mutant installed
it fails at `P14 case variant across crates`, with both files expected and none refused.

## The weakening table

Across `git diff 69fe44ee...HEAD` (the recorded dev; `cff914e3...` before the dev merges): no test is deleted, renamed or skipped, no planted case is
removed, and no filter, exclusion, skip list, sample or bound is added on the test side. The
entries are the reader's disclosed class and the two killer moves.

| entry | commit | ruling |
|---|---|---|
| `table_census.rs` R3: a one-character piece leaves the workspace-wide pool and is pooled only in the reach of a file that holds, includes or names it; a one-character join carried through a function's return value or argument (B1, B2, B8) is no longer refused, disclosed by A3 and tracked under #585 | 38d14d2b | ruling 155 + ruling 158 |
| S32411 AMENDMENT: its killer moves from `ledger_census::a_reserved_name_joined_from_literals_is_refused_in_every_spelling` to `xp_census::the_joins_the_census_names_stay_refused`, its anchor unchanged | a260b786 | ruling 155 + ruling 158 |
| S32409 AMENDMENT: its killer moves from `ledger_census::a_reserved_name_joined_from_literals_is_refused_in_every_spelling` to `xp_census::the_joins_the_census_names_stay_refused` (A2, which gained P14 at 852a0656), its anchor unchanged | 4be9d764 | ruling 167 |
| A3 population 3 -> 7 (fix round 2, ruling 177): the second-step class, a `const` naming another `const` across files, a renamed re-export, and an `include!` or `include_str!` inside a named item's initialiser (O1, O2, O3, O7), is not refused at this reader, where dev refuses each of its plants in 2 files; disclosed in SPEC-331 section 5 beside B1, B2 and B8, tracked under #585, pinned by A3's four new plants | 4d4fd945, 394a5a06 | ruling 177 |
| AD2's test `a_join_in_a_file_holding_no_literal_stays_refused` adds `.filter(|(path, _)| !path.ends_with("_join.rs"))` on its EXPECTED list only (the joining file holds no piece); the census output is asserted whole against it, and dev's reader refuses the same three files, so no file dev refuses is spared | 4d4fd945 | ruling 177 (c) |

## Fix round 2: the second-step class and the no-own-piece join

At 3e514c1b the verifier's monotone harness found four routes dev refuses and this reader does not,
each a second step: a one-character `const` initialised by `include_str!` and named in another file
(O1), a `const` alias chain across crates (O2), a re-export renamed in a third file (O3), and a
`const` initialised by `include!` of a Rust expression file (O7). Ruling 177 disclosed the class
and did not extend the reach: one step is SPEC-331's decision and a second step is #585's work.

A3 now examines 7 disclosed plants, the three first plants and O1, O2, O3 and O7, and asserts that
none is refused at the head. The same population was run at dev's reader, `69fe44ee`'s
`tools/table-census/table_census.rs` written into a scratch export of the round's first commit
(sha256 `bbab89dc...` equal to `git show 69fe44ee:<path>`). It reads `15 file(s) refused in 7 of 7
disclosed plant(s)`, and each new plant is refused in 2 files:

- O1: `o1_join.rs` and `o1_m.txt`
- O2: `o2_join.rs` and `o2_m.rs`
- O3: `o3_join.rs` and `o3_m.rs`
- O7: `o7_join.rs` and `o7_m.in`

The controls O4 (an associated const named by path), O5 (a const in an inline module) and O6 (a
const with a block initialiser) joined A2's population, which now examines 22 join plants, each
asserted refused in every file of its plant.

AD2: deleting `self.local.entry(file.to_owned()).or_default();` was killed by no census test. A new
test, `a_join_in_a_file_holding_no_literal_stays_refused`, plants three pieces in three files
(`"xp_settle"`, `'m'` and `"ent"`) joined by a fourth file that holds no literal, and asserts that
exactly the three files holding a piece are refused. With the statement deleted it fails with
`left: []`, so the mutant is KILLED, and row `S33107-A-FILE-READ-HAS-A-REACH-OF-ITS-OWN` pins it.
