# SPEC-118: a photo, voice note or document the owner sends lands in the vault inbox once, and a quick capture writes the same stub

- **Wave:** W6. **Issues:** #154 (media capture to the inbox) and #56 (a journal-shaped capture from
  the Mini App) (epic #7). **Context(s):** `deck-streak-vault` (the stem, the stub, the writes and
  the table `inbox_captures`); `deck-streak-coordination` (one use case both surfaces call);
  `deck-streak-bot` (admitting media, the download, the replies); `deck-streak-api` and the Mini App
  (the quick capture).
- **Decided by:** ADR-042 (the vault's write paths, note format and rails), ADR-059 (public text) and
  ADR-118 (a capture is claimed by its stem and written attachment first, a document keeps only a
  plain extension, and an erase never deletes a vault file).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-024, SPEC-026, SPEC-029, SPEC-042 and SPEC-110. **Mutation
  band:** `S11800-S11899`.
- **Status:** delivered in part by V1a (moved from `docs/specs/planned/` with its tests and
  `docs/red-first/SPEC-118.md`, ADR-016): R1 to R5 and R10 to R14, the criteria of section 3
  and B1, proved by `formal/tla/CaptureOnce/`. The remainder, #154's media from the bot (R6 to
  R9, A7 to A14 and B2), is delivered by V1b, section 3c.

## 1. The problem, measured

- **The predecessor's capture** (`bot.py:CommandBot._maybe_capture_media`, `_capture_file`,
  `_download_telegram_file`, `nudges.py:NudgesLayer.save_inbox_capture` and
  `vault_bridge.py:save_inbox_capture`, at `27ee2bc`): a photo keeps its last (largest) size as
  `.jpg`, a voice note is stored as `.ogg` as it came, a document keeps the text after the last dot
  of its name or `.bin`, and a document's caption defaults to its file name. The file lands in the
  inbox as `<UTC date>-<kind>-<safe unique>` with a same-stem `.md` stub, and the bot answers with
  the saved name, a failed-fetch line or a failed-save line. A missing vault is reported, never
  raised.
- **Three things DeckStreak cannot port as they are.** The predecessor creates the inbox when it is
  missing, which SPEC-042 R1 forbids at the vault's top level; it overwrites a same-stem capture;
  and a document's extension is whatever follows the last dot, so separators in a sender's file
  name reach the path (ADR-118).
- **The bot drops media today.** SPEC-026's gate admits an owner message only with text
  (`no_text`), so a photo never reaches any handler.
- **#56** adds a Mini App quick capture whose optional journal line lands as a journal-shaped
  capture in the inbox, never in a journal section. SPEC-116 keeps a journal-kind capture away from
  every model.

## 2. Requirements

The capture (vault)

