# SPEC-593: every screen reads its colours from the Tide roles in both modes, and each client keeps one colour-mode rule

- **Issue:** #1044 and #1045. **Context(s):** the web app design tokens, screens and page head
  (`web/app`), the iPhone and iPad app asset catalog and views (`ios/App`), `scripts/tests` and
  `docs/`.
- **Decided by:** ADR-615 (every screen reads its colours from the Tide roles, and the token file
  adds card, muted-text and border values for both modes); ADR-616 (one colour-mode rule per
  client); ADR-597 (the Tide palette is a mode tier in the token file both apps copy), whose clause
  that no screen is re-coloured ADR-615 changes for this delivery; ADR-028 (the token file and its
  in-repo compile).
- **Schematic:** `docs/schematics/tide-screens.md`, new.
- **Status:** delivered by two pull requests into dev: the web part by the pull request that adds
  this file, with its tests and `docs/red-first/SPEC-593.md`, and the native part (section 7) by a
  second pull request cut after the first merges. **Mutation band:** S59300-S59399. **Models:**
  none.

## 1. The problem, measured

Measured at the dev commit this SPEC is cut on, each figure with its command:

- The web screens already draw through roles: 32 lines hold a role class in 23 `.svelte` files
  (`git grep -c -E "(bg|text|border|ring|decoration|outline|fill|stroke)-(background|foreground|card|card-foreground|muted|muted-foreground|primary|link|ring|border)([^-a-z]|$)" -- "web/app/src/*.svelte"`,
  summed). `bg-card` sits on 17 lines in 17 files, 13 of them with `text-card-foreground` on the
  same line, and 4 of the 17 also carry a `border` class (`git grep -n "bg-card" -- "web/app/src/*.svelte"`,
  each line read for the two classes).
- A colour literal (a hex value, `rgb(`, `hsl(` or `oklch(`) appears on 1 line in 1 `.svelte` file
  under `web/app/src`, and it is an issue number in a comment, not a colour.
- Three sites name a role the token file lacks, or one it never draws as text: `bg-muted`
  (`AscendantChip.svelte` line 8), `border-primary` (`CourseLadder.svelte` line 29) and `text-link`
  (`TierAnimation.svelte` line 41).
- Each `color` role compiles to `var(--tg-theme-<parameter>, <hex>)`, whose fallback is a raw
  colour of the Telegram default light palette, so outside Telegram every page paints light in
  either mode. `layout.css` sets `color-scheme: var(--tg-color-scheme, light)` (line 28), applies
  `border-border` to every element (line 31) and declares a `dark` variant on `.dark` (line 13)
  that nothing sets; `app.html` holds no script; `sveltekit.config.js` sets a content security
  policy in `hash` mode; `web/app/src/lib/csp.test.ts` pins that shell free of any script, inline or
  by URL, and the script sources of that policy (SPEC-400 R7).
- The native app has 14 SwiftUI view types in 12 files under `ios/App/Sources`, 13 `View` structs
  and one `UIViewRepresentable` (`git grep -n -E "struct [A-Za-z]+ *:[^{]*View" -- ios/App/Sources`),
  and none reads a colour set: the only colour styles are `.foregroundStyle(.secondary)` in
  `VoicePickerView.swift` and `.background(.bar)` in `AnswerBar.swift`.
- The six Tide values cover a background, an ink and an accent in each mode. A card, muted text
  and a border have no Tide value, and the token grey `#707579` on the Tide light background
  measures 4.3311:1, under 4.5:1.
- With the card values of R1, the card fill on the background measures 1.2224:1 dark and 1.0752:1
  light, under 3:1, so a card edge meets 1.4.11 only by a border.

Each ratio here and in R3 is the WCAG relative-luminance formula over the 8-bit sRGB values (each
channel over 255, linear at or below 0.04045 by division by 12.92, else raised to 2.4 after
`(c + 0.055) / 1.055`; `L = 0.2126 R + 0.7152 G + 0.0722 B`; ratio `(L1 + 0.05) / (L2 + 0.05)`),
compared unrounded.

