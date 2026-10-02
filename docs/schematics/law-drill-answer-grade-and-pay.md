# Schematic: a law drill, from its mint to its answer, its grade and its pay

Kind: state machine and sequence. Read at DeckStreak `dev` 5216bcf (ADR-042, ADR-054,
`docs/schematics/vault-write-paths.md`, `docs/schematics/xp-grant-port.md`,
`docs/schematics/agent-duty-run.md`), and at the predecessor's `27ee2bc` for the behaviour it ports
(`vault_bridge.py:append_drill_answer`, `_parse_graded_drill`, `_active_drill_rollup`,
`nudges.py:NudgesLayer.submit_drill_answer`). Added by SPEC-110 (ADR-110), with SPEC-111 (the
coach's mint, archive and grade, ADR-111) and SPEC-121 (the workspace and the progress screen). It
adds to the vault's write paths the drill note's life, and to the grant port one source, `drill:`;
it rewrites neither.

## A drill note's states

```mermaid
stateDiagram-v2
  [*] --> Unanswered: the coach mints a note in Active, marker unticked, one drill_mints row
  Unanswered --> Archived: unanswered for the configured days, moved byte for byte, no AI route needed
  Unanswered --> Answered: one answer appended, marker ticked, one drill_answers row, in one write
  Answered --> Answered: a second answer is already_answered and writes nothing
  Answered --> Graded: an accepted grade updates the note and moves it to Graded
  Answered --> Answered: a withheld grade, or no AI route, leaves the note untouched
  Graded --> Paid: the post-back records drill_grades and asks the grant port once
  Paid --> Paid: a re-poll pays nothing more
  Archived --> [*]
  Paid --> [*]
```

Nothing is deleted in any state. The owner may also answer or grade a note in the vault by hand:
the post-back reads the note's frontmatter, not the path that wrote it.

## One answer, from either surface

```mermaid
sequenceDiagram
  participant W as Mini App workspace or the bot
  participant U as coordination drills answer
  participant V as vault context
  participant D as ledger
  W->>U: the drill id and one answer (the workspace joins its sections under their headings)
  U->>V: read the note through the safe stem
  alt no note in Active
    V-->>U: not_active
  else the answer is empty or all spaces
    U-->>W: empty_answer, nothing written
  else the marker is ticked or the row exists
    U-->>W: already_answered, nothing written
  else
    U->>D: one write: insert the drill_answers row
    U->>V: in the same write: tick the marker once, append the answer, atomic writer and rails
    V-->>U: appended, or rail_refused and the row rolls back
  end
  U-->>W: the outcome by name, and the workspace keeps every field unless appended
```

## The grade and the pay

```mermaid
flowchart TD
  job["the coach job, a daily run"] --> route{"AI route configured"}
  route -->|absent| skip["archive only, each skipped task recorded ai_route_absent, no alert"]
  route -->|configured| grade["drill-grade task through the SPEC-043 runner and its caps"]
  grade --> gate{"law gate green, every score an integer 0 to 10, one criterion scored"}
  gate -->|no| held["withheld: the note is untouched and waits"]
  gate -->|yes| xp["XP by the engine: 10 plus half-up of 15 times the mean over 10"]
  xp --> move["staged run: status graded, xp, the rule and the grade appended, moved to Graded"]
  move --> poll["the post-back job scans Graded each hour"]
  poll --> parse{"graded parse accepts the note"}
  parse -->|no| none["nothing is paid"]
  parse -->|yes| grades["drill_grades row, idempotent"]
  grades --> port["grant port: source drill and the id, track law, scope once, amount clamped 10 to 25"]
  port --> ledger["one xp_ledger row, ever"]
```

A model's score is advisory: only the engine's integer check and its formula set the XP, and only
the grant port writes it (SPEC-040). The progress screen (SPEC-121) counts the paid rows by their
`drill:` source and the backlog from the rollup; it reads the ledger, never a vault dashboard.
