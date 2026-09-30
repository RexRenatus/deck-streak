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

R4. **The check reads every block, and refuses what it cannot read.** It finds the blocks with the
reader (R9), and it fails on every form the reader refuses in a document under `docs/`. Among those
forms is every line that may open a `mermaid` fence in any container (blanks, `>` and list markers,
then three or more backticks or tildes, then an info string holding `mermaid` in any case, or a `&`)
and is not the first line of a block it reads. It refuses a count under 100, so a change to the
reader that finds none fails.

R5. **A refusal names the block:** the file's path under `docs/` and the block's number counted from
1, one entry per unparsable block. A form the reader refuses is named by the file's path under
`docs/`, its line counted from 1, and the form.

R6. **The check agrees with the renderer.** On the blocks the renderer was run over, the check refuses
exactly the blocks the renderer exits non-zero for: eight of 183 before the fix, and none of the 191
the merged tree holds after it.

R7. **The eight blocks are fixed by ids and syntax only.** Every label renders the same text: a
`;` becomes the entity `#59;` (rendered as `;`), and an edge label holding `(` or `@` is quoted.

R8. **A planted block whose node id is a reserved word is refused, and the same block with another
id is accepted,** in the check's own test.

R9. **Every `mermaid` fence GitHub renders as a diagram is read, and none that GitHub shows as
code; a form the reader does not read as GitHub does is refused by name.** GitHub renders Markdown
with cmark-gfm and draws a diagram for each `<pre lang="mermaid">` in the HTML it makes, and only a
fenced code block whose language word is `mermaid`, or raw HTML, writes one. The reader,
`web/app/src/lib/docs-mermaid-read.js`, reads a declared subset of Markdown, on which a CommonMark
0.31.2 parse (`commonmark`) opens the same fences as cmark-gfm, with the same text. A block is a
fenced code block (three or more backticks or tildes) whose info string is `mermaid` alone, with
spaces or tabs around it, and whose text is not blank. The parse finds it, so block quotes, list
items (with a bullet or one to nine digits, the fence on the line after the marker or on the marker
line itself), blanks and lazy lines nest in any order and at any depth up to 99 lists, exactly as
CommonMark defines them. The parse opens a fence as GitHub's cmark-gfm does, counting the fence's
indentation in characters rather than columns: when a container prefix consumes part of a tab, the
block keeps the tab's remaining columns on each line, as GitHub's does. The block's text is the text
GitHub renders as the diagram. Outside the subset the reader reads nothing, and it refuses by line
and name each form that could make, hide or change a diagram: a raw HTML block; a `<` at the start
of a line that a letter, or `/` and a letter, follows; raw inline HTML that opens its line, that is a
comment, a processing instruction, a declaration or a CDATA section, or whose tag name is in its list
(the HTML standard's special elements, the SVG and MathML roots, `image`, and every HTML block tag of
cmark-gfm and of CommonMark 0.31.2); a character in its list, wherever it stands (a byte-order mark,
each C0 and C1 control but tab, line feed and carriage return, DEL, the line and paragraph
separators, and each noncharacter); a fence whose info string holds `mermaid` in any case, or a `&`,
and is not `mermaid` alone; a `mermaid` fence in more than 99 lists; a `mermaid` block whose text is
blank; and every line that may open a `mermaid` fence and is not the first line of a block it reads.
Its lists are exported, and the test's generator reads them.

