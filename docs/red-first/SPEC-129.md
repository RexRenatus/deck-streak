# Red-first record: SPEC-129

The SPEC, ADR-129 and the SPEC-057 amendment were committed with the tests of A1 to A5 (0e39853)
against the unchanged workflow and verdict script, so every test ran and each red failed by
assertion: the missing `size` verb answers with the usage exit, and the workflow tests find no size
job. The workflow and script change (9d51fc8) turned them green, and the rows commit (ced6887)
factored the shared output writer without changing a result. The replay ran each named test on
0e39853's tree.

```red-first
A1: red at 0e39853: 2 != 0 : usage: mutation-verdict.py [-h] [--root ROOT] [--base BASE] [--head HEAD]
A1: green at ced6887
A2: red at 0e39853: 2 != 0 : usage: mutation-verdict.py [-h] [--root ROOT] [--base BASE] [--head HEAD]
A2: green at ced6887
A3: red at 0e39853: Lists differ: ['no size job'] != []
A3: green at ced6887
A4: red at 0e39853: 0 != 1 : battery: counted 3 of 3 reports whole
A4: green at ced6887
A5: red at 0e39853: 2 != 0 : usage: mutation-verdict.py [-h] [--root ROOT] [--base BASE] [--head HEAD]
A5: green at ced6887
A6: not red: it pins bounds the workflows already hold at dev, so it is green at every commit and each plant of a raised bound turns it red by assertion
```

Fix round 1 added A6 after the delivery was green. It pins bounds that dev already holds (every
`cargo mutants` command line carries `--timeout 300 --build-timeout 600`), so it has no red commit.
Each planted change turned it red by assertion, and the workflow was restored after each:

```text
rust leg --timeout 300 -> 600:        AssertionError: '--timeout 300 --build-timeout 600' not found in 'cargo mutants ... --shard "$SHARD/$SHARDS" --timeout 600 --build-timeout 600 ...'
rust leg --build-timeout 600 -> 1200: AssertionError: '--timeout 300 --build-timeout 600' not found in 'cargo mutants ... --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 1200 ...'
ci.yml   --timeout 300 -> 600:        AssertionError: ... ci.yml: cargo mutants ... --timeout 600 --build-timeout 600 ...
ci.yml   --build-timeout 600 -> 1200: AssertionError: ... ci.yml: cargo mutants ... --timeout 300 --build-timeout 1200 ...
```

Fix round 2 (2026-09-29) made A6 read whole values and every spelling of the command: commands are
found with `\bcargo\s+mutants\b[^\n]*` on text whose `\`-newline continuations are joined first,
and the bounds are matched as `(?<![\w-])--timeout 300 --build-timeout 600(?![\w.])`. A6 stays
`not red:` above, because it is green on every real workflow at every commit. Row S12906 (a build
timeout of `6000` on the rust leg) is the round's red: it survived the earlier A6 at 507570d, and
the test commit 6a1d80a kills it. Each plant below was applied to an export of 6a1d80a, and every
workflow was restored and checked by sha256 after each one:

```text
S12906 at 507570d (earlier A6): SURVIVED: its killer passed with the mutant installed; rows: examined 1: killed 0, survived 1, void 0
S12906 at 6a1d80a (new A6):     KILLED: its killer passed without the mutant and failed with it; rows: examined 1: killed 1, survived 0, void 0
prove --band S12900-S12999 at 6a1d80a: rows: examined 6: killed 6, survived 0, void 0
V8 new .yml, bare `cargo mutants`:               AssertionError: Regex didn't match: '(?<![\\w-])\\-\\-timeout\\ 300 ...' not found in 'cargo mutants' : extra.yml: cargo mutants
V9 new .yaml, bare `cargo mutants`:              AssertionError: Regex didn't match: '(?<![\\w-])\\-\\-timeout\\ 300 ...' not found in 'cargo mutants' : extra.yaml: cargo mutants
M1 rust leg --build-timeout 600 -> 6000:         AssertionError: Regex didn't match: ... not found in 'cargo mutants ... --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 6000 --output "$RUNNER_TEMP/mutation" || rc=$?'
M2 ci.yml   --build-timeout 600 -> 6000:         AssertionError: Regex didn't match: ... not found in 'cargo mutants ... --shard "$SHARD/$SHARDS" --timeout 300 --build-timeout 6000 --output "$out" || rc=$?'
M3 `cargo mutants\` continued, raised bounds:    AssertionError: Regex didn't match: ... not found in 'cargo mutants             --no-shuffle ... --timeout 900 --build-timeout 1800 --output "$RUNNER_TEMP/mutation" || r...'
M4 new .yml, folded `run: >-`, raised bounds:    AssertionError: Regex didn't match: ... not found in 'cargo mutants' : extra.yml: cargo mutants
V1 continued with ` \`, the same bounds on line 2: green (rc 0): the command is bounded, so the guard passes it
```

