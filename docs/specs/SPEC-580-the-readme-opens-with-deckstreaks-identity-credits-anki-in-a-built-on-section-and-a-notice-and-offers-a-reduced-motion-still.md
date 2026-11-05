# SPEC-580: the README opens with DeckStreak's identity, credits Anki in a Built on section and a NOTICE, and offers a reduced-motion still

- **Issue:** #1049 (the reduced-motion still); the pull request names the identity text's issue.
  **Context(s):** the repository's public documents (`README.md`, `NOTICE`, `AGENTS.md`, `docs/`),
  the brand list and its census (`scripts/`).
- **Decided by:** ADR-599 (DeckStreak's identity text opens without Anki, Telegram or the bot, and
  states what is still Anki's below it); ADR-598 and SPEC-579 (the brand files, amended for the
  still); ADR-589 (the surfaces).
- **Schematic:** `docs/schematics/identity-docs.md`, new; `docs/schematics/brand-files.md`, amended.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-580.md`, cut before SPEC-581. **Mutation band:** S58000-S58099.
  **Models:** none (ADR-599 D6).

## 1. The problem, measured

Read at dev before this delivery, by `git show <dev>:<path>` with `grep -n` and line ranges:

- The repository description reads: "A spaced-repetition study game for the web, iPhone and iPad:
  XP, streaks, quests and a morning reading from a mentor per subject that primes the day's new
  cards. Rex, DeckStreak's own memory model, becomes its only scheduler before 1.0."
- `README.md` (106 lines): `# DeckStreak` (line 1), a blank line, the hero
  `![DeckStreak](docs/assets/deckstreak-hero-tide-light.gif)` (line 3), a blank line, and an opening
  paragraph (lines 5 to 9) that names Anki three times, the Telegram Mini App once and the bot once.
  The contents (lines 16 to 23) list eight sections; `## Source code` is line 92 and `## License`
  line 98. Lines 105 and 106 are a trademark notice that names the trademark's holder.
- `AGENTS.md` line 8 reads "DeckStreak is a Telegram Mini App and bot that gamify Anki reviews and
  deliver daily pre-study".
- `docs/PRD.md` (65 lines): `## Problem` (line 6) ends "in one Telegram Mini App" (line 11);
  `## Goals` (line 18) names the Mini App and the bot (lines 21 and 22); `## Open questions` (line
  50) names the Mini App (line 52); `## Amendments` (line 55) ends at line 65.
- `docs/OWNER-SETUP.md` (91 lines): its introduction ends at line 6; section 1 is the Telegram bot
  (line 8), section 2 the owner's Telegram identity (line 27), section 6 the Mini App (line 61).
- No `NOTICE` at the root (`git ls-tree <dev>` lists none). `REUSE.toml` (41 lines) gives the brand
  files `LicenseRef-DeckStreak-Brand` (lines 25 to 41); `LICENSES/LicenseRef-DeckStreak-Brand.txt`
  line 8 states the DeckStreak name and logo are trademarks of DeckStreak.
- The engine: `Cargo.toml` lines 88 to 92 pin Anki's Rust engine to an upstream tag, and lines 165
  to 184 patch it to a commit of the maintainer's fork; ADR-018 line 19 and SPEC-022 line 13 state
  it is AGPL-3.0-or-later. `deploy/systemd/deck-streak-sync-server.service` lines 1 and 2: the sync
  server is the engine's own, built from the pin. ADR-340 line 10: AnkiMobile syncs to the same
  server.
- The brand source's light poster, the still for `prefers-reduced-motion: reduce`: 960 by 360
  pixels, 8,377 bytes, sha256 `92114e135df1b8bafe1a283e7464222c4b81f6de7850558224482acd69334584`.
- `scripts/tests/test_brand_files.py` (606 lines) holds the README's header to the title, a blank
  line, one image line and the paragraph (lines 295 to 318); row S57904 anchors on README line 3.

## 2. Requirements

R1. `README.md`'s opening, every line before its first `## ` heading, names no Anki, no Telegram, no
    bot and no Mini App. Its first line is `# DeckStreak`; after the hero (R3) and a blank line, its
    first paragraph, with its whitespace collapsed, is the repository description exactly; a second
    paragraph says the engine's stock scheduler schedules every review until Rex does; the status
    note follows unchanged.
