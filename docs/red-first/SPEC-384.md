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
A2: red at 8db9a6af: the member's unit left in the store is refused by name: ["crates/m/src/lib.rs calls settle, and only coordination's code may"]
A3: red at 8db9a6af: the git package's units remain in the store: []
A4: red at 8db9a6af: assertion `left == right` failed: each input changed alone names a store of its own, left: ["the lock file", "the toolchain pin", "the tree root", "the cargo program", …(13)], right: []
A5: red at 8db9a6af: another handle's try_lock reads WouldBlock while the census holds its store
A6: red at 8db9a6af: every pass of the census reads member units: []
```

The implementation lives in the test file, so the commit that turns these criteria green also
changes `crates/progression/tests/xp_census.rs`; it is named here when it lands.
