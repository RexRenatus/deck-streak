---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A Vitest test parses every Mermaid block under docs

## Context and Problem Statement

Eight of the 183 Mermaid blocks under `docs/` on `dev` do not parse, so they render as an error box
(#383). No job parses a diagram, so each merged green. Where does a check that parses every block
run, so that a new unparsable block is refused before merge without adding a job or a slower step?

## Decision Drivers

- The check agrees with the command-line renderer on every block: the same blocks pass and fail.
- It costs the least runner time and adds no job (the ci-efficiency lane's rule: add a check,
  weaken none).
- A refusal names the file and the block, and a planted bad block proves the check can refuse.

## Considered Options (the alternatives it was chosen against)

- Chosen, because it agrees with the renderer on all 183 blocks and adds no job: a Vitest test in
  `web/app` that calls `mermaid.parse` under the jsdom the app already uses, with `mermaid` a pinned
  devDependency, run by the `web` job's existing `vitest run`.
- The command-line renderer in the `hygiene` job: rejected, because it needs a browser installed on
  every run and renders each block, which is the slowest of the three routes measured, for the same
  verdicts as the parser.
- A regular expression for reserved-word ids and stray semicolons: rejected, because the eight
  faults are three unrelated shapes and a static rule misses the next one; a reserved word passes a
  static check and fails to render.
- Parsing under plain Node with no DOM: rejected, because the parser sanitises labels through the
  document, and 159 of the 191 blocks, every one of them valid, are refused without one.

## Decision Outcome

Chosen option: "a Vitest test in `web/app` calling `mermaid.parse` under jsdom", because on the
same 183 blocks it refuses exactly the eight blocks the renderer refuses and none after the fix,
and it runs inside a job that already exists.

### Consequences

- Good, because a new unparsable block fails the `web` job, naming its file and block number. Since
  the amendments below, that holds for every block GitHub renders as a diagram in the forms the check
  reads, and a form it does not read as GitHub does fails the job by file, line and form.
- Good, because the check adds no job, no workflow step and no browser download.
- Bad, because `mermaid` joins the Mini App's development dependencies, and its pin moves only by a
  reviewed change.
- Bad, because a diagram valid only in a newer Mermaid is refused until the pin moves.

### Confirmation

SPEC-195's A1 to A5; the check is red on the unfixed tree by the eight blocks' names, and green on
the fixed one. A5's generated tests confirm the amendments below: they were red against the
fence-line reader and against round 5's reader, and are green against the reader of round 6.

## What would make this wrong

- A block the parser accepts and the renderer refuses: R6's table is the evidence for today's
  blocks, and a later disagreement would be a reason to reopen this choice.
- A form in which the reader reads a fence that GitHub shows as code, or GitHub draws a diagram the
  reader neither reads nor refuses: the generated members are the evidence for the forms they hold,
  and such a form would be a reason to reopen the round-6 amendment.

## Amendment, round 5: the reader is a CommonMark parse that opens a fence as GitHub does

The decision above is unchanged: the check is a Vitest test in `web/app` calling `mermaid.parse`
under jsdom. The amendment changes how the test finds the blocks, which the decision left to a
fence-line regular expression. Review found a fence GitHub renders as a diagram that the expression
never read: one on a list item's marker line (`- ```mermaid`), at the top level or in a quote. A
design pass then generated 35,588 members (71,176 texts, each planted unparsable and valid) of the
class "every fence GitHub renders as a diagram, whatever container opens its line", rendered each
through GitHub's Markdown API, and measured each reader on all of them. A MISS is a text GitHub
renders as a diagram that the reader does not read, or reads with other text; a false read is a text
GitHub shows as code that the reader reads.

- The fence-line expression (rounds 1 to 4): rejected, because it read 34,238 texts otherwise than
  GitHub (32,714 not at all) and read 3,316 that GitHub shows as code. It knows blanks and `>`, and
  no list marker, laziness or column arithmetic.
- The expression with a list marker wherever it admits `>` or blanks, the continuation being the
  prefix with each marker blanked (rule (a)): rejected, because it read 22,940 texts otherwise
  (20,816 not at all) and 8,368 that GitHub shows as code. Its MISS 0 held on round 4's three-backtick
  texts only; laziness, tabs and a marker's content column escape it.
- A CommonMark parse with commonmark.js 0.31.2, reading every fenced block whose language word is
  `mermaid` (rule (b)): rejected, because it read 4,420 blank blocks that GitHub shows as code, and
  gave 368 texts other text than GitHub (whitespace only: GitHub keeps one to three more leading
  blanks on each line after a partly consumed tab). markdown-it 15.0.2 read 4,146 texts otherwise and
  4,444 falsely, and mdast-util-from-markdown 2.0.3 (micromark) matched commonmark.js with 4,348 false
  reads and more dependencies.
- Chosen: the same parse, reading a fenced block only when its language word (the info string up to
  its first ASCII blank) is `mermaid` and its text is not blank, and opening a fence as cmark-gfm
  does, counting the fence's indentation in characters rather than columns. It read all 71,176 texts
  exactly as GitHub renders them, text for text: MISS 0 and false reads 0. The docs read the same 191
  blocks, with the same text, as the expression it replaces.

The fence offset is copied by wrapping commonmark.js's fenced-code block start, which reads the
parser's `blockStarts`, `tip`, `offset` and `nextNonspace`. That dependence on internals is accepted
because the pin is exact, and because the generated test fails on the 22 members whose fence follows
a partly consumed tab if the wrapper stops matching GitHub.

The test's members are generated at test time from a grammar table (`web/app/src/lib/
docs-mermaid-fences.js`), and their GitHub rendering is recorded in
`web/app/src/lib/docs-mermaid.fences.json` by a named refresh script. Tests have no network, so
GitHub cannot be the oracle at test time. The parse could not be its own oracle, because the chosen
reader is the parse. The test asserts that the recorded table, members and count equal the
generated ones.

The generator lives under `src/lib` so that the web mutation population covers it. That was chosen
against a Python killer, because the mutation-rows job sets up no node, and against leaving the
generator unexamined, which would make the mutation run examine nothing for it. Its killers are the three tests A5 selects: the mutation run credits them with 997 of the generator's 998 killed mutants and A2's test with the other, and two more time out. The reader, `docs-mermaid-read.js`, is mutated in the same run: A5's three tests are credited with 175 of its 256 killed mutants, A2's test, which reads the documents, with 80, and A1's with 1.

**The dependency.** `commonmark` was not in `pnpm-lock.yaml` at the head; the amendment adds it:

- `commonmark` 0.31.2, BSD-2-Clause, locked as
  `resolution: {integrity: sha512-2fRLTyb9r/2835k5cwcAwOj0DEc44FARnMp5veGsJ+mEAZdi52sNopLu07ZyElQUz058H43whzlERDIaaSw4rg==}`,
  with its dependencies `entities` 3.0.1, `mdurl` 1.0.1 and `minimist` 1.2.8;
- `@types/commonmark` 0.27.10, MIT, locked as
  `resolution: {integrity: sha512-iEZobUnvlM+UX5fXWCmC4eQXwCs01Z8Xa1W0VjiWUF/XsNy4BHtskqJ9MyLZVMHbA0ezhyonCDqz3hMvsCm6Hg==}`.

Both are exact devDependencies of `web/app`. They come from the existing lockfile through the
existing `pnpm install --frozen-lockfile` step. That step runs first in the `web` job, whose
`scripts/check.sh web` runs `pnpm -r test` (its log at this head shows the install, then
`examined 205 mermaid blocks`), and in both Stryker jobs (`mutation-web` and the weekly battery's
`web`) before Stryker starts Vitest. Nothing is fetched at test time, and no step is added. `pnpm audit` over the lockfile reports no advisory. The test
imports the parser statically, so without it the test file fails to load and the `web` job fails:
it is never skipped.

- Good, because the check reads what GitHub renders, in every container form CommonMark defines,
  rather than the forms a hand-written expression lists.
- Bad, because the Mini App gains five development packages, and the reader depends on four
  internals of one of them.

## Amendment, round 6: the reader reads a declared subset, and refuses the rest by name

The decision above is unchanged, and so is the round-5 amendment's parse. Review of round 5 found
that GitHub is not a CommonMark 0.31 parser. It renders Markdown with cmark-gfm and draws a diagram
for each `<pre lang="mermaid">` in the HTML it makes, and cmark-gfm reads some lines otherwise than
commonmark.js: a byte-order mark before a fence, a character reference or a Unicode blank around
`mermaid`, a vertical tab or form feed in the info string, and the HTML block tag lists. The class is
restated against GitHub: "Every `mermaid` fence GitHub renders as a diagram is read, and none that
GitHub shows as code." A design pass took the differences from cmark-gfm's source and from GitHub's
HTML parse of its output, generated members on each, rendered every one through GitHub's Markdown
API, and measured each rule on 92,791 texts: round 4's population (50,960), round 5's design
and review populations (20,216 and 4,368), 8,497 on the new axes, and the 8,750
generated members. A MISS is a text in which GitHub draws a diagram that the reader does not read,
or reads with other text; a CODE-READ is a text GitHub shows as code that the reader reads.

| rule | MISS | CODE-READ | refused by name | new dependency |
|---|---|---|---|---|
| round 5's reader | 1,697 | 1,372 | 0 | none |
| round 5's reader with review's three corrections (a byte-order mark stripped, the language word taken as cmark-gfm takes it, cmark-gfm's type 6 tag list) | 884 | 246 | 0 | none |
| (a) the parse corrected wherever cmark-gfm's source reads a line otherwise | 5 | 79 | 0 | none |
| (b) cmark-gfm itself, compiled to WebAssembly (`gfm-wasm` 1.0.1), with GitHub's blank-text rule | 5 | 78 | 0 | a WebAssembly package |
| (c) (a), written out as cmark-gfm writes HTML and parsed as HTML5 as GitHub parses the page (`parse5` 8.0.1) | 10 | 1 | 0 | `parse5` |
| (a), with the raw HTML just before each fence parsed as HTML5 from each state earlier raw HTML can leave open | 5 | 7 | 0 | `parse5` |
| chosen: default-deny | 0 | 0 | 42,462 | none |

Each rule that reads a text as it judges GitHub does reads some otherwise, because GitHub's reading
of a fence depends on its HTML parse of the whole page: raw HTML anywhere before a fence can take
its `<pre>` into an element or an attribute value, and each new form of that is a new escape. A
reader widened by more forms does not close the class. The chosen reader was built to close it by
construction. It reads a declared subset, on which commonmark.js and cmark-gfm open the same fences
with the same text: the subset leaves out each difference cmark-gfm's source shows (a byte-order
mark, the vertical tab and form feed, the line and paragraph separators, raw HTML, a character
reference in the info string) and corrects the one it keeps (a fence's indentation). Every line on
which cmark-gfm could open a `mermaid` fence is either the first line of a block the reader reads or
a line it refuses, and the raw HTML it found could write, take in or drop a `<pre>` is refused.
Review of this round found two forms it read that GitHub shows as code, a fence in 100 or more block
quotes and list items and a raw HTML line hidden from a CommonMark reading, and round 7 below closes
both. It read 50,329 texts as GitHub renders them, refused 42,462 by name, and read none otherwise.
Of the 38,789 distinct texts it refuses, GitHub draws a diagram in 13,640, and SPEC-195 §6 names
them by form, as refused by design. The documents hold none of those forms: they read 191 blocks,
the same 191 with the same text, and nothing is refused. It adds no dependency.

Each part is needed. With one dropped at a time, over the 86,912 distinct texts:

| part dropped | MISS | CODE-READ |
|---|---|---|
| the refused characters | 278 | 0 |
| every raw HTML block | 5 | 52 |
| a `<` at the start of a line that may open raw HTML | 0 | 18 |
| raw inline HTML with a tag name in the reader's list | 0 | 4 |
| raw inline HTML that opens its line | 0 | 35 |
| every refusal of raw inline HTML | 0 | 71 |
| a fence read by its info string's first word, not refused unless `mermaid` alone | 0 | 418 |
| a `&` in the info string | 214 | 0 |
| a fence in more than 99 lists | 0 | 1 |
| a blank block | 0 | 3,560 |
| the correction: the fence's indentation counted in characters | 379 | 0 |

Three parts read nothing otherwise when dropped alone: the refusal of an unread line that may open a
`mermaid` fence, the refusal of unnamed raw inline HTML (a comment, a processing instruction, a
declaration, a CDATA section), and `mermaid` matched in any case. Each changes how many generated
members are read without a refusal, and the test pins that count (2,711), so each is held by
the test. The first is the construction's own guarantee: no difference measured today needs it, and
it names a fence commonmark.js does not open where cmark-gfm does, which is what any later
difference would be.

The reader moves out of the test into `web/app/src/lib/docs-mermaid-read.js`, under `src/lib`, so
the web mutation population covers it, as it covers the generator. The generator draws members from
the reader's exported lists (the refused characters, the refused tag names and the most lists a
fence may stand in), so an entry added to one is a member with no test edit. The wrapper of
commonmark.js's fenced-code start also keeps a fence's raw info string, so the reader depends on
five internals: `blockStarts`, `tip`, `offset`, `nextNonspace` and `currentLine`.

The reader computes nothing when it loads that can throw: it builds its character pattern each time
it reads a text, and the test reads the documents inside the two tests that use them, not when the
file loads. A module that throws while it loads fails the test file before any test runs, and the
mutation run counts that as no test failing, so a mutant that broke the reader's tables survived it.
Measured on the reader as first designed, the mutation run left 23 mutants surviving and 4
unexamined. Twenty were load failures. The other seven were a check or a default that no text
reaches: the test that a lone `<` is a text node, which only a code span holding `<` also meets, and
six empty-string defaults for strings commonmark.js always sets. With the load moved into the tests,
the defaults replaced by the list of fences the wrapper fills (each read at the line its own
position gives), and the text-node test dropped (a code span holding `<` alone that opens its line
before a letter is refused too; no member and no document holds one), none survives. This was chosen
against recording the twenty as equivalent mutants, which they are not: each changes what the reader
does, and the test fails on it once it fails inside a test.

- Good, because each form the reader lists fails by name, with its line, rather than being read.
- Bad, because a document could not hold a diagram in a form this reader refused, though GitHub drew
  one in 13,640 of the 38,789 distinct texts it refused, counted under the first form: a refused
  character 795 of 1,544, a raw HTML block 869 of 3,347, raw inline HTML 799 of 870, a `<` at the
  start of a line 188 of 206, an info string that holds `mermaid` or a `&` and is not `mermaid`
  alone 10,989 of 18,340, a fence in more than 99 lists 0 of 1, a blank block 0 of 3,560, and a line
  that may open a fence and is not read 0 of 10,921; the refusal names the line, and the writer
  moves the diagram out of the form.

## Amendment, round 7: each refusal is derived from a limit or a start condition of cmark-gfm

The decision above is unchanged, and so are the round-5 parse and the round-6 subset. Review of
round 6 rendered 7,880 new texts and found 1,993 the reader read that GitHub shows as code, in two
forms: a list item opened as the 100th or later block on its line, where cmark-gfm's depth limit
counts block quotes as well as lists, and a line that opens raw HTML after a line that begins an
inline construct (a code span, an attribute value, a link title or a lazy line), which a CommonMark
reading takes as text inside the construct and cmark-gfm's block parse reads as the start of an HTML
block. The class is unchanged. This round takes its refusals from cmark-gfm's own limits and start
conditions rather than from the forms found, read on 2026-09-30 from cmark-gfm's `blocks.c`
(`MAX_LIST_DEPTH` 100, counted over every block opened on a line and tested only for list items and
footnote definitions), `inlines.c` (`MAXBACKTICKS` 80, the 32 nested parentheses of a link
destination, and `MAX_LINK_LABEL_LENGTH` 1,000 bytes), its extended autolink, which takes text up to
an ASCII blank or `<` while it parses inlines, its table extension, which splits a row at each `|`
before it parses inlines and opens a table after the paragraph lines above it, and GitHub's page
nesting, measured: a paragraph of 253 nested tags draws the diagram after it and one of 254 does
not, and no diagram is drawn at or after that point.

| rule | escapes, earlier populations (100,671 texts) | escapes, this round (2,379) | refused and drawn, distinct design texts | documents refused |
|---|---|---|---|---|
| round 6's reader | 1,993 | 249 | 13,640 | 0 |
| (a) review's probe: block quotes and list items counted against 99, and every line that opens with `<` and a letter, `/`, `!` or `?` refused | 0 | 110 | 13,643 | 10 lines |
| (b) (a) with a tag name folded to lower case, a list marker before a tab, and a lone carriage return as a line end | 0 | 110 | 13,643 | 10 lines |
| (c) chosen: (a)'s depth, a page bound, the seven HTML block start conditions, and a tag trusted only in a code span the reader's checks find cmark-gfm forms alike | 0 | 0 | 13,439 | 0 |

(b) reads every text as (a) does: the round-6 reader already folds a tag name, takes a list marker
before a tab and splits on a lone carriage return, so the three are gaps in the test, not in the
reader, and this round's test holds each. (a) refuses ten lines of the documents at this head and at
dev: a code span that opens its line with a placeholder tag name, and an autolink that opens its
line. It also reads 110 of this round's texts that GitHub shows as code: a special tag hidden in a
code span of 81 backticks, in a link destination of 33 parentheses or a long label, after an
extended autolink, or in a table cell, and a paragraph nested past GitHub's bound. The chosen rule
refuses a line only where cmark-gfm could open raw HTML on it, and a tag inside a line only where a
code span may hide it from a CommonMark reading. A first form of the chosen rule read 55 of this
round's texts otherwise, each where it approximated cmark-gfm on decoded text (a character reference
or a no-break space in an autolink's tail, an escaped backslash before a parenthesis, an escaped
bracket in a long label, a code span across a table's first row); the chosen rule reads those on the
source text. The test's generator draws 244 more members: 193 from the reader's exported bounds and
lists (`CONTAINER_DEPTH`, `PAGE_DEPTH`, `LINE_END`, `FENCE_PREFIX`, `CODE_TICKS`, `LINK_PARENS`,
`LABEL_UNITS` and the tag list in upper case), each at its bound and past it, so a bound that moves
moves a member, with each term of the page bound on its own and a list whose items pass it only
together, the escapes, stray parentheses and parentheses closed and opened again that the reader's
counts skip or keep, each character one of the reader's patterns decides on at the edge it decides,
a special tag on the last line of a fence its text or its block quote closes, in a heading's code
span and on a line of no paragraph or heading, and 51 that put a line that may open raw HTML inside
each of 17 inline constructs that carry text across a line end (a code span, a link destination or
title, an attribute value, a reference definition, emphasis, an image's text and a plain paragraph),
as round 6's review rendered them. The test pins which members the reader reads, by a digest, beside
how many: a reader that stopped skipping escapes before counting parentheses read one member more
and one fewer, and the count alone held it.

- Good, because each refusal names a limit or a start condition that cmark-gfm's source states, or
  GitHub's page nesting as measured, and the test meets each bound the reader exports at its edge,
  rather than listing the forms found.
- Bad, because a document cannot hold a diagram in a form the reader refuses, though GitHub draws
  one in 13,439 of the 38,588 distinct texts it refuses, counted under the first form: a refused
  character 795 of 1,544, a block in more than 99 block quotes and list items 0 of 1, a block GitHub
  may nest more than 240 elements deep 0 of 0, a line that may open raw HTML 1,345 of 3,908, raw
  HTML a CommonMark reading may hide 256 of 260, an info string that holds `mermaid` or a `&` and is
  not `mermaid` alone 10,989 of 18,340, a blank block 0 of 3,560, and a line that may open a fence
  and is not read 54 of 10,975 (SPEC-195 §6); the refusal names the line, and the writer moves the
  diagram out of the form.

## More Information

SPEC-195, issue #383.
