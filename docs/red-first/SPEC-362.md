# Red-first record: SPEC-362

The SPEC, ADR-373 and the schematic were committed first (8659bb32), and the criteria's tests next,
alone (79922076): its tree is the base's code with those tests, and each red below is quoted from
the run of those tests there. The changes that turn them green followed: dad3f9cc (the verdict
script: the bound, the ceilings, the census, the prices, R7 and R8) and 59780f9f (the workflows'
six-hour legs, the 2200 s per-mutant budget, and nextest's `mutants` profile). Every fenced command
but A9's was read green at 039b003c, each selecting one test (`Ran 1 test`). A6's and A7's `0 != 3`
is the verdict's exit: it passed (0) where the criterion wants VOID (3).

```red-first
A1: red at 79922076: AssertionError: 1 != 0 : mutation: shards: REFUSED: 8367 mutant(s), projected at 1023555 s serially, need more than 256 shards within 3600 s each, the most a job matrix holds: split the change, since a run is never capped
A2: red at 79922076: AssertionError: Lists differ: ['120'] != ['360']
A3: red at 79922076: AssertionError: None != {'ci': 209, 'battery': 234}
A4: red at 79922076: AssertionError: 'REFUSED: 16615 mutant(s), projected at 2093490 s serially, need more than 234 legs' not found in 'mutation: size: REFUSED: 16615 mutant(s), projected at 2093490 s serially, need more than 256 shards within 3600 s each, the most a job matrix holds: never capped\n'
A5: red at 79922076: AssertionError: 1200 not greater than or equal to 2145 : --timeout 1200 --build-timeout 600
A6: red at 79922076: AssertionError: 0 != 3 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A7: red at 79922076: AssertionError: 0 != 3 : mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)
A8: not red: green at the base, whose verdict already walks every planned leg; the test pins that walk, and S36209's planted mutant reads it red (below)
A9: not red: ruling-189 module, never run locally; CI by name decides it
A10: red at 79922076: AssertionError: '32' != '1'
A11: red at 79922076: AssertionError: 'legs 2 of ceiling 234' not found in 'mutation: size: 3 shard(s) for 113 listed mutant(s), projected at 9040 s serially, the slowest at 3411 s of its 3600 s bound\n'
A1: green at 039b003c
A2: green at 039b003c
A3: green at 039b003c
A4: green at 039b003c
A5: green at 039b003c
A6: green at 039b003c
A7: green at 039b003c
A10: green at 039b003c
A11: green at 039b003c
A12: red at 86618b25: AssertionError: 1 != 2 : 21 listed
A12: green at debf6c01
```

**A8, by its planted mutant.** A8 is mutation coverage, not red-first. Row S36209's mutant, which
makes `whole_reports` walk the leg directories that arrived instead of the plan's legs 0 to N-1, was
planted by hand at 8839ba8a and read A8 red: `AssertionError: 'VOID mutation-rust-shard-1: no
report' not found in 'mutation: rust: crates/fix/src/lib.rs: 1 changed code line(s)\nmutation:
rust: cargo-mutants examined 95 (caught 95, missed 0, timeout 0), unviable 0, of 95 on the diff\n`
(the message continues with 48 `VOID never tested: ... listed for mutation-rust-shard-1` lines and
`verdict: VOID: 48 measurement(s) the class needs are missing`). Under the mutant the run still
reads VOID, through `partition`, but no longer names the leg that never reported, which is what the
criterion asks. `scripts/mutation-verdict.py` read sha256 `09de855c…` before the plant and after
its restore, byte for byte, and `mutation_rows.py prove` read the row KILLED.

**A9.** Its test is in `test_mutation_workflows.py`, which no builder runs. Its row, S36210, is
proved by reads: its anchor occurs once in `.config/nextest.toml` (the census), the band file holds
no duplicate key, and CI's verdict line for the row's stem decides the rest.

**Changed criteria, disclosed with no fence lines.** 79922076 also moved literals of earlier SPECs'
tests to this SPEC's figures: `ACensusPackagePaysTheCensus` (SPEC-327) and `TheShardsFitTheirBound`
(SPEC-129). Two more tests of that commit were red at the base and are not criteria here:
`test_the_sizing_constants_are_the_measured_ones` (the price table, `AssertionError: {'dec[112
chars]reak-kernel': 13, ...} != {'dec[112 chars]reak-ffi': 18, ...}`) and
`test_the_whole_tree_is_sized_at_the_batterys_ceiling` (`AssertionError: '32' != '210'`). dad3f9cc
moved 11 derived counts in `test_memory_cap_verdict.py` from 51 and 50 to 143 and 142, its
`sharded()` fixture being `2 * ((SHARD_BOUND_SECONDS - BASELINE_SECONDS) // 126) + 1`, with its
assertions unchanged; and it gave `test_mutation_verdict.py`'s `cargo_report` Baseline a
`log_path`, which R7 reads.

A12 plants, by hand on `PYTHON_SHARD_MUTANTS`: 19 reads `2 != 1 : 20 listed`, 21 reads `1 != 2 : 21 listed`
and 0 reads `ZeroDivisionError`; each fails the pin, and the file is restored byte-equal.

**The signed 2300 s budget, red first.** The pins moved alone, in the commit before the figures: the
byte pins over `BOUNDS`, `CENSUS_NEED_SECONDS`, the census and baseline sums of
`test_mutation_verdict.py`, R7's boundary, the cap verdict's examined counts and the dispatch counts
derived from the new baseline (`docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2300.md`).
Each read red at e0b635cc, the base's code, for the figure and for nothing else.

```red-first
A5: red at e0b635cc: AssertionError: 1430 != 1521
A3: red at e0b635cc: AssertionError: 1768 != 2141
A4: red at e0b635cc: AssertionError: '--timeout 2300' not found in 'cargo mutants --no-shuffle --list --json --in-place --package="$PACKAGE" --timeout 2200 --build-timeout 600 > "$RUNNER_TEMP/size/package.json"'
A5: red at e0b635cc: AssertionError: 3198 != 3662
A6: red at e0b635cc: AssertionError: "VOID mutation-rust-shard-1: the baseline's slowest test" not found, the 2300 s per-mutant timeout being absent
```
