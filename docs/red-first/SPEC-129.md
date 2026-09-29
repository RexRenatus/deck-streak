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
