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

## Addendum, 2026-09-29 (issue #395, round 4): a comment mark after a substitution

The lines above stand, and A8 keeps its one red and one green line; this replay is quoted below
them. The red commit 3a2383bc adds one loop to the comment test, four in-word hashes (`$(true)#`,
`<(true)#`, `$((1))#` and `${X//;#/}`) each followed by an unbounded command; the green commit
f15ca871 changes only the guard's word-start rule and its docstring, in
`scripts/tests/test_dispatch_shards.py`, because the guard lives in the test module. The whole test
file at the red commit fails only that test, by assertion.

```text
A8 replay: red at 3a2383bc: FAILED (failures=1), AssertionError: Lists differ: [] != ['cargo mutants --in-place']
A8 replay: green at f15ca871: Ran 18 tests, OK, examined 4 in-word hashes, examined 3 ci.yml commands, examined 4 mutation-weekly.yml commands
```

## Addendum, 2026-09-29 (issue #395, round 5): the comment class, generated

The lines above stand, and A8 keeps its one red and one green line; this replay is quoted below
them. The red commit d9cdca86 adds one test, which generates the members of the comment class (a
fragment that carries a would-be comment or a would-be closer, in each context and in a group
inside each substitution, then a `#` and an unbounded command, the bounds, or a continued line) and
has bash read each member; the green commit 3ce1d37d changes only the guard's comment rule and
its docstrings, in `scripts/tests/test_dispatch_shards.py`, because the guard lives in the test
module. The whole test file at the red commit fails only that test, by assertion.

```text
A8 replay: red at d9cdca86: FAILED (failures=1), AssertionError: Lists differ: ['cargo mutants --timeout 300 --build-time[59 chars]e\n'] != []
A8 replay: green at 3ce1d37d: Ran 19 tests, OK, examined 2004 class members, examined 4 in-word hashes, examined 3 ci.yml commands, examined 4 mutation-weekly.yml commands
```

## Addendum, 2026-09-30 (issue #395, round 6): a context left open at its line's end

The lines above stand, and A8 keeps its one red and one green line; this replay is quoted below
them. The red commit 2b3a7b33 adds to the generated class each context left open at its line's
end or continued inside it, with its closer and a `#` on the next line or the one after, and the
bounds after an unbounded command on a continued line; the green commit 0046be9d changes only
the guard's comment rule and its docstring, in `scripts/tests/test_dispatch_shards.py`, because
the guard lives in the test module. The whole test file at the red commit fails only that test, by
assertion.

```text
A8 replay: red at 2b3a7b33: FAILED (failures=1), AssertionError: Lists differ: ['cargo mutants --timeout 300 --build-time[40 chars]e\n'] != []
A8 replay: green at 0046be9d: Ran 19 tests, OK, examined 4291 class members, examined 4 in-word hashes, examined 3 ci.yml commands, examined 4 mutation-weekly.yml commands
```

## Addendum, 2026-09-30 (issues #395 and #447, round 8): the reading at the grammars

The lines above stand, and A8 keeps its one red and one green line; its replay is quoted below
them. A9, A10 and A11 are new. The red commit a2b3b953 adds three tests. The first generates the
members of the grammar class (each shell text of an axis, crossed with each YAML spelling that
reads back as that text) and has bash run each text. The second takes from those members the texts
that bash computes and hands to a shell or to a builtin that reads them as shell, and three literal
texts handed to one, and expects each refused. The third states the declared reading: a bounded
command after each leading word, which bash runs bounded, is found as it is, and a change to how
bash reads, a continued line in an expanded here-document and an unstated expression are refused
for that. The red commit also changes one expectation of the comment test: a command found before
a `;` is shown without the `;`. The green commit d76d8f2b changes only the guard and its
docstrings, in `scripts/tests/test_dispatch_shards.py`, because the guard lives in the test module:
it reads each `run:` value as YAML and bash read it, or refuses it. The whole test file at the red
commit fails only those four tests, by assertion.

```red-first
A9: red at a2b3b953: AssertionError: Lists differ: ['jobs:\n  shard:\n    runs-on: ubuntu-24.[114 chars]0\n'] != [] : 1371 of 2812 unbounded members pass
A9: green at d76d8f2b
A10: red at a2b3b953: AssertionError: Lists differ: ['cargo mutants --timeout 300 --build-timeout 600\')"'] != ['refused: a text bash computes for `bash` to read as shell']
A10: green at d76d8f2b
A11: red at a2b3b953: AssertionError: Lists differ: ['cargo mutants --timeout 300 --build-timeout 600; then :; fi'] != ['cargo mutants --timeout 300 --build-timeout 600']
A11: green at d76d8f2b
```

