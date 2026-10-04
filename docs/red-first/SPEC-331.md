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