## 2. Requirements

R1. The `modes` tier of `tokens.json` holds six roles in each mode, `background`, `ink`, `accent`,
`card`, `muted` and `border`, each an alias of a raw colour in `ref.tide`, with the values of
ADR-615 D1:

| mode | background | ink | accent | card | muted | border |
|---|---|---|---|---|---|---|
| dark | `#0f161d` | `#f4f7fa` | `#34c6c0` | `#21282f` | `#8f9499` | `#74797e` |
| light | `#f4f7fa` | `#0f161d` | `#0b6a67` | `#ffffff` | `#5f656a` | `#82868c` |

R2. The compile emits twelve custom properties, `--tide-<mode>-<role>`. Each `color` role names its
Tide role in the `tide` member of its `deck-streak` extension, beside `telegram` (`background` to
`background`, `foreground` and
`card-foreground` to `ink`, `card` to `card`, `muted-foreground` to `muted`, `border` to `border`,
and `link` and `ring` to `accent`), and the compile emits the light fallbacks in `:root` and the
dark fallbacks under `.dark`, each still inside its role Telegram variable. A role outside the six,
a role missing from either mode, or a `color` role with no Tide role fails the compile by name. The
five Telegram raw colours leave `ref`.

R3. Every declared text pair meets 4.5:1 (WCAG 2.2 1.4.3) and every declared non-text pair 3:1
(1.4.11), in both modes:

| pair | bar | dark | light |
|---|---|---|---|
| ink on background | 4.5 | 16.94 | 16.94 |
| ink on card | 4.5 | 13.86 | 18.22 |
| muted on background | 4.5 | 5.95 | 5.49 |
| muted on card | 4.5 | 4.87 | 5.91 |
| accent on background | 4.5 | 8.66 | 5.97 |
| accent on card | 4.5 | 7.09 | 6.42 |
| background on ink | 4.5 | 16.94 | 16.94 |
| border on background | 3 | 4.15 | 3.40 |
| border on card | 3 | 3.39 | 3.66 |
| focus ring (the accent) on background | 3 | 8.66 | 5.97 |
| focus ring (the accent) on card | 3 | 7.09 | 6.42 |

R4. The three off-role sites take existing roles (ADR-615 D3): `bg-muted` becomes `bg-card`,
`border-primary` becomes `border-ring`, and `text-link` becomes `text-card-foreground` with its
underline in `decoration-link`.

R5. Every card carries the border role: every class list that holds `bg-card` also holds a
`border` width class, whose colour is the border role through the base rule at `layout.css` line
31. The card edge meets 1.4.11 by that border (R3: border on card and border on background, both
modes), and the card fill is decorative under 1.4.11.

R6. Every colour class in a `.svelte` file under `web/app/src` names a declared `color` role of
`tokens.json`, and no such file holds a colour literal.

R7. The asset catalog holds six colour sets, `TideBackground`, `TideInk`, `TideAccent`, `TideCard`,
`TideMuted` and `TideBorder`, each written by program from the token file, each holding its token
light value as its universal entry and its dark value as its `luminosity` `dark` entry.

R8. The web colour mode (ADR-616 D1) is a three-way setting, `system`, `light` or `dark`, kept on
the device under the local-storage key `deckstreak.colour-mode` and changed on a settings screen.
Until the app mounts, and while the choice is `system` or nothing is stored, the stylesheet alone
sets the mode: a `prefers-color-scheme` rule in `layout.css` gives every colour role the dark
fallback the compile emits for it under `.dark`, unless the browser reports
`prefers-color-scheme: light` or the root element carries the `light` class. So a browser with no
preference paints dark first, one that reports light paints the light fallbacks of `:root`, and
the page follows a system change with no listener. After the app mounts, the root layout calls
`colour-mode.ts`, which takes the stored choice from the key, sets the `light` or the `dark` class
on the root element for a stored `light` or `dark`, and clears both for `system` or for no stored
value. No script is added to the page for any of this (R9). The stylesheet takes `color-scheme`
from `--tg-color-scheme`, with the mode as its fallback.

