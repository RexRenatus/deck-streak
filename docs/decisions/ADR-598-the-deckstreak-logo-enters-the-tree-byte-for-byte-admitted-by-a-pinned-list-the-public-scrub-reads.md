---
status: "accepted"
---

# ADR-598: the DeckStreak logo enters the tree byte for byte from the brand source, admitted by a pinned list the public scrub reads

Decides SPEC-579. It supersedes ADR-361 D16 (icons built from text, no binary in the tree), makes
the allow-list SPEC-033 section 5 deferred to the delivery that needs it, creates the iPhone and
iPad app's first asset catalog that ADR-597 adds colour sets to, and is cut before SPEC-578.

## Context and Problem Statement

The DeckStreak logo, the tide spiral mark in its own tide palette, becomes the web app's favicon
and web manifest icons, the iPhone and iPad app's icon set, and, through the hero animation, the
README header's logo. Every file comes from the DeckStreak brand source, written there by one
command and checked there. Measured at dev `57cbb5e1`:

- `web/app/src/app.html` (16 lines) links the manifest (line 9) and a touch icon (line 10) and no
  favicon. `web/app/static/` holds `manifest.webmanifest` (10 lines; icons `/icon-192.png` and
  `/icon-512.png`, lines 6 to 9) and `robots.txt`, and no image.
- The three icons are plain one-colour squares built from text at build time: `web/app/src/lib/icon.ts`
  (74 lines) and one prerendered endpoint per icon under `web/app/src/routes/` (ADR-361 D16, lines
  289 to 316 of `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md`),
  because "the public scrub refuses every binary file in the tree and in its history" (its lines
  300 and 301).
- `web/app/src/lib/manifest.test.ts` (113 lines) holds SPEC-350 A28: its `pixels` reader (lines 51
  to 72) accepts only a one-colour PNG of one image chunk.
- `scripts/public-scrub.py` (416 lines) refuses a binary by the rule `binary` whatever its path
  (`is_binary`, lines 181 to 188; SPEC-033 R2) and still searches its bytes for the private literals
  (lines 229 to 236); a file over 2,000,000 bytes is `oversize` (line 64). SPEC-033 section 5
  (lines 121 to 123) defers "an allow-list with a metadata check, made by the delivery that needs
  them".
- The product app's project spec, `ios/app.yml` (84 lines), names no asset catalog and no app icon
  setting (target lines 26 to 44). The harness's catalog (`ios/project.yml:34`, `:48`) holds a
  one-colour icon CI writes and git ignores.
- `README.md` opens with `# DeckStreak` (line 1), a blank line, and the description paragraph
  (lines 3 to 7).
- `REUSE.toml` (23 lines) licenses every file under `AGPL-3.0-or-later` (lines 5 to 9), as ADR-018
  decided; the brand source records its files under its own reserved licence.
- The brand source's export list, per file (pixels; sha256 prefix): `favicon.svg` (vector;
  `a92638069276d49c`), `favicon.ico` (16 and 32; `86a42d27d5cc9192`), `apple-touch-icon.png` (180;
  `e1798e259077840e`), `icon-192.png` (192; `b487866bb47c096b`), `icon-512.png` (512;
  `e7c5fda5b3592045`), `icon-maskable-512.png` (512; `d34831c08d65125e`), the app icon set's
  `Contents.json` (`8e290c5aab301de1`) and its `AppIcon-1024.png`, `AppIcon-dark-1024.png` (1024;
  both `2766fd033886ec3d`, the same pixels) and `AppIcon-tinted-1024.png` (1024;
  `8925f8119fd1d9a1`), and the manifest's `icons` member (three entries: 192 and 512 for purpose
  `any`, 512 for purpose `maskable`). The light hero animation is 960 by 360 pixels, 115 frames,
  looping, 134,234 bytes, sha256
  `fc891c959acfbeed4b2f68d1bb795d13b254e5ce2c81fb179185e66251b6ac14`.

Not measured here: how a device or a browser renders each icon, how GitHub renders and pauses the
animation, and which metadata chunks the brand files carry (the build's copy program measures the
last before the copy).

## Decision Drivers

- The files enter byte for byte from the brand source, by a program, never by hand.
- The public scrub keeps refusing every binary it does not know, and keeps searching every binary
  for private literals.
