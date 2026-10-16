# Red-first record: SPEC-384

The SPEC, ADR-398 and the insert-only amendments were committed first (3a9bc827), then every
criterion's test with its stubs (8db9a6af), before the code that turns them green. Each red below is
quoted from the run of that criterion's fence line at 8db9a6af, from the worktree root, as SPEC-384
section 3 writes it. The stubs keep every input and lack only the behaviour: the store census
delegates to an empty target (it never holds its store), the key returns one constant, the removal
removes nothing, the refusal never fires, the lock is not taken, and the counts stay at zero.

## The fence, line by line

Each of the 6 lines of SPEC-384 section 3's fence selects one test by `--exact`.

| # | criterion | test file | added or named |
|---|---|---|---|
| 1 | A1 | `crates/progression/tests/xp_census.rs` | added |
| 2 | A2 | `crates/progression/tests/xp_census.rs` | added |
| 3 | A3 | `crates/progression/tests/xp_census.rs` | added |
| 4 | A4 | `crates/progression/tests/xp_census.rs` | added |
| 5 | A5 | `crates/progression/tests/xp_census.rs` | added |
| 6 | A6 | `crates/progression/tests/xp_census.rs` | named: body changed |

Every red is an assertion failure, with each behaviour's assertion ahead of its `examined()` call.
A3's red is its first assertion, that the git package's units remain: the stub census compiled in an
empty target of its own, so the store held no unit for the removal to keep or take out.

```red-first
A1: red at 8db9a6af: the second census reuses a unit of the git package from the store: []
A1: green at 98d1a09e
A2: red at 8db9a6af: the member's unit left in the store is refused by name: ["crates/m/src/lib.rs calls settle, and only coordination's code may"]
A2: green at 98d1a09e
A3: red at 8db9a6af: the git package's units remain in the store: []
A3: green at 98d1a09e
A4: red at 8db9a6af: assertion `left == right` failed: each input changed alone names a store of its own, left: ["the lock file", "the toolchain pin", "the tree root", "the cargo program", …(13)], right: []
A4: green at 98d1a09e
A5: red at 8db9a6af: another handle's try_lock reads WouldBlock while the census holds its store
A5: green at 98d1a09e
A6: red at 8db9a6af: every pass of the census reads member units: []
A6: green at 98d1a09e
```

## The green

The implementation lives in the test file, so 98d1a09e (`perf(progression): the real tree's census
keeps lock-pinned units`) is the commit that changes `crates/progression/tests/xp_census.rs`
between the reds at 8db9a6af and their greens; 847c8524 between them adds only this record. Each
green above was read from that criterion's fence line, run from the worktree root on the tree
98d1a09e commits:

| criterion | the passing run |
|---|---|
| A1 | `first census: 4 pass(es), 20 member unit(s) compiled, 0 reused; second: 4 pass(es), 20 compiled, 4 reused`, `1 passed` |
| A2 | `examined 6 refusal(s) of the member's unit`, `1 passed` |
| A3 | `examined 12 entry(ies) of the store after the removal`, `1 passed` |
| A4 | `examined 17 input(s) changed alone`, `1 passed` |
| A5 | `the other handle read WouldBlock 144 time(s) while the census ran`, `1 passed` |
| A6 | `census store: 16 pass(es), reading [445, 445, 476, 476, 445, 445, 476, 476, 33, 33, 33, 33, 33, 33, 33, 33] member unit(s); 2140 member unit(s) compiled, 3948 read, 3948 of them compiled by this census; 0 store unit(s) reused`, `1 passed` |

The first runs of A1 and A3 on the green code failed for a reason no stub could show. Cargo refuses
`cargo clean --target-dir` on a directory that does not hold its cache-directory tag (`cannot clean
<store>: missing or invalid CACHEDIR.TAG file`). Cargo writes that tag only into a target directory
it makes itself, and the store's folder is made before cargo first builds in it, for its lock file.
So every census on a new store refused before its first pass, and A1 read `the second census reuses
a unit of the git package from the store: []`. An early fix skipped the removal when the tag was
absent. A3 refuted it: the store never gains the tag, so the removal never ran, and A3 read `the
removal leaves no unit of deck-streak-progression`. The fix 98d1a09e carries is in `hold`: under the
lock, it writes cargo's own tag into the store, byte for byte (`CACHEDIR_TAG`). The removal then
always runs, and a store that cannot be cleaned refuses by name.

