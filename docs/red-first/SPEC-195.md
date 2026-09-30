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
A5: red at a5881aba: AssertionError: 946 of 8750 members read otherwise: expected [ …(3) ] to deeply equal []: round 5's reader read 946 of the 8,750 generated members otherwise than GitHub renders them and refused none by name, so all three A5 tests failed
A5: green at 071043ea
A6: red at dca03020: AssertionError: expected [ …(2) ] to deeply equal []: the received list was "reads every fenced block" and "parses every block", each coming before a generated-population test, so the order test failed
A6: green at c2fa5aa0
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

Round 5 replaced the fence-line reader with a CommonMark parse that opens a fence as GitHub does,
and A5's listed axes with 3,241 members generated at test time from a grammar table and compared
with GitHub's recorded rendering of each. Its test (A5, two tests, with the grammar table, the
recorded rendering and the refresh script) was committed alone (63fb9832) against the unchanged
reader, and the reader change (57bc9aec) turned all eight green; the test file is unchanged
between those two commits except for the reader, its import and A2. The replay of A5 at 63fb9832,
over the whole file, printed `Tests  2 failed | 6 passed (8)`:

```text
A5: red at 63fb9832: AssertionError: 1665 of 3241 members read otherwise: expected [ …(3) ] to deeply equal []: the fence-line reader read 1,665 of the 3,241 generated container forms otherwise than GitHub renders them, so both new tests failed
A5: green at 57bc9aec
```

At 57bc9aec all eight tests pass and 191 blocks are examined, the same 191 with the same text.

After the reader change, the test file gained a presence assertion in each A5 test and spells its opener cross-check regular expression with `\x60` for the backtick, so the tdd probe reads the file as written: the assertions and the pattern change no verdict, and the failure quoted above is the failure of the test as committed at 63fb9832. Then c5f53771 moved the generator to `web/app/src/lib/docs-mermaid-fences.js`, changed the test's import to it, found `docs/` by walking up to `pnpm-workspace.yaml`, and added a ninth test, `refuses a grammar whose body names no row, and says which`; none changes a verdict of the eight.

Round 6 made the reader read only a declared subset of Markdown, on which commonmark.js opens the same fences as GitHub's cmark-gfm, and refuse by name every form outside it that could make, hide or change a diagram, and moved the reader into `web/app/src/lib/docs-mermaid-read.js`. A5's generator draws the class members from the reader's exported lists: 8,750 members. The test, the generator, the refresh script and the re-recorded rendering were committed alone (a5881aba) against the unchanged reader, and the reader (071043ea) turned all nine tests green. The test and the generator are unchanged between those two commits except that the red commit carries round 5's reader in the test and the reader's three lists in the generator, where the reader's commit imports them. The ninth test is renamed `refuses a generated container form it cannot write or read, and says which`, so A5 selects it with the two comparison tests. The replay of A5 at the red commit, over the whole file, printed `Tests  3 failed | 6 passed (9)`. The first member read otherwise is `info.20.decimal0.before.top`, a character reference for a space before `mermaid` at the top level, which GitHub draws as a diagram and round 5's reader does not read:

```text
A5: red at a5881aba: AssertionError: 946 of 8750 members read otherwise: expected [ …(3) ] to deeply equal []: round 5's reader read 946 of the 8,750 generated members otherwise than GitHub renders them and refused none by name, so all three A5 tests failed
A5: green at 071043ea
```

At the reader's commit all nine tests pass, the reader reads 2,711 of the 8,750 generated members and refuses the other 6,039 by name, and 191 blocks are examined, the same 191 with the same text.

Round 7 took each refusal from a limit or a start condition of cmark-gfm: a block in more than 99 block quotes and list items, a block GitHub may nest more than 240 elements deep, a line on which cmark-gfm may open raw HTML, and a special tag inside a line unless it stands in a code span both readers form alike. A5's generator draws 244 more members: each bound the reader exports, met at its edge and past it, with each term of the page bound on its own, and a line that may open raw HTML inside each inline construct that carries text across a line end: 8,994 members. The test, the generator and the re-recorded rendering were committed alone (b3b1bdbc) against the unchanged reader, with the bounds written in the generator, and the reader (55227769), which exports them, turned all nine tests green. The ninth test asserts fourteen members' refusals, form for form. The replay of A5 at the red commit, over the whole file, printed `Tests  2 failed | 7 passed (9)`. The first member read otherwise is `bound.qi.100`, a fence inside 100 block quotes and list items, alternating, where cmark-gfm opens no more than 99 on a line, which GitHub shows as code and round 6's reader reads:

```text
A5: red at b3b1bdbc: AssertionError: 39 of 2840 members read otherwise: expected [ …(3) ] to deeply equal []: round 6's reader read 39 of the 8,994 generated members otherwise than GitHub renders them, each a text GitHub shows as code, so two tests failed
A5: green at 55227769
```

At the reader's commit all nine tests pass, the reader reads 2,934 of the 8,994 generated members and refuses the other 6,060 by name, and 191 blocks are examined, the same 191 with the same text.

Round 8 measured that the mutation run's cost was the order of the tests, not the reader: a mutant is tried against the tests that cover it in file order and stops at the first that fails, and the two docs-wide tests, which take seconds, stood before the generated-population tests that kill nearly every mutant. The guard (A6) was committed alone (dca03020) against the old order, so the order test failed by assertion while the other two passed. The next commit (c2fa5aa0) moves the three generated-population tests first and edits no test: each of the nine `it` blocks is byte-equal to its block at cea1f3b8, and the file's sorted lines are equal. The guard also fails under each of three plants, a docs-wide test moved first, a title deleted and a title duplicated, and it holds the populations: 8,994 members, 2,934 read, 6,060 refused, `READ_DIGEST` and the recorded `digest`.
