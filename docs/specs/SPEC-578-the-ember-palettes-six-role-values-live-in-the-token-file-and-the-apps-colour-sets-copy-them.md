# SPEC-578: the Ember palette's six role values live in the token file, and the app's colour sets copy them

- **Issue:** #1043. **Context(s):** the web app's design tokens (`web/app`), the
  iPhone and iPad app's asset catalog (`ios/App`), `scripts/tests` and `docs/`.
- **Decided by:** ADR-597 (the Ember palette is a mode tier in the token file both apps copy, and
  no screen is re-coloured by it); ADR-028 (the token file and its in-repo compile); ADR-598 (the
  app's first asset catalog, which this delivery adds to).
- **Schematic:** `docs/schematics/ember-tokens.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-578.md`, cut after SPEC-579's pull request merges. **Mutation band:**
  S57800-S57899. **Models:** none (ADR-597 D7).

## 1. The problem, measured

Read at dev `57cbb5e1`, by `git show <dev>:<path>` and `grep -n`:

- `web/app/src/lib/design/tokens.json` (81 lines) is the one DTCG token file (SPEC-028 R5): a `ref`
  tier of five raw colours (lines 3 to 21) and a `color` tier of eight roles (lines 22 to 67), each
  naming a Telegram theme parameter. It holds no product palette.
- `web/app/scripts/build-tokens.ts` (181 lines) compiles only the `color` tier, the radius and the
  fonts (lines 133 to 176); a role with no Telegram parameter fails the compile (lines 145 to 149).
- `web/app/src/routes/layout.css` (58 lines) imports the compiled stylesheet (line 8); the page's
  scheme is Telegram's inside Telegram and light outside it (line 28).
- `web/app/src/lib/design/tokens.test.ts` (167 lines) holds SPEC-028 A8 and A9 over the `color`
  tier.
- The product app's project spec, `ios/app.yml` (84 lines), names no asset catalog at dev; SPEC-579
  adds `ios/App/Assets.xcassets` before this delivery is cut.
- `git grep` for a hex colour, `rgb(`, `hsl(`, `oklch(`, `Color(` or `UIColor` finds none in
  `web/app/src` outside `tokens.json` and none in `ios/App/Sources/`.
- The product palette, Ember, from the DeckStreak brand source: dark background `#151210`, ink
  `#faf6f2`, accent `#ff9a3c`; light background `#faf6f2`, ink `#151210`, accent `#a54d06`.
- WCAG 2.2 contrast of those pairs, by the formula `tokens.test.ts` uses: ink on background 17.35:1
  in both modes, accent on background 5.33:1 light and 8.83:1 dark. The token file's grey `#707579`
  on the Ember light background measures 4.33:1.

## 2. Requirements

R1. `tokens.json` holds the Ember raw colours as `ref.ember.neutral.50` (`#faf6f2`),
    `ref.ember.neutral.950` (`#151210`), `ref.ember.accent.400` (`#ff9a3c`) and
    `ref.ember.accent.700` (`#a54d06`), each a DTCG sRGB colour whose components agree with its hex.
R2. `tokens.json` holds a `modes` tier: `modes.ember.dark` and `modes.ember.light`, each with exactly
    the roles `background`, `ink` and `accent`, each an alias of a `ref.ember` colour (dark: 950, 50,
    400; light: 50, 950, 700).
R3. `build-tokens.ts` compiles every role of every mode of every palette under `modes` to
    `--<palette>-<mode>-<role>: <hex>;` in `:root`, and maps it into Tailwind's theme as
    `--color-<palette>-<mode>-<role>: var(--<palette>-<mode>-<role>);`.
R4. The compile refuses, naming the token, a palette missing the `dark` or the `light` mode, a mode
    missing a role, and a role outside `background`, `ink` and `accent`.
R5. In both modes, ink on background and accent on background each meet WCAG 2.2 AA for normal text
    (4.5:1).
R6. `ios/App/Assets.xcassets` holds `EmberBackground.colorset`, `EmberInk.colorset` and
    `EmberAccent.colorset`; each `Contents.json` holds a universal entry with no appearance carrying
    the light value and a universal entry with the `luminosity` appearance `dark` carrying the dark
    value, as sRGB 8-bit hex components with alpha `1.000`.
R7. A census refuses a role missing from either mode of the token file, a colour set whose value
    differs from its token, a colour set missing an appearance, and a raw colour literal in
    `web/app/scripts/build-tokens.ts`, `web/app/src/routes/layout.css` or any Swift file under
    `ios/App/Sources/`.
R8. The `color` tier, its Telegram mapping, its fallbacks and SPEC-028's A8 and A9 tests are
    byte-unchanged, so no page colour changes; no Swift file changes, and the app target sets no
    global accent colour.

## 3. Acceptance criteria of SPEC-578

| id | criterion | decided by |
|---|---|---|
| A1 | the Ember tier compiles to six custom properties carrying the tokens' values, each mapped into Tailwind's theme (R1 to R3) | `tokens.test.ts` |
| A2 | a role missing from either Ember mode, or a role outside the three, fails the compile by its name (R4) | `tokens.test.ts` |
| A3 | ink and accent on the background meet AA in both Ember modes (R5) | `tokens.test.ts` |
| A4 | each app colour set holds its token's light and dark values (R6, R7) | `test_ember_tokens.py` |
| A5 | a colour set whose value differs from its token, or that lacks an appearance, is refused (R7) | `test_ember_tokens.py` |
| A6 | a token file missing a role in either mode is refused by the census (R7) | `test_ember_tokens.py` |
| A7 | a raw colour literal outside the token route is refused, in the named web files and every app Swift file (R7) | `test_ember_tokens.py` |
| A8 | a planted complete set reads green, the control for A5 to A7 (R7) | `test_ember_tokens.py` |
| A9 | the Telegram mapping and its contrast proof are unchanged (R8) | SPEC-028's A8 and A9 tests |

```acceptance
A1: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "the Ember palette compiles to six custom properties in both modes"
A2: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "a role missing from either Ember mode fails the compile by name"
A3: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "ink and accent on the background meet AA in both Ember modes"
A4: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_each_colour_set_holds_its_tokens_values
A5: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_a_colour_set_that_differs_from_its_token_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_a_role_missing_from_either_mode_is_refused
A7: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_a_raw_colour_literal_outside_the_token_route_is_refused
A8: python3 -m unittest discover -s scripts/tests -p test_ember_tokens.py -k test_a_complete_planted_set_reads_green
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every colour token resolves to a Telegram theme variable with a fallback"
A9: pnpm exec vitest run web/app/src/lib/design/tokens.test.ts -t "every text and background pair meets AA contrast in both Telegram palettes"
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `web/app/src/lib/design/tokens.json` | tokens | changed: `ref.ember` and the `modes` tier (R1, R2) |
| `web/app/scripts/build-tokens.ts` | tokens | changed: compiles and checks the `modes` tier (R3, R4) |
| `web/app/src/lib/design/tokens.test.ts` | tests | changed: A1 to A3 added; A8 and A9 of SPEC-028 kept as they are |
| `scripts/tests/test_ember_tokens.py` | tests | new: A4 to A8 |
| `ios/App/Assets.xcassets/EmberBackground.colorset/Contents.json` | app | new (R6) |
| `ios/App/Assets.xcassets/EmberInk.colorset/Contents.json` | app | new (R6) |
| `ios/App/Assets.xcassets/EmberAccent.colorset/Contents.json` | app | new (R6) |
| `scripts/mutation-rows.d/S57800-S57899.json` | rows | new: S57801 to S57803 |
| `docs/red-first/SPEC-578.md` | records | new |
| `docs/red-first/SPEC-579.md` | records | amended append-only: a note on one later docstring and comment change in the logo delivery's test module |
| `docs/specs/SPEC-578-the-ember-palettes-six-role-values-live-in-the-token-file-and-the-apps-colour-sets-copy-them.md` | docs | new: this file |
| `docs/decisions/ADR-597-the-ember-palette-is-a-mode-tier-in-the-token-file-both-apps-copy-and-no-screen-is-re-coloured.md` | docs | new |
| `docs/schematics/ember-tokens.md` | docs | new |
| `docs/specs/SPEC-028-mini-app-shell.md` | docs | amended insert-only: R5 reads the `color` tier |
| `docs/DESIGN_SYSTEM.md` | docs | amended insert-only: the third tier |
| `changelog.d/ember-tokens-578.md` | changelog | new |

## 5. What this does NOT cover

- The palette applied to any screen: the web roles re-pointed at Ember, with the values a card,
  muted text and a border need beyond the six, and the native game and insight screens reading the
  colour sets (#1044).
- A mode switch the apps do not have: a web dark mode outside Telegram, or a mode chosen in the app
  rather than by the system (#1045).
- The bot and the Mini App, which are deprecated and gain no colour (#1046).
- What only a device shows: a colour set on a real screen in each appearance (#1047).
- The DeckStreak logo, its icons and the README header, which SPEC-579 delivers (#1042).

## 6. Risks

- **A colour set that does not compile.** The census reads JSON on Linux; the asset catalog
  compiler runs only in CI. Detected by the `apple-on-change` run's app tests and unsigned archive,
  which compile the catalog on this pull request.
- **A StrykerJS survivor in `build-tokens.ts`.** The changed file is mutated whole at
  `thresholds.break` 100. Detected by the local StrykerJS run before the push and by CI's
  `mutation-web`; a survivor is killed by a new assertion, never filtered.
- **A pin over the custom property names.** The screens round will read `--ember-<mode>-<role>`; a
  rename then is a change to A1, read by its test.
- **The catalog absent at the cut.** This delivery is cut only after SPEC-579's pull request merges;
  a cut without `ios/App/Assets.xcassets/Contents.json` at the cut's dev stops the build.