The check's own test generates its members at test time from the container and fence grammar table
in `web/app/src/lib/docs-mermaid-fences.js`. For every sequence of block quotes and list items up to
depth 4, with every list item's fence on the line after its marker and again on its marker line, it
generates the base member and every member that differs from it in one token (a level's blanks,
marker or gap, the fence's characters, info string or indentation, or the body), and for one level,
every combination of that level's tokens: 3,241 members. It adds the class members, on each axis
where cmark-gfm's source or GitHub's HTML parse reads a line otherwise than CommonMark 0.31.2: every
blank in every raw, numeric and named spelling before, after and inside the language word; each
before a backtick in a backtick fence's info string, after a list marker that interrupts a
paragraph, and as a diagram's whole text; each end of each range GitHub shows as U+FFFD and of each
range the reader refuses, and the code point either side of it; a byte-order mark before a fence on
lines 1 to 3 and in a quote and a list item; every tag of both type 6 lists and the type 1 list, and
every other HTML block start, interrupting a paragraph, opening a block and on a lazy line, at the
top and in a quote and a list item; the raw HTML before a fence that GitHub's HTML parse reads (a
run left open, a tag the tag filter escapes, a state an earlier block leaves open); raw HTML inside a
line, with every tag name of the grammar and of the reader's list; a fence in a footnote
definition, a table and a task list item; each line that holds `mermaid` and opens no fence; and a
fence in 99 lists and in 100: 8,750 members in all. `web/app/src/lib/docs-mermaid.fences.json`
records GitHub's rendering of each (`gh api markdown`), and only
`web/app/scripts/record-docs-mermaid-fences.js` refreshes it, rendering every text that holds `<` or
opens with a byte-order mark alone. The test asserts that the recorded table, member set and count
equal the generated ones, so the two cannot drift apart. For every member the reader reads without a
refusal, the blocks the check reads are exactly the diagrams GitHub renders, with GitHub's text, and
the test pins how many such members there are (2,711), so a reader that refuses more or fewer
of them fails. Each rendered member read without a refusal and planted unparsable is refused by
name, and each planted valid is accepted. A grammar the generator cannot write is refused by name,
and six members' refusals are asserted form for form and line for line.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every fenced `mermaid` block under `docs/` parses | `docs-mermaid.test.ts` `parses every block` |
| A2 | every line under `docs/` that may open a `mermaid` fence in any container is the first line of a block the check reads, no document under `docs/` holds a form the reader refuses, and at least 100 blocks are read | `docs-mermaid.test.ts` `reads every fenced block` |
| A3 | a block whose node id is a reserved word is refused, and the same block with another id is accepted | `docs-mermaid.test.ts` `reserved word` (two tests) |
| A4 | an indented fence, in a list item or by one to three spaces, is read: an unparsable one is refused by name, and a valid one of each diagram type the documents use is accepted | `docs-mermaid.test.ts` `indented` (two tests) |
| A5 | in each of 8,750 container forms generated at test time from the grammar table and the reader's lists, the check either refuses the form by name or reads exactly the fences GitHub renders as diagrams, with GitHub's text, and none GitHub shows as code, and it reads 2,711 of them; each rendered form it reads planted unparsable is refused by name, and planted valid is accepted; a grammar the generator cannot write is refused by name, and six refused forms are named with their lines | `docs-mermaid.test.ts` `generated container form` (three tests) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "parses every block"
A2: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reads every fenced block"
A3: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reserved word"
A4: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "indented"
A5: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "generated container form"
```

Each fence line selects its tests: A1 one test, A2 one, A3 two, A4 two and A5 three, and no test is
selected by two lines.

The command-line renderer is the oracle for R6: it is run once, by hand, over the extracted blocks
before and after the fix, and its table of block to exit code is the delivery's evidence. It is not
part of the gate.

## 4. File manifest

| file | context | change |
|---|---|---|
| `web/app/src/lib/docs-mermaid.test.ts` | repo | added: A1 to A5 |
| `web/app/src/lib/docs-mermaid.fences.json` | repo | added: A5, GitHub's recorded rendering of the generated members |
| `web/app/src/lib/docs-mermaid-fences.js` | repo | added: A5, the grammar table and the member generator |
| `web/app/src/lib/docs-mermaid-read.js` | repo | added: R4, R9, the reader and its exported lists |
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
  indented code block, one in more than 99 lists, one whose text is blank, and one whose language
  word is not exactly `mermaid` (#383). It refuses each of them by name, as it refuses every form R9
  names, rather than pass over it, so it also refuses some forms GitHub renders, each named in §6
  (#383). Measured on the merged tree: 191 blocks, the same 191 with the same text as the fence-line
  reader it replaced, and no form refused; the documents hold no blockquote line, no four-backtick
  fence and no tilde fence (#383).
- It owes no mutation rows, because its two production files, `web/app/src/lib/docs-mermaid-read.js`
  and `web/app/src/lib/docs-mermaid-fences.js`, are mutated by the web tool and the tests A5, A2 and
  A1 select kill their mutants (#383). The mutation plan reads the same: on this pull request it
  selects the web tool and no row.
- It changes no Rust and no Python, because the defect is in Markdown and the check is a test in the
  Mini App's suite (#383).

## 6. Risks

- **A new Mermaid syntax the pinned parser does not know.** A diagram valid in a later release
  would be refused until the pin moves; the refusal names the block, and moving the pin is one
  reviewed change to `web/app/package.json`.
- **The parser and the renderer disagree on some future block.** R6 holds for the blocks it was
  measured over, 183 before the fix and 191 on the merged tree; the two share one grammar, and the
  oracle run in this delivery is the evidence.
- **Forms GitHub renders that the check refuses (refused by design).** The reader reads only the
  subset R9 declares, so a document cannot hold a diagram in a form it refuses, though GitHub draws
  one in some of them. Over the 86,912 distinct texts of the population below, it refuses 38,789;
  GitHub draws a diagram in 13,640 of them and none in the other 25,149. Counted once each, under
  the first form named in R9's order: a refused character in 1,544 texts (GitHub draws a diagram in
  795), a raw HTML block in 3,347 (869), raw inline HTML in 870 (799), a `<` at the start of a line
  in 206 (188), an info string that holds `mermaid` or a `&` and is not `mermaid` alone in 18,340
  (10,989, such as a word after `mermaid`, or `mermaid` spelled with a character reference), a fence
  in more than 99 lists in 1 (0), a blank block in 3,560 (0), and a line that may open a fence and
  is not read in 10,921 (0). Each refusal names the file, the line and the form, so a writer moves
  the diagram out of the form; the documents hold none.
- **A byte-order mark.** cmark-gfm skips one U+FEFF at the start of a text and commonmark.js does
  not; elsewhere the two read it alike. The reader refuses U+FEFF wherever it stands, at the start
  too, so a document saved with a byte-order mark fails by name on line 1 rather than being read as
  one of the two parsers reads it.
- **Footnotes and GitHub's other extensions.** commonmark.js parses no footnote definition, table,
  strikethrough, extended autolink or task list item. GitHub draws no diagram for a fence inside a
  footnote definition, and a task item's checkbox leaves a fence on its marker line as code; the
  generated members hold a fence in each, and the reader reads each as GitHub does or refuses it.
- **A form outside the population.** The test compares the check with GitHub on the table's members
  only. A form outside the table is read by the same rule: read only in the subset R9 declares, and
  refused by name wherever it could make, hide or change a diagram. The rule was measured on 92,791
  texts, each rendered by GitHub: round 4's population (50,960), round 5's design and review
  populations (20,216 and 4,368), this round's texts on the axes cmark-gfm's source names (8,497)
  and the 8,750 generated members. It read 50,329 as GitHub renders them, refused 42,462 by name,
  and read none otherwise: no diagram missed and no code read. The reader round 5 used missed or
  misread the diagram GitHub draws in 1,697 of those texts, and read 1,372 that GitHub shows as
  code. A form found later is one more alternative in the table and one run of the refresh script.
- **The reader copies one GitHub behaviour through commonmark.js's internals.** cmark-gfm counts a
  fence's indentation in characters where CommonMark counts columns. The reader wraps commonmark.js
  0.31.2's fenced-code start to do the same, and to keep the fence's raw info string, which reads the
  parser's `blockStarts`, `tip`, `offset`, `nextNonspace` and `currentLine`. The pin is exact, so
  these move only with a reviewed change. The 22 generated members whose fence follows a partly
  consumed tab (after a `>`, or in the fence's own indentation) fail if the wrapper stops matching
  GitHub, and a fence whose info string the wrapper stops keeping reads as having none, so each of
  its `mermaid` lines is refused by name.
- **GitHub changes how it reads a container.** The recorded rendering is GitHub's on the day it was
  recorded. After such a change the check keeps the recorded reading until the refresh script is
  run, and the test then names every member whose rendering moved.

## 7. References

Issue #383; ADR-195; the ci-efficiency lane's rules.
