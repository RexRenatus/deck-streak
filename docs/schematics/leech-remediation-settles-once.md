# Schematic: a leech's remediation settles once, and an undo settles it to zero

Kind: state machine and sequence. Read at DeckStreak `dev` dd98601 (`crates/curriculum/src/`
holds only `lib.rs`) and at the predecessor's `27ee2bc` (`leeches.py:cards_to_leech_rows`,
`build_board`; `pipeline.py:GamifyPipeline._persist_leeches`, `remediate_leech`,
`unremediate_leech`; `database.py:GamifyStore.grant_leech_xp_once`, `delete_xp_source`,
`day_base_xp`). Decided by ADR-072 (derived XP is settled, and the owner's correction replaces an
amount) and ADR-093 (a remediation settles `leech:<card id>` once, and an undo corrects it to zero).
Added by SPEC-093.

It sits beside `docs/schematics/leeches-state-machine.md` and does not change it. That schematic's
undo edge says the XP grant is removed; in DeckStreak the 40 XP is a settlement of the derived
source `leech:<card id>` (`docs/schematics/xp-grants-and-settlement.md`), and an undo is the owner's
correction of that settlement to 0 on its own study day. Nothing is deleted from the XP ledger.

## One card's status and its pay

```mermaid
stateDiagram-v2
  [*] --> Active: the leech step sees lapses at or above the threshold
  Active --> Holding: remediated, and no positive leech settlement exists, so 40 settles today
  Holding --> Regressed: a later snapshot holds lapses above the lapses recorded
  Regressed --> Holding: remediated again, and a positive settlement exists, so nothing settles
  Holding --> Active: undone, every leech settlement of the card settles to 0 on its own day
  Regressed --> Active: undone, every leech settlement of the card settles to 0 on its own day
  Active --> [*]: lapses fall below the threshold, the row leaves the snapshot
```

The status is derived at each snapshot: `active` without a remediation record, `regressed` when the
lapses exceed those recorded, else `holding`. A remediation record outlives its card's leaving the
snapshot. After an undo, a remediation finds no positive settlement and pays 40 again, on the study
day it is made.

## The owner remediates a card

```mermaid
sequenceDiagram
  participant o as owner
  participant s as the route or the bot
  participant c as coordination
  participant cu as curriculum
  participant p as progression
  o->>s: remediate card, with an action and a note
  s->>c: the remediate use case, owner session only
  c->>cu: is the card in the leech snapshot
  cu-->>c: yes, with its lapses
  c->>cu: record the action, the note, the study day and the lapses
  c->>p: is a positive leech settlement of the card on any day
  p-->>c: none
  c->>p: settle 40 on the current study day, language track, owner correction
  Note over c,p: one transaction, the level is read again from the totals
  c-->>s: the card holds, with 40 XP
```

A card not in the snapshot is refused and nothing is written. The day base leaves every `leech:`
source out (SPEC-072 R17), so a remediation never feeds the consistency bonus or the coin mint.

## The owner undoes it

```mermaid
sequenceDiagram
  participant o as owner
  participant c as coordination
  participant cu as curriculum
  participant p as progression
  o->>c: undo card, owner session only
  c->>cu: delete the remediation record
  c->>p: settle every leech settlement of the card to 0 on its own study day, owner correction
  Note over c,p: one transaction, a closed day included
  c-->>o: the card is active again
```