R2. `README.md` holds a `## Built on` section after `## Source code` and before `## License`, linked
    from the contents, whose five items open with the bold leads `**The engine.**`,
    `**The collection format and the sync protocol.**`, `**DeckStreak's own sync server.**`,
    `**AnkiMobile, until the switch.**` and `**The name.**`; the last holds the trademark line
    "Anki® is a registered trademark, and DeckStreak is not affiliated with or endorsed by its
    holder." The trademark notice at the README's foot is removed.
R3. `README.md`'s hero is a `<picture>` element on lines 3 to 6: one `<source>` with
    `media="(prefers-reduced-motion: reduce)"` and `srcset="docs/assets/deckstreak-hero-poster-tide-light.png"`,
    and one `<img>` with `src="docs/assets/deckstreak-hero-tide-light.gif"`, `width="960"`,
    `height="360"` and `alt="DeckStreak"`. The still is the brand source's light poster byte for
    byte, copied by a program, listed in `scripts/brand-files.json` and annotated in `REUSE.toml`
    under `LicenseRef-DeckStreak-Brand`; no brand file is edited by hand.
R4. A root `NOTICE` states what DeckStreak is and where its source is; that its own code is
    AGPL-3.0-or-later with the text in `LICENSE` and every file's licence in `REUSE.toml`; that it
    includes Anki's Rust engine, through the maintainer's fork, under the same licence and with its
    own copyright notices; that the vendored deepmerge copy is MIT; that the brand files are under
    no licence, as `LICENSES/LicenseRef-DeckStreak-Brand.txt` states; and two trademark lines,
    DeckStreak's and Anki's, Anki's naming no holder.
R5. `AGENTS.md` line 8 reads "DeckStreak is a spaced-repetition study game for the web, iPhone and
    iPad, with daily pre-study"; line 9 is unchanged.
R6. `docs/PRD.md` and `docs/OWNER-SETUP.md` change insert-only: the PRD gains a pointer line under
    `## Problem`, `## Goals` and `## Open questions` and one amendment paragraph after line 65;
    OWNER-SETUP gains one paragraph after line 6. No line of either file is removed or changed.
R7. SPEC-579 R4, ADR-598 D4 and `docs/schematics/brand-files.md` are amended insert-only, each by an
    appended section; row S57904 anchors on the hero's `<img>` line, with its killer unchanged.
R8. No web, Swift or Rust file changes; the two owner rulings dated for the app's surfaces and the
    owner's taps, the bot, the Mini App, every screen, the scheduler, the engine and every
    repository setting are byte-unchanged. SPEC-334 keeps every line it has and gains only an
    appended section 11 naming the owner's signed identity ruling, which this pull request adds
    under `docs/rulings/` in its own commit.

## 3. Acceptance criteria of SPEC-580