```text
A8 replay: red at a2b3b953: FAILED (failures=4), AssertionError: {'planted.yml': ['cargo mutants --in-place;']} != {'planted.yml': ['cargo mutants --in-place']}
A8 replay: green at d76d8f2b: Ran 22 tests, OK, examined 6485 grammar members, examined 22 computed texts, examined 3 literal texts, examined 13 leading words, examined 12 texts the reading does not read, examined 4291 class members, examined 4 in-word hashes, examined 3 ci.yml commands, examined 4 mutation-weekly.yml commands
```

## Addendum, 2026-09-30 (issues #395 and #447, round 8): a computed word before the bounds, and the weekly sweep in literal words

The lines above stand. A12 and A13 are new. The weekly sweep's two package-bearing `cargo mutants`
commands change spelling in `.github/workflows/mutation-weekly.yml`, and a pin test holds the
head's two commands as literals and has bash run both spellings with a stub cargo. Commit 484c8da0
adds the pin test with the workflow. Its plant is a copy of the rewritten block that drops the
package word from the set branch; the pin test reads it red, by assertion. Commit 903b9cd2 adds the
R5 tests, which fail at the guard of commit 3 by assertion, and commit 154d51e8 changes only the
guard. The whole test file at 903b9cd2 fails only the R5 tests. The green line is the whole module
at 154d51e8.

```red-first
A13: not red: the pin test is added with the workflow it pins, so it is green at the one commit that has it, and its plant of the set branch dropping the package word turns it red by assertion
A12: red at 903b9cd2: AssertionError: Lists differ: ['X=--; set -- --; cargo mutants --in-plac[37 chars]0\n'] != [] : 20 of 20 members that lose the bounds pass
A12: green at 154d51e8
```

```text
A13 replay: red at 484c8da0 (plant: the set branch drops the package word), green at 484c8da0: Ran 25 tests, OK, examined 6 mutation-weekly.yml commands, examined 40 package values, examined 80 old-against-new argvs, examined 6 dash-led values
A12 replay: red at 903b9cd2: FAILED (failures=3), AssertionError: Lists differ: [False, False, False] != [True, True, True]
A12 replay: green at 154d51e8: Ran 29 tests, OK, examined 980 R5 members, examined 20 R5 members bash runs without the bounds, examined 3 named R5 members, examined 6 weekly commands, examined 9 real-tree commands, examined 6485 grammar members, examined 4291 class members, examined 6 mutation-weekly.yml commands, examined 3 ci.yml commands
```

## Addendum, 2026-09-30 (issues #395 and #447, round 8, after the merge of `dev`): the memory scope's wrapper

The lines above stand. A14 is new. `dev` runs each `cargo mutants` that runs tests inside the memory
scope's wrapper, which the guard refused as another program's command, so the merge at 04fb802e
left the two real-tree tests red. Commit 387dee53 pins `test_memory_scope.py`'s weekly commands in
their literal branches. Commit 644aabcf adds the wrapper tests, and plants the wrapper where the pin
test's bash runs the weekly block; at the guard of 04fb802e the two new class tests fail by
assertion, beside the two real-tree tests. Commit 0f8eacad changes only the guard and the mutation
rows. The whole test file at 644aabcf fails only those four tests. The green line is the whole module
at 0f8eacad.

```red-first
A14: red at 644aabcf: AssertionError: Lists differ: [('exact', 'shell text', '@', "python3 scr[69 chars]\n")] != [] : 108 of 616 members that lose the bounds pass
A14: green at 0f8eacad
```

```text
A14 replay: red at 644aabcf: FAILED (failures=4), AssertionError: Lists differ: [(('exact', 'bounded', '@'), ['refused: `c[56 chars]s'])] != [] : 32 of 32 members
A14 replay: green at 0f8eacad: Ran 33 tests, OK, examined 1 options the wrapper declares, examined 374 wrapper argvs, examined 3 wrapper plants, examined 1160 wrapper-axis members, examined 616 wrapper-axis members bash runs without the bounds, examined 32 declared-form members, examined 6 weekly commands, examined 9 real-tree commands
```
