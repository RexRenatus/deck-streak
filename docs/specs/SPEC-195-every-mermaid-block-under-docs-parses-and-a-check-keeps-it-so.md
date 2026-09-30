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

R9. **Every `mermaid` fence GitHub renders as a diagram is read, and none that GitHub shows as code;
a form the reader does not read as GitHub does is refused by name.** GitHub renders Markdown with
cmark-gfm and draws a diagram for each `<pre lang="mermaid">` in the HTML it makes, and only a
fenced code block whose language word is `mermaid`, or raw HTML, writes one. The reader,
`web/app/src/lib/docs-mermaid-read.js`, reads a declared subset of Markdown, on which a CommonMark
0.31.2 parse (`commonmark`) opens the same fences as cmark-gfm, with the same text. A block is a
fenced code block (three or more backticks or tildes) whose info string is `mermaid` alone, with
spaces or tabs around it, and whose text is not blank. The parse finds it, so block quotes, list
items (with a bullet or one to nine digits, the fence on the line after the marker or on the marker
line itself), blanks and lazy lines nest in any order, in up to 99 block quotes and list items
together. The parse opens a fence as GitHub's cmark-gfm does, counting the fence's indentation in
characters rather than columns: when a container prefix consumes part of a tab, the block keeps the
tab's remaining columns on each line, as GitHub's does. The block's text is the text GitHub renders
as the diagram. Outside the subset the reader reads nothing. It refuses by line and name each form
that cmark-gfm's own limits and start conditions show could make, hide or change a diagram, in this
order: a character in its list, wherever it stands (a byte-order mark, each C0 and C1 control but
tab, line feed and carriage return, DEL, the line and paragraph separators, and each noncharacter);
a block in more than 99 block quotes and list items together, because cmark-gfm opens no list item
as the 100th or later block it opens on one line and counts block quotes among them; a block GitHub
may nest more than 240 elements deep, because GitHub draws no diagram at or after the first point
where its HTML nests more than 254 elements (a block quote counts one, a list item two, each
character inside the block that may open an element one, and each tag in the text one); every line
outside a fence's text that may open raw HTML, that is, after its container prefix, `<` and then
`!`, `?`, or a tag name after an optional `/` that is in its list in any case, or any tag name when
the line ends with `>` (the seven HTML block start conditions, the seventh taken wider); a tag name
in its list in any case, a comment, a processing instruction, a declaration or a CDATA section
anywhere else outside a fence's text, unless each one in its paragraph or heading stands in a code
span that cmark-gfm forms as commonmark.js does, which it does not after a run of more than 80
backticks, a link destination nesting more than 32 parentheses, a bracketed label of 250 characters
or more, an extended autolink's text that runs to a backtick, a full reference whose label holds a
backtick, or in a table, around a code span that holds `|` or crosses a line; a fence whose info
string holds `mermaid` in any case, or a `&`, and is not `mermaid` alone; a `mermaid` block whose
text is blank; and every line that may open a `mermaid` fence and is not the first line of a block
it reads. Its tag list holds the HTML standard's special elements, the SVG and MathML roots,
`image`, and every HTML block tag of cmark-gfm and of CommonMark 0.31.2. Its lists and bounds are
exported, and the test's generator reads them.

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
run left open, a tag the tag filter escapes, a state an earlier block leaves open); raw HTML inside
a line, with every tag name of the grammar and of the reader's list; a fence in a footnote
definition, a table and a task list item; each line that holds `mermaid` and opens no fence; a fence
in 99 lists and in 100; and the reader's bounds and lists met at their edges: a fence in 99 block
quotes and list items and in 100, in three orders, and after a paragraph; a paragraph whose tags
nest to the page bound, one past it, and to GitHub's own bound and one past that, and one whose
block quotes, list items, inline openers of each kind or upper-case tags meet the page bound and
pass it; a list whose items hold more openers together than the page bound and fewer each; an
indented code block, whose openers the bound does not count; a raw HTML block after a paragraph,
with each line end the reader splits on; a special tag on the last line of a fence the text's end
closes and of one its block quote closes; a line that may open raw HTML after each list marker and
blank the reader takes; each construct cmark-gfm forms otherwise than commonmark.js, at the reader's
bound and past it, with the escapes, stray parentheses and parentheses closed and opened again that
the reader's counts skip or keep; each character one of the reader's patterns decides on, at the
edge it decides (a digit beside `_`, a `*` list marker, a `<` at a line end, a hard line break in a
table row, each blank of a delimiter row, a tab, a line end or a `<` after an autolink, and a
bracket before a code span); every tag name of the grammar and of the reader's list in upper case; a
special tag in a heading's code span, and on a line of no paragraph or heading; and a line that may
open raw HTML inside each inline construct that carries text across a line end: 8,994 members in
all. `web/app/src/lib/docs-mermaid.fences.json` records GitHub's rendering of each (`gh api
markdown`), and only `web/app/scripts/record-docs-mermaid-fences.js` refreshes it, rendering every
text that holds `<` or opens with a byte-order mark alone. The test asserts that the recorded table,
member set and count equal the generated ones, so the two cannot drift apart. For every member the
reader reads without a refusal, the blocks the check reads are exactly the diagrams GitHub renders,
with GitHub's text, and the test pins how many such members there are (2,934) and, by a digest of
their ids and texts, which they are, so a reader that reads another set of them fails, even a set of
the same size. Each rendered member read without a refusal and planted unparsable is refused by
name, and each planted valid is accepted. A grammar the generator cannot write is refused by name,
and fourteen members' refusals are asserted form for form and line for line.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every fenced `mermaid` block under `docs/` parses | `docs-mermaid.test.ts` `parses every block` |
| A2 | every line under `docs/` that may open a `mermaid` fence in any container is the first line of a block the check reads, no document under `docs/` holds a form the reader refuses, and at least 100 blocks are read | `docs-mermaid.test.ts` `reads every fenced block` |
| A3 | a block whose node id is a reserved word is refused, and the same block with another id is accepted | `docs-mermaid.test.ts` `reserved word` (two tests) |
| A4 | an indented fence, in a list item or by one to three spaces, is read: an unparsable one is refused by name, and a valid one of each diagram type the documents use is accepted | `docs-mermaid.test.ts` `indented` (two tests) |
| A5 | in each of 8,994 container forms generated at test time from the grammar table and the reader's lists, the check either refuses the form by name or reads exactly the fences GitHub renders as diagrams, with GitHub's text, and none GitHub shows as code, and it reads 2,934 of them, the set the test pins by digest; each rendered form it reads planted unparsable is refused by name, and planted valid is accepted; a grammar the generator cannot write is refused by name, and fourteen refused forms are named with their lines | `docs-mermaid.test.ts` `generated container form` (three tests) |

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
  indented code block, one in more than 99 block quotes and list items, one whose text is blank, and
  one whose language word is not exactly `mermaid` (#383). It refuses each of them by name, as it
  refuses every form R9 names, rather than pass over it, so it also refuses some forms GitHub
  renders, each named in §6 (#383). Measured on the merged tree: 193 blocks, the same 193 with the
  same text as the fence-line reader it replaced, and no form refused; the documents hold no
  blockquote line, no four-backtick fence and no tilde fence (#383).
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
  one in some of them. Over the 86,912 distinct texts of round 6's population below, it refuses
  38,588; GitHub draws a diagram in 13,439 of them and none in the other 25,149. Counted once each,
  under the first form named in R9's order: a refused character in 1,544 texts (GitHub draws a
  diagram in 795), a block in more than 99 block quotes and list items in 1 (0), a block GitHub may
  nest more than 240 elements deep in 0 (0), a line that may open raw HTML in 3,908 (1,345, such as
  a line that opens with a tag of any name, `<div>` or `<x-y>`, or with a comment or a declaration),
  raw HTML a CommonMark reading may hide in 260 (256, a tag of R9's list inside a paragraph, such as
  `Text <div> x.`), an info string that holds `mermaid` or a `&` and is not `mermaid` alone in
  18,340 (10,989, such as a word after `mermaid`, or `mermaid` spelled with a character reference),
  a blank block in 3,560 (0), and a line that may open a fence and is not read in 10,975 (54, a
  fence after a line that opens with a tag and a no-break space). Each refusal names the file, the
  line and the form, so a writer moves the diagram out of the form; the documents hold none.
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
  refused by name where it meets a limit or a start condition R9 names; whether it reads as GitHub
  does is measured on the texts below only. The rule was measured on 103,050 texts, each rendered by
  GitHub: round 4's population (50,960), round 5's design and review populations (20,216 and 4,368),
  round 6's texts on the axes cmark-gfm's source names (8,497), round 6's review population (7,880),
  this round's texts on the limits and start conditions R9 names (2,135) and the 8,994 generated
  members. It read 53,479 as GitHub renders them, refused 49,571 by name, and read none otherwise:
  no diagram missed and no code read. The reader round 5 used missed or misread the diagram GitHub
  draws in 1,697 of round 6's first 92,791 texts, and read 1,372 that GitHub shows as code; the
  reader round 6 used read 1,993 of round 6's review population, 210 of this round's texts and 39 of
  the new generated members that GitHub shows as code, each in a form R9 now refuses. A form found
  later is one more alternative in the table and one run of the refresh script.
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
