---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The vault adapter: two write paths, the reading note in the predecessor's format, and the vault-duties rails

## Context and Problem Statement

The vault adapter writes two kinds of file. Agent duties (the daily note, the synthesis, the inbox
curator) write notes the vault-duties pack judges as staged runs. The readings' archive copy is
written by DeckStreak's own code, and it must keep the predecessor's byte-preserving roll, which
patches `rolls`, `last_rolled` and `first_generated` and which the parity oracle proves. The two
packs that could judge a reading note each refuse that format: a staged run requires every note to
carry an agent schema (vault-duties `note-properties`), and a note that carries the persona schema
may hold no calendar date anywhere (persona-core `no-dates`). The predecessor's own readings
validator refuses every `<` followed by a letter, which would refuse the furigana ruby the
Japanese mentor's readings require. How should the adapter write, and which rails hold?

## Decision Drivers

- Vault writes are atomic and pass the rails that block executable content (the owner's rule).
- The roll is proved against the predecessor's function, not re-derived (constraint 8, ADR-012).
- The owner's tick is never destroyed, and the owner's own edits are never overwritten.
- The readings folder has one writer (ADR-011), and the service may not create a top-level folder.

## Considered Options (the alternatives it was chosen against)

- Two write paths: staged runs through a three-verb executor behind the vault-duties classes for agent-written notes, and the readings date tree's own operations (create, roll, archive, stamp, tick, body replacement) for the adapter-written reading note in the predecessor's format, with the vault-duties `no-executable` rails ported from `rails.json` and proved against the pack — chosen: each file is judged by rules that fit it, and the roll golden holds.
- Write the reading note as the persona output itself and stage it as a run — rejected because a persona-claimed note may hold no date, so the roll's `last_rolled` and `first_generated` could not exist and the golden could not hold.
- Keep the predecessor's readings validator as the rails — rejected because it refuses `<ruby>` and `<rt>`, so no Japanese reading with furigana could reach the vault.
- Run the vault-duties probe on every date-tree write — rejected because its classes judge staged runs whose notes carry an agent schema, which the reading note by design does not; the ported rails read the same `rails.json`, and a test holds them to the probe.
- Create the readings folder when it is missing, as the predecessor did — rejected because the vault root does not allow it (the predecessor's first real night would have failed) and top-level folders are the operator's.

## Decision Outcome

Chosen option: the two write paths. The reading note keeps the predecessor's eight keys, with
calendar dates in `date`, `first_generated` and `last_rolled` (a private file no pack claims, and no
date there is forward-looking), plus `ai_generated: true`. Its writes pass the ported rails, stay
inside the configured readings folder, never overwrite, and use temp names the bridge ignores. The
owner's tick is written only by the read-tap use case; a regeneration replaces the body only when
the note's body is still the one the adapter wrote. A missing readings folder fails loud.

### Consequences

- Good, because the predecessor's roll and archive goldens prove the date tree.
- Good, because Japanese readings with furigana can be archived.
- Bad, because the rails exist twice (the pack's probe and the adapter's port of its data); A3 of
  SPEC-042 keeps them equal.
- Bad, because the reading note is judged by no persona or vault-duties row after it is written; the
  text it holds was judged by the agent's output gate before it was written.

### Confirmation

SPEC-042's acceptance tests, the goldens of `reading_notes.py:_roll_note_text` and
`reading_notes.py:_archive_dir`, and the vault-duties rails rows over the committed synthetic run.

## What would make this wrong

- The vault-duties pack learns a reading-note schema whose properties may hold the date tree's days
  (then the reading note becomes a staged run like every other).
- A new rail kind lands in `rails.json` that is not data-driven (then the port needs code, and A3
  shows the gap).

## More Information

SPEC-042; the second-brain file contract's C1, C6, C7 and C8 (private input, summarised in SPEC-042);
the vault-duties, persona-core and language-mentors packs; docs/schematics/vault-write-paths.md.
