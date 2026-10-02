---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A capture is claimed by its stem and written attachment first, a document keeps only a plain extension, and an erase never deletes a vault file

## Context and Problem Statement

#154 ports the predecessor's media capture: a photo, voice note or document the owner sends lands in
the vault inbox with a stub note, and #56 adds a Mini App capture that writes the same stub. The
predecessor (`vault_bridge.py:save_inbox_capture`, `bot.py:CommandBot._maybe_capture_media`) creates
the inbox when it is missing, overwrites a capture with the same stem, writes the attachment and the
stub with plain writes, and keeps whatever follows the last dot of a document's name as its
extension. SPEC-042 R1 forbids creating a folder at the vault's top level, SPEC-042 R2 requires the
atomic write, and a capture is personal data under SPEC-021's export and erase. What does DeckStreak
keep, and what does it change?

## Decision Drivers

- The stub's shape and the stem are the predecessor's product: the owner's vault and the curator
  read them.
- A capture is the owner's only copy of what they sent.
- A sender's file name is untrusted input.
- Export and erase are symmetric, and the vault is the owner's folder, not DeckStreak's.

## Considered Options (the alternatives it was chosen against)

- Keep the stem and the stub, and make every write safe: chosen, because it keeps the product and
  removes the three unsafe writes. Each capture is claimed by a unique stem in `inbox_captures`,
  the attachment is written first and the stub last through the atomic writer, the inbox is never
  created, only a plain extension is kept, and an erase deletes the rows but no vault file.
- Port the writes as they are: rejected because a document named with separators after its last
  dot would write outside the inbox, and a resent file would overwrite a capture the curator may be
  reading.
- Hold the capture in the ledger and let the nightly pass write it into the vault: rejected because
  the owner would not see a capture in their vault until the pass, and a second copy of every file
  would live in the ledger.
- Read the whole download into memory, then write it: rejected because streaming into the temporary
  file bounds a capture's memory to one buffer, whatever the file's size.
- Let an erase delete the capture files too: rejected because the vault is the owner's own folder,
  which DeckStreak writes into and does not own, so an erase there would delete the owner's notes.
- Create a missing inbox, as the predecessor did: rejected because SPEC-042 R1 forbids it, and a
  missing inbox usually means the vault is not the one configured.

## Decision Outcome

Chosen option: "keep the stem and the stub; claim by stem, attachment first, no folder created, a
plain extension, and an erase that keeps the files", because it is the only option that ports the
product while every write stays inside SPEC-042's rules.

- **The claim.** One `BEGIN IMMEDIATE` transaction inserts the row, renames the attachment into
  place and writes the stub; a stem already recorded answers `already_captured` with the existing
  name, so a resend is idempotent.
- **The extension.** `^[A-Za-z0-9]{1,10}$` after the last dot, else `.bin`.
- **The size.** The Bot API's 20 MB download limit is checked before the fetch and while streaming.
- **The erase.** The rows go; the files stay, and the owner removes them in the vault.

### Consequences

- Good, because a capture can never land outside the inbox, overwrite another, or appear as a stub
  whose attachment is missing.
- Good, because a Telegram resend and a Mini App retry both answer the same name.
- Bad, because an erase leaves the capture files in the vault. `PRIVACY.md` says so, and the
  owner's answer to the hand-back's question can change it by a later decision.

### Confirmation

SPEC-118's A2 to A4, A8 to A10 and A19, and its rows S11804 to S11806 and S11809 to S11811.

## What would make this wrong

- An owner who wants an erase to clear the inbox too: an erase would then delete only files whose
  stem is still in `inbox_captures` and still in the inbox, never a filed one.
- A Bot API that raises its download limit: the cap moves with it, and the streamed write already
  bounds memory.

## Amendment (V1a): an attachment never takes its stub's name

The predecessor names a capture's attachment `<stem><extension>` and its stub `<stem>.md`
(`vault_bridge.py:save_inbox_capture` at `27ee2bc`). A document whose extension is `md` therefore
has its stub's own name, and the stub's write replaced the document: the owner lost the only copy of
what they sent, and the stub's `[[...]]` named the stub itself. SPEC-118 R7 keeps `md`, which
matches `^[A-Za-z0-9]{1,10}$`, so a Markdown document sent to the bot reaches that path. Parity does
not excuse a data loss production can reach.

The stub stays `<stem>.md` for every capture. An attachment whose extension equals `md` in any case
(a synced vault may sit on a filesystem that ignores case) is named `<stem>.attachment.<extension>`,
its extension kept as it came, and the stub's `attachment:` key and its `[[...]]` name it. No stem
holds a dot, because its day, its kind and its safe unique hold only digits, ASCII letters, `-` and
`_`. So every stub name holds exactly one dot and this attachment's name holds two, and no stub
name equals it in any case. Every other attachment keeps the predecessor's name, and the golden
`inbox_capture_stub` holds those at parity. SPEC-118's A23 proves the departure, and
`formal/tla/CaptureOnce/` states it as `StubNeverNamesTheAttachment`, with the stub's write modelled
as replacing whatever its file held.

### What the attachment's name was chosen against

- Read an `md` extension as `.bin`: rejected because the document loses its extension and stops
  opening as Markdown in the owner's vault.
- Keep the predecessor's names: rejected because the stub's write replaces the owner's document, a
  data loss a Markdown document from the bot reaches.
- Name it `<stem>-attachment.md`: rejected because that is itself the stub name of the stem whose
  unique ends in `-attachment`, so a later capture's stub could replace it.
- Rename the stub instead: rejected because the curator and the owner's vault find a capture by its
  stub at `<stem>.md`, whatever its kind (SPEC-116).

What would make this wrong: a stem that can hold a dot. The second dot would then no longer set the
attachment apart from every stub name, and the name would need another separator.

## More Information

SPEC-118, SPEC-042 R1 and R2, SPEC-116 (the curator), SPEC-021 (export and erase), and the W6
schematic `docs/schematics/inbox-capture-and-curation.md`.