R9. No script enters the page for the mode: `app.html` and the content security policy in
`sveltekit.config.js` stay unchanged, so the SPEC-400 R7 shell and page-policy tests in
`web/app/src/lib/csp.test.ts` hold unchanged.

R10. Inside Telegram the chat theme keeps painting every role: the SPEC-028 A8 and A9 tests hold
unchanged.

R11 (the native part, section 7). Every native view reads its colours only through one `Color`
extension that names the six colour sets, and no view or property list sets a colour scheme
(`preferredColorScheme`, `overrideUserInterfaceStyle` or `UIUserInterfaceStyle`), so the app
follows the system appearance (ADR-616 D2).

## 3. Acceptance criteria of SPEC-593 (the web part)

| id | criterion | decided by |
|---|---|---|
| A1 | the Tide tier compiles to twelve custom properties carrying the tokens values, in both modes (R1, R2) | `tokens.test.ts` |
| A2 | a role outside the six, a role missing from either mode, or a colour role with no Tide role fails the compile by name (R2) | `tokens.test.ts` |
| A3 | every declared text pair meets 4.5:1 and every non-text pair 3:1 in both modes, and a planted low-contrast value is refused (R3) | `tokens.test.ts` |
| A4 | every colour role falls back to its Tide role in each mode, light in `:root` and dark under `.dark`, inside its Telegram variable (R2) | `tokens.test.ts` |
| A5 | each of the six colour sets holds its token light and dark values (R7) | `test_tide_tokens.py` |
| A6 | every colour class in a screen names a declared role, and a planted off-role class or colour literal is refused (R4, R6) | `test_tide_screens.py` |
| A7 | until the app mounts the stylesheet paints the system mode, dark unless the browser reports light, its `layout.css` rule giving every colour role the dark fallback the compile emits; after mount a stored light or dark sets its class on the root element and `system` clears it; a planted rule that misses a role, and a planted apply that ignores the stored choice, are each refused (R8) | `colour-mode.test.ts` |
| A8 | no script enters the page for the mode: the shell carries no script and the page policy admits nothing new (R9) | the SPEC-400 tests in `csp.test.ts`, unchanged |
| A9 | the Telegram mapping and its contrast proof are unchanged (R10) | the SPEC-028 A8 and A9 tests |
| A13 | every class list that holds `bg-card` holds a `border` width class, and a planted card without the border is refused (R5) | `test_tide_screens.py` |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "the Tide palette compiles to twelve custom properties in both modes"
A2: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a role outside the six or missing from either Tide mode fails the compile by name"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every declared Tide pair meets its contrast bar in both modes"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a planted low-contrast Tide value is refused"
A4: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour role falls back to its Tide role in each mode"
A5: python3 -m unittest discover -s scripts/tests -p test_tide_tokens.py -k test_each_colour_set_holds_its_tokens_values
A6: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_every_screen_colour_class_is_a_declared_role
A6: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_a_planted_off_role_class_or_literal_is_refused
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "the stylesheet paints the system mode first, dark unless the browser reports light"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "a planted system rule that misses a colour role is refused"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "the stored choice is applied to the root element after mount"
A7: pnpm exec vitest run web/app/src/lib/settings/colour-mode.test.ts -t "a planted apply that ignores the stored choice is refused"
A8: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page shell carries no script, inline or by URL"
A8: pnpm exec vitest run web/app/src/lib/csp.test.ts -t "the page policy admits WebAssembly compilation and nothing else new"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour token resolves to a Telegram theme variable with a fallback"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every text and background pair meets AA contrast in both Telegram palettes"
A13: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_every_card_carries_the_border_role
A13: python3 -m unittest discover -s scripts/tests -p test_tide_screens.py -k test_a_planted_card_without_the_border_is_refused
```

A10 to A12 are the native part, delivered by the second pull request (section 7). A13 is carried by
the web part, beside A6 in the same census module.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-593-every-screen-reads-its-colours-from-the-tide-roles-in-both-modes-and-each-client-keeps-one-colour-mode-rule.md` | web | new: this SPEC |
| `docs/decisions/ADR-615-every-screen-reads-its-colours-from-the-tide-roles-and-the-token-file-adds-card-muted-text-and-border-values-for-both-modes.md` | web | new |
| `docs/decisions/ADR-616-the-web-app-follows-the-system-colour-scheme-with-a-stored-setting-applied-after-mount-and-the-app-follows-the-system-appearance.md` | web | new |
| `docs/schematics/tide-screens.md` | web | new: the flow from the token file to each client, its screens and its modes |
| `docs/DESIGN_SYSTEM.md` | web | changed: the card, muted and border roles, the card border rule and the colour-mode rule |
| `docs/red-first/SPEC-593.md` | web | new: the web part reds and greens |
| `scripts/mutation-rows.d/S59300-S59399.json` | web | new: rows S59301 to S59304 |
| `changelog.d/tide-screens-593.md` | web | new |
| `web/app/src/lib/design/tokens.json` | web | changed: the new `ref.tide` neutral steps, six roles per mode, each colour role naming its Tide role, the five Telegram raw colours removed (R1, R2) |
| `web/app/scripts/build-tokens.ts` | web | changed: emits twelve custom properties and the per-mode fallbacks, and refuses a role with no Tide role (R2) |
| `web/app/src/lib/design/tokens.test.ts` | web | changed: A1 to A4; of its three SPEC-578 tests, the closure test plants nothing and asserts a literal list of the twelve R1 and R2 properties, and the other two plant `glow` where they planted `muted` |
| `web/app/src/routes/layout.css` | web | changed: the `prefers-color-scheme` system rule that paints the dark fallbacks unless the browser reports light, and `color-scheme` per mode under the Telegram scheme (R8) |
| `web/app/src/routes/+layout.svelte` | web | changed: applies the stored colour mode after mount (R8) |
| `web/app/src/app.html` | web | unchanged: no script enters the page shell (R9) |
| `web/app/sveltekit.config.js` | web | unchanged: the page policy admits nothing new (R9) |
| `web/app/src/lib/settings/colour-mode.ts` | web | new: the key, the stored choice and the class it sets on the root element after mount (R8) |
| `web/app/src/lib/settings/colour-mode.test.ts` | web | new: A7 |
| `web/app/src/lib/csp.test.ts` | web | unchanged: its SPEC-400 tests decide A8 (R9) |
| `web/app/src/lib/settings/ColourModeSetting.svelte` | web | new: the three-way control (R8) |
| `web/app/src/routes/settings/+page.svelte` | web | new: the settings screen that holds the control (R8) |
| `web/app/src/lib/routes.ts` | web | changed: the settings screen joins the route list the accessibility audit covers (R8) |
| `web/app/src/lib/study/card-health.test.ts` | web | changed: the settings screen joins its frozen list of route files (R8) |
| `web/app/src/lib/startapp.test.ts` | web | changed: the settings screen joins its list of screens opened by their paths (R8) |
| `web/app/src/routes/+page.svelte` | web | changed: a link to the settings screen, and the border class on its card (R5, R8) |
| `web/app/messages/en.json` | web | changed: the setting labels (R8) |
| `web/app/messages/es.json` | web | changed: the setting labels (R8) |
| `web/app/messages/fr.json` | web | changed: the setting labels (R8) |
| `web/app/messages/ja.json` | web | changed: the setting labels (R8) |
| `web/app/messages/ko.json` | web | changed: the setting labels (R8) |
| `web/app/messages/zh-Hans.json` | web | changed: the setting labels (R8) |
| `web/app/messages/zh-Hant.json` | web | changed: the setting labels (R8) |
| `web/app/src/lib/level/AscendantChip.svelte` | web | changed: `bg-muted` to `bg-card`, with the border class (R4, R5) |
| `web/app/src/lib/progress/CourseLadder.svelte` | web | changed: `border-primary` to `border-ring` (R4) |
| `web/app/src/lib/ladder/TierAnimation.svelte` | web | changed: `text-link` to `text-card-foreground` with `decoration-link`, and the border class on its card (R4, R5) |
| `web/app/src/lib/settings/SignInMethods.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/badges/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/capture/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/insights/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/law/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/level/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/progress/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/records/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/score/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/streak/+page.svelte` | web | changed: the border class on its card (R5) |
| `web/app/src/routes/wallet/+page.svelte` | web | changed: the border class on its card (R5) |
| `ios/App/Assets.xcassets/TideCard.colorset/Contents.json` | web | new: written by program from the token file (R7) |
| `ios/App/Assets.xcassets/TideMuted.colorset/Contents.json` | web | new: written by program from the token file (R7) |
| `ios/App/Assets.xcassets/TideBorder.colorset/Contents.json` | web | new: written by program from the token file (R7) |
| `scripts/tests/test_tide_tokens.py` | web | changed: A5 over the six colour sets |
| `scripts/tests/test_tide_screens.py` | web | new: A6 and A13 |
| `web/app/src/lib/settings/ColourModeSetting.test.ts` | web | new: kill tests for `web/app/src/lib/settings/ColourModeSetting.svelte` (R8) |
| `web/app/src/lib/settings/ColourModeSetting.server.test.ts` | web | new: kill tests for `web/app/src/lib/settings/ColourModeSetting.svelte` before it mounts (R8) |
| `web/app/src/routes/settings.test.ts` | web | new: kill tests for `web/app/src/routes/settings/+page.svelte` and its link in `web/app/src/routes/+page.svelte` (R8) |
| `web/app/src/routes/layout-colour-mode.test.ts` | web | new: kill tests for `web/app/src/routes/+layout.svelte` (R8) |
| `scripts/mutation-equivalent.d/miniapp.json` | web | changed: six equivalence records for colour-mode.ts (R8) |
| `docs/specs/planned/SPEC-057-every-surviving-mutant-is-killed-or-recorded-equivalent-before-the-first-mutation-gated-release.md` | web | changed: the miniapp campaign row refilled from its battery pair (R8) |
| `ios/App/Sources/TideColours.swift` | native | new: one `Color` extension naming the six colour sets (R11) |
| `ios/App/Sources/AccountView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/AnswerBar.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/CardFaceView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/CardHealthView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/DataView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/DeckListView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/ReviewChrome.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/ReviewView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/SyncView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/UndoControl.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/UndoDialogView.swift` | native | changed: colours through the Tide extension (R11) |
| `ios/App/Sources/VoicePickerView.swift` | native | changed: colours through the Tide extension (R11) |
| `scripts/tests/test_tide_native.py` | native | new: A10 to A12 |
| `docs/specs/SPEC-593-every-screen-reads-its-colours-from-the-tide-roles-in-both-modes-and-each-client-keeps-one-colour-mode-rule.md` | native | changed: A10 to A12 move into section 3 and its acceptance block |
| `docs/red-first/SPEC-593.md` | native | changed: the native part reds and greens, appended |
| `scripts/mutation-rows.d/S59300-S59399.json` | native | changed: row S59305 |
| `changelog.d/tide-native-593.md` | native | new |

