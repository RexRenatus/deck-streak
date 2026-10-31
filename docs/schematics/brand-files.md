# Brand files: copied once, listed once, admitted and checked from the list

A data-flow schematic of SPEC-579 (ADR-598). Every `path:line` citation was read at dev `57cbb5e1`;
the new paths are SPEC-579's.

## 1. Where each brand file goes

```mermaid
flowchart LR
  SRC["the DeckStreak brand source, its export list and sha256 values"] --> COPY["the build's copy program: sha256 checked before and after each copy"]
  COPY --> WEB["web/app/static: favicon.svg, favicon.ico, apple-touch-icon.png, icon-192.png, icon-512.png, icon-maskable-512.png"]
  COPY --> APP["ios/App/Assets.xcassets/AppIcon.appiconset: Contents.json and three 1024-pixel icons"]
  COPY --> HERO["docs/assets/deckstreak-hero-tide-light.gif"]
  COPY --> LIST["scripts/brand-files.json: path, sha256, bytes, kind, pixels"]
  WEB --> MANIFEST["manifest.webmanifest icons: three entries"]
  WEB --> HEAD["app.html head: manifest, apple-touch-icon, svg icon"]
  APP --> SPEC["ios/app.yml: App/Assets.xcassets source and the AppIcon setting"]
  HERO --> README["README.md line 3, below the title"]
  LIST --> SCRUB["public-scrub.py: a listed binary is admitted, still searched for literals"]
  LIST --> CENSUS["test_brand_files.py: kind, pixels, metadata, README header, icon set"]
```

## 2. The scrub's decision for one binary

```mermaid
flowchart TD
  BLOB["a binary file or history blob"] --> KEY{"path, sha256 and bytes equal one list entry?"}
  KEY -->|yes| SEARCH["search for every private literal; no binary finding"]
  KEY -->|no| REFUSE["finding: binary; and search for every private literal"]
  LISTFILE["scripts/brand-files.json"] --> READ{"readable, each entry whole, no path twice?"}
  READ -->|no| STOP["exit 2, by name"]
  READ -->|yes| KEY
```

## 3. Each step, its proof, its red and its row

| step | what holds it | red planted | row |
|---|---|---|---|
| each file is the brand's, byte for byte, kind and pixels | A1, `test_brand_files.py` | at the base no list and no file exist | none |
| a missing file, a changed byte, other pixels, a text or time chunk | A2 to A5, over fixtures | each planted in a copy of the complete set | none |
| the complete set | A6, the control | none | none |
| the README's title, hero and paragraph in order | A7 to A9 | at the base line 3 is the paragraph; planted: the image above the title, an image that is not listed | S57904 (the hero's path) |
| the app target names the catalog and its icon set | A10 to A12 | at the base no catalog line and no setting; planted: an icon removed, the setting removed | S57905 (the icon setting) |
| the manifest, the static icons and the head links | A13, A14, `manifest.test.ts` | at the base the icons are endpoints and two entries; planted: a manifest without the maskable entry | none (web data: no rows table) |
| a listed binary passes both scans | A15, `test_public_scrub.py` | at the base every binary is refused | none |
| other bytes at a listed path, listed bytes at another path | A16, A17 | not red: kept refusals, pinned | S57901 (admission by path alone), S57902 (admission by content alone) |
| a literal inside a listed binary | A18 | not red: the kept literal search, pinned | S57903 (the admitted branch skips the search) |
| a list that cannot be read | A19 | at the base the scrub ignores the list | none |

## 4. What stays as it is

Every screen, the harness's catalog and its project spec (`ios/project.yml:34`, `:48`), the bot and
the Mini App, `csp.test.ts`'s page policy, and the scrub's handling of every unlisted binary.

## 5. Amended by SPEC-580: the hero's reduced-motion still

The hero is no longer README line 3 alone: lines 3 to 6 are a picture whose reduced-motion source is
the still and whose image is the GIF. The still enters by the same copy route as every brand file.

```mermaid
flowchart LR
  POSTER["the brand source's light poster"] --> COPY["copy program: sha256 before and after"]
  COPY --> STILL["docs/assets/deckstreak-hero-poster-tide-light.png"]
  COPY --> LIST["scripts/brand-files.json: the still's entry"]
  STILL --> PICTURE["README.md lines 3 to 6: source (reduced motion) and img (the gif)"]
  LIST --> CENSUS["test_brand_files.py: the still's pin, the picture's two paths"]
```

| step | criteria | red at the base | row |
|---|---|---|---|
| the picture serves the still to reduced motion and the gif otherwise | SPEC-580 A10, A12 | dev's line 3 is a bare image; planted: a hero with no reduced-motion source | S58001, S58002, S57904 (now the `<img>` line) |
| the still is the brand source's poster, byte for byte | SPEC-580 A11, A13 | no still at dev; planted: the still missing, or other bytes | S58003 |
