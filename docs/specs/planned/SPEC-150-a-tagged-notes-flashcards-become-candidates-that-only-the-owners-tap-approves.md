# SPEC-150: a tagged note's flashcards become candidates that only the owner's tap approves

- **Wave:** W9. **Issue:** #65 (the vault to Anki flashcard bridge, `SB-U19`) (epic #10); the owner's
  questions are #379. **Context(s):** `deck-streak-vault` (the walk over the owner's notes, the
  card markup, the refusals, the note's identity and the card's GUID; the tables
  `vault_card_candidates` and `vault_card_revisions`, their guards and the approved-cards view);
  `deck-streak-ingest` (the plain fronts of the collection's in-scope notes, for the duplicate
  flag); `deck-streak-coordination` (the scan, the pending list and the decision, the one use case
  both surfaces call); `deck-streak-bot` and `deck-streak-api` (the owner's tap: /vaultcards and
  its buttons, and the vault-card routes).
- **Decided by:** ADR-006 (owner-only gating is a safety property), ADR-011 (each vault contract has
  exactly one writer), ADR-113 (a requested duty answers as its command's reply), ADR-116 (the
  journal never leaves the vault), ADR-118 (an erase never deletes a vault file), ADR-150 (a card's
  GUID is derived from its note's identity and its block key, and stored at first sight) and ADR-152
  (both surfaces decide through one use case).
- **Prerequisites:** SPEC-020, SPEC-021, SPEC-022, SPEC-023, SPEC-024, SPEC-025, SPEC-026, SPEC-041,
  SPEC-042, SPEC-110 (planned, W6: the vault's data-rights port and SPEC-042 R12's amendment) and
  SPEC-118 (planned, W6: the layout in force). **Mutation band:** `S15000-S15099`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-150.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** At `dev` a4036b3 the vault crate (`crates/vault/src/`) holds the readings
  tree, the staged-run executor and its layout, and no code that reads a note for a card; ingest
  (`crates/ingest/src/reader.rs`) reads the collection's copy and never its notes' fields. SPEC-001
  Appendix B lists `SB-U19`, the vault to Anki flashcard bridge, as a `build` row of W9, not a port.
- **There is nothing to port.** A search of the predecessor at `27ee2bc` for a flashcard, package or
  GUID writer (`git grep -i -l` over `genanki`, `.apkg`, `flashcard`, `ankiconnect`) finds comments
  and documentation only, no code path that turns a note into a card. So this SPEC has no golden,
  and every constant below is chosen, with its reason beside it.
- **What the owner asked for** (#65): a candidate reaches Anki only after the owner's tap, and
  re-exporting an approved card keeps its GUID. The package that carries the cards is SPEC-151's;
  this SPEC makes the candidates, holds the owner's decisions, and names the approved cards.
- **Traps a first design falls into.**
  - A GUID hashed from the card's text changes when the owner fixes a typo, and Anki then imports the
    fixed card as a new note beside the old one, with its scheduling lost. A GUID from the card's
    position changes when a card is inserted above it. ADR-150 derives it from the note's identity
    and a key the owner wrote, and stores it.
  - The layout's periodic folders may be the vault root (`""`), and `staged.rs::under(path, "")` is
    true for every path. An exclusion built on it would exclude the whole vault.
  - The journal is in the layout the owner provides (SPEC-118 R4), not in the vendored default,
    whose `journal` is empty. A walk that reads the vendored layout reads the journal, which ADR-116
    forbids.
  - The owner's own imported cards are in the collection: a duplicate check over the collection's
    fronts finds every approved card's own note and flags it, unless that note is left out by its GUID.
  - Anki's note field normalisation strips control characters silently, so a card holding one would
    reach Anki changed. It is refused here instead.
  - `notes.sfld` is declared an integer column in Anki's schema, so the plain front is read from the
    first field of `notes.flds`, which is also what Anki's own duplicate check compares.

## 2. Requirements

The setting and the walk

R1. `DECKSTREAK_VAULT_CARD_TAG` (`crates/vault/src/config.rs`, example value `flashcard`, listed in
    `.env.example` unset) names the tag that marks a note for cards: 1 to 64 characters of letters,
    digits, `_`, `-` and `/`, without a leading `#`. A malformed value refuses the start and names the
    setting, never its value (as `VaultSettings::from_env` does). Unset, the scan answers
    `not_configured` (R15) and reads nothing.
R2. The walk starts at `DECKSTREAK_VAULT_ROOT` and reads regular `.md` files. It never reads or
    follows a dot entry, a symbolic link or any other entry that is not a regular file or a folder.
    It enters none of these folders, compared case-insensitively as `staged.rs::under` compares,
    under the layout in force (SPEC-118 R4): each `journal` folder and everything under it (ADR-116);
    the `inbox`; each duty's `writes` and `moves_to` folders (DeckStreak's own notes and the unfiled
    captures are not the owner's cards); the daily and weekly folders, each only when it is not the
    vault root (a root folder excludes nothing); and the readings folder
    (`DECKSTREAK_VAULT_READINGS_FOLDER`) with the archive inside it. A missing vault root answers
    `vault_missing` and changes nothing.
R3. The walk counts every entry it meets and stops at `WALK_CAP`, 50,000 entries (chosen: ten times a
    large personal vault, and a bound on one request's work). A capped walk reports `walk_capped`,
    stores what it read, and marks no card absent (R12), because an unseen note is not a removed one.
    The cap is a parameter, so a test drives it small, and the constant is pinned.
R4. A note over 1 MiB (1,048,576 bytes; chosen: far past any hand-written note, and a bound on one
    read) is reported `note_too_large` and not read; a note that is not UTF-8 is `note_unreadable`.
    Each leaves its cards' presence as it was.
R5. A note is tagged when its frontmatter's `tags` holds the tag, as a block list of `- ` lines, a
    flow list `[a, b]` or one scalar, quoted or not, with or without a leading `#`; or when its body
    holds the inline tag `#<tag>` outside fenced and inline code. The match ignores case, and a
    nested tag (`<tag>/<more>`) counts. The deprecated `tag` property does not count, and neither
    does a longer tag that only begins with the tag's text. The frontmatter is the lines between a
    first line `---` and the next `---`; only `tags` and `id` are read, by a line reader of the
    vault's own (no YAML library is added), and a frontmatter it cannot read is taken as absent.

The card markup, the refusals and the identity

R6. The cards are in each section opened by a level-two heading whose text is `Flashcards` (compared
    case-insensitively) and closed by the next heading of any level; fenced code inside it is
    skipped. A card is a paragraph whose first line begins `Q: ` and which holds a later line
    beginning `A: `: the front is the text after `Q: ` up to that line, and the back runs from after
    `A: ` to the paragraph's end. The paragraph's last line ends with a block id, ` ^<key>`, where
    the key is 1 to 64 of `A-Z`, `a-z`, `0-9` and `-` (Obsidian's block id), and the block id is not
    part of the back. Any other paragraph in the section is the owner's prose and is ignored. CRLF
    reads as LF, and each side is trimmed.
R7. A card's outcome set is exactly {candidate, `answer_missing`, `key_missing`, `key_repeated`,
    `side_empty`, `control_character`, `side_too_long`, `identity_shared`}. `key_repeated` refuses
    every card of a note that shares a key; `control_character` is any Unicode `Cc` character other
    than LF and TAB; `identity_shared` refuses every card of two notes with one identity (R8). A side
    is never cut: a front over 1,000 UTF-16 code units or a back over 2,500 is `side_too_long`
    (chosen: a card with its note's name and its buttons then fits one Telegram message of 4,096,
    `MAX_TEXT_UTF16`, SPEC-026 R6, and a card is meant to be answered at a glance). Refusals are
    reported with the note's file name and the reason's words, and a log line names only the reason
    and a count.
R8. A note's identity is `id:` and its frontmatter `id` when that is 1 to 128 of `A-Z`, `a-z`, `0-9`,
    `.`, `_` and `-`, and `path:` and its vault-relative path, with `/` between folders, otherwise.
R9. A card's GUID is `vc1-` followed by the first 32 lowercase hex characters of the SHA-256 of
    `deckstreak.vault-card.v1`, a 0x00 byte, the identity, a 0x00 byte and the key, computed with
    the vault's own `sha256.rs` (ADR-150). It is computed once, when the candidate is first stored,
    and read from the table ever after; no side of the card enters it.

The candidates and their revisions

R10. `migrations/015001_vault_card_candidates.sql` (`STRICT`, `created_at`, SPEC-020 R18;
    instants in epoch milliseconds, `UtcMillis`, SPEC-020 R6) creates:
    - `vault_card_candidates`: `candidate_id` (primary key), `note_identity`, `card_key`, `guid`
      (unique), `note_path` (where the note was last read), `present` (`CHECK` 0 or 1), and a unique
      (`note_identity`, `card_key`);
    - `vault_card_revisions`: `revision_id` (primary key), `candidate_id` (a foreign key), `origin`
      (`CHECK` `note` or `owner_edit`), `front`, `back`, `text_digest` (the SHA-256 hex of the front,
      a 0x00 byte and the back), `state` (`DEFAULT 'pending'`, `CHECK` one of `pending`, `approved`,
      `rejected`, `edited`, `withdrawn`), `duplicate` (`CHECK` `none`, `found` or `unchecked`),
      `decided_at` and `surface` (`CHECK` `bot` or `mini_app`).
R11. The migration holds every guard, so no query can skip one (a key inside `sqlx::query!` cannot be
    hand-mutated): a partial unique index allows one `pending` revision per candidate, another one
    `approved`; a unique (`candidate_id`, `text_digest`) where the origin is `note` stores each note
    text once; `CHECK`s allow `edited` only for the `note` origin and only `approved` or `withdrawn`
    for `owner_edit`, and hold `decided_at` and `surface` set for `approved`, `rejected` and `edited`
    and absent for `pending`; the trigger `vault_card_revision_transitions` allows only `pending` to
    `approved`, `rejected`, `edited` or `withdrawn`, `approved` to `withdrawn`, and `withdrawn` to
    `pending`, so `rejected` and `edited` are final. The view `vault_approved_cards` is the approved
    revision of each present candidate, with its GUID, sides and `decided_at`: SPEC-151's package
    reads nothing else.
R12. The scan's store step runs in one `Db::write` (SPEC-020 R16), so two scans, or a scan and a
    decision, serialise. For each card read, with its text digest D, the scan looks D up among the
    candidate's `note` revisions only, which the unique index keeps to one row at most:
    - no candidate: the candidate is stored with its GUID, and a `pending` revision with D;
    - a `note` revision with D that is `pending`: nothing changes;
    - one that is `approved`: any other `pending` revision is `withdrawn`;
    - one that is `rejected` or `edited`: any other `pending` revision is `withdrawn` and nothing is
      added, because the owner has decided that text;
    - one that is `withdrawn`: it returns to `pending` (its `decided_at` and `surface` cleared), and
      any other `pending` revision is `withdrawn`;
    - no `note` revision with D, and the candidate's `approved` revision is an `owner_edit` with D:
      any `pending` revision is `withdrawn` and nothing is added, because the note now holds the
      owner's own edit;
    - otherwise: any `pending` revision is `withdrawn` and a `pending` revision with D is added.
    A scan never returns an `owner_edit` revision to `pending` (R11 allows it only `approved` or
    `withdrawn`). An `approved` revision of another text stays approved until the owner decides the
    new one.
    `present` becomes 1 for every card read, and 0 when a note that was read no longer holds the
    card or the tag, or when a walk that was not capped met no note of the candidate's identity. A
    candidate that becomes absent has its `pending` revision `withdrawn`, so no list offers a card
    that no package can hold; read again with that text, it returns to `pending` by the `withdrawn`
    rule above. A note refused by R4 changes no presence.
R13. The duplicate flag of a new `note` revision is `found` when its front, normalised, equals the
    normalised plain front of an in-scope collection note whose GUID is not the candidate's own (the
    owner's imported card is not its own duplicate), `none` when no such note exists, and
    `unchecked` when the fronts could not be read; an `owner_edit` revision is `unchecked`. The one
    normaliser is the vault's: Unicode lowercase, every whitespace run as one space, trimmed. The
    flag is shown to the owner and never blocks a decision.
R14. `ingest::fronts` reads, for each note with a card in SPEC-023 R2's deck scope, its GUID and its
    plain front: the first field of `notes.flds` (fields split at 0x1F), its HTML tags removed,
    `<br>` read as a space and its entities decoded. It opens the copy as SPEC-023 R1 does (the
    kernel's `Db::open_foreign_read_only`, under SPEC-022 R7's shared collection lock, on the kernel's
    `Offload`), selects by deck id and never by a name predicate (SPEC-023 R4), returns only
    DeckStreak's types (SPEC-023 R5), and holds the fronts for one scan only.

The decision and the use case

R15. `coordination::vault_cards` is the one use case both surfaces call: `scan`, `pending` (the pending
    revisions, oldest first, with the approved count) and `decide`. `scan` answers `not_configured`
    when the tag or the vault root is unset, and otherwise reads the fronts (an unreadable copy is
    not an error: the new revisions are `unchecked`), walks, and stores. It returns the report: read,
    tagged, new, pending, approved, withdrawn, absent, the refusals, `walk_capped` and whether
    duplicates were checked.
R16. `decide` takes a revision, a decision and the surface, and runs in one `Db::write`:
    - approve: `pending` to `approved`, and the candidate's prior `approved` revision `withdrawn`;
    - reject: `pending` to `rejected`, and the prior `approved` revision stays;
    - edit, with a new front and back: the sides pass R7's refusals and caps (a failure answers
      `refused` with the reason and changes nothing), the revision goes `pending` to `edited`, an
      `owner_edit` revision is added `approved`, and the prior `approved` revision is `withdrawn`.
    A revision that is not `pending` answers `not_pending`, and one that does not exist `unknown`;
    neither changes anything. Two decisions of one revision at once apply one, and the other answers
    `not_pending` (the conditional update and the trigger, R11). No decision grants XP or coins
    (#379).

The surfaces

R17. `/vaultcards` scans, then replies with the report's counts, up to 10 refusals (the note's file
    name and the reason's words), a line when the walk was capped or duplicates were unchecked, and
    the oldest pending card: its front, its back, its note's file name, the duplicate line when
    `found`, and the buttons Approve, Edit and Reject, whose data are `va:`, `ve:` and `vr:` followed
    by the revision id. With nothing pending it says so. `not_configured` and `vault_missing` each
    reply with their own line and nothing else. The walk runs on the kernel's `Offload`.
R18. Every tap is answered (SPEC-026 R9), decided through R16, and followed by the next oldest
    pending card or the all-decided line; a tap on a revision that is not pending, or that no longer
    exists, is answered with the already-decided line and changes nothing. After Edit the bot sends
    the form (`Q: ` and `A: ` lines with the current sides); the owner's next message that is not a
    command, in that form, is the edit; a command cancels it; a message not in the form, or an edit
    R16 answers `refused`, is refused with the reason's words and the form, and the edit stays
    pending; an edit R16 answers `not_pending` or `unknown` (the revision was decided, withdrawn or
    erased meanwhile) is answered with the already-decided line, ends the edit and changes nothing.
    The bot holds at most one pending edit, in memory. Every reply is HTML-escaped (SPEC-026 R6),
    and /vaultcards is registered for the owner's chat only (SPEC-026 R11).
R19. `GET /api/vault-cards` (`pending`), `POST /api/vault-cards/scan` (`scan`) and `POST
    /api/vault-cards/{revision}/approve`, `/edit` (a JSON body `{"front", "back"}`) and `/reject`
    (`decide`) answer the authenticated owner only (SPEC-024 R7's `OwnerSession`). Each `POST`
    follows SPEC-024 R9. `not_pending` answers 409, `unknown` 404 and `refused` 422 with the reason;
    the scan's `not_configured` and `vault_missing` answer 200 with the outcome by name and change
    nothing; every body is the use case's.
R20. The bot's new replies, their callers and the callback dispatch are named in SPEC-041's census
    (`crates/notifications/tests/one_router.rs`) as command replies. A scan raises no occasion and
    sends nothing the owner did not ask for.

Data rights and the rules

R21. Both tables are user data (SPEC-021's six files): the context map's own-tables rows,
    coordination's data-rights registry and its symmetry test's seeds, the `vault-cards` category in
    `privacy.json` (source `learner-input`, basis `contract`, retention until account deletion,
    exported, erased by deletion) and its line in `PRIVACY.md`, exported and erased by the vault's
    data-rights port (`crates/vault/src/data_rights.rs`, SPEC-110). The notes are the owner's files:
    an erase deletes these rows and never a note (ADR-118). The vault still depends on the kernel
    only.
R22. The vault is read for this feature and never written (ADR-011): no block id, tag or mark is
    written into a note, and a card without a block id is refused `key_missing` for the owner to key.
R23. LEXICON: `candidate in deck-streak-vault, deck-streak-coordination: suggestion, proposal` and
    `decision in deck-streak-vault: review`, with their glossary rows, and the context map's
    "Overloaded words" rows for review (an ingest answer, never the owner's decision on a card) and
    card (an Anki card in `ingest`, a vault card candidate here, always qualified).
R24. CHARTER 10's eleven anti-goals bind this SPEC as one block; the ones it touches are no claim the
    sync cannot honour (no reply says a card is in Anki: an approved card is "in the next package"),
    no unbounded notification volume (a scan answers the request that started it and pushes
    nothing), and no dishonest copy.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | a tagged note's Flashcards sections, each closed by the next heading, yield each keyed Q and A paragraph, the prose and fenced code ignored, CRLF read as LF, the sides trimmed and the block id off the back | `the_flashcards_section_yields_each_keyed_card` |
| A2 | each of the eight outcomes is returned for its case, and a refusal names the note and the reason | `each_card_outcome_is_returned_for_its_case` |
| A3 | a front of 1,000 and a back of 2,500 UTF-16 units are taken, 1,001 and 2,501 are `side_too_long`, and neither is cut | `a_side_over_its_cap_is_refused_never_cut` |
| A4 | a key of 64 characters is a key, and one of 65 is `key_missing` | `a_key_of_65_characters_is_not_a_key` |
| A5 | a block list, a flow list, a scalar, quoted or not, a `#` prefix, an inline tag, a nested tag and any case mark a note; the `tag` property, a tag in code, a longer tag and a frontmatter the reader cannot read do not | `the_card_tag_is_matched_as_obsidian_reads_it` |
| A6 | a valid frontmatter id is the identity, an invalid or absent one gives the path, and two notes with one id refuse all their cards | `a_notes_identity_is_its_id_or_its_path` |
| A7 | the GUID equals its pinned vectors and is `vc1-` and 32 lowercase hex | `the_guid_matches_its_pinned_vectors` |
| A8 | an edit of a card's sides keeps its stored GUID, and a new key or identity gives a new candidate | `an_edit_keeps_the_guid` |
| A9 | the walk reads each regular `.md` note outside the excluded folders, and never a dot entry, a symbolic link, another entry that is not a file or a folder, a journal folder, the inbox, a duty's `writes` or `moves_to` folder, or the readings folder with its archive, in any case | `the_walk_reads_only_the_owners_notes` |
| A10 | a daily or weekly folder at the vault root excludes nothing, and one below it excludes its notes | `a_periodic_folder_at_the_root_excludes_nothing` |
| A11 | a cap of 3 reads a 3-entry vault whole and caps a 4-entry one with `walk_capped` and no card absent, within 5 seconds, and `WALK_CAP` is 50,000 | `a_capped_walk_marks_nothing_absent` |
| A12 | a note of 1,048,576 bytes is read, one byte more is `note_too_large`, a non-UTF-8 note is `note_unreadable`, and each leaves its cards' presence as it was | `an_unreadable_note_leaves_its_cards_as_they_were` |
| A13 | a missing vault root is `vault_missing` and changes nothing | `a_missing_vault_root_changes_nothing` |
| A14 | a new card is stored `pending` with its GUID, and a rescan of the unchanged note changes nothing | `a_new_card_is_pending_and_a_rescan_changes_nothing` |
| A15 | each scan transition of R12 holds, for a text that is pending, approved, rejected, edited, withdrawn and new, the approved owner edit's text, and a withdrawn owner edit's text, which is added as a new `note` revision and never returns the edit to pending | `each_scan_transition_holds` |
| A16 | approve, reject and edit each move a pending revision as R16 says, the prior approved withdrawn by approve and edit and kept by reject | `each_decision_moves_only_a_pending_revision` |
| A17 | a decision of a revision that is not pending is `not_pending`, of an unknown one `unknown`, and an edit with a refused side `refused`; none changes a row | `a_decided_revision_is_not_decided_again` |
| A18 | two handles deciding one revision at once apply one decision, and the other is `not_pending` | `two_decisions_at_once_apply_one` |
| A19 | the migration refuses a second pending, a second approved, a repeated note text, a pending owner edit, a state outside the set and each transition the trigger forbids | `the_store_refuses_what_its_guards_forbid` |
| A20 | the approved-cards view holds the approved revision of each present candidate and nothing pending, rejected, edited, withdrawn or absent | `the_approved_view_holds_only_approved_present_cards` |
| A21 | a card gone from its note, an untagged note and a note gone from a whole walk are absent, their pending revision withdrawn, and leave the view, and a card read again is present with its text pending again | `an_absent_card_leaves_the_view_and_returns` |
| A22 | the duplicate flag is `found` for another note's equal front, `none` otherwise, `unchecked` with no fronts, and a note carrying the candidate's own GUID is no duplicate | `the_duplicate_flag_ignores_the_cards_own_note` |
| A23 | both tables are exported and erased, and an erase deletes no note | `the_vault_card_tables_are_exported_and_erased_and_no_note` |
| A24 | the fronts are the in-scope notes' first fields with their GUIDs, tags removed, `<br>` a space and entities decoded, and an out-of-scope note is not read | `the_fronts_are_the_in_scope_first_fields` |
| A25 | with the tag or the vault root unset the scan is `not_configured` and reads no note | `an_unset_tag_or_root_reads_nothing` |
| A26 | with the copy unreadable the scan stores its cards and every new revision is `unchecked` | `an_unreadable_copy_leaves_the_flag_unchecked` |
| A27 | the bot's and the API's decisions call `coordination::vault_cards::decide`, and nothing else in either crate writes a revision | `both_surfaces_decide_through_one_use_case` |
| A28 | /vaultcards replies with the counts, at most 10 refusals, the capped and unchecked lines when they hold, and the oldest pending card with its note's file name, its duplicate line when `found` and its three buttons, each datum within 64 bytes, or the none line, and the not-configured and vault-missing lines; each tap is answered and shows the next card or the all-decided line | `vaultcards_shows_the_oldest_pending_card` |
| A29 | a tap on a decided revision, or on one that no longer exists, is answered with the already-decided line and changes nothing | `a_stale_tap_is_answered_already_decided` |
| A30 | after Edit the next message in the form is the edit, a command cancels it, a message out of the form or a refused edit is refused with the reason and the form, and an edit of a revision decided or withdrawn meanwhile is answered with the already-decided line and ends the edit | `edit_takes_the_next_message_in_the_form` |
| A31 | the menu registered for the owner's chat holds /vaultcards | `the_menu_is_registered_for_the_owners_chat_only` |
| A32 | the routes answer the owner and refuse any other session, a non-JSON or cross-site `POST` is refused, 409, 404 and 422 answer `not_pending`, `unknown` and `refused`, and the scan's `not_configured` and `vault_missing` answer 200 by name | `the_vault_card_routes_answer_only_the_owner` |
| A33 | the scan's report counts read, tagged, new, pending, approved, withdrawn and absent cards, lists the refusals, and says whether the walk was capped and whether duplicates were checked | `the_scan_report_counts_each_field` |
| A34 | the census names each vault-card reply and its caller as a command reply, and no send in the tree goes around the port | `no_delivery_goes_around_the_port` |

```acceptance
A1: cargo test -p deck-streak-vault --test vault_card_parse -- --exact the_flashcards_section_yields_each_keyed_card
A2: cargo test -p deck-streak-vault --test vault_card_parse -- --exact each_card_outcome_is_returned_for_its_case
A3: cargo test -p deck-streak-vault --test vault_card_parse -- --exact a_side_over_its_cap_is_refused_never_cut
A4: cargo test -p deck-streak-vault --test vault_card_parse -- --exact a_key_of_65_characters_is_not_a_key
A5: cargo test -p deck-streak-vault --test vault_card_parse -- --exact the_card_tag_is_matched_as_obsidian_reads_it
A6: cargo test -p deck-streak-vault --test vault_card_parse -- --exact a_notes_identity_is_its_id_or_its_path
A7: cargo test -p deck-streak-vault --test vault_card_guid -- --exact the_guid_matches_its_pinned_vectors
A8: cargo test -p deck-streak-vault --test vault_card_guid -- --exact an_edit_keeps_the_guid
A9: cargo test -p deck-streak-vault --test vault_card_scan -- --exact the_walk_reads_only_the_owners_notes
A10: cargo test -p deck-streak-vault --test vault_card_scan -- --exact a_periodic_folder_at_the_root_excludes_nothing
A11: cargo test -p deck-streak-vault --test vault_card_scan -- --exact a_capped_walk_marks_nothing_absent
A12: cargo test -p deck-streak-vault --test vault_card_scan -- --exact an_unreadable_note_leaves_its_cards_as_they_were
A13: cargo test -p deck-streak-vault --test vault_card_scan -- --exact a_missing_vault_root_changes_nothing
A14: cargo test -p deck-streak-vault --test vault_card_store -- --exact a_new_card_is_pending_and_a_rescan_changes_nothing
A15: cargo test -p deck-streak-vault --test vault_card_store -- --exact each_scan_transition_holds
A16: cargo test -p deck-streak-vault --test vault_card_store -- --exact each_decision_moves_only_a_pending_revision
A17: cargo test -p deck-streak-vault --test vault_card_store -- --exact a_decided_revision_is_not_decided_again
A18: cargo test -p deck-streak-vault --test vault_card_store -- --exact two_decisions_at_once_apply_one
A19: cargo test -p deck-streak-vault --test vault_card_store -- --exact the_store_refuses_what_its_guards_forbid
A20: cargo test -p deck-streak-vault --test vault_card_store -- --exact the_approved_view_holds_only_approved_present_cards
A21: cargo test -p deck-streak-vault --test vault_card_store -- --exact an_absent_card_leaves_the_view_and_returns
A22: cargo test -p deck-streak-vault --test vault_card_store -- --exact the_duplicate_flag_ignores_the_cards_own_note
A23: cargo test -p deck-streak-vault --test vault_card_store -- --exact the_vault_card_tables_are_exported_and_erased_and_no_note
A24: cargo test -p deck-streak-ingest --test fronts -- --exact the_fronts_are_the_in_scope_first_fields
A25: cargo test -p deck-streak-coordination --test vault_cards -- --exact an_unset_tag_or_root_reads_nothing
A26: cargo test -p deck-streak-coordination --test vault_cards -- --exact an_unreadable_copy_leaves_the_flag_unchecked
A27: cargo test -p deck-streak-coordination --test vault_cards -- --exact both_surfaces_decide_through_one_use_case
A28: cargo test -p deck-streak-bot --test vault_card_commands -- --exact vaultcards_shows_the_oldest_pending_card
A29: cargo test -p deck-streak-bot --test vault_card_commands -- --exact a_stale_tap_is_answered_already_decided
A30: cargo test -p deck-streak-bot --test vault_card_commands -- --exact edit_takes_the_next_message_in_the_form
A31: cargo test -p deck-streak-bot --test commands -- --exact the_menu_is_registered_for_the_owners_chat_only
A32: cargo test -p deck-streak-api --test vault_card_routes -- --exact the_vault_card_routes_answer_only_the_owner
A33: cargo test -p deck-streak-coordination --test vault_cards -- --exact the_scan_report_counts_each_field
A34: cargo test -p deck-streak-notifications --test one_router -- --exact no_delivery_goes_around_the_port
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, telegram-platform and
notifications-policy packs stay enforced; no check is deferred or lifted for this delivery, so the
private wiring does not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/vault/src/data_rights.rs`: the `vault-cards` category names `vault_card_candidates` and `vault_card_revisions` with its purpose, basis and retention, and export and erase cover both | the privacy-gdpr pack |
| B2 | over `crates/bot/src/vault_card_commands.rs` and `crates/bot/src/commands.rs`: every callback datum the vault-card buttons build is 1 to 64 bytes, every callback is answered, and every reply is escaped and within the message length | the telegram-platform pack |
| B3 | over `crates/notifications/tests/one_router.rs` and `crates/bot/src/commands.rs`: every vault-card send is a command reply the census names, and none is a second delivery path | the notifications-policy pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/vault/src/candidates.rs` | `deck-streak-vault` | added: the tag match, the markup, the refusals, the identity, the GUID and the normaliser |
| `crates/vault/src/candidate_walk.rs` | `deck-streak-vault` | added: the walk, its exclusions, its cap and its note caps |
| `crates/vault/src/candidate_store.rs` | `deck-streak-vault` | added: the scan's store step, the decisions and the approved-cards read |
| `crates/vault/src/staged.rs` | `deck-streak-vault` | changed: the layout's folders are readable by the walk |
| `crates/vault/src/config.rs` | `deck-streak-vault` | changed: `DECKSTREAK_VAULT_CARD_TAG` |
| `crates/vault/src/data_rights.rs` | `deck-streak-vault` | changed: the two tables |
| `crates/vault/src/lib.rs` | `deck-streak-vault` | changed: the modules |
| `crates/vault/Cargo.toml` | `deck-streak-vault` | changed: the workspace dependencies the store uses, where SPEC-110 has not added them |
| `crates/vault/tests/vault_card_parse.rs` | `deck-streak-vault` | added: A1 to A6 |
| `crates/vault/tests/vault_card_guid.rs` | `deck-streak-vault` | added: A7, A8 |
| `crates/vault/tests/vault_card_scan.rs` | `deck-streak-vault` | added: A9 to A13 |
| `crates/vault/tests/vault_card_store.rs` | `deck-streak-vault` | added: A14 to A23 |
| `crates/vault/tests/fixtures/vault_cards/` | `deck-streak-vault` | added: synthetic notes |
| `migrations/015001_vault_card_candidates.sql` | `deck-streak-vault` | added: both tables, their guards, the trigger and the view |
| `crates/ingest/src/fronts.rs` | `deck-streak-ingest` | added: the plain fronts |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the module |
| `crates/ingest/tests/fronts.rs` | `deck-streak-ingest` | added: A24 |
| `crates/coordination/src/vault_cards.rs` | `deck-streak-coordination` | added: `scan`, `pending` and `decide` |
| `crates/coordination/src/lib.rs` | `deck-streak-coordination` | changed: the module |
| `crates/coordination/src/data_rights_registry.rs` | `deck-streak-coordination` | changed: the two tables |
| `crates/coordination/tests/data_rights_symmetry.rs` | `deck-streak-coordination` | changed: a seeded row in each table |
| `crates/coordination/tests/vault_cards.rs` | `deck-streak-coordination` | added: A25 to A27 and A33 |
| `crates/bot/src/vault_card_commands.rs` | `deck-streak-bot` | added: the buttons, the edit form's parse and the replies' text |
| `crates/bot/src/commands.rs` | `deck-streak-bot` | changed: gains /vaultcards, the `va:`, `ve:` and `vr:` callbacks and the pending edit; `Commands` gains the coordination use-case handle (the bot depends on kernel, identity, notifications and coordination, not on vault or ingest) |
| `crates/bot/src/lib.rs` | `deck-streak-bot` | changed: the module |
| `crates/bot/tests/vault_card_commands.rs` | `deck-streak-bot` | added: A28 to A30 |
| `crates/bot/tests/commands.rs` | `deck-streak-bot` | changed: A31 holds the menu's new entry |
| `crates/bot/tests/messages/help.msg.json`, `start.msg.json` | `deck-streak-bot` | changed: the command list gains /vaultcards |
| `crates/bot/tests/messages/vault-cards-summary.msg.json`, `vault-cards-card.msg.json`, `vault-cards-none.msg.json`, `vault-cards-not-configured.msg.json`, `vault-cards-vault-missing.msg.json`, `vault-cards-decided.msg.json`, `vault-cards-edit-form.msg.json`, `vault-cards-edit-refused.msg.json` | `deck-streak-bot` | added |
| `crates/notifications/tests/one_router.rs` | `deck-streak-notifications` | changed: the census names the vault-card replies and their callers (A34) |
| `crates/api/src/vault_card_routes.rs` | `deck-streak-api` | added: the five routes |
| `crates/api/src/router.rs` | `deck-streak-api` | changed: the routes are mounted; `ApiState` gains the coordination use-case handle (the api depends on kernel, identity, notifications and coordination, not on vault or ingest) |
| `crates/api/src/lib.rs` | `deck-streak-api` | changed: the module |
| `crates/api/tests/vault_card_routes.rs` | `deck-streak-api` | added: A32 |
| `crates/daemon/src/role_bot.rs` | `deck-streak-daemon` | changed: the bot role builds the coordination use-case handle from the vault and ingest settings and hands it to its commands at start |
| `crates/daemon/src/role_api.rs` | `deck-streak-daemon` | changed: the api role hands the same handle to `ApiState` at start |
| `.env.example` | repo | changed: `DECKSTREAK_VAULT_CARD_TAG`, by name, unset |
| `docs/CONTEXT-MAP.md` | docs | changed: the own-tables rows, the vault's ownership row, and the "Overloaded words" rows for review and card |
| `docs/LEXICON.md` | docs | changed: `candidate` and `decision`, their fence lines and glossary rows |
| `privacy.json` | repo | changed: the `vault-cards` category |
| `PRIVACY.md` | docs | changed: one line for the category |
| `scripts/mutation-rows.d/S15000-S15099.json` | repo | added: the rows of §9 |
| `Cargo.lock`, `.sqlx/` | workspace | changed |
| `docs/specs/SPEC-150-a-tagged-notes-flashcards-become-candidates-that-only-the-owners-tap-approves.md` | docs | moved from `docs/specs/planned/` |
| `docs/schematics/vault-card-candidate-lifecycle.md` | docs | added by the W9 architect turn; this delivery corrects it only where the code proves it wrong |
| `docs/red-first/SPEC-150.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It builds no package and sends no file; SPEC-151 does (#65).
- It builds no Mini App screen; SPEC-152 does (#65).
- It writes nothing into the vault: no block id is added to a keyless card, which is refused for the
  owner to key (#65).
- It drafts no card with a model; model-drafted candidates are not part of W9 (#379).
- It scans on the owner's request only, never on a schedule (#379).
- It grants no XP and no coins for a decision (#379).
- It makes no cloze, image or reversed card: a card is a front and a back (#379).
- It imports no predecessor row; the predecessor had no candidate table (#61).
- It adds no settings-screen entry for the card tag (#57).

## 6. Risks

- **A card reaching Anki without the owner's tap.** Prevented by the `pending` default, the
  transition trigger and the approved-cards view that SPEC-151 reads alone; detected by A14, A19 and
  A20, and rows S15001 to S15005 and S15017.
- **A GUID that drifts** when the owner fixes a typo or inserts a card. Prevented by ADR-150's
  derivation from the identity and the key, stored at first sight; detected by A7 and A8.
- **A rename re-keys a path identity.** Accepted and named by ADR-150: the old candidate goes absent
  and the new one is pending, and a frontmatter `id` avoids it.
- **The journal read into the database.** Prevented by R2's exclusions from the layout in force,
  which is why SPEC-118 is a prerequisite; detected by A9.
- **A lost decision** when the bot and the Mini App decide one card at once. Prevented by the
  conditional update inside `Db::write` and the trigger; detected by A18.
- **Every approved card flagged as a duplicate of itself** once imported. Prevented by R13's own-GUID
  rule; detected by A22.
- **A walk without end** over a very large vault. Prevented by `WALK_CAP`; detected by A11 within its
  5-second bound.

## 7. Parity goldens

None. `SB-U19` is a `build` row, and the predecessor at `27ee2bc` has no code that turns a note into
a card (§1), so there is no function to generate a golden from. The GUID's pinned vectors (A7) and the
bot's message goldens (`crates/bot/tests/messages/vault-cards-*.msg.json`) are DeckStreak's own.

## 8. Tables and the v9 import

| table | owner | created by | from the predecessor's | export and erase |
|---|---|---|---|---|
| `vault_card_candidates` | `deck-streak-vault` | `migrations/015001_vault_card_candidates.sql` | nothing: the predecessor made no card from a note (#61) | exported and erased; the note stays in the owner's vault |
| `vault_card_revisions` | `deck-streak-vault` | `migrations/015001_vault_card_candidates.sql` | nothing (#61) | exported and erased; a card already imported into Anki is the owner's, and no erase reaches it |

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S15001-PENDING-DEFAULT` | `migrations/015001_vault_card_candidates.sql` | a new revision is `pending` (a script-mutation row) | `vault_card_store::a_new_card_is_pending_and_a_rescan_changes_nothing` |
| `S15002-VIEW-APPROVED` | `migrations/015001_vault_card_candidates.sql` | the view reads `approved` only (a script-mutation row) | `vault_card_store::the_approved_view_holds_only_approved_present_cards` |
| `S15003-VIEW-PRESENT` | `migrations/015001_vault_card_candidates.sql` | the view reads present candidates only (a script-mutation row) | `vault_card_store::an_absent_card_leaves_the_view_and_returns` |
| `S15004-ONE-PENDING` | `migrations/015001_vault_card_candidates.sql` | one pending revision per candidate (a script-mutation row) | `vault_card_store::the_store_refuses_what_its_guards_forbid` |
| `S15005-ONE-APPROVED` | `migrations/015001_vault_card_candidates.sql` | one approved revision per candidate (a script-mutation row) | `vault_card_store::the_store_refuses_what_its_guards_forbid` |
| `S15006-TEXT-ONCE` | `migrations/015001_vault_card_candidates.sql` | each note text is stored once per candidate (a script-mutation row) | `vault_card_store::the_store_refuses_what_its_guards_forbid` |
| `S15007-GUID-DOMAIN` | `crates/vault/src/candidates.rs` | the GUID's domain string | `vault_card_guid::the_guid_matches_its_pinned_vectors` |
| `S15008-GUID-32` | `crates/vault/src/candidates.rs` | the GUID's 32 hex characters | `vault_card_guid::the_guid_matches_its_pinned_vectors` |
| `S15009-KEY-64` | `crates/vault/src/candidates.rs` | a key of at most 64 characters; the case list names 64 and 65 | `vault_card_parse::a_key_of_65_characters_is_not_a_key` |
| `S15010-FRONT-1000` | `crates/vault/src/candidates.rs` | the front's cap; the case list names 1,000 and 1,001 | `vault_card_parse::a_side_over_its_cap_is_refused_never_cut` |
| `S15011-BACK-2500` | `crates/vault/src/candidates.rs` | the back's cap; the case list names 2,500 and 2,501 | `vault_card_parse::a_side_over_its_cap_is_refused_never_cut` |
| `S15012-APPROVE-DATUM` | `crates/bot/src/vault_card_commands.rs` | the Approve button's `va:` | `vault_card_commands::vaultcards_shows_the_oldest_pending_card` |
| `S15013-FLASHCARDS-HEADING` | `crates/vault/src/candidates.rs` | the section's heading | `vault_card_parse::the_flashcards_section_yields_each_keyed_card` |
| `S15014-NESTED-TAG` | `crates/vault/src/candidates.rs` | a nested tag counts | `vault_card_parse::the_card_tag_is_matched_as_obsidian_reads_it` |
| `S15015-NOTE-1MIB` | `crates/vault/src/candidate_walk.rs` | the note cap; the case list names 1,048,576 and 1,048,577 bytes | `vault_card_scan::an_unreadable_note_leaves_its_cards_as_they_were` |
| `S15016-WALK-CAP` | `crates/vault/src/candidate_walk.rs` | the walk's 50,000 | `vault_card_scan::a_capped_walk_marks_nothing_absent` |
| `S15017-FINAL-DECISIONS` | `migrations/015001_vault_card_candidates.sql` | the trigger keeps `rejected` and `edited` final (a script-mutation row) | `vault_card_store::the_store_refuses_what_its_guards_forbid` |
| `S15018-OWNER-EDIT-CHECK` | `migrations/015001_vault_card_candidates.sql` | an owner edit is never pending (a script-mutation row) | `vault_card_store::the_store_refuses_what_its_guards_forbid` |
| `S15019-JOURNAL-EXCLUDED` | `crates/vault/src/candidate_walk.rs` | the walk never enters a journal folder | `vault_card_scan::the_walk_reads_only_the_owners_notes` |
| `S15020-ROOT-EXCLUDES-NOTHING` | `crates/vault/src/candidate_walk.rs` | a periodic folder at the root excludes nothing | `vault_card_scan::a_periodic_folder_at_the_root_excludes_nothing` |
| `S15021-OWN-GUID` | `crates/vault/src/candidate_store.rs` | a note with the candidate's own GUID is no duplicate | `vault_card_store::the_duplicate_flag_ignores_the_cards_own_note` |
| `S15022-CONTROL-CHARACTER` | `crates/vault/src/candidates.rs` | a `Cc` character other than LF and TAB is refused | `vault_card_parse::each_card_outcome_is_returned_for_its_case` |
| `S15023-CAPPED-NO-ABSENCE` | `crates/vault/src/candidate_store.rs` | a capped walk marks nothing absent | `vault_card_scan::a_capped_walk_marks_nothing_absent` |
