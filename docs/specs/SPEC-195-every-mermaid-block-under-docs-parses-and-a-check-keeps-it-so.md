# SPEC-195: every Mermaid block under docs parses, and a check refuses one that does not

- **Wave:** none of its own: issue #383 carries no wave label (the ci-efficiency lane's rules bind
  it). **Issue:** #383. **Context(s):** `repo` (`docs/schematics/` and `web/app`'s test suite).
- **Decided by:** ADR-195 (this SPEC's own: where the check runs and what it was chosen against).
- **Status:** delivered. It holds `docs/red-first/SPEC-195.md`.

## 1. The problem, measured

- **Eight diagrams do not render.** Mermaid's command-line renderer, run over every fenced
  `mermaid` block under `docs/` on `dev`, exits non-zero for eight of 183 blocks, so a reader sees
  an error box instead of a diagram (#383):
  `alert-and-slo-path.md` blocks 1 and 2, `data-rights-export-and-erase.md` block 3,
  `mutation-testing.md` block 1, `owner-session.md` blocks 1 and 3, `service-lifecycle.md` block 3
  and `sync-cycle-and-change-gate.md` block 5.
- **The faults are three.** A `;` in a sequence or state label ends the statement, so the rest of
  the label reads as a second statement (seven blocks). A `|...|` edge label that holds
  parentheses, and one that holds an `@`, is read as shape syntax or an edge id (two blocks).
- **Nothing refuses one.** No job parses a diagram, so each of the eight merged green.

## 2. Requirements

R1. **Every fenced `mermaid` block in every Markdown file under `docs/` parses** with Mermaid's own
parser, `mermaid.parse`, which returns a diagram type for a valid block and `false` for an invalid
one when `suppressErrors` is set.

R2. **The check is one Vitest file in `web/app`,** `src/lib/docs-mermaid.test.ts`, run by the `web`
job's existing `vitest run`. It adds no job, no workflow step and no browser download, and it
weakens no existing check.

R3. **`mermaid` and `commonmark` are pinned devDependencies of `web/app`,** each an exact version
(`mermaid` 11.17.2; `commonmark` 0.31.2, with its types `@types/commonmark` 0.27.10), so neither the
parser that judges the blocks nor the parser that finds them moves with a caret range.

R4. **The check reads every block.** It finds the blocks with a CommonMark parse (R9), and
cross-checks the parse on the documents: every line under `docs/` that opens a `mermaid` fence in
any container (blanks, `>` and list markers, then three or more backticks or tildes and `mermaid`) is
the first line of a block it reads. It refuses a count under 100, so a change to the reader that
finds none fails.

R5. **A refusal names the block:** the file's path under `docs/` and the block's number counted from
1, one entry per unparsable block.

R6. **The check agrees with the renderer.** On the blocks the renderer was run over, the check refuses
exactly the blocks the renderer exits non-zero for: eight of 183 before the fix, and none of the 191
the merged tree holds after it.

R7. **The eight blocks are fixed by ids and syntax only.** Every label renders the same text: a
`;` becomes the entity `#59;` (rendered as `;`), and an edge label holding `(` or `@` is quoted.

R8. **A planted block whose node id is a reserved word is refused, and the same block with another
id is accepted,** in the check's own test.

R9. **Every fence GitHub renders as a diagram is read, whatever container opens its line, and no
other.** A block is a fenced code block (three or more backticks or tildes) whose info string's first
word, up to its first ASCII blank, is `mermaid`, and whose text is not blank. A CommonMark 0.31.2
parse (`commonmark`) finds it, so block quotes, list items (with a bullet or one to nine digits, the
fence on the line after the marker or on the marker line itself), blanks and lazy lines nest in any
order and at any depth, exactly as CommonMark defines them. The parse opens a fence as GitHub's
cmark-gfm does, counting the fence's indentation in characters rather than columns: when a container
prefix consumes part of a tab, the block keeps the tab's remaining columns on each line, as GitHub's
does. The block's text is the text GitHub renders as the diagram.

The check's own test generates its members at test time from the container and fence grammar table
in `web/app/scripts/docs-mermaid-fences.js`. For every sequence of block quotes and list items up to
depth 4, with every list item's fence on the line after its marker and again on its marker line, it
generates the base member and every member that differs from it in one token (a level's blanks,
marker or gap, the fence's characters, info string or indentation, or the body), and for one level,
every combination of that level's tokens: 3,241 members. `web/app/src/lib/docs-mermaid.fences.json`
records GitHub's rendering of each (`gh api markdown`), and only
`web/app/scripts/record-docs-mermaid-fences.js` refreshes it. The test asserts that the recorded
table, member set and count equal the generated ones, so the two cannot drift apart. For every
member, the blocks the check reads are exactly the diagrams GitHub renders, with GitHub's text. Each
rendered member planted unparsable is refused by name, and each planted valid is accepted.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every fenced `mermaid` block under `docs/` parses | `docs-mermaid.test.ts` `parses every block` |
| A2 | every line under `docs/` that opens a `mermaid` fence in any container is the first line of a block the check reads, and at least 100 blocks are read | `docs-mermaid.test.ts` `reads every fenced block` |
| A3 | a block whose node id is a reserved word is refused, and the same block with another id is accepted | `docs-mermaid.test.ts` `reserved word` (two tests) |
| A4 | an indented fence, in a list item or by one to three spaces, is read: an unparsable one is refused by name, and a valid one of each diagram type the documents use is accepted | `docs-mermaid.test.ts` `indented` (two tests) |
| A5 | in each of 3,241 container forms generated at test time from the grammar table the test imports, the check reads exactly the fences GitHub renders as diagrams, with GitHub's text, and none GitHub shows as code; each rendered form planted unparsable is refused by name, and planted valid is accepted | `docs-mermaid.test.ts` `generated container form` (two tests) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "parses every block"
A2: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reads every fenced block"
A3: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reserved word"
A4: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "indented"
A5: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "generated container form"
```

Each fence line selects its tests: A1 one test, A2 one, A3 two, A4 two and A5 two, and no test is
selected by two lines.

The command-line renderer is the oracle for R6: it is run once, by hand, over the extracted blocks
before and after the fix, and its table of block to exit code is the delivery's evidence. It is not
part of the gate.

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/docs-mermaid.test.ts` | repo | added: A1 to A5 |
| `web/app/src/lib/docs-mermaid.fences.json` | repo | added: A5, GitHub's recorded rendering of the generated members |
| `web/app/scripts/docs-mermaid-fences.js` | repo | added: A5, the grammar table and the member generator |
| `web/app/scripts/record-docs-mermaid-fences.js` | repo | added: A5, the script that refreshes the recorded rendering |
| `web/app/package.json` | repo | changed: R3, the `mermaid`, `commonmark` and `@types/commonmark` devDependencies |
| `pnpm-lock.yaml` | repo | changed: the lock of those dependencies |
| `docs/schematics/alert-and-slo-path.md` | repo | changed: R7, two blocks |
| `docs/schematics/data-rights-export-and-erase.md` | repo | changed: R7, one block |
| `docs/schematics/mutation-testing.md` | repo | changed: R7, one block |
| `docs/schematics/owner-session.md` | repo | changed: R7, two blocks |
| `docs/schematics/service-lifecycle.md` | repo | changed: R7, one block |
| `docs/schematics/sync-cycle-and-change-gate.md` | repo | changed: R7, one block |
| `docs/specs/SPEC-195-every-mermaid-block-under-docs-parses-and-a-check-keeps-it-so.md` | repo | added |
| `docs/decisions/ADR-195-a-vitest-test-parses-every-mermaid-block-under-docs.md` | repo | added |
| `docs/red-first/SPEC-195.md` | repo | added |
| `changelog.d/docs-mermaid-parse-195.md` | repo | added |

No new schematic: the change adds no component; it corrects six existing ones.

## 5. What this does NOT do

- It does not render a diagram or check how one looks, because a parser answers whether a block is
  valid and nothing else (#383).
- It adds no job and no workflow step, because the `web` job's `vitest run` already runs every test
  file under `web/app/src` (#383).
- It does not change any diagram's meaning: no label's text, no node, no edge (#383).
- It does not read a `mermaid` fence that GitHub shows as code: one inside a longer fence or an
  indented code block, one whose text is blank, and one whose language word is not exactly
  `mermaid` (#383). Measured on the merged tree: 191 blocks, the same 191 with the same text as the
  fence-line reader it replaced, and the documents hold no blockquote line, no four-backtick fence
  and no tilde fence (#383).
- It owes no mutation rows, because no production file changes and the check is test code (#383).
  The mutation plan reads the same: on this pull request it selects no tool and no row.
- It changes no Rust and no Python, because the defect is in Markdown and the check is a test in the
  Mini App's suite (#383).

## 6. Risks

- **A new Mermaid syntax the pinned parser does not know.** A diagram valid in a later release
  would be refused until the pin moves; the refusal names the block, and moving the pin is one
  reviewed change to `web/app/package.json`.
- **The parser and the renderer disagree on some future block.** R6 holds for the blocks it was
  measured over, 183 before the fix and 191 on the merged tree; the two share one grammar, and the
  oracle run in this delivery is the evidence.
- **A container form the grammar table does not generate.** The test compares the check with
  GitHub on the table's members only. A form outside the table (a GFM footnote definition or alert,
  an HTML block, a table beside a fence) is read by CommonMark's rules and is not compared on each
  run. Before the table was written, the reader was measured on 71,176 texts (35,588 generated
  members, each planted unparsable and valid) and on probes of an alert, a `<details>` block with a
  blank line, a table after a fence and two footnote definitions, and it read exactly what GitHub
  renders in each. A form found later is one more alternative in the table and one run of the
  refresh script.
- **False reads, on the fail-closed side: none measured.** Over those 71,176 texts and the 3,241
  generated members, the check reads no fence that GitHub shows as code. The fence-line reader it
  replaced read 3,316 such texts (an opener such as `mermaidx`, top-level indentation of four
  columns, a fence inside a four-backtick example, a blank block). A fence GitHub shows as code that
  the check did read would be refused by name when its text does not parse, and accepted when it
  does.
- **The reader copies one GitHub behaviour through commonmark.js's internals.** cmark-gfm counts a
  fence's indentation in characters where CommonMark counts columns. The check wraps commonmark.js
  0.31.2's fenced-code start to do the same, which reads the parser's `blockStarts`, `tip`, `offset`
  and `nextNonspace`. The pin is exact, so these move only with a reviewed change, and the 22
  generated members whose fence follows a partly consumed tab (after a `>`, or in the fence's own
  indentation) fail if the wrapper stops matching GitHub.
- **GitHub changes how it reads a container.** The recorded rendering is GitHub's on the day it was
  recorded. After such a change the check keeps the recorded reading until the refresh script is
  run, and the test then names every member whose rendering moved.

## 7. References

Issue #383; ADR-195; the ci-efficiency lane's rules.