M4's folded scalar reaches the guard as `cargo mutants` alone, so the bounds on its later lines
are not read, and the command is refused for want of them. The earlier plants B1 to B4 and V2 to
V7 stay red with the new A6 as they were with the old one.

## Addendum, 2026-09-29 (issue #395): A7 for every spelling of the command

The lines above stand. The red commit moves the guard's scan into a function of a directory and adds
three planted-workflow tests over the old pattern; the green commit 9a7b2141 changes only the pattern and its
comment, in `scripts/tests/test_dispatch_shards.py`, because the guard's pattern lives in the test module. The whole test file at the red commit fails only the three new tests, each by assertion.

```red-first
A7: red at 1b6bc977: AssertionError: {} != {'planted.yml': ['cargo +nightly mutants --in-place']} (toolchain); AssertionError: {} != {'planted.yml': ['cargo-mutants mutants --in-place']} (binary form); AssertionError: 1 != 2 : ['cargo mutants --timeout 300 --build-timeout 600 ; cargo mutants --in-place'] (two commands on one line)
A7: green at 9a7b2141
A8: red at 7b9ae9cc: AssertionError: {} != {'planted.yml': ['cargo --config net.retry=2 mutants --in-place']} (a flag's separate value); AssertionError: {'pla[13 chars]cargo mutants --in-place # --timeout 300 --build-timeout 600']} != {'pla[13 chars]cargo mutants --in-place']} (a comment)
A8: green at 1e54fe48
```

## Addendum, 2026-09-29 (issue #395, round 2): a flag's value, a comment, and A7 replayed

The lines above stand. The red commit 7b9ae9cc adds three planted-workflow tests; the green commit
1e54fe48 changes only the start pattern (a global flag with a separate value) and cuts comments
before any command is read, in `scripts/tests/test_dispatch_shards.py`, because the guard lives in
the test module. The whole test file at the red commit fails only the three new tests, each by
assertion. A8 is recorded in the fence above; A7 keeps its one red and one green line, and its new
test's replay is quoted here.

```text
A7 replay: red at 7b9ae9cc: AssertionError: 1 != 2 : ['cargo mutants --timeout 300 --build-timeout 600 && cargo -C crates mutants --in-place']
A7 replay: green at 1e54fe48: Ran 18 tests, OK
```

## Addendum, 2026-09-29 (issue #395, round 3): a comment right after a shell operator

The lines above stand, and A8 keeps its one red and one green line; this replay is quoted below
them. The red commit cc3cfb8e adds one assertion to the comment test, a comment that starts right
after `;`; the green commit 39426b86 changes only the guard's word-start rule and its docstring, in
`scripts/tests/test_dispatch_shards.py`, because the guard lives in the test module. The whole test
file at the red commit fails only that test, by assertion.

```text
A8 replay: red at cc3cfb8e: FAILED (failures=1), AssertionError: {'pla[13 chars]cargo mutants --in-place;# --timeout 300 --build-timeout 600']} != {'pla[13 chars]cargo mutants --in-place;']}
A8 replay: green at 39426b86: Ran 18 tests, OK, examined 3 ci.yml commands, examined 4 mutation-weekly.yml commands
```
