---
status: "accepted"
date: "2026-10-29"
decision-makers: "the DeckStreak maintainers"
---

# The owner's export is reachable from the study clients through one owner-session route

## Context and Problem Statement

The owner's export (SPEC-021) is one JSON document, `deckstreak.export.v1`, written by the host's
`deckstreakd data export` and sent by the bot's `/export`. The web and native study clients cannot
fetch it, keep their own device backups out of the owner's sight on every screen but the web sync
page, and show the server's backup age nowhere. How does a study client reach the export, and where
do the device backups and the server's backup age show (SPEC-516)?

## Decision Drivers

- One document: the copy a screen saves must be byte for byte the one the host writes, so a test
  can hold the two equal and the policy describes one thing.
- No new way in: the route sits behind the owner's session and the state-change guard every other
  writing route already passes, with bounds of its own.
- No new engine pair: a backup made on the owner's word reaches the engine as the full-sync
  choice's backup does, through the core's fixed statement.
- One retention rule: the native backups are kept by the core's rule the web already applies
  (SPEC-377 R17).

## Considered Options (the alternatives it was chosen against)

- **One owner-session route, `POST /api/data/export`, answering `export_document`'s bytes** —
  chosen.
- **An MCP export tool** — withdrawn: the MCP server serves the agent, not the owner's screens,
  and its export tool was withdrawn (SPEC-369 R8).
- **A GET link** — a link starts a download when anything opens it, a preview or a prefetch
  included, and a GET takes no state-change guard; the export is the owner's whole record.
- **An Anki package from a new engine pair** — it adds a pair to the per-transport table and an
  engine write to review, for a format whose own delivery comes later; the JSON document is the
  owner's data, and the collection's package is a different thing.

## Decision Outcome

Chosen option: "one owner-session route", because it serves the one document from the one
function the host role calls, behind the guards every writing route has, and adds nothing to the
engine's surface.

- **D1. The route.** `POST /api/data/export`: the state-change guard, the owner's session, a
  window of its own (6 a minute), a body of at most 64 bytes, the database; then 200 with
  `export_document`'s line, `no-store`, and an attachment's file name.
- **D2. One document.** `coordination::data_rights_registry::export_document` answers the export's
  one line with its newline; the `data` role writes it to standard output and the route answers
  it, so the two are equal by construction and a composed test holds them equal over one database.
- **D3. Back up now.** `engine_core::backup_now::back_up` fills a new file with the open collection
  through the core's fixed `VACUUM INTO` statement on the database door, refusing the open
  collection's own name and a file that holds a row, and reads it back through a private engine,
  refusing a backup that lacks a device id. The web engine names its file through `files.rs` and
  the native adapter in the collection's folder, as each names a choice's backup.
- **D4. The device backups on both screens.** The web data screen shows the sync screen's own list
  and export (SPEC-377 R15, R16). The native adapter lists its folder's backups and server copies
  by the stamp their names carry and applies the core's retention, and the app exports one through
  `fileExporter`; neither removes a file while a full-sync choice's stage is held.
- **D5. The server's backup age.** Both screens show the snapshot route's age (SPEC-377 R12 to
  R14); an answer of unknown, a refusal or no session shows unknown in words.
- **D6. Inside Telegram on a phone.** The web screen cannot see a refused save, so where the client
  is Telegram on iOS or Android it names `/export` beside the download.
- **D7. The native JSON copy waits for a credential.** The native app holds no owner session; its
  screen names the web screen and `/export` until a device credential reaches the route.

### Consequences

- Good, because a screen's copy and the host's are one document, held equal by a test.
- Good, because the native app's backups stop growing without bound: the core's rule keeps three
  of each kind on both clients.
- Bad, because the native screen cannot save the JSON copy itself until it holds a credential.
- Bad, because a large export rides inside the shell's 10-second bound; past it the bot and the
  host remain.

### Confirmation

SPEC-516's acceptance fence: the route's own tests, the composed byte equality, back up now's
tests in the core, the native adapter's backups tests, the web screen's unit tests and locales, the
rendered audit of `/data`, the policy's test, and the census of the per-transport table's rows.

## More Information

SPEC-516; SPEC-021 R2, R8; SPEC-377 R12 to R17; ADR-388 D14 to D18.
