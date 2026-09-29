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

- Good, because a new unparsable block fails the `web` job, naming its file and block number.
- Good, because the check adds no job, no workflow step and no browser download.
- Bad, because `mermaid` joins the Mini App's development dependencies, and its pin moves only by a
  reviewed change.
- Bad, because a diagram valid only in a newer Mermaid is refused until the pin moves.

### Confirmation

SPEC-195's A1 to A5; the check is red on the unfixed tree by the eight blocks' names, and green on
the fixed one.

## What would make this wrong

- A block the parser accepts and the renderer refuses: R6's table is the evidence for today's
  blocks, and a later disagreement would be a reason to reopen this choice.

## More Information

SPEC-195, issue #383.
