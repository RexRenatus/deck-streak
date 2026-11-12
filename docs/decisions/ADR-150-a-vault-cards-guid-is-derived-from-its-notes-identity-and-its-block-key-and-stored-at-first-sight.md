---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A vault card's GUID is derived from its note's identity and its block key, and stored at first sight

## Context and Problem Statement

W9 turns the flashcards in the owner's tagged notes into Anki notes (#65, SPEC-150, SPEC-151). Anki
matches an imported note to one it already holds by the note's GUID alone: the same GUID updates the
note in place and keeps its card's scheduling, and a new GUID adds a second note beside the first.
The second criterion of #65 is that re-exporting an approved card keeps its GUID. The owner edits
notes freely: fixes a typo, inserts a card above another, renames or moves a note. And the vault is read,
never written, for this feature (ADR-011), so DeckStreak cannot write an id into a note. What is a
card's GUID made from, so that each of these edits keeps it?

## Decision Drivers

- An edit of a card's text keeps its GUID, or the owner's scheduling for it is lost.
- An inserted, removed or reordered card leaves every other card's GUID where it was.
- Nothing is written into the vault to make the GUID stable.
- An erase followed by a rescan gives the same GUIDs, so the owner's Anki is not doubled.
- The derivation is pinned by a test and a mutation row, so no change moves it unseen.

## Considered Options (the alternatives it was chosen against)

- The note's identity and a block key the owner writes on the card, hashed with a domain and stored at first sight: chosen, because every edit the owner makes to a card's text or position keeps both parts.
  The identity is the note's frontmatter `id` when it has a valid one, else its vault-relative path;
  the key is the card's Obsidian block id (` ^key`), which Obsidian already uses to link to a block,
  so the owner's markup costs nothing new. The GUID is `vc1-` and 32 hex characters of the SHA-256
  of a domain string, the identity and the key; it is stored when the candidate is first stored and
  read from the table ever after.
- A hash of the card's text: rejected because a fixed typo changes the GUID, and Anki then imports the
  fixed card as a new note with its scheduling lost, which is exactly #65's criterion broken.
- The card's position in its note: rejected because inserting or removing a card above another
  re-points every GUID below it, so an import rewrites one card's note with another card's text.
- An id DeckStreak writes into the note: rejected because the vault is read for this feature and
  DeckStreak is no writer of any contract that would hold it (ADR-011).
- A random id, stored: rejected because an erase followed by a rescan mints new GUIDs, and the owner's
  next import doubles every card already in Anki.
- The file's inode or creation time: rejected because a copy, a sync between devices or a restore
  gives the same note another one, and neither is part of the note's text.
- The note's path alone, with the card's key: rejected as the only identity because a rename or a
  move re-keys every card of the note; it is kept as the fallback when a note has no valid `id`.

## Decision Outcome

Chosen option: "the note's identity and a block key the owner writes, hashed with a domain and stored
at first sight", because it is the only option that survives every text and position edit while
writing nothing into the vault.

- **The identity** is `id:` and the frontmatter `id` (1 to 128 of letters, digits, `.`, `_`, `-`),
  else `path:` and the vault-relative path. Two notes with one identity have all their cards
  refused, so one GUID never names two cards.
- **The key** is the block id (1 to 64 of letters, digits and `-`). A card without one is refused
  `key_missing`, never keyed by DeckStreak; two cards of one note with one key are both refused.
- **The GUID** is `vc1-` followed by the first 32 lowercase hex characters of the SHA-256 of
  `deckstreak.vault-card.v1`, a 0x00 byte, the identity, a 0x00 byte and the key. The `vc1-` prefix
  and the domain string version the derivation: a new derivation is a new prefix and a new ADR, and
  never recomputes a stored GUID.
- **Stored at first sight.** The candidate row keeps the GUID; the package reads it and never
  recomputes it, so a later change to the derivation cannot move a card already in Anki.

### Consequences

- Good, because a typo fix, a reorder and a new card above another all keep every existing GUID, and
  the owner's scheduling survives each import.
- Good, because an erase and a rescan give the same GUIDs, so the owner's Anki is never doubled.
- Bad, because a note with no frontmatter `id` re-keys its cards when it is renamed or moved: the old
  candidates go absent and the new ones are pending. The owner avoids it with an `id`, and SPEC-150
  names the risk.
- Bad, because the owner must write a block key on each card. Obsidian offers to create one when the
  owner links to a block, and a keyless card is reported, never silently skipped.

### Confirmation

SPEC-150's A6 to A8 (the identity, the pinned vectors, an edit keeping the GUID), rows `S15007` and
`S15008`, and SPEC-151's A1 and A3 (the package carries the stored GUID, and a later package updates
the note in place) with row `S15110`.

## What would make this wrong

- An Obsidian change that rewrites block ids on its own: the key would stop being the owner's.
- An owner who keeps no `id` and renames notes often: the fallback would then re-key cards often, and
  writing an `id` for the owner would need a vault contract with its one writer (ADR-011).

## More Information

SPEC-150 (the candidates), SPEC-151 (the package), ADR-011, ADR-151, and the schematic
`docs/schematics/vault-card-candidate-lifecycle.md`.
