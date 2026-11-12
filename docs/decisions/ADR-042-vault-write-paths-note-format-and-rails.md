---
status: accepted
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

### Decisions the delivery made (SPEC-042 §7)

- **The rails read a note as the pack's probe does, and never less strictly.** The port reads the
  whole text as the note's body and never parses a JSON frontmatter, and case-folds as Python does
  for every letter a rail compares; a fixture per rail row, indexed by `rows.json`, holds the adapter
  (A2) and the probe (A3) to the same verdicts. Chosen against running the probe on every date-tree
  write (a Python process per tick of a box, and the probe judges staged runs, not bare notes) and
  against a port that parses the frontmatter as the probe does (it would be looser than the probe
  wherever the two parsers disagree, and only ever as strict where they agree). The adapter adds one
  rail of its own, NUL and every other control character but the tab, the line feed and the carriage
  return, chosen against leaving them to the pack, which has no such rail.
- **An intended divergence (ADR-012's `diverges`): the roll reads `rolls` as an optional sign and
  ASCII digits within a 128-bit integer.** The predecessor's `int` also read a digit separator,
  digits outside ASCII and a count of any size; the golden marks those cases `class: python-only`
  and A4 holds each to a refusal, as ADR-020 decided for the settings. Chosen against porting
  Python's integer grammar (a Unicode digit table and arbitrary precision for a count the adapter
  writes itself, from 0) and against refusing the signs and leading zeros Python reads (the golden
  proves those, and the owner's hand may write them).
- **A malformed note is reported, and only an integrity failure stops a roll-forward.** A note stays
  with one of four bounded reasons: malformed, a note at the roll's destination, a rail, or not a
  regular file. A destination that does not read back as written is removed, its source kept, and
  the roll-forward stops. Chosen against the predecessor, whose non-numeric `rolls` and taken archive
  name each stopped the whole call and wedged every later night on the same folder.
- **A taken archive name takes the next free number, compared case-insensitively.** Chosen against
  refusing the archive (a regeneration's superseded note, SPEC-048, would have nowhere to go) and
  against overwriting (never).
- **The directory sync is part of the write.** A failed sync after the rename is an error, so a roll
  keeps its source unless its destination is known durable. Chosen against the predecessor's
  degrade-to-no-op, which could report a roll done whose destination a crash could still lose.
- **A symbolic link in the date tree is never followed.** A linked note or day folder is refused or
  reported, and every folder the adapter makes is proved inside the readings folder before the next
  is made. Chosen against following a link that resolves inside the folder (a link is not a shape
  the adapter writes, so one in the tree is someone else's).
- **The gate is the pack's own probe, once per blocking class (ADR-043).** A class that examined
  nothing of the run is passed over, and every other ending fails closed; the executor's own checks
  run before the gate and again after it, before the first write. Chosen against treating VOID as
  red (no daily-note run could pass `synthesis-cites`) and against checking only once (the owner's
  devices write the vault while the gate runs).
- **SHA-256 is written by hand.** No ADR admits a hashing crate; the digest proves bytes unchanged,
  an integrity check rather than a secret, and the standard's examples hold it. Chosen against a new
  crate without an ADR, and against a non-cryptographic hash, whose value a stored hash of a body
  could not rely on across releases.
- **The golden of `_roll_note_text` writes a day in a note as `{day:N}`.** A golden may hold no
  calendar date; the token is lossless and is expanded before the predecessor is called. Chosen
  against hexadecimal text, which would hide the dates from the golden check rather than keep them
  out.

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
