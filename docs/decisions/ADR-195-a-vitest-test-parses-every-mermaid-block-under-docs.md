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
  document, and 158 of the 190 blocks, every one of them valid, are refused without one.

## Decision Outcome

Chosen option: "a Vitest test in `web/app` calling `mermaid.parse` under jsdom", because on the
same 183 blocks it refuses exactly the eight blocks the renderer refuses and none after the fix,
and it runs inside a job that already exists.

### Consequences

- Good, because a new unparsable block fails the `web` job, naming its file and block number. Since
  the amendment below, that holds for every block GitHub renders as a diagram, in any container.
- Good, because the check adds no job, no workflow step and no browser download.
- Bad, because `mermaid` joins the Mini App's development dependencies, and its pin moves only by a
  reviewed change.
- Bad, because a diagram valid only in a newer Mermaid is refused until the pin moves.

### Confirmation

SPEC-195's A1 to A5; the check is red on the unfixed tree by the eight blocks' names, and green on
the fixed one. A5's generated test confirms the amendment below: it was red against the fence-line
reader and is green against the parse.

## What would make this wrong

- A block the parser accepts and the renderer refuses: R6's table is the evidence for today's
  blocks, and a later disagreement would be a reason to reopen this choice.

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
generator unexamined, which would make the mutation run examine nothing for it. The A5 test is its
killer.

**The dependency.** `commonmark` was not in `pnpm-lock.yaml` at the head; the amendment adds it:

- `commonmark` 0.31.2, BSD-2-Clause, locked as
  `resolution: {integrity: sha512-2fRLTyb9r/2835k5cwcAwOj0DEc44FARnMp5veGsJ+mEAZdi52sNopLu07ZyElQUz058H43whzlERDIaaSw4rg==}`,
  with its dependencies `entities` 3.0.1, `mdurl` 1.0.1 and `minimist` 1.2.8;
- `@types/commonmark` 0.27.10, MIT, locked as
  `resolution: {integrity: sha512-iEZobUnvlM+UX5fXWCmC4eQXwCs01Z8Xa1W0VjiWUF/XsNy4BHtskqJ9MyLZVMHbA0ezhyonCDqz3hMvsCm6Hg==}`.

Both are exact devDependencies of `web/app`. They come from the existing lockfile through the
existing `pnpm install --frozen-lockfile` step. That step runs first in the `web` job, whose
`scripts/check.sh web` runs `pnpm -r test` (its log on this pull request shows the install, then
`examined 191 mermaid blocks`), and in both Stryker jobs (`mutation-web` and the weekly battery's
`web`) before Stryker starts Vitest. Nothing is fetched at test time, and no step is added. `pnpm audit` over the lockfile reports no advisory. The test
imports the parser statically, so without it the test file fails to load and the `web` job fails:
it is never skipped.

- Good, because the check reads what GitHub renders, in every container form CommonMark defines,
  rather than the forms a hand-written expression lists.
- Bad, because the Mini App gains five development packages, and the reader depends on four
  internals of one of them.

## More Information

SPEC-195, issue #383.