| id | criterion | decided by |
|---|---|---|
| A1 | the README's opening names no barred word and its first paragraph is the description (R1) | `test_identity_docs.py` |
| A2 | an opening naming Anki, Telegram, the bot or the Mini App is refused, each word in turn (R1) | `test_identity_docs.py` |
| A3 | the README's Built on section holds its five items and the trademark line, and the contents link it (R2) | `test_identity_docs.py` |
| A4 | a Built on section missing any one item is refused, each item in turn (R2) | `test_identity_docs.py` |
| A5 | a trademark line that names a holder is refused (R2) | `test_identity_docs.py` |
| A6 | NOTICE holds each of its named items (R4) | `test_identity_docs.py` |
| A7 | a missing NOTICE, and a NOTICE missing any one item, are refused (R4) | `test_identity_docs.py` |
| A8 | AGENTS.md line 8 is the identity line (R5) | `test_identity_docs.py` |
| A9 | a planted complete tree reads green, the control for A2, A4, A5 and A7 | `test_identity_docs.py` |
| A10 | the README opens with its title, then the hero's picture with the listed still and the listed gif, then its paragraph (R3) | `test_brand_files.py` |
| A11 | the still is listed and present with the brand source's pinned sha256 and size (R3) | `test_brand_files.py` |
| A12 | a README whose hero shows the gif with no reduced-motion source is refused (R3) | `test_brand_files.py` |
| A13 | a missing still, and a still whose bytes differ from the pin, are refused (R3) | `test_brand_files.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_the_readme_opening_is_the_identity
A2: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_an_opening_naming_a_barred_word_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_the_readme_built_on_names_each_item
A4: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_built_on_section_missing_an_item_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_trademark_line_naming_a_holder_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_notice_names_each_item
A7: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_missing_or_incomplete_notice_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_agents_line_8_is_the_identity
A9: python3 -m unittest discover -s scripts/tests -p test_identity_docs.py -k test_a_complete_planted_tree_reads_green
A10: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_readme_opens_with_the_title_then_the_hero
A11: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_still_is_listed_by_its_pinned_sha256
A12: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_hero_with_no_reduced_motion_source_is_refused
A13: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_missing_or_altered_still_is_refused
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `README.md` | docs | changed: the opening, the contents, the Built on section, the foot's trademark notice removed (R1, R2, R3) |
| `NOTICE` | licensing | new (R4) |
| `AGENTS.md` | docs | changed: line 8 (R5) |
| `docs/PRD.md` | docs | amended insert-only (R6) |
| `docs/OWNER-SETUP.md` | docs | amended insert-only (R6) |
| `docs/assets/deckstreak-hero-poster-tide-light.png` | docs | new: the still, copied from the brand source (R3) |
| `scripts/brand-files.json` | brand | changed insert-only: the still's entry (R3) |
| `REUSE.toml` | licensing | changed insert-only: the still's path in the brand annotation (R3) |
| `scripts/tests/test_identity_docs.py` | tests | new: A1 to A9 |
| `scripts/tests/test_brand_files.py` | tests | changed: the still, the hero's picture, A10 to A13 |
| `scripts/mutation-rows.d/S57900-S57999.json` | rows | changed: S57904 anchored on the hero's `<img>` line (R7) |
| `scripts/mutation-rows.d/S58000-S58099.json` | rows | new: S58001 to S58008 |
| `docs/red-first/SPEC-580.md` | records | new |
| `docs/specs/SPEC-580-the-readme-opens-with-deckstreaks-identity-credits-anki-in-a-built-on-section-and-a-notice-and-offers-a-reduced-motion-still.md` | docs | new: this file |
| `docs/decisions/ADR-599-deckstreaks-identity-text-opens-without-anki-telegram-or-the-bot-and-states-what-is-still-ankis-below-it.md` | docs | new |
| `docs/schematics/identity-docs.md` | docs | new |
| `docs/schematics/brand-files.md` | docs | amended insert-only: section 5 (R7) |
| `docs/specs/SPEC-579-the-deckstreak-logo-enters-the-tree-byte-for-byte-from-the-brand-source-as-the-web-icons-the-app-icon-set-and-the-readme-hero.md` | docs | amended insert-only: R4 (R7) |
| `docs/decisions/ADR-598-the-deckstreak-logo-enters-the-tree-byte-for-byte-admitted-by-a-pinned-list-the-public-scrub-reads.md` | docs | amended insert-only: D4 (R7) |
| `docs/rulings/OWNER-RULING-2026-10-31-deckstreaks-identity.md` | rulings | new: the owner's signed identity ruling, in its own commit (R8) |
| `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md` | docs | amended insert-only: section 11 names the signed ruling (R8) |
| `changelog.d/ds-identity-580.md` | changelog | new |

## 5. What this does NOT cover

- The repository's social preview, its description and its topics, which are repository settings (#1048).
- The README's install, usage and deploy lines and `AGENTS.md` line 10, which name the Mini App or the bot as the code still does; they change with the retirement of the bot and the Mini App (#1046).
- How the forge renders the picture element and its media query, and what a device shows, read after the pull request (#1047).

## 6. Risks

- **The forge ignores the media query.** Then every reader sees the GIF, as at dev; nothing in the
  tree can show it, and #1047 reads the rendered page.
- **A test that pins the README's header.** `test_brand_files.py` reads it by exact lines; it
  changes in the same commit as its red, and A10 to A13 hold the new form.
- **A row anchored on a changed line.** S57904 anchors on README line 3; it is re-anchored in the
  commit that changes the README, so no commit holds a find that occurs zero times.
- **A still carrying metadata.** The copy program reads the PNG's chunks before the copy and A1 of
  SPEC-579 reads them in the tree; a text, time or comment chunk stops the copy as a question, never
  a stripped copy.
- **The description changing before the build.** The build reads it once at its cut; a difference
  from section 1 stops the build.
- **A secret scanner reading the still's sha256 as a key.** The build's range scan reads the list
  and the rows; a finding is a stop to report, never a widened rule.
