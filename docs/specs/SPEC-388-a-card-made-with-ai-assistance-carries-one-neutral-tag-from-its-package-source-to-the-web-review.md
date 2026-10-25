# SPEC-388: a card made with AI assistance carries one neutral tag from its package source to the web review

- **Issue:** `#840`. A card made with AI assistance carries one neutral marker, recorded where its
  package is made. The web review says so on the card, and the course screen says once how such
  cards are marked. Neither text names a model or a maker.
- **Context(s):** `engine-core` (`crates/engine-core`), `web-engine` (`crates/web-engine`),
  `miniapp` (`web/app/src/lib/engine`, `web/app/src/lib/study`, `web/app/src/routes`,
  `web/app/messages`).
- **Decided by:** ADR-402 (this SPEC's own). It works under ADR-151 (the package is the engine's
  own export of a throwaway collection with a frozen note type), ADR-301 (a packaged note changes
  only at its package's source) and ADR-356 D4 (the engine core depends on the engine and on no
  crate of this workspace), and it keeps all three.
- **Schematic:** `docs/schematics/vault-card-package-delivery.md`, amended by one section appended
  at its end (the marker from the package source to each client's card view). No existing line of
  it changes.
- **Status:** one delivery. **Mutation band:** `S38800-S38899` (section 9). **Model:** none
  (section 8).

## 1. The problem, measured

Read at `3ca06142`. Each count is the output of the `git grep` named beside it, at that commit.

### 1a. No card says how it was made

- `git grep -l -i -E 'ai-assisted|ai_assisted|aiAssisted'` lists 0 files: no tag, field, string or
  test names the marker.
- No card is drafted with a model today. The vault-card candidate SPEC drafts no card with a model,
  and model-drafted candidates are their own decision (#379). The AI-safety manifest
  (`ai-safety.json`) declares two tasks, both daily readings, and no task that makes a card; its
  disclosure is the coach's line on the bot's first reply (`crates/bot/src/commands.rs:302-307`).
- So the marker is wanted before a card made with AI assistance ships, and nothing has to be
  marked after the fact.

### 1b. Where a card's source lives, and what rides with it

- The package is the engine's own export of a throwaway in-memory collection with the frozen note
  type `DeckStreak vault card` (Front, Back); a change to that type is a new type and a new ADR
  (ADR-151). A packaged note is changed at its package's source, never in the collection
  (ADR-301). The package builder is decided and not yet built: `git grep -c -E
  'export_apkg|import_apkg'` finds the names in 3 documents and in 0 source files.
- A note's tags are the note's own field beside its note type's fields: one text of tags separated
  by blanks (`notes.tags`), which the engine exports, imports, stores and syncs with the note.
  Ingest already reads them in its card read and reduces them to a tier there, comparing each whole
  tag without regard to case (`crates/ingest/src/reader.rs:41-44`, `:308`;
  `crates/ingest/src/tier.rs`, `parse_tier`). The tags themselves never leave ingest
  (`reader.rs:126-128`).

### 1c. What reaches each client's card view today

- The web engine builds the shown card's view in `current_card`
  (`crates/web-engine/src/wasm.rs:634-700`): the counts, then the card's id, ordinal, flag, both
  sides, CSS, labels, undo and `late`. Nothing in it reads the card's note.
- The engine core reads the open collection only through fixed statements it holds
  (`crates/engine-core/src/dispatch.rs:139-148`, `Read`; `:436`, `query`), and a rule both clients
  share lives in the core (`late::past_due_day`, called at `wasm.rs:699`).
- The page's `CardView` (`web/app/src/lib/engine/protocol.ts:123-133`) has nine fields, each one
  word: `git grep` finds 0 multi-word keys in `wasm.rs`'s JSON and 0 multi-word fields in
  `protocol.ts`'s interfaces.
- The review screen draws the late line above the card
  (`web/app/src/lib/study/ReviewScreen.svelte:192-194`), then the card frame (`:195-202`).
- The course screen is the progress page. Its ok section lists each course's ladder, or says there
  is none yet (`web/app/src/routes/progress/+page.svelte:27-34`). Its strings live in the 7 message
  files under `web/app/messages`, which hold 0 AI-assisted strings.
- The native review exists, and the native app holds no localized strings (#738).

## 2. Requirements

R1. The marker is the note tag `ai-assisted`, written in the note's tags at the package source. The
engine core names it once, as `ai_assisted::MARKER`.

R2. A note is marked when its tags hold `ai-assisted` as a whole tag, compared without regard to
ASCII case, as the engine compares tags. A longer tag (`pre-ai-assisted`, `ai-assisted-review`), a
child tag (`ai-assisted::draft`) or a part of the tag does not mark it, and a note with no tags is
unmarked. The rule is one pure function in the engine core, `ai_assisted::carries`.

R3. The engine core reads a note's tags through one fixed statement it holds (`Read::NoteTags`) and
answers through `Dispatcher::ai_assisted`, judged by `carries`. A note the collection lacks reads
unmarked. No adapter passes SQL for it or judges the tags itself.

R4. The marker rides unchanged through the engine's own package export and import, through the
update of a note by a newer package in both directions, through the collection file, and through
full and normal sync. No DeckStreak code carries it on any of those paths; the tests of section 3
prove the engine does.

R5. The web engine's card view carries `assisted`, the core's answer for the shown card's note,
built in one place (`current_card`). The page's `CardView` declares `assisted: boolean`.

R6. When `assisted` is true, the web review shows one line above the card, on the question side and
on the answer side, below the late line when both show. In English it reads `This card was made
with AI assistance.` It is plain text in the late line's style: no icon, colour, link, motion or
live region.

R7. A card whose note is unmarked shows no line and claims nothing about how it was made.

R8. The progress screen shows one sentence once, below the course list, when at least one course is
listed, and none when there is no course. In English it reads `Cards made with AI assistance carry
the tag ai-assisted.`

R9. Both texts are in all 7 locales, under `study_ai_assisted` and `progress_ai_assisted`. Each
locale's course sentence keeps the literal tag `ai-assisted`, and no locale but `en` holds the
English text.

R10. Neither text names a maker or a model, in any locale: it holds no digit, and no Latin word that
begins with a capital letter except the locale's word for AI (`AI` or `IA`) or the text's first
word.

R11. Nothing else changes: no server code, store, table, migration, bot text, persona, privacy
inventory or AI-safety manifest. Ingest still reduces tags to a tier inside its read and keeps no
tag; its test `the_reader_keeps_each_cards_tier_and_never_its_tags` stays as it is and green.

## 3. Acceptance criteria of the ai-assisted marker

| # | criterion | red at the base, for this reason | test |
|---|---|---|---|
| A1 | `MARKER` is the literal `ai-assisted`, and a note whose only tag is `ai-assisted` is marked | the stub answers false for every tag set | `crates/engine-core/tests/ai_assisted.rs` `the_marker_is_the_tag_ai_assisted` (added) |
| A2 | A note whose tags hold `ai-assisted` among other tags, at the start, middle or end, is marked | the stub answers false for every tag set | `ai_assisted.rs` `a_note_among_other_tags_is_marked` (added) |
| A3 | `AI-Assisted` and `AI-ASSISTED` mark a note | the stub answers false for every tag set | `ai_assisted.rs` `the_marker_is_read_in_any_case` (added) |
| A4 | No tags, `pre-ai-assisted`, `ai-assisted-review`, `ai-assisted::draft`, `ai` and `assisted` leave a note unmarked | not red: the stub answers false; it guards the whole-tag rule and the default | `ai_assisted.rs` `only_the_whole_tag_marks_a_note` (added) |
| A5 | On a collection the engine built and wrote to its file, the core's answer for a marked note and for a note tagged only `vocab` equals the engine's own reading of each note's tags | the stub answers false for the marked note | `ai_assisted.rs` `the_core_reads_the_engines_own_tags` (added) |
| A6 | A note id the collection lacks reads unmarked, with no refusal | not red: the stub answers false; it guards the no-row arm | `ai_assisted.rs` `a_note_the_collection_lacks_reads_unmarked` (added) |
| A7 | A marked note and an unmarked note, exported by the engine's own package export and imported into a fresh collection, keep their tags, and the core reads them marked and unmarked | the stub answers false for the imported marked note | `ai_assisted.rs` `the_marker_survives_the_engines_package_export_and_import` (added) |
| A8 | A newer package of the same note adds the marker to an imported unmarked note, and a still newer one without it takes the marker away | the stub answers false after the marking package | `ai_assisted.rs` `a_newer_package_moves_the_marker_both_ways` (added) |
| A9 | A marked note reaches a second device by full upload and full download, and a marker added on the first device afterwards reaches the second by normal sync | the stub answers false on the second device | `ai_assisted.rs` `the_marker_survives_full_and_normal_sync` (added) |
| A10 | The web engine's card view carries `assisted` from the core's read of the shown card's note, in one place; each planted view is refused by name | the census finds no `assisted` key in `current_card` | `crates/web-engine/tests/ai_assisted_view.rs` `the_shown_card_carries_the_cores_marker_answer` (added) |
| A11 | A marked card shows the English line above the card, on the question side and on the answer side | no line is drawn | `web/app/src/lib/study/ai-assisted-line.test.ts` "a marked card shows the line above the card" (added) |
| A12 | An unmarked card shows its question and no line, and the line goes when an unmarked card follows a marked one | not red: the base draws no line; it guards the absence beside a drawn card | `ai-assisted-line.test.ts` "an unmarked card shows no line" (added) |
| A13 | Every locale holds both texts; the English texts are exactly R6's and R8's; each course sentence holds `ai-assisted`; no locale but `en` holds the English text; each plant is refused by name | all 7 locales lack both keys | `ai-assisted-line.test.ts` "every locale holds both AI-assisted texts" (added) |
| A14 | No text of either key, in any locale, holds a digit or a capital word beyond R10's; a planted maker name and a planted digit are refused by name | all 7 locales lack both keys, each reported by name | `ai-assisted-line.test.ts` "the AI-assisted texts name no maker" (added) |
| A15 | With courses listed, the progress screen shows the English sentence exactly once, below the course list | no sentence is drawn | `web/app/src/routes/progress.test.ts` "states once how AI-assisted cards are marked, under the courses" (added) |
| A16 | With no course, the progress screen shows no such sentence | not red: the base draws none; it guards the absence beside the no-course line | `progress.test.ts` "says nothing about AI-assisted cards when there is no course" (added) |

```acceptance
A1: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact the_marker_is_the_tag_ai_assisted
A2: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact a_note_among_other_tags_is_marked
A3: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact the_marker_is_read_in_any_case
A4: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact only_the_whole_tag_marks_a_note
A5: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact the_core_reads_the_engines_own_tags
A6: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact a_note_the_collection_lacks_reads_unmarked
A7: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact the_marker_survives_the_engines_package_export_and_import
A8: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact a_newer_package_moves_the_marker_both_ways
A9: cargo test -p deck-streak-engine-core --test ai_assisted -- --exact the_marker_survives_full_and_normal_sync
A10: cargo test -p deck-streak-web-engine --test ai_assisted_view -- --exact the_shown_card_carries_the_cores_marker_answer
A11: pnpm exec vitest run web/app/src/lib/study/ai-assisted-line.test.ts -t "a marked card shows the line above the card"
A12: pnpm exec vitest run web/app/src/lib/study/ai-assisted-line.test.ts -t "an unmarked card shows no line"
A13: pnpm exec vitest run web/app/src/lib/study/ai-assisted-line.test.ts -t "every locale holds both AI-assisted texts"
A14: pnpm exec vitest run web/app/src/lib/study/ai-assisted-line.test.ts -t "the AI-assisted texts name no maker"
A15: pnpm exec vitest run web/app/src/routes/progress.test.ts -t "states once how AI-assisted cards are marked, under the courses"
A16: pnpm exec vitest run web/app/src/routes/progress.test.ts -t "says nothing about AI-assisted cards when there is no course"
```

Each tag set of A1 to A4 is written literally in the test; no expected value is computed by
`carries`, and `MARKER` is compared with the literal `ai-assisted`. The oracle of A5 to A9 is the
engine's own note, read through the engine's own interface on the same collection file, never
through the core; A7 to A9 assert that oracle before the core's answer, so each is red at the base
on the core's answer alone. A8 writes each package's note time as a fixed literal, never from the
clock, as the package builder writes the decision's time (ADR-151). The censuses of A10, A13 and
A14 judge before they count, report what they examined, refuse zero, and carry planted refusals.

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-388-a-card-made-with-ai-assistance-carries-one-neutral-tag-from-its-package-source-to-the-web-review.md` | docs | added |
| `docs/decisions/ADR-402-deckstreak-is-a-personal-study-tool-and-a-card-made-with-ai-assistance-carries-the-ai-assisted-tag-at-its-package-source.md` | docs | added |
| `docs/schematics/vault-card-package-delivery.md` | docs | one section appended at its end; no existing line changes |
| `docs/schematics/the-app-campaigns-surfaces-each-carry-a-stride-table-whose-every-control-cites-a-line-that-holds.md` | docs | three cited line numbers re-pointed to the lines this delivery moved (`lib.rs` 49 to 51, `dispatch.rs` 153 to 156, `protocol.ts` 295 to 297); no row or control text changes |
| `docs/red-first/SPEC-388.md` | docs | added |
| `changelog.d/ai-assisted-marker-388.md` | docs | added |
| `scripts/mutation-rows.d/S38800-S38899.json` | docs | added (section 9) |
| `crates/engine-core/src/ai_assisted.rs` | `engine-core` | added: `MARKER` and `carries` (R1, R2) |
| `crates/engine-core/src/lib.rs` | `engine-core` | `pub mod ai_assisted;` and its line in the module list |
| `crates/engine-core/src/dispatch.rs` | `engine-core` | the fixed read of a note's tags and `Dispatcher::ai_assisted` (R3) |
| `crates/engine-core/tests/ai_assisted.rs` | `engine-core` | added: A1 to A9 |
| `crates/web-engine/src/wasm.rs` | `web-engine` | `current_card`'s view carries `assisted` (R5) |
| `crates/web-engine/tests/ai_assisted_view.rs` | `web-engine` | added: A10 |
| `web/app/src/lib/engine/protocol.ts` | `miniapp` | `CardView.assisted` (R5) |
| `web/app/src/lib/study/ReviewScreen.svelte` | `miniapp` | the line (R6, R7) |
| `web/app/src/routes/progress/+page.svelte` | `miniapp` | the course sentence (R8) |
| `web/app/messages/en.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/es.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/fr.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/ja.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/ko.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/zh-Hans.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/messages/zh-Hant.json` | `miniapp` | `study_ai_assisted`, `progress_ai_assisted` (R9) |
| `web/app/src/lib/study/ai-assisted-line.test.ts` | `miniapp` | added: A11 to A14 |
| `web/app/src/routes/progress.test.ts` | `miniapp` | added: A15, A16; no existing test changes |
| `web/app/src/lib/study/audio.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/late-line.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-screen.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review-templates.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/review.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/voice.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/routes/study.test.ts` | `miniapp` | `assisted: false` in its card view fixtures; no assertion changes |
| `web/app/src/lib/study/update-now.test.ts` | `miniapp` | `assisted: false` in its card view helper, a file the type check names beyond the seven; no assertion changes |
| `formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla` | `formal` | the `current_card` cover's `digest=` moved and one abstraction comment line; no action, property or witness changes |

The seven fixture files are those whose card view literals carry `late` at the base
(`git grep -c -E '\blate: (false|true)'` under `web/app`). A file the type check names beyond them
joins this table in the same commit and is named in the pull request.

## 5. What this does NOT cover

- It builds no package and changes no package code. The package builder is the vault-card
  package's own work, decided and not yet built; when it is built, it writes the tag on every note
  whose card was made with AI assistance, as ADR-402 D2 decides (#65).
- It adds no AI duty that makes a card, and no task to the AI-safety manifest. No duty makes a card
  today; one that does is declared there with its own gate, the output mark among it (#379).
- It adds no line to the native review. The native app holds no localized strings, so a line in
  every locale there needs its own string catalog first; the core's read is the one the native
  review will call (#738).
- It does not restore a tag the owner removes or renames in their own collection. DeckStreak writes
  no card into the collection, so such a card reads unmarked until a newer package of it is
  imported (#65).

## 6. Risks

- **The engine merges tags when a newer package updates a note.** Then a marker could never be taken
  back at the source. Detected by A8; the delivery stops there and reports, rather than writing
  around the engine.
- **A sibling change to the shared files.** Open pull requests touch `dispatch.rs`, `lib.rs`,
  `wasm.rs`, `protocol.ts` and the message files. The builder re-measures each shared path at its
  cut, and a changed fact this SPEC cites is a stop, not a guess.
- **The owner edits the tag in their own collection.** The card then reads as the collection says,
  which is the owner's act (section 5).
- **A translation names a maker in lowercase.** A14 reads capital words and digits, so a lowercase
  name passes it. The English texts are pinned whole (A13), and each translation is read in review.
- **A read of the note fails.** The view is refused, as a failed read of the engine's day already
  refuses it; both fail only when the engine itself fails.

## 7. What only CI or a device proves

| # | what | where |
|---|---|---|
| C1 | The type check passes with `assisted` required on every card view | the pull request's `web` job (svelte-check) |
| C2 | The web engine builds for its own target with the view's new key | the pull request's `web-engine` job |
| C3 | Every changed production file under `web/app` has its mutants killed or recorded equivalent | `mutation-web`, on the pull request |
| C4 | The diff's Rust mutants are caught or recorded equivalent, and the band's rows are killed by their named killers | `mutation-rust` and `mutation-rows`, by stem |
| V1 | On a device, a card whose note carries `ai-assisted` shows the line in the app's locale, a card without it shows none, and the progress screen shows the sentence once under the courses | the owner, in an acceptance session |

## 8. Formal model

None, by surface (ADR-402 D4). No actor, store or write path is added. The core reads one note's
tags inside the one synchronous export that builds the card view, in the one Worker, and the tags
change only by the owner's import or a sync, each of which builds the view again. `carries` is a
total function over a text, tested on every arm. Of the 251 `@phx covers` lines at the base, 0 name
a web path or `ai_assisted.rs`, 6 name `crates/engine-core/src/dispatch.rs` (`handshake`, `run`,
`full_sync`, `private`, `run_undo`, `run_restore`) and 6 name `crates/web-engine/src/wasm.rs`
(`undo`, `undo_offer`, `rate`, `bury`, `flag`, `current_card`). The delivery edits one covered
item, `wasm.rs`'s `current_card` (`formal/tla/UndoOwnAnswer/UndoOwnAnswer.tla`): its read of the
shown card's note tags reaches none of the model's variables, so its cover is re-stamped and the
read is named among the model's abstractions; the other eleven items stay byte for byte as they are.

## 9. Mutation rows

The band `S38800-S38899`, in `scripts/mutation-rows.d/S38800-S38899.json`, table `MUTATIONS`, nine
rows from `S38800`:

- `S38800` the marker's spelling: a constant's literal, which no generated mutant changes; killed by
  A1.
- `S38801` the case rule: an exact comparison in place of the case-free one; killed by A3.
- `S38802` the whole-tag rule: a substring match over the tags; killed by A4.
- `S38803` the any-tag rule: every tag in place of any tag; killed by A2.
- `S38804` the fixed read: the statement selects another note's tags; killed by A5.
- `S38805` the no-row arm: a note the collection lacks reads marked; killed by A6.
- `S38806` the core's rule in the read: any tag marks the note; killed by A5.
- `S38807` the view's key: `assisted` always false; killed by A10.
- `S38808` the view's read: the card's own id in place of its note's; killed by A10.

`S38807` and `S38808` are killed by A10's census, the one native killer for code only the web
engine's own target compiles. Web code is proved by StrykerJS on the pull request (C3), never by
hand rows.
