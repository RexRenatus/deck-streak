# Schematic: a capture, from the owner's message to its place in the vault

Kind: data flow and state machine. Read at DeckStreak `dev` 5216bcf (ADR-042, ADR-054,
`docs/schematics/vault-write-paths.md`, `docs/schematics/bot-update-loop.md`,
`docs/schematics/notification-router.md`), and at the predecessor's `27ee2bc` for the behaviour it
ports (`bot.py:CommandBot._maybe_capture_media`, `_capture_file`,
`vault_bridge.py:save_inbox_capture`). Added by SPEC-118 (ADR-118) and SPEC-116 (ADR-116). It adds
to `vault-write-paths.md` one writer (the capture, streamed through the atomic writer) and one
mover (the curator's filing); it rewrites neither of the paths drawn there.

## The capture

```mermaid
flowchart LR
  media["the owner's photo, voice note or document, with its caption"] --> gate["the bot's owner gate"]
  gate --> choose["the first present: photo, voice, document"]
  choose --> size{"declared size within the download limit"}
  size -->|no| failfetch["reply: couldn't fetch, nothing saved"]
  size -->|yes| stream["getFile, then the file streamed into a temporary file, stopped at the limit"]
  stream -->|past the limit, or a failed fetch| failfetch
  quick["the Mini App capture screen: text or journal, a retry key"] --> write
  stream --> write["one transaction: insert the inbox_captures row, rename the attachment, write the stub last"]
  write -->|the stem exists, or a Mini App retry's capture key| dup["already_captured, the temporary file removed"]
  write -->|committed| saved["reply or 201: the stub's name"]
```

The stub is written after its attachment, inside the same transaction as its row, so the curator
never sees a stub without its file, and a capture is written once however often it is retried.
A Mini App retry is matched by its capture key, so a retry after UTC midnight, whose stem would
carry the next day, still answers the first name (ADR-118's capture-key amendment).

## A capture's states

```mermaid
stateDiagram-v2
  [*] --> Captured: row and stub written, state captured
  Captured --> Captured: journal kind, never an input, never moved
  Captured --> Filed: the curator's run moves stub and attachment, row records destination and study day
  Captured --> Captured: a refused plan, a changed or moved source, or no AI route, nothing moved
  Filed --> [*]
```

## The curation

```mermaid
flowchart TD
  snap["the inbox snapshot: each captured stub, its attachment and each file's SHA-256"] --> route{"AI route configured"}
  route -->|absent| rec["recorded ai_route_absent, nothing moved"]
  route -->|configured| task["inbox-curator task: ids, kinds, names and captions only, no bytes, no tool"]
  task --> plan{"plan: every id in the snapshot, every destination in the layout, no id twice"}
  plan -->|no| refused["the plan is refused whole"]
  plan -->|yes| reread{"each source re-read: same SHA-256 and still in the inbox"}
  reread -->|no| changed["capture_changed or capture_moved, nothing moved"]
  reread -->|yes| move["each move through the rails, byte for byte, the stub's text never edited"]
  move --> row["each row set to filed"]
  row --> occ["when one or more is filed, one occasion through the router: the count, no file name"]
```

An erase deletes the rows and never a vault file: the vault is the owner's own folder.
