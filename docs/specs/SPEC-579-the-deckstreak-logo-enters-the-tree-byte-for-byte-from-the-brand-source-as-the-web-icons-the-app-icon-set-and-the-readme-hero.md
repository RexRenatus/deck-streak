# SPEC-579: the DeckStreak logo enters the tree byte for byte from the brand source, as the web icons, the app's icon set and the README's hero

- **Issue:** #1042. **Context(s):** the web app's install surface (`web/app`), the iPhone
  and iPad app's project and asset catalog (`ios/`), the public scrub (`scripts/`), the README and
  `docs/`.
- **Decided by:** ADR-598 (the DeckStreak logo enters the tree byte for byte, admitted by a pinned
  list the public scrub reads); ADR-033 and SPEC-033 (the scrub); ADR-361 (D16, superseded);
  ADR-018 (the licence, amended for the brand files).
- **Schematic:** `docs/schematics/brand-files.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-579.md`, cut before SPEC-578. **Mutation band:** S57900-S57999.
  **Models:** none (ADR-598 D7).

## 1. The problem, measured

Read at dev `57cbb5e1`, by `git show <dev>:<path>` and `grep -n`:

- `web/app/static/` holds `manifest.webmanifest` and `robots.txt` and no image; the manifest's two
  icons (lines 6 to 9) and the touch icon `app.html` links (line 10) are plain squares built from
  text by `web/app/src/lib/icon.ts` (74 lines) through three prerendered endpoints; `app.html`
  links no favicon.
- `web/app/src/lib/manifest.test.ts` (113 lines) reads a PNG only if it is one colour in one image
  chunk (lines 51 to 72).
- `scripts/public-scrub.py` (416 lines) refuses every binary, in the tree and in history (lines 181
  to 188, 263 to 302), and searches its bytes for private literals (lines 229 to 236).
- `ios/app.yml` (84 lines) gives the `DeckStreak` target no asset catalog and no app icon setting
  (lines 26 to 44).
- `README.md` (104 lines): `# DeckStreak` (line 1), a blank line, the description paragraph (lines
  3 to 7).
- The brand source's exports, by pixels and sha256 prefix: `favicon.svg` (vector,
  `a92638069276d49c`), `favicon.ico` (16 and 32, `86a42d27d5cc9192`), `apple-touch-icon.png` (180,
  `e1798e259077840e`), `icon-192.png` (192, `b487866bb47c096b`), `icon-512.png` (512,
  `e7c5fda5b3592045`), `icon-maskable-512.png` (512, `d34831c08d65125e`), the icon set's
  `Contents.json` (`8e290c5aab301de1`), `AppIcon-1024.png` and `AppIcon-dark-1024.png` (1024,
  `2766fd033886ec3d`), `AppIcon-tinted-1024.png` (1024, `8925f8119fd1d9a1`); the light hero, 960 by
  360 pixels, 134,234 bytes, sha256 `fc891c959acfbeed4b2f68d1bb795d13b254e5ce2c81fb179185e66251b6ac14`.
  The largest is under the scrub's 2,000,000-byte limit.

## 2. Requirements

R1. Each brand file is in the tree at its path below, byte for byte the brand source's, copied by
    a program: `web/app/static/favicon.svg`, `favicon.ico`, `apple-touch-icon.png`, `icon-192.png`,
    `icon-512.png` and `icon-maskable-512.png`; `ios/App/Assets.xcassets/AppIcon.appiconset/`
    `Contents.json`, `AppIcon-1024.png`, `AppIcon-dark-1024.png` and `AppIcon-tinted-1024.png`; and
    `docs/assets/deckstreak-hero-tide-light.gif`.
R2. `scripts/brand-files.json` lists each file of R1 by its path, full sha256, byte count, kind and
    pixel sizes, and a census refuses a listed file that is missing, differs by a byte, has other
    pixel sizes than its entry, or carries text, time or comment metadata.
R3. `web/app/static/manifest.webmanifest`'s `icons` member is the brand source's three entries; its
    other members are unchanged. `web/app/src/app.html` links `favicon.svg` as an `image/svg+xml`
    icon beside its two existing links. `web/app/src/lib/icon.ts` and the endpoints at
    `apple-touch-icon.png`, `icon-192.png` and `icon-512.png` are removed.
R4. `README.md`'s first line stays `# DeckStreak`; its third line is
    `![DeckStreak](docs/assets/deckstreak-hero-tide-light.gif)`; a blank line separates each, and the
    description paragraph follows unchanged.
