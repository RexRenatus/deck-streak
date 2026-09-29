# SPEC-151: the package holds only approved cards, keeps each GUID, and reaches only the owner's chat

- **Wave:** W9. **Issue:** #65 (the vault to Anki flashcard bridge, `SB-U19`) (epic #10); the owner's
  questions are #379. **Context(s):** `deck-streak-ingest` (the package builder: a port beside the
  engine port, implemented over Anki's engine with a throwaway in-memory collection, the frozen note
  type and the escaped fields); `deck-streak-coordination` (the package use case, which reads only
  SPEC-150's approved-cards view); `deck-streak-bot` (/vaultpack and the document it sends).
- **Decided by:** ADR-006 (owner-only gating is a safety property), ADR-009 (ingest uses Anki's own
  engine), ADR-058 (the engine's pinned fork), ADR-150 (the stored GUID), ADR-151 (the package is
  built by the engine's own export from a throwaway collection, with a frozen note type and the
  decision's time as each note's modification time) and ADR-152 (the package is a document the bot
  sends to the owner's chat).
- **Prerequisites:** SPEC-020, SPEC-022, SPEC-026, SPEC-041, SPEC-055 and SPEC-150 (planned, W9).
  **Mutation band:** `S15100-S15199`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-151.md` (ADR-016).

## 1. The problem, measured

- **Nothing builds a package yet.** At `dev` a4036b3 ingest reaches Anki's engine through one port,
  `AnkiEngine` (`crates/ingest/src/engine.rs`), for the sync and the new-card queue; nothing exports.
  The bot sends one document today, the data-rights export (`Commands::export`, through
  `Transport::send_document`, which uploads from memory and holds a caption to
  `MAX_CAPTION_UTF16`, 1,024).
- **What the engine offers** (the pinned fork at `57382da`, ADR-058): `Collection::export_apkg`
  writes a package of the notes a search selects, with or without scheduling, deck options and media,
  in the modern or the legacy format, through two temporary files of its own (a copy of the
  selected collection, and the package beside its output path), and renames the package into place;
  the copy is removed when it returns, and it returns the number of notes. Media are gathered only
  when the options ask for them. `CollectionBuilder` with no path gives a collection in
  memory. `add_or_update_notetype_with_existing_id` keeps a note type's id and modification time,
  where `add_notetype` assigns the present time. `add_note` stamps the present time on the note.
- **What the owner's import does with it** (the engine's importer, which the owner's Anki runs): a
  note whose GUID the collection already holds is updated when the package's note is newer, under
  the default condition "if newer"; the package's note keeps its own modification time; a card the
  collection already holds is skipped, so its scheduling stays the owner's; a note type whose schema
  differs is imported beside the old one.
- **Traps a first design falls into.**
  - Built at the present time, every note in a re-exported package is newer than the owner's, so each
    import overwrites any edit the owner made in Anki since. Each note's time is the owner's decision
    instead (ADR-151).
  - A note type added at the present time changes on every build, and an import may then add a
    second note type beside the first. The note type is frozen: a fixed id, time, field set,
    template and style.
  - A field is HTML in Anki: a card whose text holds `<` or `&` would render as markup, and a line
    break would vanish. Every field is escaped.
  - Telegram accepts a bot's document up to 50 MB (`sendDocument`); a larger file fails at upload
    after the work is done. The size is checked before the send.

## 2. Requirements

The use case

R1. `coordination::vault_cards::package` is the one use case that makes a package. It reads SPEC-150's
    view `vault_approved_cards` and nothing else, so no pending, rejected, edited, withdrawn or absent
    revision can reach a package. It answers `not_configured` when `DECKSTREAK_VAULT_CARD_DECK` is
    unset, and `nothing_approved` when the view is empty; neither builds anything. Otherwise it
    re-applies SPEC-150 R7's side checks to each card (a card that fails is left out and counted,
    never cut), hands the rest to the package builder on the kernel's `Offload`, and returns the
    package's bytes with the count of cards in it and the count left out.
R2. `DECKSTREAK_VAULT_CARD_DECK` (`crates/ingest/src/settings.rs`, example value `Vault cards`, listed
    in `.env.example` unset) names the deck a new card goes to: 1 to 100 characters, no control
    character, and no `::` at either end (Anki's `::` separates a deck's levels). A malformed value
    refuses the start and names the setting, never its value.

The builder

R3. `ingest::PackageBuilder` is a port beside `AnkiEngine` in `crates/ingest/src/engine.rs`, so every
    engine type stays inside ingest (ADR-009) and no existing engine double changes; `RslibEngine`
    implements it. For each build it:
    - makes a collection in memory, never the owner's collection or its copy, and opens no file of
      the copy;
    - adds the frozen note type `DeckStreak vault card` with its fixed id and modification time
      (`crates/ingest/src/package.rs`): the fields `Front` and `Back`; one template, `Card 1`, whose
      question is `{{Front}}` and whose answer is `{{FrontSide}}<hr id=answer>{{Back}}`; and a fixed
      style;
    - adds one note per card, in the configured deck, with the card's stored GUID (ADR-150), and each
      field escaped: `&`, then `<`, `>` and `"`, as entities, and each newline as `<br>`;
    - sets each note's modification time to its decision's second (`decided_at` divided by 1,000),
      on the in-memory collection's own note row after the add;
    - exports every note of that collection with `export_apkg` in the modern format (legacy off),
      without scheduling, deck options or media, into a temporary directory it creates, reads the
      file into memory, and removes the directory before it returns.
    A failure is one `EngineError` kind, carrying no card text, path or GUID (SPEC-022 R9).
R4. The import contract: imported into a collection under the default condition "if newer", a
    package gives one note per card with its GUID; a later package holding an edit the owner
    approved updates that note in place, with the same card and its scheduling unchanged; and an edit
    the owner made in Anki after the decision is kept.

The delivery

R5. `/vaultpack` (the owner gate, SPEC-026 R4; registered for the owner's chat only, SPEC-026 R11)
    calls R1 and sends the package as the document `vault-cards.apkg` with `send_document` into the
    chat the command came from, which the gate admits only as the owner's. The caption gives the
    count of cards and the count left out, within `MAX_CAPTION_UTF16`, and says the file is to import,
    never that a card is in Anki. The document is sent without Telegram's `protect_content`, because
    the owner saves the file to import it.
R6. A package over `MAX_DOCUMENT_BYTES`, 50,000,000 bytes (Telegram's bound on a bot's uploaded
    document), is refused `package_too_large` and nothing is sent. `not_configured`,
    `nothing_approved` and a build failure each have their own line, and nothing is sent.
R7. The package is the owner's private file: it exists only in memory and in the one document; no
    route of the API serves it; no job or schedule builds it; and a log line of the use case, the
    builder or the command names only counts and outcome words, never a side, a note's name or path,
    or a GUID.
R8. The builder has one production caller chain: the bot's `Commands::vault_pack`, then
    `coordination::vault_cards::package`, then `PackageBuilder`. SPEC-041's census
    (`crates/notifications/tests/one_router.rs`) names the reply, its caller, and
    (`crates/bot/src/commands.rs`, `Commands::vault_pack`, `send_document`) among its named sends.

The rules

R9. CHARTER 4 holds: the package is a file the owner chooses to import. DeckStreak opens neither the
    owner's collection nor its copy for writing, uploads nothing through the sync, and writes no
    Anki collection other than the in-memory one it builds the file from (ADR-151).
R10. LEXICON: `package in deck-streak-ingest, deck-streak-coordination, deck-streak-bot: bundle`, with
    its glossary row, and the context map's "Overloaded words" row for package (the owner's Anki file
    of approved vault cards, never the data-rights export). No table is added.
R11. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no claim the
    sync cannot honour (the caption says the file is to import, never that a card is in Anki) and no
    dishonest copy.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a package imported into a fresh collection gives one note per card with its stored GUID, the frozen note type and the configured deck | `the_package_imports_as_one_note_per_card_with_its_guid` |
| A2 | a field holding `&`, `<`, `>`, `"` and a newline arrives escaped, with `<br>` for the newline, and an HTML tag arrives as text | `each_field_is_escaped_for_html` |
| A3 | a later package holding an approved edit updates the note in place: one note, the same GUID, the same card, its scheduling unchanged | `a_later_package_updates_the_note_in_place` |
| A4 | an edit made in the collection after the decision is kept when the package is imported again | `an_edit_made_in_anki_after_the_decision_survives` |
| A5 | each note's modification time is its decision's second, and the note type's id, time, fields, template and style are the frozen ones | `the_times_are_the_decisions_and_the_note_type_is_frozen` |
| A6 | the package is the modern format with no scheduling, deck options or media, and the builder leaves no file behind | `the_package_is_modern_and_leaves_no_file` |
| A7 | with the deck unset the use case is `not_configured` and the builder is not called | `an_unset_deck_builds_nothing` |
| A8 | with no approved card the use case is `nothing_approved` and the builder is not called | `no_approved_card_builds_nothing` |
| A9 | the builder receives exactly the view's cards: a pending, rejected, edited, withdrawn and absent revision beside them never reaches it | `the_package_holds_only_the_approved_view` |
| A10 | a stored card that fails the side checks is left out and counted, and the rest are built | `a_card_failing_the_checks_is_left_out_and_counted` |
| A11 | the use case's log lines hold counts and outcome words, and no side, note name, path or GUID | `the_package_logs_counts_only` |
| A12 | `coordination::vault_cards::package` is called only by the bot's `Commands::vault_pack`, and `PackageBuilder` only by the use case | `the_package_has_one_caller_chain` |
| A13 | /vaultpack sends `vault-cards.apkg` into the command's chat with a caption of the card count and the count left out within `MAX_CAPTION_UTF16`, and replies with its own line for `not_configured`, `nothing_approved` and a failed build, sending nothing | `vaultpack_sends_the_package_to_the_owners_chat` |
| A14 | a package of exactly the limit is sent, one byte more is `package_too_large` with nothing sent, and `MAX_DOCUMENT_BYTES` is 50,000,000 | `a_package_over_the_limit_is_not_sent` |
| A15 | the menu registered for the owner's chat holds /vaultpack | `the_menu_is_registered_for_the_owners_chat_only` |
| A16 | /vaultpack from anyone but the owner is dropped: nothing is built and nothing is sent | `a_vaultpack_from_another_user_sends_nothing` |
| A17 | the census names the package's reply, its caller and its document, and no send in the tree goes around the port | `no_delivery_goes_around_the_port` |

```acceptance
A1: cargo test -p deck-streak-ingest --test package -- --exact the_package_imports_as_one_note_per_card_with_its_guid
A2: cargo test -p deck-streak-ingest --test package -- --exact each_field_is_escaped_for_html
A3: cargo test -p deck-streak-ingest --test package -- --exact a_later_package_updates_the_note_in_place
A4: cargo test -p deck-streak-ingest --test package -- --exact an_edit_made_in_anki_after_the_decision_survives
A5: cargo test -p deck-streak-ingest --test package -- --exact the_times_are_the_decisions_and_the_note_type_is_frozen
A6: cargo test -p deck-streak-ingest --test package -- --exact the_package_is_modern_and_leaves_no_file
A7: cargo test -p deck-streak-coordination --test vault_package -- --exact an_unset_deck_builds_nothing
A8: cargo test -p deck-streak-coordination --test vault_package -- --exact no_approved_card_builds_nothing
A9: cargo test -p deck-streak-coordination --test vault_package -- --exact the_package_holds_only_the_approved_view
A10: cargo test -p deck-streak-coordination --test vault_package -- --exact a_card_failing_the_checks_is_left_out_and_counted
A11: cargo test -p deck-streak-coordination --test vault_package -- --exact the_package_logs_counts_only
A12: cargo test -p deck-streak-coordination --test vault_package -- --exact the_package_has_one_caller_chain
A13: cargo test -p deck-streak-bot --test vault_pack_commands -- --exact vaultpack_sends_the_package_to_the_owners_chat
A14: cargo test -p deck-streak-bot --test vault_pack_commands -- --exact a_package_over_the_limit_is_not_sent
A15: cargo test -p deck-streak-bot --test commands -- --exact the_menu_is_registered_for_the_owners_chat_only
A16: cargo test -p deck-streak-bot --test vault_pack_commands -- --exact a_vaultpack_from_another_user_sends_nothing
A17: cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The telegram-platform and privacy-gdpr packs stay
enforced; no check is deferred or lifted for this delivery, so the private wiring does not change
when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `crates/bot/src/commands.rs` and `crates/bot/src/vault_card_commands.rs`: the package's caption stays within 1,024 characters, and every /vaultpack reply is escaped and within the message length | the telegram-platform pack |
| B2 | over `crates/ingest/src/package.rs`, `crates/ingest/src/engine.rs`, `crates/coordination/src/vault_cards.rs` and `crates/bot/src/commands.rs`: no log line carries a card's side, a note's path or a GUID | the privacy-gdpr pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/package.rs` | `deck-streak-ingest` | added: the package's types, the frozen note type and the field escaping |
| `crates/ingest/src/engine.rs` | `deck-streak-ingest` | changed: the `PackageBuilder` port and `RslibEngine`'s implementation |
| `crates/ingest/src/settings.rs` | `deck-streak-ingest` | changed: `DECKSTREAK_VAULT_CARD_DECK` |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module and the port |
| `crates/ingest/Cargo.toml` | `deck-streak-ingest` | changed: `tempfile` moves from the dev-dependencies to the dependencies, for the builder's temporary directory |
| `crates/ingest/tests/package.rs` | `deck-streak-ingest` | added: A1 to A6 |
| `crates/coordination/src/vault_cards.rs` | `deck-streak-coordination` | changed: `package` |
| `crates/coordination/Cargo.toml` | `deck-streak-coordination` | changed: `tracing-subscriber` as a dev-dependency, for A11's log capture |
| `crates/coordination/tests/vault_package.rs` | `deck-streak-coordination` | added: A7 to A12 |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: gains /vaultpack; `Commands` gains the package use case |
| `crates/bot/src/vault_card_commands.rs` | `deck-streak-bot` | changed: the package's replies, its caption and `MAX_DOCUMENT_BYTES` |
| `crates/bot/tests/vault_pack_commands.rs` | `deck-streak-bot` | added: A13, A14, A16 |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: A15 holds the menu's new entry |
| `crates/bot/tests/messages/help.msg.json`, `start.msg.json` | `deck-streak-bot` | changed: the command list gains /vaultpack |
| `crates/bot/tests/messages/vault-pack-caption.msg.json`, `vault-pack-none.msg.json`, `vault-pack-not-configured.msg.json`, `vault-pack-too-large.msg.json`, `vault-pack-failed.msg.json` | `deck-streak-bot` | added |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names the reply, its caller and its document (A17) |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role builds the package use case over the engine and hands it to its commands at start |
| `.env.example` | repo | changed: `DECKSTREAK_VAULT_CARD_DECK`, by name, unset |
| `docs/CONTEXT-MAP.md` | docs | changed: the "Overloaded words" row for package |
| `docs/LEXICON.md` | docs | changed: `package`, its fence line and glossary row |
| `scripts/mutation-rows.d/S15100-S15199.json` | repo | added: the rows of §9 |
| `Cargo.lock` | workspace | changed |
| `docs/specs/SPEC-151-the-package-holds-only-approved-cards-keeps-each-guid-and-reaches-only-the-owners-chat.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/vault-card-package-delivery.md` | docs | added by the W9 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-151.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It makes, stores and decides no candidate; SPEC-150 does (#65).
- It builds no package on a schedule and pushes none: the owner asks with /vaultpack (#379).
- It never writes the owner's collection or its copy, and never delivers a card through a sync;
  the owner imports the file (#65).
- It carries no media, scheduling or deck options in the package (#65).
- It offers no download from the Mini App; the document comes from the bot (#65).
- It keeps no record of which packages were sent, so it adds no table (#65).

## 6. Risks

- **An unapproved card in a package.** Prevented by R1's single read of SPEC-150's view, whose own
  guards keep it to approved, present cards; detected by A9 and rows S15101 and S15102.
- **A re-import that duplicates or re-schedules a card.** Prevented by the stored GUID and the
  engine's importer, which skips a card the collection holds; detected by A1 and A3.
- **A re-import that overwrites the owner's edit in Anki.** Prevented by the decision's time as the
  note's time; detected by A4 and row S15105.
- **A second note type after an upgrade.** Prevented by the frozen note type; detected by A5. A
  change to the note type is a new ADR and a new id, never an edit of the frozen one.
- **The package reaching anyone but the owner.** Prevented by the owner gate and by sending into the
  command's chat only; detected by A13 and A16.
- **A temporary file left behind.** Prevented by the builder removing its directory before it
  returns; detected by A6.

## 7. Parity goldens

None. The predecessor built no package (SPEC-150 §1). The frozen note type's constants and the bot's
message goldens (`crates/bot/tests/messages/vault-pack-*.msg.json`) are DeckStreak's own.

## 8. Tables and the v9 import

None: this SPEC adds no table. The package is built from SPEC-150's tables and kept nowhere.

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S15101-VIEW-ONLY` | `crates/coordination/src/vault_cards.rs` | the package reads the approved-cards view | `vault_package::the_package_holds_only_the_approved_view` |
| `S15102-NOTHING-APPROVED` | `crates/coordination/src/vault_cards.rs` | an empty view builds nothing | `vault_package::no_approved_card_builds_nothing` |
| `S15103-DECK-UNSET` | `crates/coordination/src/vault_cards.rs` | an unset deck builds nothing | `vault_package::an_unset_deck_builds_nothing` |
| `S15104-RECHECK` | `crates/coordination/src/vault_cards.rs` | a card failing the side checks is left out | `vault_package::a_card_failing_the_checks_is_left_out_and_counted` |
| `S15105-DECISION-TIME` | `crates/ingest/src/engine.rs` | a note's time is its decision's second | `package::the_times_are_the_decisions_and_the_note_type_is_frozen` |
| `S15106-NOTETYPE-ID` | `crates/ingest/src/package.rs` | the frozen note type's id | `package::the_times_are_the_decisions_and_the_note_type_is_frozen` |
| `S15107-ESCAPE-AMPERSAND` | `crates/ingest/src/package.rs` | `&` is escaped first | `package::each_field_is_escaped_for_html` |
| `S15108-ESCAPE-LESS-THAN` | `crates/ingest/src/package.rs` | `<` is escaped | `package::each_field_is_escaped_for_html` |
| `S15109-NEWLINE-BREAK` | `crates/ingest/src/package.rs` | a newline is `<br>` | `package::each_field_is_escaped_for_html` |
| `S15110-GUID-KEPT` | `crates/ingest/src/engine.rs` | each note carries its stored GUID | `package::the_package_imports_as_one_note_per_card_with_its_guid` |
| `S15111-MODERN-FORMAT` | `crates/ingest/src/engine.rs` | the legacy format is off | `package::the_package_is_modern_and_leaves_no_file` |
| `S15112-NO-SCHEDULING` | `crates/ingest/src/engine.rs` | no scheduling is exported | `package::the_package_is_modern_and_leaves_no_file` |
| `S15113-DOCUMENT-LIMIT` | `crates/bot/src/vault_card_commands.rs` | the 50,000,000-byte limit; the case list names the limit and one byte more | `vault_pack_commands::a_package_over_the_limit_is_not_sent` |
| `S15114-FILE-NAME` | `crates/bot/src/vault_card_commands.rs` | the document's name | `vault_pack_commands::vaultpack_sends_the_package_to_the_owners_chat` |
