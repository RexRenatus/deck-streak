# Red-first record: SPEC-195

The SPEC and ADR-195 were committed alone (506d60af). Then came the check (0cf79dda) against the
unchanged diagrams, so every test ran and the one red failed by assertion. The fix of the eight
blocks (2a3615ba) turned it green. The replay ran the check on 0cf79dda's tree: four tests, one red
by assertion.

```red-first
A1: red at 0cf79dda: AssertionError: expected [ …(8) ] to deeply equal []: the received list was "schematics/alert-and-slo-path.md block 1", "schematics/alert-and-slo-path.md block 2", "schematics/data-rights-export-and-erase.md block 3", "schematics/mutation-testing.md block 1", "schematics/owner-session.md block 1", "schematics/owner-session.md block 3", "schematics/service-lifecycle.md block 3", "schematics/sync-cycle-and-change-gate.md block 5"
A1: green at 2a3615ba
A2: not red: the reader finds the blocks on the unfixed tree too, since a block that does not parse is still a block; the criterion pins that it keeps finding them
A3: not red: the parser refuses a reserved-word id on any tree; the criterion pins that the check itself can refuse
A4: red at cbb4047b: AssertionError: expected [] to deeply equal [ 'planted.md block 1' ]: the reader found no block in an indented fence, in a list item or by three spaces, so both new tests failed
A4: green at 5855fa5b
```

The reader was then extended to read an indented fence. Its test (A4) was committed alone
(cbb4047b) against the unchanged reader, so both new tests failed by assertion while the other four
passed; the reader change (5855fa5b) turned all six green. The test file is unchanged between those
two commits except for the reader and the opener count.

The replay of A1 ran the check on 0cf79dda's tree from a detached checkout with
`pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "parses every block"` (exit 1):

```text
AssertionError: expected [ …(8) ] to deeply equal []
- Expected
+ Received
- []
+ [
+   "schematics/alert-and-slo-path.md block 1",
+   "schematics/alert-and-slo-path.md block 2",
+   "schematics/data-rights-export-and-erase.md block 3",
+   "schematics/mutation-testing.md block 1",
+   "schematics/owner-session.md block 1",
+   "schematics/owner-session.md block 3",
+   "schematics/service-lifecycle.md block 3",
+   "schematics/sync-cycle-and-change-gate.md block 5",
+ ]
```

The replay of A4 at cbb4047b, over the whole file (exit 1, 2 failed, 4 passed):

```text
AssertionError: expected [] to deeply equal [ 'planted.md block 1' ]
```

The same line is printed for both tests, `reads an indented fence and refuses one that does not parse`
and `accepts an indented valid block of each diagram type the docs use`. At 5855fa5b all six tests
pass and 183 blocks are examined, the same 183 as before.