After the green, eabb4d42 (`test(progression): rows for the census store`) changes the test file
once more. A3's own `cargo metadata` call, added at 8db9a6af, spelt the arguments exactly as row
S07249's find does, so that find occurred twice in the file. The commit reorders A3's arguments
(the same command), so S07249's find occurs once again, and A3 reads green at eabb4d42 as at
98d1a09e.

## P1: the existing examined counts, equal by name

Each side ran `cargo test -p deck-streak-progression --test xp_census -- --exact
only_progression_writes_xp_settlement_and_only_coordination_settles --nocapture`. The base side ran
at 3a9bc827, whose test code is the cut's, and the head side ran at eabb4d42 (P3's run 2). Both
print the same five lines:

```text
table census: examined 31 member(s), 318 file(s) outside progression, 11541 piece(s) (2725 of one character, pooled only in a reach), 137786 word(s), 28 include(s) (0 joined onto OUT_DIR, disclosed) for xp_settlement
examined 340 crate source file(s)
examined 33 migration(s)
table census: examined 4 member(s), 10 file(s) outside progression, 0 piece(s) (0 of one character, pooled only in a reach), 84 word(s), 0 include(s) (0 joined onto OUT_DIR, disclosed) for xp_settlement
examined 13 planted crate source file(s)
```

Beside them, the head side prints the new counts of R7: `census store: 16 pass(es), reading [445,
445, 476, 476, 445, 445, 476, 476, 33, 33, 33, 33, 33, 33, 33, 33] member unit(s); 2140 member
unit(s) compiled, 3948 read, 3948 of them compiled by this census; 2464 store unit(s) reused`.

## P2: the census-only kills at the head

