# Schematic: the vault-card package, from the approved cards to the owner's chat

Kind: data flow. Read at DeckStreak `dev` a4036b3 (ADR-006, ADR-009, ADR-058,
`docs/schematics/data-flow.md`, `docs/schematics/engine-pin-lifecycle.md`,
`docs/schematics/bot-update-loop.md`, `docs/schematics/notification-router.md`), and at the engine's
pinned fork `57382da` for the export and the importer it relies on. Added by SPEC-151 (ADR-151,
ADR-152), after SPEC-150's `vault-card-candidate-lifecycle.md`. It adds to `data-flow.md` one
outbound file, sent as a command reply; it adds no write to the owner's collection or its copy, and
it rewrites no accepted schematic.

## The build and the send

```mermaid
flowchart LR
  ask["the owner: /vaultpack"] --> gate["the owner gate"]
  gate --> usecase["coordination vault_cards package"]
  usecase --> deckset{"the package deck configured"}
  deckset -->|no| notconf["not_configured line, nothing sent"]
  deckset -->|yes| view["read vault_approved_cards only"]
  view -->|empty| none["nothing_approved line, nothing sent"]
  view --> recheck["the side checks again: a failing card left out and counted"]
  recheck --> builder["ingest PackageBuilder on the Offload"]
  builder --> mem["a collection in memory: the frozen note type, one note per card, its stored GUID, escaped fields, the decision's time"]
  mem --> export["export_apkg, modern format, no scheduling, deck options or media, into a temporary directory"]
  export --> bytes["the bytes in memory, the directory removed"]
  bytes --> size{"at most 50,000,000 bytes"}
  size -->|no| toolarge["package_too_large line, nothing sent"]
  size -->|yes| send["send_document vault-cards.apkg into the command's chat, with the counts caption"]
```

The package exists in memory and in that one document. No route of the API serves it, no job builds
it, and the log lines along this path carry counts and outcome words only.

## What the owner's import does

```mermaid
flowchart TD
  file["the owner imports vault-cards.apkg in Anki"] --> guid{"a note with this GUID exists"}
  guid -->|no| add["a new note and card in the configured deck"]
  guid -->|yes| newer{"the package's note is newer, under the default if newer"}
  newer -->|yes| update["the note's fields updated in place, its card and scheduling kept"]
  newer -->|no| keep["the owner's own edit in Anki kept"]
```

Each note's time is the owner's decision (ADR-151), so a package re-sent without a new decision never
overwrites an edit the owner made in Anki, and an approved edit does update the note. The import is
the owner's act; DeckStreak writes no Anki collection but the in-memory one it builds the file from.
