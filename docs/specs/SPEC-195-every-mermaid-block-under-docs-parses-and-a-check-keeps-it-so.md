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

R3. **`mermaid` is a pinned devDependency of `web/app`,** an exact version, so the parser that
judges the blocks does not move with a caret range.

R4. **The check reads every block.** It finds the blocks by the fence line, counts them against the
number of fence-opening lines, and refuses a count under 100, so a change to the reader that finds
none fails.

R5. **A refusal names the block:** the file's path under `docs/` and the block's number counted from
1, one entry per unparsable block.

R6. **The check agrees with the renderer.** On the blocks the renderer was run over, the check refuses
exactly the blocks the renderer exits non-zero for: eight of 183 before the fix, and none of the 190
the merged tree holds after it.

R7. **The eight blocks are fixed by ids and syntax only.** Every label renders the same text: a
`;` becomes the entity `#59;` (rendered as `;`), and an edge label holding `(` or `@` is quoted.

R8. **A planted block whose node id is a reserved word is refused, and the same block with another
id is accepted,** in the check's own test.

R9. **A fence at any indentation is read.** A three-backtick `mermaid` fence inside a list item, or
indented one to three spaces at top level, is a block, and it is closed by a fence at the same
indentation. GitHub renders both as diagrams, so the check reads both. The check's own test plants
an unparsable block in each and refuses it by name, and plants a valid block of each diagram type
the documents use (`sequenceDiagram`, `flowchart`, `stateDiagram-v2`) and accepts it.

The same three-backtick `mermaid` fence is also read inside a blockquote, with a space after the `>`
or none, and as a list item inside a blockquote: the `>` markers are part of the prefix, stripped like
indentation, the closing fence carries the same prefix, and a quoted blank line (the prefix without
its trailing blanks) becomes an empty line. So is a fence with blanks between the backticks and
`mermaid`. GitHub renders each of these as a diagram. The check's own test plants an unparsable
block in each and refuses it by name, and accepts a valid quoted block that holds a quoted blank line.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every fenced `mermaid` block under `docs/` parses | `docs-mermaid.test.ts` `parses every block` |
| A2 | the check reads every fenced block, and at least 100 of them | `docs-mermaid.test.ts` `reads every fenced block` |
| A3 | a block whose node id is a reserved word is refused, and the same block with another id is accepted | `docs-mermaid.test.ts` `reserved word` (two tests) |
| A4 | an indented fence, in a list item or by one to three spaces, is read: an unparsable one is refused by name, and a valid one of each diagram type the documents use is accepted | `docs-mermaid.test.ts` `indented` (two tests) |
| A5 | a fence in a blockquote, and one spaced before `mermaid`, is read: an unparsable one is refused by name, and a valid quoted one with a quoted blank line is accepted | `docs-mermaid.test.ts` `quoted` (two tests) |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "parses every block"
A2: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reads every fenced block"
A3: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "reserved word"
A4: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "indented"
A5: pnpm exec vitest run web/app/src/lib/docs-mermaid.test.ts -t "quoted"
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
| `web/app/package.json` | repo | changed: R3, the `mermaid` devDependency |
| `pnpm-lock.yaml` | repo | changed: the lock of that dependency |
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
- It does not read a `~~~` fence or a four-backtick one, because the repository writes every diagram
  as a three-backtick `mermaid` fence (#383). What it does read is that form at any indentation,
  inside a blockquote, and with blanks before `mermaid`: the opener's prefix (leading spaces or tabs
  and any `>` markers) is captured, the closing fence must carry the same, and the prefix is
  stripped from each body line before the parse.
- It reads a three-backtick `mermaid` fence that GitHub shows as code too. A `mermaid` fence shown
  inside a four-backtick example, or inside an indented code block, is read. That may refuse an
  example; a fence in a form R9 does not name is outside R1, by the bullet above (#383). Measured
  on the merged tree: 190 blocks, and the documents hold no blockquote line, no four-backtick
  fence, no tilde fence and no `mermaid` fence after an indented code line (#383).
- It owes no mutation rows, because no production file changes and the check is test code (#383).
  The mutation plan reads the same: on this pull request it selects no tool and no row.
- It changes no Rust and no Python, because the defect is in Markdown and the check is a test in the
  Mini App's suite (#383).

## 6. Risks

- **A new Mermaid syntax the pinned parser does not know.** A diagram valid in a later release
  would be refused until the pin moves; the refusal names the block, and moving the pin is one
  reviewed change to `web/app/package.json`.
- **The parser and the renderer disagree on some future block.** R6 holds for the blocks it was
  measured over, 183 before the fix and 190 on the merged tree; the two share one grammar, and the
  oracle run in this delivery is the evidence.
- **A block that only the fence-line reader misses** (an unusual fence) is not examined. A2 counts
  opener lines with the reader's own spelling, so it detects an unclosed fence (the reader finds one
  block fewer than the openers) but not a spelling the reader misses. A4 and A5 cover indentation,
  blockquotes and blanks before `mermaid`; a fence in another form, a tilde or four-backtick one,
  stays outside R1 by section 5.
- **False refusals, on the fail-closed side.** The check goes red on a valid diagram, rather than
  passing an unread one, in these shapes: a closing fence at a different indentation or quote prefix
  than its opener (A2's count of openers then differs from the blocks read, without naming a file,
  and the opener may run on to the next fence and refuse a block by name), an opener such as
  `mermaidx`, a top-level block indented four spaces or by a tab, and a `mermaid` fence shown inside
  a four-backtick example (the last three are refused by name). The merged tree holds none of them:
  the check reads 190 blocks and refuses none.

## 7. References

Issue #383; ADR-195; the ci-efficiency lane's rules.
