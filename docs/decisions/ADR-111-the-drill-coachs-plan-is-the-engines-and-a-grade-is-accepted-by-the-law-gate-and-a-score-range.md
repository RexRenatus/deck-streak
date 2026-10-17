---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The drill coach's plan is the engine's, and a grade is accepted by the law gate and a score range

## Context and Problem Statement

#46 asks for a drill coach that mints law drills on a cadence and grades the owner's answers, so
that SPEC-110's post-back can pay them. The predecessor's product only read drills back: its poll
clamps a model-written `xp` to 10..25 and defaults it to 15 (`vault_bridge.py:_parse_graded_drill`),
and the plan (which day, which subject, whether to mint while drills wait) had no product function
at `27ee2bc`. Three things are open: who decides the plan, what a model's grade must pass before it
counts, and who writes the XP the post-back pays.

## Decision Drivers

- A model's grade or score is advisory until a deterministic rule accepts it (the W6 plan's content
  rule), and the engine, never the model, writes an output's frontmatter (SPEC-044 R6).
- No-AI is the default (ADR-054): with the route absent, drills already in the vault stay
  answerable and nothing is minted or graded.
- The vault-duties pack forbids a calendar date in a vault-duty note, and a backlog of unanswered
  drills is the failure the owner sees.
- A mutation row's killer runs against a scripted runner, never a model (SPEC-039).

## Considered Options (the alternatives it was chosen against)

- The engine plans and maps accepted integer scores to XP: chosen, because every number that
  reaches the ledger is computed by a rule a test can kill, and the model writes only the drill
  and the integer scores, text the gate checks.
- The model writes the grade's `xp` and the post-back clamps it: rejected because a clamp bounds
  a wrong number without refusing it, so a hallucinated 25 pays in full; this was the
  predecessor's product.
- The model plans the run (which day, which subject, whether to mint): rejected because the plan
  is configuration and state, a model's choice is not reproducible, and a scripted runner could not
  kill a mutant of it.
- Mint on every cadence day regardless of waiting drills: rejected because unanswered drills pile
  up while the owner is away, and the archive then moves work nobody saw.
- A drill's `created` date in the note, as imported notes carry: rejected because the vault-duties
  pack's `no-dates` class refuses a date in a note a duty writes, so the mint's study day lives in
  a table.

## Decision Outcome

Chosen option: "the engine plans, the model writes the drill and integer scores, and the engine
maps accepted scores to XP", because it keeps the ledger's input deterministic and the model's text
gated.

- **The plan.** A cadence of seven entries (configuration, unset by default, so nothing is minted);
  a mint only when no drill waits unanswered; one mint per study day; the subject rotates through
  the roster's law topics by the oldest mint.
- **The acceptance.** A grade counts only when its law gate passes (the classes SPEC-111 R8 and R12
  name, `grading-cites` included), every score is an integer from 0 to 10, and at least one
  criterion is scored. Anything else is withheld, and the drill waits for the next run.
- **The mapping.** `xp = 10 + round_half_up(15 × mean / 10)`, so the range is exactly the
  predecessor's pay band, 10 to 25. The post-back still clamps and defaults as the predecessor's
  golden proves, so an imported note keeps its pay.
- **The date.** The mint's study day lives in `drill_mints`; a minted note holds none.

### Consequences

- Good, because a grade the model did not justify with the corpus never pays, and the pay band is
  unchanged for every note.
- Good, because the whole duty's rows are killed by a scripted runner.
- Bad, because a mean that rounds the other way from what a human grader would pick pays one XP
  less or more. The band is the predecessor's, and the mapping is one line to re-price in W7's XP
  work (#281).

### Confirmation

SPEC-111's A3, A4, A10, A11 and A12, and its rows S11102, S11103, S11108 to S11110 and S11113.

## What would make this wrong

- An owner who wants the model's own judgement of a drill's worth to pay: the mapping would become
  a setting, which is W7's (#57).
- A law gate too strict to pass any real grade: every drill would wait. The withheld verdicts in
  `agent_runs` show it, and the gate's classes are the law-professors pack's to tune.

## More Information

SPEC-111, SPEC-110, ADR-110 (the pay), ADR-043 (the gate and caps), ADR-054 (no-AI), SPEC-044 R6
(the engine writes the frontmatter), and the W6 schematic
`docs/schematics/law-drill-answer-grade-and-pay.md`.
