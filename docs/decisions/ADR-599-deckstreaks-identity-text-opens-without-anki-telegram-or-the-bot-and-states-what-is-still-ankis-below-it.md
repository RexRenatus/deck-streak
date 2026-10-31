---
status: "proposed"
---

# ADR-599: DeckStreak's identity text opens without Anki, Telegram or the bot, and states what is still Anki's below it

Decides SPEC-580. It amends SPEC-579 R4, ADR-598 D4 and `docs/schematics/brand-files.md` insert-only
for the README's hero, which becomes a picture with a reduced-motion still, and it is cut before
SPEC-581, the tagline.

## Context and Problem Statement

DeckStreak's repository description reads: "A spaced-repetition study game for the web, iPhone and
iPad: XP, streaks, quests and a morning reading from a mentor per subject that primes the day's new
cards. Rex, DeckStreak's own memory model, becomes its only scheduler before 1.0." CHARTER item 22
(`CHARTER.md:120`) makes Rex DeckStreak's own memory model and its only scheduler, and no 1.0 release
ships without Rex scheduling every review (`CHARTER.md:149`). ADR-589 deprecated the Telegram Mini
App and the bot, and the web app and the iPhone and iPad app are the surfaces.

The texts a reader meets first say otherwise. Read at dev before this delivery:

- `README.md` (106 lines): its opening paragraph (lines 5 to 9) turns "your Anki reviews" into a
  game "in a Telegram Mini App", reads reviews "through your own Anki sync server", and has "the
  bot" send a digest. Its trademark notice (lines 105 and 106) names the trademark's holder.
- `README.md` line 3 shows the light hero GIF, which loops for as long as the page is open; no
  still is offered to a reader who asks for reduced motion.
- `AGENTS.md` line 8 calls DeckStreak "a Telegram Mini App and bot that gamify Anki reviews".
- `docs/PRD.md` frames the product as one Telegram Mini App (line 11), and its goals and first open
  question name the Mini App and the bot (lines 21, 22 and 52).
- `docs/OWNER-SETUP.md` sets up the Telegram bot (section 1, line 8), the owner's Telegram identity
  (section 2, line 27) and the Mini App (section 6, line 61) with no word that they are deprecated.
- No root `NOTICE` exists. `LICENSE` holds the AGPL's text; `LICENSES/` holds the AGPL, MIT and
  brand texts; `REUSE.toml` records each file's licence (lines 5 to 41).

What is still Anki's is real and stays: the study engine is Anki's own Rust engine, pinned in
`Cargo.toml` (lines 88 to 92) to an upstream tag and patched to a commit of the maintainer's fork
(lines 165 to 184), and it is licensed AGPL-3.0-or-later (ADR-018 line 19, ADR-022 line 84,
SPEC-022 line 13). DeckStreak studies an Anki collection over Anki's sync protocol; the sync server a
deployment runs is the engine's own server, built from the pin and shipped in each release
(`deploy/systemd/deck-streak-sync-server.service` lines 1 and 2); and AnkiMobile studies the same
collection through the same server (ADR-340 line 10).

## Decision Drivers

- The opening says what DeckStreak is, in the repository description's own words, and names no Anki,
  no Telegram and no bot.
- What is still Anki's is stated once, lower down, as facts the tree supports, with a trademark line
  that names no holder.
- A root `NOTICE` gives the licences and the trademark lines in one place, beside `LICENSE`.
- A reader who asks for reduced motion sees a still, with no script.
- Ruled text keeps its words: an amendment is insert-only, and a signed ruling changes only by the
  owner's ruling.
- Every new statement is held by a test that refuses its absence, red first.

## D1, the surfaces, measured: what it was chosen against

