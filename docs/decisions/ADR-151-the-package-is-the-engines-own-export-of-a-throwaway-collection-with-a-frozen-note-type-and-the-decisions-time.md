---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The package is the engine's own export of a throwaway collection, with a frozen note type and the decision's time

## Context and Problem Statement

The owner's approved vault cards reach Anki as a file the owner imports (#65, SPEC-151). DeckStreak
already carries Anki's own engine, pinned to a fork (ADR-009, ADR-058), behind ingest's engine port.
CHARTER 4 makes the skip day the only write back to Anki, and the W9 plan's binding 4 says DeckStreak
never writes an Anki collection directly and never talks to the predecessor's sync. The owner's
import updates a note whose GUID it holds only when the package's note is newer, under its default
condition, and keeps the package's own modification time. What builds the file, in what format, and
with which times, so that an import adds each new card once, updates an approved edit in place, and
never overwrites an edit the owner made in Anki?

## Decision Drivers

- The owner's collection and its copy are never written; the import is the owner's act.
- A re-import neither doubles a card nor overwrites the owner's own edit in Anki.
- The note type stays one note type across every package and every DeckStreak release.
- No new dependency and no change to the engine's pinned fork for this feature.
- Fields are escaped for Anki's HTML, and nothing is cut.

## Considered Options (the alternatives it was chosen against)

- The engine's `export_apkg` over a throwaway collection built in memory, with a frozen note type and each note's time set to the owner's decision: chosen, because it is Anki's own writer of Anki's own format, already in the workspace.
  The collection lives in memory and is dropped after the export; the note type has a fixed id,
  time, field set, template and style; each note carries its stored GUID (ADR-150) and the second of
  its decision; the package is the modern format, without scheduling, deck options or media.
- A package written by hand, as a zip of a SQLite file: rejected because it reimplements Anki's
  schema and its format's versions, which the engine already writes and the importer reads.
- A package library such as genanki: rejected because it is a new dependency, in another language or
  a port of one, that tracks Anki's format from outside while the engine is already here.
- A text file for Anki's text import, with a GUID column: rejected because it carries no note time, so a re-import either overwrites every owner edit in Anki or keeps every note unchanged.
  It also needs the note type to exist in the owner's collection before the first import, which the
  owner would make by hand; the package carries its own note type.
- A write through an Anki add-on's local API, or into the collection's copy before a sync: rejected
  because it writes the owner's collection, which CHARTER 4 and binding 4 forbid, and the second
  would upload through the sync.
- The legacy package format: rejected because it exists for Anki clients older than the modern format, and it would be a second format to test for no gain.
  The modern format is the export's default when the legacy option is off.
- Each note's time set to the build's time: rejected because every note of every package would then
  be newer than the owner's, and each import would overwrite an edit the owner made in Anki.
- A patch to the engine's fork so an added note keeps a given time: rejected because ADR-058 keeps the
  fork to the one upstream fix; the time is set on the throwaway collection's own note row instead.

## Decision Outcome

Chosen option: "the engine's `export_apkg` over a throwaway collection, with a frozen note type and
the decision's time", because it keeps every write inside a collection DeckStreak made and drops, and
lets the owner's importer decide each note by the owner's own clock.

- **Binding 4's reading.** "Never writes an Anki collection directly" is read as the owner's
  collection and its copy. The throwaway collection is DeckStreak's own, lives in memory, and exists
  only to be exported; it is not a collection the owner studies from.
- **The note type** `DeckStreak vault card` has the fields `Front` and `Back`, one template, `Card 1`,
  whose question is `{{Front}}` and whose answer is `{{FrontSide}}<hr id=answer>{{Back}}`, a fixed
  style, and a fixed id and time, added with the engine's call that keeps both. A change to any of
  it is a new note type with a new id and a new ADR, never an edit of this one.
- **The times.** Each note's time is its decision's second. A later package without a new decision
  holds the same time, so the owner's import leaves a note the owner edited in Anki as it is; an
  approved edit carries a later decision, so it updates the note.
- **The fields** are escaped (`&` first, then `<`, `>` and `"`, and each newline as `<br>`), and a card
  that fails the side checks is left out and counted, never cut.
- **The file** is written into a temporary directory the builder creates, read into memory, and the
  directory removed before the builder returns.

### Consequences

- Good, because the import adds a new card once, updates an approved edit in place with its card's
  scheduling kept, and keeps the owner's own later edits.
- Good, because no dependency is added and the engine's fork is unchanged; ingest's `tempfile`
  moves from its test dependencies to its dependencies.
- Bad, because the owner's "always update" import option would overwrite the owner's own Anki edits
  with the approved text; that is the owner's choice at import, and the reply never claims a card is
  in Anki.
- Bad, because the engine's export writes two temporary files of its own while it runs, a copy of
  the throwaway collection and the package beside its output; the engine removes the copy, the
  builder's directory holds the package and is removed, and neither is the owner's.

### Confirmation

SPEC-151's A1 to A6 (import into a fresh collection, escaping, an in-place update, the owner's edit
surviving, the frozen times, the modern format and no file left), and rows `S15105` to `S15112`.

## What would make this wrong

- An engine release whose importer stops honouring the package's note time, or changes its default
  condition: A3 and A4 would fail on the engine move, and the design would need a new ADR.
- A reading of CHARTER 4 that counts a file the owner imports as a write back to Anki: the feature
  would then need the owner's ruling before it is built.

## More Information

SPEC-151, ADR-009 (ingest uses Anki's own engine), ADR-058 (the pinned fork), ADR-150 (the GUID), and
the schematic `docs/schematics/vault-card-package-delivery.md`.