Each row was installed once by its find and replacement, its killer run by `--exact` (`running 1
test`), and the saved bytes written back, the target's sha256 equal before and after. The control is
the same killer at eabb4d42 without the mutant: the real-tree test passes there (P3's runs 1 and 2),
and so does `the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target`.

| row | killer | at eabb4d42 | the failing assertion |
|---|---|---|---|
| S07274 | the real-tree test | KILLED | `found.refused` holds `deck-streak-progression's build script (crates/progression/build.rs) is not the one pinned by PROGRESSION_BUILD_SHA256` |
| S07275 | the real-tree test | KILLED, on a store warm from the control (2464 units reused) | `the fold's XP step calls settle; every file that does: {}` |
| S07283 | the real-tree test | KILLED | the same refusal as S07274's: the pinned digest no longer names the build script |
| S07295 | `the_order_pair_p3_is_judged_alike_in_both_orders_on_one_target` | KILLED | `the second tree of each order on a target the first tree used` |

## The rows of SPEC-384 section 9

Each row was proved the same way. S38400 was proved at eabb4d42. S38401 to S38406 were proved at
a9bdefb8, whose test file differs from eabb4d42's only in A3's argument order, which no row's find
or killer reads.

| row | killer | the failing assertion |
|---|---|---|
| S38400 | A3 | `the removal leaves no unit of deck-streak-progression` |
| S38401 | A2 | `the member's unit left in the store is refused by name` |
| S38402 | A4 | `each input changed alone names a store of its own`, left `["the lock file"]` |
| S38403 | A4 | the same, left the five inherited names' value and absence pairs |
| S38404 | A4 | the same, left `["the tree root"]` |
| S38405 | A5 | `another handle's try_lock reads WouldBlock while the census holds its store` |
| S38406 | A1 | `the second census compiles every member unit the first did: 0 and 0` |

At eabb4d42 the bands that name `tests/xp_census.rs` hold 81 rows on it (S07200 52, S07300 2,
S32400 7, S33600 13, S38400 7), and each find occurs exactly once. `cargo mutants --list --in-diff`
over the branch's diff lists no mutant: the diff changes no source file of progression, and
cargo-mutants mutates no test target.

## P3: a cold and a warm store report one killed and survived set

Every run is the real-tree command of P1, at eabb4d42, with `TMPDIR` set to a folder of the
builder's scratch. A COLD run follows the store's folder emptied, `ls -A` reading it empty. A WARM run
follows the cold run of the same tree, with nothing removed. A row's mutant is installed by its find
and replacement before its run and its saved bytes written back after it, the sha256 equal. The log
paths are in the builder's round directory.

| # | state | tree | result | the counts of R7 | head | log |
|---|---|---|---|---|---|---|
| 1 | cold | unmutated | passes | 16 passes, 2140 member units compiled, 3948 read, 3948 compiled by this census, 0 reused | eabb4d42 | `p3/1-unmutated-cold.log` |
| 2 | warm | unmutated | passes | 16 passes, 2140 compiled, 3948 read, 3948 compiled by this census, 2464 reused | eabb4d42 | `p3/2-unmutated-warm.log` |
| 3 | cold | S07274 | KILLED | not printed: the census refuses the build script by name before its first pass | eabb4d42 | `p3/3-S07274-cold.log` |
| 4 | warm | S07274 | KILLED | not printed, as in run 3 | eabb4d42 | `p3/4-S07274-warm.log` |
| 5 | cold | S07275 | KILLED | 16 passes, 2140 compiled, 3948 read, 3948 compiled by this census, 0 reused | eabb4d42 | `p3/5-S07275-cold.log` |
| 6 | warm | S07275 | KILLED | 16 passes, 2140 compiled, 3948 read, 3948 compiled by this census, 2464 reused | eabb4d42 | `p3/6-S07275-warm.log` |
| 7 | cold | S07283 | KILLED | not printed, as in run 3 | eabb4d42 | `p3/7-S07283-cold.log` |
| 8 | warm | S07283 | KILLED | not printed, as in run 3 | eabb4d42 | `p3/8-S07283-warm.log` |

The unmutated tree passes in both states, and each of the three rows reads KILLED in both, so the
killed and survived set is the same cold and warm. S07274 and S07283 are refused before the store is
read, so their warm runs follow a store that their cold runs left holding only its lock file and its
tag. Their P2 runs above followed a store warm from the control, and read the same refusal.

## F1: the formal entry

`formal/tla/CensusStore` was committed with every `digest=` at 64 zeros (7bccd0bb), checked, and
stamped from the check's `STALE` lines (a9bdefb8). The covers name `census_on_store`, `hold`,
`removal`, `store_key` and `not_compiled_here`. The entry check `formal check --entry
tla/CensusStore --gate`, with the development branch at the cut as the base, printed at a9bdefb8:

```text
FORMAL EXAMINED entries=1 properties=2 witnesses=2 configs=1 landings=20 rulings=0 deferred=0
FORMAL TOOLCHAIN compiled identity=a2518571360483e12161e99b64695e6c3e8845230129ce0ad179b5bddf9a8dc4
FORMAL SURFACES deny=0 contexts=0 rows=2653 land=0 retired=0
FORMAL OK exit=0
```

Its report read `"verdict": "OK"` and `"findings": []`, and each property row read `clean: true`
with `ramp: report` and `flip: unmoved`:

| property | clean | ramp |
|---|---|---|
| `AcceptedVerdictReadsItsOwnMembers` | true | report |
| `NoMemberSurvivesTheRemoval` | true | report |

Both witnesses were examined, and the output holds no `WITNESS_SURVIVED`, `VOID`, `TIMEOUT`,
`MODEL_ERROR` or `STALE` line. A hand run of each configuration read the same: `MCCensusStore.cfg`
finds no error in 648 distinct states, and each witness violates its own invariant
(`AcceptedVerdictReadsItsOwnMembers` with the lock off, `NoMemberSurvivesTheRemoval` with the removal
off). With three censuses in place of two, the model finds no error in 13760 distinct states. The
ratchet-only check at eabb4d42 printed `FORMAL EXAMINED entries=41 properties=134 witnesses=169
configs=35`, `FORMAL OK exit=0`, with no finding and no weakening.
