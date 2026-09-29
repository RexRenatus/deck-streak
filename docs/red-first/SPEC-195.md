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
A5: red at 9de26193: AssertionError: expected [] to deeply equal [ 'planted.md block 1' ]: the reader found no block in a quoted fence or in one spaced before `mermaid`, so both new tests failed
A5: green at e39a1d01
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

The reader was extended once more, to read a fence inside a blockquote (with a space after `>`, with
none, and as a list item in a quote) and one with blanks before `mermaid`. Its test (A5) was
committed alone (9de26193) against the unchanged reader, and the reader change (e39a1d01) turned all
eight green; the test file is unchanged between those two commits except for the reader and the
opener count. The replay of A5 at 9de26193, over the whole file, printed `Tests  2 failed | 6 passed (8)`
and, for both tests, `reads a quoted fence and one spaced before its info string, and refuses one that
does not parse` and `accepts a quoted valid block with a quoted blank line in it`:

```text
AssertionError: expected [] to deeply equal [ 'planted.md block 1' ]
```

At e39a1d01 all eight tests pass and 190 blocks are examined, the count of the merged tree.

Round 3 replaced A5's hand-listed quoted plants with a population generated from the reader's own
prefix grammar: five quote prefixes up to depth 2, alone and followed by a list marker, with two
fence spellings, which is 30 members, each planted unparsable (refused by name) and valid (accepted).
The test asserts the count and prints `examined 30 quoted fence forms`. The generated test cannot be
red against the correct reader, so it is recorded not red, and its proof that it can fail is a kill
of two mutants of the reader, each applied to a scratch copy of the test file at 787b3534 (the tree
holds no such copy):

```text
A5: not red: the generated test passes at the reader as it is (8 passed, 190 blocks); it is killed by two mutants of the reader
M1 (OPENER `>[ \t]?` changed to `>[ \t]`): Tests  1 failed | 7 passed (8)
AssertionError: {"quote":">","marker":"","fence":"```mermaid"}: expected [] to deeply equal [ 'planted.md block 1' ]
M2 (the quote group `(?:>[ \t]?)*` changed to `(?:>[ \t]?)?`, depth 1 only): Tests  1 failed | 7 passed (8)
AssertionError: {"quote":"> > ","marker":"","fence":"```mermaid"}: expected [] to deeply equal [ 'planted.md block 1' ]
```

The member `>` with no marker and the fence ```` ```mermaid ```` is the string `>```mermaid` that
review planted by hand, so that plant is one member of the population.
