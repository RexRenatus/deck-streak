# Design System

How the Mini App looks and behaves. The ui-styles, vibecode-polish, ux-laws, accessibility and
cjk-typography packs judge it.

## Principles

- Native to Telegram: the Mini App takes Telegram's theme, its BackButton and MainButton, and its
  haptics, so it feels part of the chat it opens from.
- Calm by default: celebrations are earned and rare; nothing pressures, shames or fabricates.
- Readable in every script the owner studies.

## Tokens

The design tokens are one DTCG file (format 2025.10), `web/app/src/lib/design/tokens.json`, in two
tiers (ADR-028):

- **`ref`**, the raw colours: Telegram's default light palette, which the page shows outside
  Telegram. Only the `color` tier reads them, through DTCG aliases such as `{ref.white}`.
- **`color`**, the roles. Each names the Telegram theme parameter that paints it inside Telegram,
  in its `deck-streak` extension, and aliases its fallback from `ref`. A text colour lists under
  `text-on` the backgrounds it is drawn on.

`web/app/scripts/build-tokens.ts` compiles the file when a build or the dev server starts. Each
colour becomes a custom property that reads Telegram's variable with its fallback, and is mapped
into Tailwind's theme as a colour of the same name; a colour with no Telegram parameter fails the
compile. The radius and the font stack compile the same way.

| role | Telegram parameter | outside Telegram | drawn as text on |
|---|---|---|---|
| `background` | `bg_color` | `#ffffff` | |
| `foreground` | `text_color` | `#000000` | `background`, `card` |
| `card` | `secondary_bg_color` | `#f4f4f5` | |
| `card-foreground` | `text_color` | `#000000` | `card` |
| `muted-foreground` | `hint_color` | `#707579` | `background` |
| `link` | `link_color` | `#3390ec` | never: it colours a link's underline |
| `border` | `hint_color` | `#707579` | |
| `ring` | `link_color` | `#3390ec` | never: it colours the focus ring |

Every declared text and background pair meets WCAG 2.2 AA (4.5:1) in both of Telegram's default
palettes and in the fallbacks; `tokens.test.ts` measures the compiled variables, and the rendered
axe audit measures the painted page. Two of Telegram's own default pairs fall short, and the table
is built around them: the link colour on the page background (3.3:1 light, 4.3:1 dark), so a link
is text in the text colour underlined in the link colour; and the hint colour on the secondary
background in the light palette (4.2:1), so muted text is never drawn on a card. A third, the button
text colour on the button colour (3.3:1 light, 3.7:1 dark), is why the table declares no button
pair: the first screen that draws a button declares one that passes.

## Typography

A relative type scale with named roles. CJK text uses the house stylesheet (`cjk.css`): one font
stack per language, `line-break: strict` for Japanese, `word-break: keep-all` for Korean, line height
about 1.7, and ruby at half size.

## Components

shadcn-svelte components on Bits UI, restyled by the tokens. Each screen has one `<h1>`, its name.

## Accessibility

WCAG 2.2 level AA: every control reachable by keyboard and named, focus visible, motion reduced on
request, orientation never locked, and an axe audit of every route in both Telegram themes.

## Theming

The theme follows Telegram at run time; nothing is hard-coded to a colour. Telegram's script sets
the `--tg-theme-*` variables and resets them when the chat's theme changes, so the page repaints
without code; the wrapper (`telegram.svelte.ts`) follows `themeChanged` for any screen that needs
the colours in script. The page takes Telegram's colour scheme (`--tg-color-scheme`), its stable
viewport height rather than `100vh`, and its safe areas.

## Amendments

Amendment (the app-surfaces ruling): the first principle, "Native to Telegram", is amended by the
owner's signed ruling `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`, and ADR-341 carries
it. The principle's text above is kept as it was. The ruling words what replaces it:

> **Design principle 1: an identity of its own.** A dark-first, colour-rich palette with its own
> reference tier and role mappings, WCAG 2.2 AA in both modes. The Mini App keeps its Telegram
> mapping until it retires.

