# SPEC-623: the architecture code map names every crate and ios and both maps name the telegram surfaces only as deprecated

- **Wave:** none; the architecture lane of the documentation cleanup ADR-589 asks for.
  **Issue:** none. **Context(s):** none changes (two documents and their census).
- **Decided by:** ADR-645, ADR-589 (the web app and the iPhone and iPad app are DeckStreak's
  surfaces; the Telegram Mini App and bot are deprecated).
- **Schematic:** `docs/schematics/architecture-code-map.md`.
- **Status:** judged by this delivery, with its tests and `docs/red-first/SPEC-623.md`.
- **Mutation band:** S62300-S62399.

## 1. The problem, measured

Every figure below was read at dev at the cut, the commit this delivery branched from.

| fact | figure | how it was read |
|---|---|---|
| F1 | `Cargo.toml` declares `members = ["crates/*"]`, and `crates/` holds 38 folders | `grep -n members Cargo.toml`; `git ls-tree --name-only` of `crates/`, counted |
| F2 | ARCHITECTURE.md's code map names 24 of the 38; the 14 it lacks are collection, companion, companion-slot, engine-core, ffi, fsrs7, mcp, media, push, search, study-day, sync, web-engine and xp; it has no `ios/` row; its `web/site/` row says planned, as the map's `landing` row does | the code spans of each code-map row's first cell, brace groups expanded, against F1 |
| F3 | `ios/` holds the app (`ios/App`), the harness (`ios/Harness`) and their tests, and the map's `ios-harness` row names only the harness | `git ls-tree --name-only` of `ios/`; the fence row read by line |
| F4 | outside code spans, ARCHITECTURE.md names a Telegram surface on 9 lines and docs/CONTEXT-MAP.md on 9 more (fence names and edge lists set aside), none beside the word deprecated; the Observability item names a Telegram alert path where `deploy/slo.json` declares the alert template's journal line (SPEC-585) | a case-insensitive match of `mini[\s-]?app`, `telegram`, `\bbots?\b` and `initdata` per line, code spans removed |
| F5 | no test or script reads ARCHITECTURE.md; three Rust tests and six scripts read docs/CONTEXT-MAP.md, each through the fence's names, paths and edge lists, the ownership register, one sentence and one row description that holds a fixed word | `git grep -l -F` over the tree for each page's name, then each reader's parsing lines |

## 2. Requirements

R1. ARCHITECTURE.md's code map names every folder under `crates/` exactly once, alone
(`crates/<name>/`) or in a brace group (`crates/{a,b}/`), and names no crate folder the tree lacks.
Each new row describes its crates by a short form of their own fence descriptions in
docs/CONTEXT-MAP.md and adds no claim the map does not make.

R2. The code map has one row whose path cell is `ios/`, naming the universal iPhone and iPad app
(`ios/App`) and the SwiftUI harness (`ios/Harness`) over the FFI crate (ADR-335).

R3. Every path the code map names exists in the tree, or its row says `(planned)` and the context
map's fence holds a path row under that path whose description says planned.

R4. ARCHITECTURE.md presents the web app and the iPhone and iPad app as DeckStreak's surfaces.
Outside the drawing, every unit that names the Telegram Mini App, the bot, Telegram or `initData`,
code spans set aside, also says deprecated (ADR-589). The Observability item names the alert route
`deploy/slo.json` declares (SPEC-585), not a Telegram one. Every other claim of a rewritten unit is
kept in substance.

R5. The drawing draws the web app and the iPhone and iPad app, each a labelled node with its edge
into the service, and no Telegram, bot or Mini App node or edge, since no install runs a Telegram
surface (ADR-601). Each edge it draws is backed by a line of ADR-335, ADR-336 or the code map,
quoted in ADR-645.

R6. In docs/CONTEXT-MAP.md every unit (a fence row's description; outside the fence, a table row,
a list item or a paragraph), code spans and each fence row's name and edge list set aside, that
names the Telegram Mini App, the bot, Telegram or `initData` also says deprecated. The `api`,
`notifications`, `identity` and front-end descriptions say what serves the two apps, each claim
backed by a module path or an ADR.