Every surface was read at dev before this delivery by `git show` with `grep -n` and line ranges, after a fenced
census of the tree. The README names Anki, Telegram, the bot or the Mini App on lines 5 to 8, 27,
36, 52, 58 to 60 and 105; its header is the title (line 1), a blank line, the hero GIF
`docs/assets/deckstreak-hero-tide-light.gif` with alt text `DeckStreak` (line 3), a blank line and
the opening paragraph (line 5). The licence and notice files are `LICENSE`, `LICENSES/AGPL-3.0-or-later.txt`,
`LICENSES/MIT.txt`, `LICENSES/LicenseRef-DeckStreak-Brand.txt` (its trademark line is line 8),
`vendor/deepmerge/license.txt` and `REUSE.toml`. The tests that read these files are
`scripts/tests/test_brand_files.py` (the README's header, lines 295 to 318; the brand list's files,
lines 41 to 53), `test_privacy_policy.py` (the README's Privacy section, lines 161 to 176),
`test_fork_rule.py` (AGENTS.md's fork rule) and `test_github_setup.py` (two phrases of
OWNER-SETUP, lines 95 to 98); mutation row S57904 anchors on README line 3. How the forge renders a
`<picture>` element with a media query was not measured.

- Chosen: one fenced census of the tree, then reads of the files it lists, because every claim here cites a line read at one commit
- A reading of the rendered pages on the forge: rejected because a render is not a tree fact and no test in the tree can hold it

## D2, the README's opening and its Built on section: what it was chosen against

The opening is the level-1 title, the hero, the repository description quoted as its first
paragraph, one paragraph on the scheduler until Rex takes over, and the status note, which stays.
It names no Anki, no Telegram, no bot and no Mini App. A `## Built on` section, placed after
`## Source code` and before `## License` and linked from the contents, states what is still Anki's
in five items, each a fact the tree supports: the engine (Anki's own Rust engine, pinned in
`Cargo.toml` and patched to a commit of the maintainer's fork, AGPL-3.0-or-later as DeckStreak is,
its copyright notices its own); the collection format and the sync protocol; DeckStreak's own sync
server, which is the engine's own server; AnkiMobile, which studies the same collection until Rex
is DeckStreak's only scheduler; and one trademark line that names no holder. The trademark notice
at the README's foot goes, since the Built on section carries it. A test refuses an opening that
names any of the four words, an opening whose first paragraph is not the description, a Built on
section missing any item, and a trademark line that names a holder.

- Chosen: the description as the opening and a Built on section lower down, because a reader learns what DeckStreak is first and what it takes from Anki in one named place
- An opening that names Anki as the engine in its first paragraph: rejected because the first paragraph is the product, and the engine is a part, stated in Built on
- Credits only in NOTICE, with no Built on section: rejected because the README is what a reader opens, and NOTICE is what a redistributor reads
- Keep the trademark notice at the README's foot beside Built on: rejected because two copies drift, and one of them names a holder

## D3, NOTICE and AGENTS.md: what it was chosen against

A new root `NOTICE` states what DeckStreak is, that its own code is AGPL-3.0-or-later with the text
in `LICENSE` and every file's licence in `REUSE.toml`, that it includes Anki's Rust engine through
the maintainer's fork under the same licence with its own copyright notices, that the vendored
deepmerge copy is MIT with its notice in `vendor/deepmerge/license.txt`, that the brand files are
under no licence as `LICENSES/LicenseRef-DeckStreak-Brand.txt` states, and two trademark lines: the
DeckStreak name and logo as the brand licence's line 8 states them, and Anki's with no holder named.
`NOTICE` restates nothing `LICENSE` or the brand licence holds; it points to each, and
`REUSE.toml`'s first annotation already covers it. `AGENTS.md` line 8 becomes the identity's first
clause; line 9 continues it unchanged. A test refuses a missing `NOTICE`, a `NOTICE` missing any
named item, and an `AGENTS.md` line 8 that is not the identity line.

- Chosen: a short root NOTICE that points to the licence texts, because a redistributor finds the licences, the engine and the trademarks in one file without a second copy of any text
- Append the notices to LICENSE: rejected because LICENSE is the AGPL's verbatim text, which a licence check reads by its digest
- Rewrite AGENTS.md's whole overview paragraph: rejected because only line 8 frames the product, and its other lines name paths an agent needs

## D4, the reduced-motion still: what it was chosen against

The README's hero becomes a `<picture>` element on four lines after the title and a blank line: its
one `<source>` names `media="(prefers-reduced-motion: reduce)"` and the still, and its `<img>` names
the GIF with its width, height and alt text `DeckStreak`, as the brand source's own hero notes
show. No script runs. The still is the brand source's light poster, committed at
`docs/assets/deckstreak-hero-poster-tide-light.png` under the brand source's own file name. A program
copies it from the brand source at a pinned head, checks its sha256 against the pinned value before
the copy and again after, and inserts its entry into `scripts/brand-files.json` (path, sha256, byte
count, kind and pixels, sorted by path, so it lands first); `REUSE.toml` gains its path in the brand
annotation; both insert-only. No brand file is edited by hand. `test_brand_files.py` lists the still
among the brand paths, pins its sha256 and size, reads the picture element in place of the image
line, and refuses a missing still, a still whose bytes differ from the pin, and a README that shows
the GIF with no reduced-motion source. Row S57904 is re-anchored on the GIF's new line. SPEC-579
R4, ADR-598 D4 and the brand-files schematic are amended insert-only.

- Chosen: a picture element whose reduced-motion source is the brand's own still, because it needs no script and keeps the GIF for every other reader
- The still alone, with no animation: rejected because it removes the hero from every reader to serve the readers who asked
- A link under the hero to the still: rejected because a reader who asked for reduced motion still sees the animation first
- The dark poster too, under a colour-scheme query: rejected because the README shows the light hero alone today, and a dark pair is its own change

## D5, the PRD and OWNER-SETUP framing, and the ruled ANK-01 text: what it was chosen against

`docs/PRD.md` is amended insert-only, in its own Amendments form: a pointer line under `## Problem`,
`## Goals` and `## Open questions` names the identity amendment, and a new amendment paragraph after
the last one says what DeckStreak is, that the web app and the iPhone and iPad app are its surfaces,
that the Mini App and the bot are deprecated (ADR-589), and that Rex becomes the only scheduler
before 1.0; the lines it reads anew are kept as they were. `docs/OWNER-SETUP.md` gains one
paragraph after its introduction saying what DeckStreak is and that sections 1, 2 and 6 set up the
deprecated Telegram surfaces, which stay until their removal and gain no feature; no step changes.
This delivery changes no ruled text: the two signed rulings that state ANK-01 and SPEC-334 R2 keep
their words, and the README and NOTICE state only what is true under them (Anki's engine opens the
collection and records each answer).

- Chosen: insert-only pointers and one amendment paragraph, because the PRD's own Amendments section keeps earlier text as it was
- Rewrite the PRD's problem and goals in place: rejected because a reader would lose what the product began as, which the amendments form keeps
- Edit SPEC-334 R2 and the rulings' ANK-01 lines in this delivery: rejected because a signed ruling's text and the requirement it decides change only by the owner's ruling

## D6, the shape, the tests and the rows: what it was chosen against

One delivery, cut before SPEC-581. Every red runs locally over fixtures the tests write:
`scripts/tests/test_identity_docs.py` (new: the opening, Built on, NOTICE and AGENTS.md line 8) and
`scripts/tests/test_brand_files.py` (the still and the picture). The first test commit holds each
new refusal's helper as an accept-all stub, so every planted refusal is red for its refusal; the
next commit writes the helpers, and each planted refusal turns green while each tree criterion reads
red on the unchanged texts; the third changes the texts and copies the still. A planted complete
tree reads green as the control. Band S58000-S58099 holds script rows on the README's still line,
the brand list's pinned sha256, the opening's description, a Built on item, the trademark line,
NOTICE's engine licence and AGENTS.md line 8. One push. FORMAL: not applicable to TLA+ (no
interleaving: every check reads files at rest) and not applicable to Lean (no pure algorithm of
DeckStreak's is added; the checks are string reads in test code).

- Chosen: Python census tests with accept-all stubs first, because each refusal is seen red for its own reason before its helper exists
- One delivery for the docs and the tagline: rejected because the tagline changes the web app's catalogs and tests, and the docs change none of the web app
- The tagline first: rejected because the tagline restates the identity this delivery writes

## D7, not this round: what it was chosen against

The repository's social preview, description and topics (repository settings), the README's install,
usage and deploy lines that name the Mini App and the bot, `AGENTS.md` line 10, the engine's removal
and every scheduler change, the retirement of the bot and the Mini App, every screen's layout, the
tagline (SPEC-581), and what only the forge's render or a device shows. Each is in SPEC-580 section 5.

- Chosen: the identity text and the still only, because each excluded item is a setting, a retirement, a screen or a render of its own
- Rename the Mini App in the README's install and usage lines now: rejected because those lines change with the retirement that removes the Mini App's code

## Decision Outcome

D2, D3, D4 and D5 are built by SPEC-580 in one pull request, cut before SPEC-581 (D6); D1 measures
them; D7 bounds them.

### Consequences

- Good, because a reader meets DeckStreak, its surfaces and Rex first, and finds what is Anki's in one named section.
- Good, because a redistributor finds every licence and trademark line in `NOTICE`.
- Good, because a reader who asks for reduced motion sees a still.
- Bad, because the README's header is HTML now, which the brand census reads by its exact lines.
- Bad, because the PRD reads in layers: its earlier lines stay, and the amendment below them binds.

### Confirmation

SPEC-580's acceptance criteria A1 to A13, its red-first record, its rows S58001 to S58008, row S57904
re-anchored, and CI's scripts suite at the pull request's head.

## What would make this wrong

- The forge strips or ignores the picture's media query, so every reader sees the GIF.
- A fact Built on states stops being true: the engine is replaced, or AnkiMobile no longer studies the collection.
- The repository description changes and the README is not changed with it.

## More Information

Built by SPEC-580; drawn in `docs/schematics/identity-docs.md`. ADR-589 (the surfaces), ADR-598 and
SPEC-579 (the brand files), ADR-018 (the licence), ADR-022 and ADR-058 (the engine pin).