- The README's title stays its first line and its description paragraph follows the hero.
- The logo never waits on a colour change; no screen's layout changes.
- No new dependency.

## D1, the icon and README surfaces measured: what it was chosen against

- Chosen: the measurements above, each cited at dev `57cbb5e1`, because they show the icons are generated placeholders, the scrub refuses any committed image, and the app has no catalog
- Treating the harness's icon set as the app's: rejected because it belongs to the harness target and its image is written by CI and ignored by git

## D3, the app's first asset catalog: what it was chosen against

`ios/App/Assets.xcassets` is created with its root `Contents.json` (the harness catalog's own
bytes: `info`, `author` `xcode`, `version` 1) and `AppIcon.appiconset` holding the brand export's
`Contents.json` and its three 1024-pixel icons (any appearance, dark, tinted). `ios/app.yml` lists
`App/Assets.xcassets` among the `DeckStreak` target's sources and sets
`ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon` in its base settings, the harness's own two lines.
A project-spec census run locally, `scripts/tests/test_brand_files.py`, reads both lines and the icon
set, and CI's `apple-on-change` run compiles the catalog in the app's tests and its unsigned archive.

- Chosen: one catalog wired in the project spec, checked by a census on Linux and compiled in CI, because it is the harness's proven shape and needs no Swift
- A Swift test that reads the app icon name from the bundle: rejected because it needs the two-push form for a fact the project spec states, and the archive step already compiles the catalog
- The setting in `Config/App.xcconfig`: rejected because the harness keeps it in the spec, and one census reads one place
- One catalog per delivery: rejected because two catalogs in one target collide on the compiled asset file

## D4, the web icons and the README header: what it was chosen against

The export's web files are copied into `web/app/static/`: `favicon.svg`, `favicon.ico`,
`apple-touch-icon.png`, `icon-192.png`, `icon-512.png` and `icon-maskable-512.png`. The manifest's
`icons` member becomes the export's three entries, copied as they are (their `src` paths are
relative, and the manifest sits at the root). `app.html` gains one link,
`<link rel="icon" href="%sveltekit.assets%/favicon.svg" type="image/svg+xml" />`; the touch icon
link stays; `/favicon.ico` is served at the root as browsers ask for it. `icon.ts` and the three
endpoints are removed, so no prerendered answer shadows a static file. `manifest.test.ts` keeps its
test name and reads the static files: the three entries, each icon's size from its header, the
touch icon at 180 pixels, both favicons present and the three head links.

The README gains two lines after its title: a blank line stays after `# DeckStreak`, then
`![DeckStreak](docs/assets/deckstreak-hero-tide-light.gif)`, then a blank line, then the unchanged
description paragraph. The hero serves as the header's logo: it draws the logo's mark and types the
word, and any second image would sit either above the title or between the hero and the paragraph.

- Chosen: the export's files at the static root, linked as the export's own notes say, and the hero as the README's one image, because each file is the brand's and the README keeps its order
- Keeping the endpoints and serving the brand bytes through them: rejected because a build step would then hold binary bytes in source, and a static file is what the export is for
- A separate logo image in the README: rejected because it would break either "the title first" or "the paragraph follows the hero"
- The logo inline in the title line: rejected because the title's text and its anchor would change
- The hero linked from outside the repository: rejected because the README must not depend on an outside file
- A `<picture>` with a reduced-motion still: rejected for this round because the still is a second file the round does not name; it is a question for its own issue

## D5, the copy route, the scrub's admission and the licence: what it was chosen against

Every brand file enters by a program the delivery runs once: it reads the brand source at its
verified head, checks each file's sha256 against the export list before the copy (the hero
against its full sha256), writes it to its tree path, and checks it again after. The same program
writes `scripts/brand-files.json`, one entry per copied file: its tree path, full sha256, byte
count, kind (`png`, `ico`, `gif`, `svg`, `json`) and pixel sizes. No brand file is edited by hand.

`public-scrub.py` admits a binary only when its path, its sha256 and its byte count equal an entry
of `scripts/brand-files.json` at the root it scans, in the tree scan and in the history scan; an
admitted binary is still searched for every private literal; every other binary is still refused
by `binary`; an absent list admits nothing; a list that cannot be read stops the scrub as a usage
error, by name. The metadata check is `test_brand_files.py`'s: each listed file's kind, its pixel
sizes from its own header, and no text, time or comment metadata in it.

