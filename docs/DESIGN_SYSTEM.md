# Design System

How the Mini App looks and behaves. The ui-styles, vibecode-polish, ux-laws, accessibility and
cjk-typography packs judge it.

## Principles

- Native to Telegram: the Mini App takes Telegram's theme, its BackButton and MainButton, and its
  haptics, so it feels part of the chat it opens from.
- Calm by default: celebrations are earned and rare; nothing pressures, shames or fabricates.
- Readable in every script the owner studies.

## Tokens

Design tokens are DTCG files (format 2025.10) mapped onto Telegram's `--tg-theme-*` variables with
fallbacks, so light and dark themes follow the chat. Colour pairs meet WCAG 2.2 AA contrast.

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

The theme follows Telegram's `themeChanged` event at run time; nothing is hard-coded to a colour.