R1. `vault::inbox::stem(kind, unique, when)` is `<UTC date of when>-<kind>-<safe unique>`, where the
    safe unique is `unique` with every character outside `[A-Za-z0-9_-]` removed, cut to 32
    characters, or `capture` when that leaves nothing (`vault_bridge.py:save_inbox_capture`,
    `_UNIQUE_SAFE`; golden `inbox_capture_stub`). An extension without a leading dot gains one, and
    an empty one reads `.bin`. An attachment whose extension is `md`, in any case, is named
    `<stem>.attachment.<extension>`, so it never takes its stub's name `<stem>.md`: a departure
    from the predecessor, whose stub replaced such a document (ADR-118's amendment; A23).
R2. A Telegram capture's stub has exactly the predecessor's shape: the frontmatter `status:
    captured`, `source: telegram`, `kind`, `captured` (the UTC instant to the second, with its
    offset), `attachment` and `tags: [inbox, telegram-capture]`, in that order; then the line
    "Captured via Telegram. Attachment: [[<attachment name>]]"; then, when the stripped caption is
    not empty, a blank line and the caption (golden `inbox_capture_stub`).
R3. A capture is written through SPEC-042 R2's atomic writer, which gains a streamed form: the
    attachment's bytes land in its temporary file first; then one `BEGIN IMMEDIATE` transaction
    inserts the `inbox_captures` row, renames the attachment into place, writes the stub last, and
    commits. A row that already holds the stem refuses with `already_captured` and the temporary
    file is removed, so a capture is written once on the host, and the curator never sees a stub
    before its attachment.
R4. The layout in force is the file `DECKSTREAK_VAULT_LAYOUT` names (the owner's layout, which is
    private and never enters the repository), else the vendored default
    (`crates/vault/data/layout.json`). The inbox is its `inbox` and the journal its `journal`
    folders. A missing vault root or a missing inbox folder refuses with `vault_missing`, and the
    adapter never creates either (SPEC-042 R1).
R5. Every file the vault context writes goes through the atomic writer, which refuses a path under a
    folder the layout names in `journal` with `journal_refused`. No code path writes the journal
    (#56).

Media from the bot

R6. SPEC-026 R4's gate admits an owner message that carries a photo, a voice note or a document as
    media, with its caption; the first present wins, in the predecessor's order
    (`bot.py:CommandBot._maybe_capture_media`; golden `media_capture_choice`): a photo's last size
    as `.jpg`; a voice note as `.ogg`, stored as it came with no speech to text; a document with its
    name's extension or `.bin`, its caption defaulting to its file name. Media with no file id is
    ignored silently. A non-owner's media is dropped as every non-owner update is. The poll keeps
    requesting `message` and `callback_query`, which carry media too.
R7. A document keeps an extension only when the text after its name's last dot matches
    `^[A-Za-z0-9]{1,10}$`; any other reads `.bin` (ADR-118). This departs from #154's "every number is a golden": the Bot API's size cap (R8) and this extension rule are limits the plan sets for the Rust adapter, so unit tests prove them (A8, A9) and no golden holds them.
R8. The download is `getFile`, then the file's URL, streamed into R3's temporary file. A declared
    `file_size` over 20 MB (20 × 1024 × 1024 bytes, the Bot API's download limit) is never fetched,
    and a stream that passes that count is stopped and its temporary file removed; both read as a
    failed fetch. The file URL carries the bot token and never reaches a log line (SPEC-020's
    redaction).
R9. The bot replies with the predecessor's lines (`bot.py:CommandBot._capture_file`; golden
    `media_capture_replies`): "📥 Saved to Inbox as <code><name></code> — the nightly pass will file
    it." (the name HTML-escaped; `already_captured` names the existing file); "⚠️ Couldn't fetch that
    file from Telegram — nothing saved."; or "⚠️ Couldn't save that capture to the vault inbox." for
    any refusal of the save, `vault_missing` among them, which is reported and never raised.

The quick capture (#56)

R10. `POST /api/inbox/captures`, behind SPEC-024 R7's `OwnerSession`, takes `{capture_id, kind,
    text}`: `capture_id` is the client's retry key (its safe form is R1's unique), `kind` is `text`
    or `journal`, and `text` is 1 to 4000 characters after trimming (the plan's bound: a Telegram message holds at most 4096 characters, so 4000 leaves room for the stub's own lines). It writes R3's capture with no
    attachment: the stub has R2's keys without `attachment`, with `source: miniapp` and `tags:
    [inbox, miniapp-capture]`, then the line "Captured via the Mini App.", a blank line and the text.
    It answers 201 with the file name, 200 with the existing name for `already_captured`, 422 for a
    text out of bounds, and 503 for `vault_missing`. A retry is matched by its `capture_id`, not by
    its stem, so a retry sent after UTC midnight answers the first name (ADR-118's capture-key
    amendment; A24).
R11. The Mini App's capture screen (`/capture`) has one text field and a "journal" choice, sends a
    fresh `capture_id` per capture and the same one on a retry, and shows the saved name or one
    failure line.

The table and the data rights

R12. The vault context owns `inbox_captures` (`migrations/011801_vault_inbox_captures.sql`, `STRICT`,
    `created_at`, per SPEC-020 R15 and R18): the stem (unique), the capture key (R1's safe unique,
    unique among the `miniapp` captures, R10), kind (`photo`, `voice`, `document`,
    `text` or `journal`), source (`telegram` or `miniapp`), the attachment's name (null without
    one), the captured instant, the state (`captured` or `filed`), and the destination and the
    filing's study day (null until SPEC-116 files it). Its six files are in §4. An export lists the
    rows; an erase deletes the rows and never a vault file, because the vault is the owner's own
    folder (ADR-118). The predecessor kept its captures as files only, so W8's import maps nothing
    into the table.
R13. `privacy.json` declares the category `inbox-captures`: the captions, texts and attachments'
    names are personal data; the vault holds the files; no model reads a capture unless the
    curator's route is configured (SPEC-116 R15), and a journal-kind capture never.
R14. CHARTER 10's eleven anti-goals bind this SPEC as one block; the one it touches is no dishonest
    copy: the bot says "saved" only after the commit.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the stem and the stub equal the predecessor's golden for every case | `the_stub_and_stem_match_the_predecessors_golden` |
| A2 | the attachment is in place before its stub is written | `the_attachment_lands_before_its_stub` |
| A3 | a missing vault root or inbox refuses with `vault_missing` and creates no folder | `a_missing_inbox_is_refused_and_never_created` |
| A4 | a capture with a stem already recorded is written once and answers `already_captured` | `a_capture_sent_twice_is_written_once` |
| A5 | a write to a path under a journal folder is refused with `journal_refused` | `no_vault_write_reaches_a_journal_folder` |
| A6 | every file-writing call in `crates/vault/src` is inside the atomic writer | `every_vault_file_write_is_the_atomic_writer` |
| A15 | a quick capture writes the stub of R10 | `a_quick_capture_writes_the_miniapp_stub` |
| A16 | a journal quick capture lands in the inbox with `kind: journal` | `a_journal_quick_capture_lands_in_the_inbox` |
| A17 | a retried quick capture answers the same name and writes nothing | `a_retried_quick_capture_answers_the_same_name` |
| A18 | the route answers 401 without the session, 422 at 0 and 4001 characters, and 201 at 4000 | `the_quick_capture_is_owner_only_and_bounds_its_text` |
| A19 | every `inbox_captures` row is exported, and an erase leaves no row and every vault file | `inbox_captures_export_and_erase_leave_the_files` |
| A20 | the screen sends the text, the chosen kind and one capture id per capture | `sends the text, the kind and one capture id` |
| A21 | the screen shows the saved name, or its failure line on 503 | `shows the saved name or the failure line` |
| A22 | with `DECKSTREAK_VAULT_LAYOUT` unset the vendored layout is in force, and with it set the owner's is | `the_layout_in_force_is_the_owners_or_the_default` |
| A23 | an attachment never takes its stub's name, and its bytes survive the stub, for every extension | `an_md_attachment_never_takes_its_stubs_name`, `every_extension_keeps_its_bytes_apart_from_the_stub` |
| A24 | a Mini App retry on a later UTC day answers the first name and writes nothing, and a Telegram capture of the same unique on a later day is a new capture | `a_miniapp_retry_on_a_later_utc_day_answers_the_first_name` |

```acceptance
A1: cargo test -p deck-streak-vault --test inbox_capture -- --exact the_stub_and_stem_match_the_predecessors_golden
A2: cargo test -p deck-streak-vault --test inbox_capture -- --exact the_attachment_lands_before_its_stub
A3: cargo test -p deck-streak-vault --test inbox_capture -- --exact a_missing_inbox_is_refused_and_never_created
A4: cargo test -p deck-streak-vault --test inbox_capture -- --exact a_capture_sent_twice_is_written_once
A5: cargo test -p deck-streak-vault --test atomic -- --exact no_vault_write_reaches_a_journal_folder
A6: cargo test -p deck-streak-vault --test atomic -- --exact every_vault_file_write_is_the_atomic_writer
A15: cargo test -p deck-streak-api --test inbox_capture_route -- --exact a_quick_capture_writes_the_miniapp_stub
A16: cargo test -p deck-streak-api --test inbox_capture_route -- --exact a_journal_quick_capture_lands_in_the_inbox
A17: cargo test -p deck-streak-api --test inbox_capture_route -- --exact a_retried_quick_capture_answers_the_same_name
A18: cargo test -p deck-streak-api --test inbox_capture_route -- --exact the_quick_capture_is_owner_only_and_bounds_its_text
A19: cargo test -p deck-streak-vault --test inbox_capture -- --exact inbox_captures_export_and_erase_leave_the_files
A20: pnpm exec vitest run web/app/src/lib/capture/QuickCapture.test.ts -t "sends the text, the kind and one capture id"
A21: pnpm exec vitest run web/app/src/lib/capture/QuickCapture.test.ts -t "shows the saved name or the failure line"
A22: cargo test -p deck-streak-vault --test layout_in_force -- --exact the_layout_in_force_is_the_owners_or_the_default
A23: cargo test -p deck-streak-vault --test inbox_capture -- --exact an_md_attachment_never_takes_its_stubs_name
A23: cargo test -p deck-streak-vault --test inbox_capture -- --exact every_extension_keeps_its_bytes_apart_from_the_stub
A24: cargo test -p deck-streak-vault --test inbox_capture -- --exact a_miniapp_retry_on_a_later_utc_day_answers_the_first_name
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. This delivery changes no pack's state; the rows judge
the files it adds under packs that are already enforced.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/vault/src/data_rights.rs`: the `inbox-captures` category names `inbox_captures` with purpose, basis and retention, and export and erase cover it | the privacy-gdpr pack |
| B2 | over `crates/bot/src/capture.rs`: every reply is escaped for the parse mode and stays within the message length | the telegram-platform pack |

## 3c. Delivered by the next pull requests

This SPEC lands in two pull requests, in order. This one (V1a) delivers the capture in the vault,
the quick capture and their table: R1 to R5 and R10 to R14, the criteria of section 3's table and
its fence, and B1. V1b delivers #154's media from the bot: R6 to R9, the criteria below and B2.
The table below holds the criteria V1b delivers, each row naming it, and the lines under it are
their fence lines, each prefixed `V1b:`. V1b moves each of its criteria back verbatim: the row into
section 3's table, without the `delivered by` column, and the fence line into the acceptance
fence, without the prefix.

| id | criterion | decided by | delivered by |
|---|---|---|---|
| A7 | the media choice equals the predecessor's golden for every case | `media_choice_matches_the_predecessors_golden` | V1b |
| A8 | a 10-character extension is kept, and an 11-character one or one with a separator reads `.bin` | `a_document_extension_off_the_rule_reads_bin` | V1b |
| A9 | a declared size of 20971520 bytes is fetched and 20971521 is not | `a_file_over_twenty_megabytes_is_never_fetched` | V1b |
| A10 | a stream that passes the cap is stopped and leaves no file | `a_stream_past_the_cap_is_stopped_and_discarded` | V1b |
| A11 | the three replies equal the predecessor's golden | `capture_replies_match_the_predecessors_golden` | V1b |
| A12 | with the vault missing, the owner gets the failed-save line and nothing is raised | `a_missing_vault_is_reported_not_raised` | V1b |
| A13 | the file URL never reaches a log line | `the_file_url_never_reaches_a_log_line` | V1b |
| A14 | an owner's media message is admitted, and a non-owner's is dropped | `media_is_admitted_from_the_owner_only` | V1b |

V1b: A7: cargo test -p deck-streak-bot --test media_capture -- --exact media_choice_matches_the_predecessors_golden
V1b: A8: cargo test -p deck-streak-bot --test media_capture -- --exact a_document_extension_off_the_rule_reads_bin
V1b: A9: cargo test -p deck-streak-bot --test media_capture -- --exact a_file_over_twenty_megabytes_is_never_fetched
V1b: A10: cargo test -p deck-streak-bot --test media_capture -- --exact a_stream_past_the_cap_is_stopped_and_discarded
V1b: A11: cargo test -p deck-streak-bot --test media_capture -- --exact capture_replies_match_the_predecessors_golden
V1b: A12: cargo test -p deck-streak-bot --test media_capture -- --exact a_missing_vault_is_reported_not_raised
V1b: A13: cargo test -p deck-streak-bot --test media_capture -- --exact the_file_url_never_reaches_a_log_line
V1b: A14: cargo test -p deck-streak-bot --test media_capture -- --exact media_is_admitted_from_the_owner_only

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/vault/src/inbox.rs` | `deck-streak-vault` | added: the stem, the stub, the capture's write |
| `crates/vault/src/capture_store.rs` | `deck-streak-vault` | added: `inbox_captures` |
| `crates/vault/src/atomic.rs`, `crates/vault/src/fs.rs` | `deck-streak-vault` | changed: the streamed write and the journal refusal |
| `crates/vault/src/config.rs` | `deck-streak-vault` | changed: the layout in force |
| `crates/vault/tests/layout_in_force.rs` | `deck-streak-vault` | added: A22 |
| `.env.example` | repo | changed: `DECKSTREAK_VAULT_LAYOUT`, by name, unset |
| `crates/vault/src/data_rights.rs` | `deck-streak-vault` | changed: `inbox_captures` exported and erased (the port SPEC-110 adds) |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the modules |
| `crates/vault/tests/inbox_capture.rs` | `deck-streak-vault` | added: A1 to A4, A19, A23, A24 |
| `crates/vault/tests/atomic.rs` | `deck-streak-vault` | changed: A5, A6 |
| `migrations/011801_vault_inbox_captures.sql` | `deck-streak-vault` | added |
| `crates/coordination/src/inbox_capture.rs` | `deck-streak-coordination` | added: the one use case both surfaces call |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: `inbox_captures` registered |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row |
| `docs/CONTEXT-MAP.md` | docs | changed: `inbox_captures` in the vault's own tables |
| `privacy.json`, `PRIVACY.md` | repo | changed: `inbox-captures` |
| `crates/bot/src/gate.rs` | `deck-streak-bot` | changed: media admitted from the owner |
| `crates/bot/src/capture.rs` | `deck-streak-bot` | added: the choice, the extension rule, the replies |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the module |
| `crates/bot/src/transport.rs` | `deck-streak-bot` | changed: `getFile` and the streamed download |
| `crates/bot/tests/media_capture.rs` | `deck-streak-bot` | added: A7 to A14 |
| `crates/api/src/inbox_capture_route.rs` | `deck-streak-api` | added: the quick capture |
| `crates/api/src/router.rs`, `crates/api/src/lib.rs` | `deck-streak-api` | changed: the route and the module; `ApiState` gains the port |
| `crates/api/tests/inbox_capture_route.rs` | `deck-streak-api` | added: A15 to A18 |
| `web/app/src/routes/capture/+page.svelte` | Mini App | added |
| `web/app/src/lib/capture/QuickCapture.svelte`, `web/app/src/lib/capture/capture.ts` | Mini App | added |
| `web/app/src/lib/capture/QuickCapture.test.ts` | Mini App | added: A20, A21 |
| `web/app/src/lib/routes.ts` | Mini App | changed: /capture |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the use case in the `bot` and `api` roles |
| `tools/parity-oracle/registry/spec_118.py` | repo | added: the three goldens of §7 |
| `tools/parity-oracle/goldens/inbox_capture_stub.json`, `tools/parity-oracle/goldens/media_capture_choice.json`, `tools/parity-oracle/goldens/media_capture_replies.json` | repo | added: generated by §7 |
| `scripts/mutation-rows.d/S11800-S11899.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-118-a-photo-voice-note-or-document-the-owner-sends-lands-in-the-vault-inbox-once-and-a-quick-capture-writes-the-same-stub.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/inbox-capture-and-curation.md` | docs | added by the W6 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-118.md` | docs | added |
| `formal/tla/CaptureOnce/` | formal | added: the model of R3 and R5, with its witnesses |
| `docs/decisions/ADR-118-a-capture-is-claimed-by-its-stem-and-written-attachment-first-a-document-keeps-only-a-plain-extension-and-an-erase-never-deletes-a-vault-file.md` | docs | changed: the amendment naming an `md` attachment apart from its stub |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: `Commands` gains the capture use case |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role hands the capture use case to its commands at start |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role hands the capture use case to `ApiState` at start (POST /api/inbox/captures) |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It files no capture; the curator does (#50).
- It transcribes no voice note (#154).
- It captures no audio file, video, sticker or animation; only the three kinds the predecessor
  captured (#154).
- It writes no journal section, and it adds no journal screen beyond the capture's journal choice
  (#56).
- It deletes no vault file on an erase; the vault is the owner's folder (#154, ADR-118).
- It imports no capture from the predecessor (#61).

## 6. Risks

- **A path escape through a sender's file name.** R7 keeps only a plain extension; detected by A8.
- **A half-written capture the curator files.** R3 lands the attachment first and records the row
  last; detected by A2.
- **A file with no row after a failed commit.** It stays in the inbox for the owner, outside the
  curator's snapshot (SPEC-116 R1 reads the rows); the bot says the save failed.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_118.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `inbox_capture_stub` | `vault_bridge.py:save_inbox_capture` | adapter | a temporary vault root with its inbox; cases for each kind, an empty and a blank caption, a unique with symbols, one of 32 characters, one of 40 and an empty one, an extension without a dot and an empty one |
| `media_capture_choice` | `bot.py:CommandBot._maybe_capture_media` | adapter | a bot built with a synthetic token and chat id whose file capture is recorded; a photo of three sizes, a voice note, a document named with one dot, two dots and none, a message with a photo and a document, an empty photo list, and a text message |
| `media_capture_replies` | `bot.py:CommandBot._capture_file` | adapter | the same bot, with the download and the inbox writer replaced by fakes that succeed or fail, and the sent lines recorded; a name that needs escaping |

## 8. Tables and the v9 import

`inbox_captures` (vault, `migrations/011801_vault_inbox_captures.sql`). The predecessor kept its
captures as files only, so W8's import maps nothing into it and it starts empty.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S11801-SAFE-CHARS` | `crates/vault/src/inbox.rs` | the unique's allowed characters | `inbox_capture::the_stub_and_stem_match_the_predecessors_golden` |
| `S11802-CUT-32` | `crates/vault/src/inbox.rs` | 32 characters; the golden names 32 and 40 | `inbox_capture::the_stub_and_stem_match_the_predecessors_golden` |
| `S11803-CAPTURE-FALLBACK` | `crates/vault/src/inbox.rs` | `capture` for an empty unique | `inbox_capture::the_stub_and_stem_match_the_predecessors_golden` |
| `S11804-ATTACH-FIRST` | `crates/vault/src/inbox.rs` | the stub is written last | `inbox_capture::the_attachment_lands_before_its_stub` |
| `S11805-NO-MKDIR` | `crates/vault/src/inbox.rs` | a missing inbox is refused | `inbox_capture::a_missing_inbox_is_refused_and_never_created` |
| `S11806-ONCE` | `migrations/011801_vault_inbox_captures.sql` | the stem is unique (a script-mutation row) | `inbox_capture::a_capture_sent_twice_is_written_once` |
| `S11807-JOURNAL-REFUSED` | `crates/vault/src/atomic.rs` | a journal path is refused | `atomic::no_vault_write_reaches_a_journal_folder` |
| `S11808-PHOTO-LAST` | `crates/bot/src/capture.rs` | the last photo size | `media_capture::media_choice_matches_the_predecessors_golden` |
| `S11809-EXT-RULE` | `crates/bot/src/capture.rs` | 1 to 10 plain characters; the test names 10 and 11 | `media_capture::a_document_extension_off_the_rule_reads_bin` |
| `S11810-SIZE-CAP` | `crates/bot/src/capture.rs` | over 20 MB is not fetched; the test names the cap and one byte more | `media_capture::a_file_over_twenty_megabytes_is_never_fetched` |
| `S11811-STREAM-CAP` | `crates/bot/src/transport.rs` | a stream past the cap stops; the fake stream is finite, so a mutant that keeps reading ends | `media_capture::a_stream_past_the_cap_is_stopped_and_discarded` |
| `S11812-TEXT-BOUND` | `crates/api/src/inbox_capture_route.rs` | 1 to 4000 characters; the test names 0, 4000 and 4001 | `inbox_capture_route::the_quick_capture_is_owner_only_and_bounds_its_text` |
| `S11813-OWNER-MEDIA` | `crates/bot/src/gate.rs` | media is admitted from the owner only | `media_capture::media_is_admitted_from_the_owner_only` |
| `S11814-RETRY-KEY` | `migrations/011801_vault_inbox_captures.sql` | a Mini App capture key is unique (a script-mutation row) | `inbox_capture::a_miniapp_retry_on_a_later_utc_day_answers_the_first_name` |

## 10. Amendments, 2026-10-02: R10 refuses a malformed request by name, the api role serves the capture, and the files V1a adds beside the manifest (#56, #154)

Insert-only: sections 1 to 9 are kept as they were, and this section is appended after them. It
adds no criterion. Every path below is a manifest row this delivery adds, extends or leaves to V1b.

- **R10's answers gain two refusals.** A `capture_id` that is not its own safe form (R1's unique),
  such as an empty id, one holding a blank or a `.`, or one longer than the unique keeps, answers
  422 `invalid_capture_id`, so two captures never share a retry key; a `kind` other than `text` or
  `journal` answers 422 `unknown_kind`. Both are pinned by
  `inbox_capture_route::the_quick_capture_refuses_a_bad_request_and_a_missing_vault`, beside R10's
  `vault_missing`. The route also answers 503 `vault_not_open` when the role serves no inbox, 503
  `database_not_open` before the database is open, and 500 `vault_unwritable` when the vault
  refuses the write.
- **The api role serves the capture over its configured vault.** The role composes the inbox from
  `DECKSTREAK_VAULT_ROOT` and the layout in force (R4) at start. An unset root, a root that is not
  absolute, or a layout file that cannot be read or is not a layout serves no capture: the route
  answers 503 `vault_not_open`, the refusal is logged by its rule and never by its value, and
  nothing is created. The vault is an owner choice (ADR-011), so none of these refuses the role's
  start.
- **Mutation row S11812.** R10's bound is `quick_text_fits` in `crates/vault/src/inbox.rs`, which the
  coordination use case applies before any write; the route holds no bound of its own. The row's
  target is therefore `crates/vault/src/inbox.rs`, and its killer is section 9's route test, which
  sends 0, 4000 and 4001 characters through the whole stack.
- **A6, as amended, reads:** every call in `crates/vault/src` that writes a file's bytes is inside the atomic writer.
  Its test, `every_vault_file_write_is_the_atomic_writer`, lists exactly those calls. A call that
  renames, creates or removes a path writes no file's bytes, and section 11 names each such call
  outside the writer (#56).
- **R5, as amended, is stated for the inbox capture's writes.** Every file the inbox capture
  writes, its attachment and its stub, goes through the atomic writer, which refuses with
  `journal_refused` a path under a folder the layout names in `journal`, whether the path names
  that folder or reaches it through a link: the guard resolves each journal folder when it is
  built, and resolves a target's deepest existing folder before it compares. An inbox that is a
  journal folder, by its name or through a link, is refused when it is located. The vault's other
  writers are section 11's (#56).

The files, against section 4's manifest:

| file | context | change |
|---|---|---|
| `crates/coordination/tests/inbox_capture.rs` | `deck-streak-coordination` | added: the use case's six tests (R3, R4, R5, R10) |
| `crates/api/tests/insights_routes.rs` | `deck-streak-api` | changed: the state's `Debug` line names the inbox port (`inbox: false`) |
| `crates/vault/src/fs.rs` | `deck-streak-vault` | extended: `VaultFile` is `Send`, so a capture whose attachment is streaming can be held across an await |
| `crates/vault/src/inbox.rs` | `deck-streak-vault` | extended: `QUICK_TEXT_CHARS` and `quick_text_fits`, R10's bound |
| `crates/vault/tests/inbox_capture.rs` | `deck-streak-vault` | extended: R10's bound, `a_quick_text_fits_one_to_four_thousand_characters_after_the_trim` |
| `crates/daemon/src/wiring.rs` | `deck-streak-daemon` | changed: the api role's half, `inbox_captures`; the bot role's half is V1b's |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: as its row says, `api_state` composes the inbox |
| `crates/daemon/tests/inbox_capture_composed.rs` | `deck-streak-daemon` | added: the role's composed router serves the capture over its configured vault alone |
| `web/app/src/lib/api.ts` | Mini App | changed: `createApi` gains `capture`, the POST through its one session handshake and its single 401 renewal |
| `web/app/src/lib/api.test.ts` | Mini App | changed: the capture's request shape, its renewal and its refusals |
| `web/app/src/lib/startapp.ts` | Mini App | changed: the token `capture` opens the /capture screen; ruling (m): the closed token map covers every route (ADR-028) |
| `web/app/src/lib/startapp.test.ts` | Mini App | changed: the capture token's case, and the destinations list gains `capture`; ruling (m): the closed token map covers every route (ADR-028) |
| `web/app/messages/en.json` | Mini App | changed: the capture screen's messages |
| `docs/red-first/SPEC-118.md` | docs | extended: the screen's and the wiring's records |
| `tools/parity-oracle/registry/spec_118.py` | repo | added with the `inbox_capture_stub` golden; V1b adds the two media goldens |
| `scripts/mutation-rows.d/S11800-S11899.json` | repo | added with V1a's rows, S11801 to S11807, S11812 and S11814; V1b adds S11808 to S11811 and S11813 |

The Mini App's screen is delivered as its rows say: `web/app/src/routes/capture/+page.svelte`,
`web/app/src/lib/capture/QuickCapture.svelte`, `web/app/src/lib/capture/capture.ts`,
`web/app/src/lib/capture/QuickCapture.test.ts` and `web/app/src/lib/routes.ts`.

The rows V1b delivers, each left as it is here:

- `crates/bot/src/gate.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/bot/src/capture.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/bot/src/lib.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/bot/src/transport.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/bot/src/commands.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/bot/tests/media_capture.rs`: unchanged in this part; delivered by V1b (#154).
- `crates/daemon/src/role_bot.rs`: unchanged in this part; delivered by V1b (#154).
- `tools/parity-oracle/goldens/media_capture_choice.json`: unchanged in this part; delivered by V1b (#154).
- `tools/parity-oracle/goldens/media_capture_replies.json`: unchanged in this part; delivered by V1b (#154).

## 11. What this does NOT do, as amended 2026-10-02 (#56)

- It moves no rename and no directory call into the atomic writer. Each one writes no file's
  bytes, so A6 as amended does not name it: `crates/vault/src/staged.rs` line 961 (`rename`) and
  line 984 (`create_dir`), and `crates/vault/src/readings_tree.rs` lines 396 and 426
  (`create_dir`), 450 (`rename`), 496 and 501 (`remove_file`) and 596 (`remove_dir`). The journal
  guard over every vault writer is SPEC-118's second pull request, which closes #56.
- It composes the journal guard into no other vault writer. The drill notes that the api, the bot
  and the daemon write go through `RealFs`, whose `journal()` is empty, so none of them refuses a
  journal path, and R5 as amended holds for the inbox capture's writes alone. The journal guard
  over every vault writer is SPEC-118's second pull request, which closes #56.