R5. `ios/App/Assets.xcassets/Contents.json` exists; `ios/app.yml`'s `DeckStreak` target lists
    `App/Assets.xcassets` among its sources and sets `ASSETCATALOG_COMPILER_APPICON_NAME: AppIcon`;
    the icon set names exactly its three icons with their appearances.
R6. `scripts/public-scrub.py` admits a binary only when its path, sha256 and byte count equal an
    entry of `scripts/brand-files.json` at the root it scans, in the tree and in history; it still
    searches an admitted binary for every private literal; it refuses every other binary as before;
    a list it cannot read stops it with exit 2, by name.
R7. `REUSE.toml` gives exactly the listed paths the brand source's licence by one override
    annotation, with its text in `LICENSES/LicenseRef-DeckStreak-Brand.txt`.
R8. Every screen's layout and behaviour, the harness's catalog and project spec, the bot and the
    Mini App are byte-unchanged; no Swift file changes.

## 3. Acceptance criteria of SPEC-579

| id | criterion | decided by |
|---|---|---|
| A1 | every listed brand file matches its entry: present, byte for byte, its kind and its pixel sizes (R1, R2) | `test_brand_files.py` |
| A2 | a listed file that is missing is refused (R2) | `test_brand_files.py` |
| A3 | a listed file that differs by one byte is refused (R2) | `test_brand_files.py` |
| A4 | a file whose pixel sizes differ from its entry is refused (R2) | `test_brand_files.py` |
| A5 | a PNG carrying a text or a time chunk is refused (R2) | `test_brand_files.py` |
| A6 | a planted complete set reads green, the control for A2 to A5 (R2) | `test_brand_files.py` |
| A7 | the README opens with its title, then the hero from a listed file with alt text `DeckStreak`, then its paragraph (R4) | `test_brand_files.py` |
| A8 | a README with the image above the title is refused (R4) | `test_brand_files.py` |
| A9 | a README whose image is not a listed committed file is refused (R4) | `test_brand_files.py` |
| A10 | the app target names the catalog and its icon set, and the icon set names its three icons (R5) | `test_brand_files.py` |
| A11 | an icon set missing an icon is refused (R5) | `test_brand_files.py` |
| A12 | an app target without the icon setting is refused (R5) | `test_brand_files.py` |
| A13 | the manifest meets the install criteria from the static files, and the head links all three (R3) | `manifest.test.ts` |
| A14 | a manifest missing an icon entry is refused (R3) | `manifest.test.ts` |
| A15 | a listed binary passes the tree scan and the history scan (R6) | `test_public_scrub.py` |
| A16 | other bytes at a listed path are refused as binary (R6) | `test_public_scrub.py` |
| A17 | listed bytes at an unlisted path are refused as binary (R6) | `test_public_scrub.py` |
| A18 | a private literal inside a listed binary is still found (R6) | `test_public_scrub.py` |
| A19 | a list the scrub cannot read stops it by name (R6) | `test_public_scrub.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_every_listed_file_matches_its_entry
A2: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_missing_file_is_refused
A3: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_changed_byte_is_refused
A4: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_pixel_size_other_than_the_lists_is_refused
A5: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_text_or_time_chunk_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_complete_planted_set_reads_green
A7: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_readme_opens_with_the_title_then_the_hero
A8: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_an_image_above_the_title_is_refused
A9: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_an_image_that_is_not_a_listed_file_is_refused
A10: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_the_app_target_names_the_catalog_and_its_icon_set
A11: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_an_icon_set_missing_an_icon_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_brand_files.py -k test_a_target_without_the_icon_setting_is_refused
A13: pnpm exec vitest run web/app/src/lib/manifest.test.ts -t "the manifest meets the install criteria"
A14: pnpm exec vitest run web/app/src/lib/manifest.test.ts -t "a manifest missing an icon entry is refused"
A15: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_listed_binary_passes_the_tree_and_history_scans
A16: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_listed_path_with_other_bytes_is_refused_as_binary
A17: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_listed_bytes_at_an_unlisted_path_are_refused_as_binary
A18: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_private_literal_inside_a_listed_binary_is_still_found
A19: python3 -m unittest discover -s scripts/tests -p test_public_scrub.py -k test_a_malformed_list_stops_the_scrub_by_name
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/brand-files.json` | brand | new: the list (R2) |
| `scripts/public-scrub.py` | scrub | changed: the admission (R6) |
| `scripts/tests/test_public_scrub.py` | tests | changed: class `AListedBrandBinaryIsAdmitted`, A15 to A19 |
| `scripts/tests/test_brand_files.py` | tests | new: A1 to A12 |
| `web/app/static/favicon.svg` | web | new (R1) |
| `web/app/static/favicon.ico` | web | new (R1) |
| `web/app/static/apple-touch-icon.png` | web | new (R1) |
| `web/app/static/icon-192.png` | web | new (R1) |
| `web/app/static/icon-512.png` | web | new (R1) |
| `web/app/static/icon-maskable-512.png` | web | new (R1) |
| `web/app/static/manifest.webmanifest` | web | changed: the `icons` member (R3) |
| `web/app/src/app.html` | web | changed: the `favicon.svg` link (R3) |
| `web/app/src/lib/manifest.test.ts` | tests | changed: A13 reads the static files; A14 added |
| `web/app/src/lib/icon.ts` | web | removed (R3) |
| `web/app/src/routes/apple-touch-icon.png/+server.ts` | web | removed (R3) |
| `web/app/src/routes/icon-192.png/+server.ts` | web | removed (R3) |
| `web/app/src/routes/icon-512.png/+server.ts` | web | removed (R3) |
| `ios/App/Assets.xcassets/Contents.json` | app | new (R5) |
| `ios/App/Assets.xcassets/AppIcon.appiconset/Contents.json` | app | new (R1, R5) |
| `ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-1024.png` | app | new (R1) |
| `ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-dark-1024.png` | app | new (R1) |
| `ios/App/Assets.xcassets/AppIcon.appiconset/AppIcon-tinted-1024.png` | app | new (R1) |
| `ios/app.yml` | app | changed: the catalog source and the icon setting (R5) |
| `docs/assets/deckstreak-hero-tide-light.gif` | docs | new (R1) |
| `README.md` | docs | changed: the hero line and its blank line (R4) |
| `REUSE.toml` | licensing | changed: the brand files' annotation (R7) |
| `LICENSES/LicenseRef-DeckStreak-Brand.txt` | licensing | new (R7) |
| `scripts/mutation-rows.d/S57900-S57999.json` | rows | new: S57901 to S57905 |
| `docs/red-first/SPEC-579.md` | records | new |
| `docs/specs/SPEC-579-the-deckstreak-logo-enters-the-tree-byte-for-byte-from-the-brand-source-as-the-web-icons-the-app-icon-set-and-the-readme-hero.md` | docs | new: this file |
| `docs/decisions/ADR-598-the-deckstreak-logo-enters-the-tree-byte-for-byte-admitted-by-a-pinned-list-the-public-scrub-reads.md` | docs | new |
| `docs/schematics/brand-files.md` | docs | new |
| `docs/specs/SPEC-033-the-public-repository-is-re-seeded-clean.md` | docs | amended insert-only: the admission |
| `docs/specs/SPEC-350-the-web-study-screens-review-a-card-from-the-engine-in-the-browser-in-the-sandboxed-frame-by-touch-keys-or-a-remote.md` | docs | amended insert-only: R17's icons |
| `docs/decisions/ADR-361-the-web-review-answers-only-the-card-it-showed-and-the-frame-stays-sealed.md` | docs | amended insert-only: D16 superseded |
| `docs/decisions/ADR-018-license-and-relicensing.md` | docs | amended insert-only: the brand files' licence |
| `changelog.d/ds-logo-579.md` | changelog | new |