Mutation rows, band S59300-S59399. The web part owes S59301 to S59304, each a one-byte change to
a value the A5 census holds equal (the `TideCard` dark entry, the `TideMuted` light entry, the
`TideBorder` dark entry, and the light `muted` value in `tokens.json`), each killed by
`test_each_colour_set_holds_its_tokens_values`. The native part owes S59305, a planted
`.preferredColorScheme(.light)` in one view, killed by the A11 test. A13 owes no row: its planted
card is the control inside its own test, as the planted class of A6 is. S59300 and S59306 to
S59399 stay unused.

## 5. What this does NOT cover

- The Telegram mapping inside the Mini App: there the chat theme keeps painting every role, and the
  Tide roles serve only as fallbacks, until the Mini App retires (#1046).
- A role beyond the six, such as an error or a success colour: no measured screen line needs one,
  and the first screen that does opens its own issue against the palette (#1044).
- Text on an accent fill, or any pair the token file does not declare: A3 judges the declared
  pairs only, and a new pair joins the token file first (#1044).
- An in-app colour-mode setting on the iPhone and iPad app: ADR-616 D2 rejects it, and a learner
  need for one is raised as its own issue (#1045).
- The rendered colours of a screen: section 8 names who checks them and where the check is
  recorded (#1044).

## 6. Risks

- **A card the census misses.** A card drawn from a component property rather than a class
  literal escapes a class census. A13 reads every class list in every `.svelte` file under
  `web/app/src`, and its planted card shows it can go red; a new card component is read by the same
  census.
- **A stored choice opposite the system.** Until the app mounts the stylesheet paints the system
  mode, so such a choice shows the other mode for the first frames; section 8 names who checks it.
  A system rule out of step with the compile would paint a wrong dark value: A7 holds every role of
  the `layout.css` rule equal to the dark fallback the compile emits, and refuses a planted rule
  that misses one.
- **The colour scheme outranking Telegram.** A scheme set on the root element would outrank the
  Telegram scheme inside the Mini App; R8 keeps `color-scheme` in the stylesheet under
  `--tg-color-scheme`, and A9 holds the Telegram mapping.
- **A survivor in `build-tokens.ts`.** The compile changes, and the web mutation stage mutates the
  changed file whole; each new branch, a mode fallback and a refused role, has its own killing test
  in `tokens.test.ts` (A2, A4).
- **A Swift compile red.** The native part compiles only in continuous integration; a compile or
  simulator red is read there by its test name on the red push, and the census of A10 to A12 runs
  wherever Python runs.
- **An open pull request on the same lines.** Some files this delivery changes are changed by open
  pull requests too (a settings component, two native views and the message files); each part is
  cut on current dev and rebases onto any of them that lands first.

## 7. Delivered by the native pull request

| id | criterion | delivered by |
|---|---|---|
| A10 | every native view reads its colours only through the Tide extension, and a planted `Color(red:green:blue:)` in a view is refused (R11) | the native pull request, `test_tide_native.py` |
| A11 | no view or property list overrides the colour scheme, and a planted `.preferredColorScheme(.light)` is refused (R11) | the native pull request, `test_tide_native.py` |
| A12 | the Tide extension names every Tide colour set the catalog holds, all six, and a set it does not name is refused (R7, R11) | the native pull request, `test_tide_native.py` |

The native pull request changes the views listed in section 4 and adds `TideColours.swift`: the
root of each screen takes `TideBackground`, its text `TideInk` or `TideMuted`, its panels
`TideCard`, its rules `TideBorder` and its tint `TideAccent` (ADR-615 D4). In the commit that adds
`test_tide_native.py`, it moves these three rows into section 3 and these lines into its acceptance
block:

```text
A10: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_every_view_reads_colours_only_through_the_tide_extension
A10: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_a_planted_colour_literal_in_a_view_is_refused
A11: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_no_view_or_property_list_overrides_the_colour_scheme
A11: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_a_planted_scheme_override_is_refused
A12: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_the_tide_extension_names_every_colour_set_in_the_catalog
A12: python3 -m unittest discover -s scripts/tests -p test_tide_native.py -k test_a_colour_set_the_extension_does_not_name_is_refused
```

The Swift change itself is compiled in continuous integration, and its compile is read there by
name; no census here stands in for it.

## 8. What no repository test decides

The tests decide the token values, every declared pair, every class and Swift colour reference,
the system rule in `layout.css` and the stored choice applied after mount. They cannot decide what
a screen shows: the rendered colours of each screen in each mode in a browser, on the simulator and
on a device; how many frames a stored choice opposite the system shows the system mode before the
app mounts; and each appearance on a real display. A maintainer checks
each web screen in both modes after the web pull request merges, and each native screen in both
appearances on a device after the native pull request merges, and records each check, with what
was seen, as a closing section of `docs/red-first/SPEC-593.md`.