Amendment (the app-surfaces ruling): the second principle, "Calm by default", is amended by the same
ruling, `docs/rulings/OWNER-RULING-2026-10-04-app-surfaces.md`, and ADR-341 carries it. The
principle's text above is kept as it was. The ruling words what replaces it:

> **Design principle 2: vivid in session, calm out of session.** In a session: haptics, motion,
> sound, instant XP and ladder celebrations. Outside one: nothing pressures, shames or fabricates,
> and at most one nudge a day. Every charter 10 anti-goal is kept.

Amendment (SPEC-578): the token file holds a third tier, `modes`, beside `ref` and `color`. It holds DeckStreak's product
palette, Tide (a deep blue-black with a teal accent), as six role values: a background, an ink and an accent for each of a dark
and a light mode, each an alias of a raw colour in `ref.tide`. The section "Tokens" above is kept as it was; its two tiers and
the Telegram mapping of `color` are unchanged.

| mode | background | ink | accent |
|---|---|---|---|
| dark | `#0f161d` | `#f4f7fa` | `#34c6c0` |
| light | `#f4f7fa` | `#0f161d` | `#0b6a67` |

`web/app/scripts/build-tokens.ts` compiles each to a custom property, `--tide-<mode>-<role>`, mapped into Tailwind's theme as
`--color-tide-<mode>-<role>`, and the iPhone and iPad app copies them as three colour sets, `TideBackground`, `TideInk` and
`TideAccent`, each with a light entry and a dark entry; `scripts/tests/test_tide_tokens.py` holds every copy equal to the token
file. Ink on background measures 16.94:1 in both modes, and accent on background 5.97:1 light and 8.66:1 dark: each meets WCAG
2.2 AA for text. No screen reads the tier yet: the screens take it in their own round, which also decides the card, muted-text
and border values the three roles do not cover.

Amendment (SPEC-593): the Tide tier gains a card, a muted-text and a border role, so each mode holds six roles, and every screen
reads its colours from them. Each `color` role names its Tide role in the `tide` member of its `deck-streak` extension, beside
`telegram`: `background` takes `background`, `foreground` and `card-foreground` take `ink`, `card` takes `card`,
`muted-foreground` takes `muted`, `border` takes `border`, and `link` and `ring` take `accent`. The compile emits the twelve
properties `--tide-<mode>-<role>`, each role's light value as its fallback in `:root` and its dark value under `.dark`, each
still inside its role's Telegram variable, and a role outside the six, a role missing from either mode, or a `color` role with
no Tide role fails the compile by name. The Telegram raw colours leave `ref`, and the section "Tokens" above is kept as it was.

| mode | background | ink | accent | card | muted | border |
|---|---|---|---|---|---|---|
| dark | `#0f161d` | `#f4f7fa` | `#34c6c0` | `#21282f` | `#8f9499` | `#74797e` |
| light | `#f4f7fa` | `#0f161d` | `#0b6a67` | `#ffffff` | `#5f656a` | `#82868c` |

Every card carries the border role: a class list that holds `bg-card` also holds the `border` width class, whose colour the
base rule in `layout.css` gives, so the card edge meets 3:1 against the card and the page in both modes, and the card fill is
decorative. Every text pair meets 4.5:1 and every border and focus ring 3:1, in both modes.

The web app keeps one colour-mode rule (ADR-616): a choice of `system`, `light` or `dark`, kept on the device under
`deckstreak.colour-mode` and changed on the settings screen. Until the app mounts, and while the choice is `system` or nothing
is stored, the stylesheet alone sets the mode: a `prefers-color-scheme` rule in `layout.css` gives every colour role its dark
Tide fallback unless the browser reports light or the root element carries the `light` class, so a browser with no preference
shows the dark mode. Once the app mounts, a stored `light` or `dark` sets that class on the root element, and `system` clears
it. No script enters the page, and inside Telegram the chat theme and its scheme still paint every role.