## 5. What this does NOT cover

- The Ember palette and the app's colour sets, which SPEC-578 delivers after this (#1043).
- The palette applied to any screen, and the native screens (#1044).
- A mode switch the apps do not have (#1045).
- The repository's social preview, a repository setting the brand source's social card serves
  (#1048).
- The bot and the Mini App, which are deprecated and gain no icon (#1046).
- A landing page and its icons (#59).
- A reduced-motion still for the README's hero (#1049).
- What only a device or a later CI run shows: each icon on a home screen and in the app's icon
  appearances, and the hero as GitHub renders it (#1047).

## 6. Risks

- **A brand file carrying metadata.** The copy program reads each file's chunks before the copy, and
  A1 reads them in the tree; a text, time or comment chunk stops the copy as a question, never a
  stripped copy.
- **The history scan meeting a listed binary under another first path.** Two listed icons share
  their bytes; both paths are listed with the same sha256, so either first path is admitted. A15
  holds the history scan.
- **A test that pins `app.html`'s head or the manifest.** The build lists every test that reads
  either file at its cut and runs each; A13's assertions are the install surface's.
- **The catalog failing to compile.** Read in CI's `apple-on-change` run (the app's tests and its
  unsigned archive) on this pull request.
- **A secret scanner reading a sha256 as a key.** The build's gitleaks range scan reads the list; a
  finding is a STOP to report, never a widened rule.