The brand files keep the brand source's own licence: one `REUSE.toml` override annotation names
exactly the listed files, its licence text sits under `LICENSES/`, and ADR-018 is amended
insert-only. Every other file stays under the repository's AGPL.

Byte-unchanged: every screen's layout and behaviour, the harness's catalog and project spec, the
bot and the Mini App. No kept check is loosened: the scrub refuses every binary it refused before
except the listed bytes at their listed paths, and searches those for literals as before.

- Chosen: a program copy checked twice, and one pinned list the scrub admits from, because a listed binary is exactly one known set of bytes at one path
- An admission by path alone: rejected because any bytes at that path would then pass
- An admission by content alone: rejected because the same bytes could then enter anywhere
- Parsing each format inside the scrub: rejected because it adds code to a gate for a check a test makes over the same list
- A committed copy program: rejected because it would name the brand source's location in the tree, and the list and the census hold the result
- Images hosted outside the repository: rejected because the README and the app must carry their own files
- The brand files under the repository's AGPL like every other file: rejected because the brand source records them under its own licence, and a project's mark is its identity rather than part of the program

## D6, the shape: what it was chosen against

As ADR-597 D6: this delivery is cut first and creates the catalog; SPEC-578 is cut after it
merges. The amendments are insert-only: SPEC-033 (R2's admission), SPEC-350 (R17's icons come from
the brand source), ADR-361 (D16 superseded), and ADR-018 under the licence answer.

- Chosen: the logo first, because it waits on no colour change and creates the catalog the colour sets join
- The scrub's admission as its own earlier delivery: rejected because an allow-list with no file to admit has no test subject, and SPEC-033 gives it to the delivery that needs it

## D7, tests and rows: what it was chosen against

Every red runs locally: `test_brand_files.py` (the files, the README header, the app's icon set),
new tests in `test_public_scrub.py` (the admission), and `manifest.test.ts` (the web icons). Each
refusal has a planted red over a fixture whose bytes the test spells, and a planted complete set
reads green as the control. The band S57900-S57999 holds script rows on the scrub's admission and
literal search, the README's image line and the app's icon setting. One push. FORMAL: not
applicable (no interleaving; the admission is a set membership of three fields).

- Chosen: Python and Vitest reds run locally with fixtures spelt in the tests, because every fact the delivery adds is a file or a line a test can read
- Hand rows on the web manifest: rejected because the rows table takes no Vitest killer and the file is data

## D8, not this round: what it was chosen against

The palette on screens, a mode switch, the repository's social preview (a repository setting), the
bot and the Mini App, a landing page, a reduced-motion still for the README, and what only a device
or a later CI run shows. Each is in SPEC-579 section 5 with its issue.

- Chosen: the icons and the header only, because each excluded item is a setting, a screen or a device measurement of its own
- Setting the social preview from the export now: rejected because it is a repository setting, not a tree change

## Decision Outcome

D3, D4 and D5 are built by SPEC-579 in one pull request, cut before SPEC-578 (D6); D7 names its
proof; D1 and D8 bound it.

### Consequences

- Good, because the product shows its own mark in browsers, on home screens and on the app icon.
- Good, because the scrub's binary rule gains one exact door, and every other binary still fails.
- Bad, because the history now holds about 270,000 bytes of images, which every history scan reads.
- Bad, because a brand update means a new copy, a new list and a new pull request.

### Confirmation

SPEC-579's acceptance criteria A1 to A19, its red-first record, its rows S57901 to S57905, and CI's
scrub stage and `apple-on-change` run at the pull request's head.

## What would make this wrong

- A brand file whose bytes carry private data the literal search does not name.
- A browser or a device that refuses an icon the census passed.
- The project needing binaries often, when ADR-033 says an allow-list becomes the rule.

## More Information

Built by SPEC-579; drawn in `docs/schematics/brand-files.md`. SPEC-033 and ADR-033 (the scrub),
SPEC-350 and ADR-361 (the install surface), ADR-018 (the licence), ADR-597 (the colour sets).
