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
the owner's act; DeckStreak writes no Anki collection but the in-memory one it builds the file from
and the export's own temporary copy of it, which the engine removes.

## The ai-assisted marker, from the package source to each card view

Kind: data flow. Added by SPEC-388 (ADR-402), read at DeckStreak `dev` `3ca06142`. It adds no
outbound file, no write and no store. It draws how one tag, `ai-assisted`, rides in a field every
note already has, its tags, from the package source to each client's card view, and where ADR-402
and the course sentence bind it. The tag sits in the note, not in the note type, so the frozen note
type and the package's shape above are unchanged.

### From the source to every collection

```mermaid
flowchart LR
  source["a package source: a note whose card was made with AI assistance"] --> tag["its tags hold ai-assisted"]
  tag --> mem["the collection in memory: the frozen note type, the note's tags beside its fields"]
  mem --> export["export_apkg: each note with its tags"]
  export --> imp{"the owner imports the package"}
  imp -->|a new GUID| add["a new note, its tags as packaged"]
  imp -->|the package's note is newer| upd["the note updated, its tags as packaged, marked or unmarked"]
  imp -->|not newer| keep["the owner's own note kept, its tags as they are"]
  add --> col["the collection file: notes.tags"]
  upd --> col
  keep --> col
  col --> sync["full and normal sync: each note with its tags"]
  sync --> device["each device's collection"]
  col --> ingest["the server's private copy: ingest reduces tags to a tier and keeps no tag"]
```

The engine carries the tag on every edge of this figure: export, import, update, the collection
file and sync are the engine's own, and no DeckStreak code reads or writes the tag on them. SPEC-388
A7 to A9 drive those edges end to end. Ingest's card read already reads `notes.tags`
(`crates/ingest/src/reader.rs:41-44`) and keeps only the tier it reduces them to (`:126-128`,
`:308`), so the server stores no marker.

### From a collection to each card view

```mermaid
flowchart TD
  device["a device's collection"] --> web["web engine current_card: the shown card and its note id"]
  web --> read["engine core Dispatcher ai_assisted: one fixed read of the note's tags"]
  read --> rule{"ai_assisted carries: the whole tag ai-assisted, in any ASCII case"}
  rule -->|yes| marked["the view's assisted is true"]
  rule -->|no, or no such note| unmarked["the view's assisted is false"]
  marked --> line["the review: one line above the card, on both sides, below the late line"]
  unmarked --> noline["the review: no line"]
  device --> native["the native review: no line until it has localized strings"]
```

| layer | where | what it does with the tag |
|---|---|---|
| the rule | `crates/engine-core/src/ai_assisted.rs` (added) | `MARKER` and `carries`: a whole tag, without regard to ASCII case |
| the read | `crates/engine-core/src/dispatch.rs` (`Read`, `query`, `Dispatcher::ai_assisted`) | one fixed statement for one note's tags; a note the collection lacks reads unmarked |
| the view | `crates/web-engine/src/wasm.rs` (`current_card`) | the key `assisted`, from the core's read of the shown card's note |
| the page's type | `web/app/src/lib/engine/protocol.ts` (`CardView`) | `assisted: boolean` |
| the line | `web/app/src/lib/study/ReviewScreen.svelte` | `study_ai_assisted`, below the late line, above the card |
| the course sentence | `web/app/src/routes/progress/+page.svelte` | `progress_ai_assisted`, once, below the course list |
| the native review | the native app | nothing yet: it holds no localized strings (#738) |

### Where the decisions bind

```mermaid
flowchart LR
  purpose["ADR-402 D1: a personal study tool, not high-risk, Article 50 kept"] --> mark["ADR-402 D2: the tag is the card's machine-readable mark"]
  mark --> sources["every package source that makes a card with AI assistance writes the tag"]
  mark --> reviewline["the review's line on a marked card"]
  purpose --> sentence["ADR-402 D3: the progress screen's one sentence below the courses"]
  sentence --> tagname["it names the tag and no model or maker"]
```

ADR-402 D1 binds the whole figure: the purpose is one learner's own study, so the tag is a fact
shown to that learner, never a grade or a filter for anyone else. D2 binds the source: a source
that makes a card with AI assistance writes the tag on its note, and the package builder, when it
is built, writes it too (#65). D3 binds the one sentence, `Cards made with AI assistance carry the
tag ai-assisted.`, to the progress screen, the one screen that lists the learner's courses.