R7. The fence row `ios-harness  ios/` keeps its name and path, and its description names the
iPhone and iPad app at `ios/App` and the harness at `ios/Harness`, each a folder in the tree,
written without backticks, since the fence is a code block.

R8. Every path row of the fence names a path in the tree or says planned.

R9. Nothing else changes. The fence's names, path columns and edge lists, every table line from
the ownership register to the end of the map, the sentence that counts v9's tables, the `landing`
and `deck-streak-migration` rows, and the text the S36007 mutation row anchors on stay byte for
byte. Renames (the `miniapp` and `ios-harness` names) are a later delivery's, as ADR-589 keeps the
earlier identifiers until then.

## 3. Acceptance criteria of this delivery

Each criterion is one test of `scripts/tests/test_architecture_map.py`.

| id | criterion | decided by | test |
|---|---|---|---|
| A1 | The code map names every crate folder exactly once and no folder the tree lacks; planted maps missing `xp`, naming `crates/ghost/`, and naming `crates/xp/` twice are each refused (R1) | ADR-645 D1 | `test_the_code_map_names_every_crate_folder_exactly_once` |
| A2 | The code map has one `ios/` row naming `ios/App` and `ios/Harness`, each a folder in the tree (R2) | ADR-645 D1 | `test_the_code_map_has_a_row_for_the_iphone_and_ipad_app_folder` |
| A3 | Every code-map path exists, or its row says planned and the map's fence plans it; a planted row naming an absent path without planned is refused (R3) | ADR-645 D1 | `test_every_code_map_path_exists_or_is_planned_in_the_context_map` |
| A4 | Every ARCHITECTURE.md unit outside the drawing that names a Telegram surface says deprecated; a bare unit and a pair of words wrapped across two lines are refused, and a unit that says deprecated and a code span alone are passed (R4) | ADR-645 D2 | `test_the_architecture_names_a_telegram_surface_only_beside_deprecated` |
| A5 | The drawing labels the web app and the iPhone and iPad app and draws no Telegram surface; the drawing dev held at the cut, planted as a copy, is refused (R5) | ADR-645 D3 | `test_the_drawing_shows_the_two_apps_and_no_telegram_surface` |
| A6 | Every context-map unit that names a Telegram surface says deprecated, fence names and edge lists set aside; a planted row named `deck-streak-bot` that says deprecated is passed and one naming Telegram bare is refused (R6) | ADR-645 D2, D4 | `test_the_context_map_names_a_telegram_surface_only_beside_deprecated` |
| A7 | The map's `ios-harness` row names `ios/App` and `ios/Harness`, each in the tree; a planted row naming an absent `ios/Ghost` is refused (R7) | ADR-645 D4 | `test_the_context_maps_ios_row_names_the_app_and_the_harness` |
| A8 | Every path row of the fence names a path in the tree or says planned; a planted row for an absent path without planned is refused (R8) | ADR-645 D1 | `test_every_context_map_path_row_exists_or_is_planned` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_code_map_names_every_crate_folder_exactly_once
A2: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_code_map_has_a_row_for_the_iphone_and_ipad_app_folder
A3: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_every_code_map_path_exists_or_is_planned_in_the_context_map
A4: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_architecture_names_a_telegram_surface_only_beside_deprecated
A5: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_drawing_shows_the_two_apps_and_no_telegram_surface
A6: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_context_map_names_a_telegram_surface_only_beside_deprecated
A7: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_the_context_maps_ios_row_names_the_app_and_the_harness
A8: python3 -m unittest discover -s scripts/tests -p test_architecture_map.py -k test_every_context_map_path_row_exists_or_is_planned
```

With the two pages as dev held them, six tests read red, each an assertion failure that names
what it found: the 14 missing crate folders, the missing `ios/` row, the units without the word
deprecated on each page, the Telegram lines of the drawing, and the harness-only `ios-harness`
row. The other two guard what dev already holds, each with its planted refusal inside the test.

## 4. File manifest

| path | change | context |
|---|---|---|
| `docs/specs/SPEC-623-the-architecture-code-map-names-every-crate-and-ios-and-both-maps-name-the-telegram-surfaces-only-as-deprecated.md` | added (C1) | this SPEC |
| `docs/schematics/architecture-code-map.md` | added (C1) | the census data flow |
| `docs/decisions/ADR-645-a-census-holds-the-architecture-code-map-to-the-crate-folders-and-every-telegram-surface-is-named-beside-deprecated.md` | added (C1) | its decision |
| `changelog.d/docs-architecture-map-623.md` | added (C1) | the changelog fragment |
| `scripts/tests/test_architecture_map.py` | added (C2): A1 to A8 | the census |
| `docs/red-first/SPEC-623.md` | added (C2), appended at C4 | the red-first record |
| `ARCHITECTURE.md` | changed (C3): only cut lines 9-15 (the bird's eye paragraph), 18-28 (the drawing's body; its fence lines 17 and 29 kept), 37, 45 and 47 (the three code-map rows that name a Telegram surface), 67-69 (the Boundaries paragraph) and 74-75 (the Observability item), plus new code-map rows inserted between cut lines 34 and 53; every other cut line stays byte for byte and in order | the architecture page |
| `docs/CONTEXT-MAP.md` | changed (C3): only cut lines 15, 26, 34, 35, 52 and 53 (six fence rows' descriptions), 151-156 (layer item 3), 165 (the identity anti-corruption row), 168-171 (Upstream suppliers), 173-175 (The front end) and 261-265 (the `settings_kv` paragraph); every other cut line stays byte for byte and in order | the binding context map |
| `scripts/mutation-rows.d/S62300-S62399.json` | added (C3) | the band's script rows |

A cut line is a line number of the page as dev held it at the cut. No other path changes: no
crate, no manifest, no lockfile, no other test, and no landed record.

## 5. What this does NOT cover

- README, AGENTS, the docs index, the PRD and the guides: other lanes of the same cleanup own them
  (ADR-589).
- ADR-589's status line, which stays as it is (ADR-589).
- Renaming `miniapp` and `ios-harness`, which feed labels and tests: a rename delivery's (ADR-589).
- Removing the Telegram code from the tree: the removal delivery's, after native push carries the
  router (#640).
- Whether the landing site is still planned: its rows stay as they are (ADR-589).

## 6. Risks

- A pull request that adds a crate folder must add its code-map row in the same change, and open
  pull requests add crate folders now, so whichever lands second carries the other's row; the
  first test then names the folder the map lacks.
- The map's text is read by the tests F5 names. A description inside a fence row may change, but
  a name, a path column, an edge list or a register row may not, and no new description may hold
  the one word a sealed-sync census finds the reading context by.
- The unit rule reads the word deprecated anywhere in a unit, so a long unit could name a
  deprecated surface and a current one together; units here are kept short for that reason.

## 7. Mutation rows

Each row's killer is a test of `test_architecture_map`; `mutation-rows` proves them in CI.

| row | mutant | killed by |
|---|---|---|
| `S62301-THE-CODE-MAP-LOSES-A-CRATE` | a code-map row drops one crate folder | `test_the_code_map_names_every_crate_folder_exactly_once` |
| `S62302-THE-IOS-ROW-LOSES-ITS-PATH` | the `ios/` row's path cell changes | `test_the_code_map_has_a_row_for_the_iphone_and_ipad_app_folder` |
| `S62303-THE-BOT-ROW-LOSES-DEPRECATED` | the code map's bot row loses its word deprecated | `test_the_architecture_names_a_telegram_surface_only_beside_deprecated` |
| `S62304-THE-DRAWING-GAINS-A-TELEGRAM-NODE` | a Telegram node is added to the drawing | `test_the_drawing_shows_the_two_apps_and_no_telegram_surface` |
| `S62305-THE-MAP-BOT-ROW-LOSES-DEPRECATED` | the context map's bot fence row loses its word deprecated | `test_the_context_map_names_a_telegram_surface_only_beside_deprecated` |
| `S62306-THE-IOS-HARNESS-ROW-LOSES-THE-APP` | the `ios-harness` row loses `ios/App` | `test_the_context_maps_ios_row_names_the_app_and_the_harness` |
