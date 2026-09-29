# Schematic: a vault card, from the owner's tagged note to an approved candidate

Kind: data flow and state machine. Read at DeckStreak `dev` a4036b3 (ADR-006, ADR-011, ADR-116,
ADR-118, `docs/schematics/data-flow.md`, `docs/schematics/vault-write-paths.md`,
`docs/schematics/bot-update-loop.md`, `docs/schematics/owner-session.md`,
`docs/schematics/data-rights-export-and-erase.md`). Added by SPEC-150 (ADR-150, ADR-152). It adds to
`data-flow.md` one read of the vault (the card walk) and one read of the collection's copy (the plain
fronts); it adds no path to `vault-write-paths.md`, because this feature writes nothing into the
vault, and it rewrites neither.

## The scan

```mermaid
flowchart LR
  ask["the owner: /vaultcards, or Scan in the Mini App"] --> gate["the owner gate, or the owner's session"]
  gate --> usecase["coordination vault_cards scan"]
  usecase --> tagset{"the card tag and the vault root configured"}
  tagset -->|no| notconf["not_configured, nothing read"]
  tagset -->|yes| fronts["ingest fronts: the in-scope notes' first fields and GUIDs, read-only under the shared lock"]
  fronts -->|the copy unreadable| unchecked["every new revision flagged unchecked"]
  fronts --> walk["the card walk: regular .md notes, at most 50,000 entries"]
  unchecked --> walk
  walk -->|skipped| excluded["dot entries, links, the journal, the inbox, each duty's folders, the readings folder"]
  walk --> note{"a note of at most 1 MiB, UTF-8, carrying the tag"}
  note -->|no| reported["note_too_large, note_unreadable, or not tagged"]
  note -->|yes| parse["the Flashcards section: each Q and A paragraph with its block key"]
  parse -->|refused| refusal["a refusal with the note's name and the reason"]
  parse --> store["one Db write: candidates, revisions, presence"]
  store --> report["the report: counts, refusals, capped, checked"]
```

The walk and the fronts are reads; only the store step writes, and it writes DeckStreak's own two
tables. A capped walk stores what it read and marks nothing absent.

## A revision's states

```mermaid
stateDiagram-v2
  [*] --> Pending: a new text of a card, or a new card, stored by a scan
  Pending --> Approved: the owner approves
  Pending --> Rejected: the owner rejects
  Pending --> Edited: the owner edits, and an owner edit is stored approved
  Pending --> Withdrawn: a scan reads another text of the card, or finds the card absent
  Approved --> Withdrawn: the owner approves another revision of the card
  Withdrawn --> Pending: a scan reads this text again
  Rejected --> [*]
  Edited --> [*]
```

`Rejected` and `Edited` are final for their text: the migration's trigger refuses any move out of
them, and a scan that reads that text again adds nothing. An owner edit is stored `Approved` and can
only become `Withdrawn`. One `Pending` and one `Approved` revision at most per candidate, each held by
a partial unique index.

## The decision

```mermaid
flowchart LR
  bot["the bot: va, ve or vr and the revision id"] --> decide["coordination vault_cards decide"]
  api["the API: POST approve, edit or reject"] --> decide
  decide --> write["one Db write, a conditional update"]
  write -->|the revision was pending| applied["applied: the prior approved withdrawn by approve and edit"]
  write -->|it was not| notpending["not_pending, nothing changed"]
  write -->|no such revision| unknown["unknown, nothing changed"]
  applied --> view["vault_approved_cards: approved and present only"]
```

Both surfaces reach the store through the one use case (ADR-152), so two decisions of one revision at
once apply one. The view is what SPEC-151's package reads, and nothing else is read for it.

## Data rights

Both tables are declared by the vault's data-rights port and exported and erased with the owner's
data (`data-rights-export-and-erase.md`). An erase deletes these rows and never a note (ADR-118); a
card the owner has already imported into Anki is the owner's and no erase reaches it.
