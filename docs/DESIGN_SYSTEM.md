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
