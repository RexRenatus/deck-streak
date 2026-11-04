# Tide screens: from the token file to each client, its screens and its modes

Kind: data flow. For SPEC-593, decided by ADR-615 (every screen reads the Tide roles) and ADR-616
(one colour-mode rule per client). It extends `docs/schematics/tide-tokens.md`, the token flow both
apps copy. Paths are read at the dev commit SPEC-593 is cut on.

## The web app

```mermaid
flowchart LR
  TJ["tokens.json: modes.tide.dark and modes.tide.light, six roles each"] --> BT["build-tokens.ts compile"]
  BT --> TP["twelve custom properties, --tide-MODE-ROLE"]
  BT --> RF["each color role in :root: var(--tg-theme-X, its light Tide value)"]
  BT --> DF["each color role under .dark: var(--tg-theme-X, its dark Tide value)"]
  RF --> CSS["tokens.css, imported by layout.css"]
  DF --> CSS
  CSS --> SCR["every .svelte screen: role classes only, every card with a border class"]
  LS["local storage: deckstreak.colour-mode"] --> HS["head script in app.html"]
  MQ["media query: prefers-color-scheme"] --> HS
  CSP["sveltekit.config.js: script-src lists the head script sha256"] --> HS
  HS --> DK["dark class on the root element, set or cleared before first paint"]
  SET["settings screen: system, light or dark"] --> LS
  SET --> CM["colour-mode.ts: applies the choice, listens while it is system"]
  MQ --> CM
  CM --> DK
  DK --> DF
  DK --> SCH["color-scheme: var(--tg-color-scheme, the class mode)"]
  TG["Telegram theme variables, inside the Mini App only"] --> RF
  TG --> DF
  TG --> SCH
```

Outside Telegram no `--tg-theme-*` variable is set, so every role takes its Tide fallback: the
light value from `:root`, or the dark value once the root element carries `dark`. Inside the Mini
App the Telegram variables win in both rules, so the chat theme keeps painting every role and
`--tg-color-scheme` keeps setting the scheme.

## The iPhone and iPad app

```mermaid
flowchart LR
  TJ2["tokens.json: the same six roles per mode"] --> GEN["colour sets written by program"]
  GEN --> SETS["six colour sets: TideBackground, TideInk, TideAccent, TideCard, TideMuted, TideBorder, each a universal entry and a luminosity dark entry"]
  SYS["system appearance"] --> SETS
  SETS --> EXT["TideColours.swift: one Color extension naming the six sets"]
  EXT --> VW["every SwiftUI view: root TideBackground, text TideInk or TideMuted, panels TideCard, rules TideBorder, tint TideAccent"]
```

No view and no property list sets a colour scheme, so the system appearance picks each set entry.

## The tests that hold each edge

| edge | test |
|---|---|
| `tokens.json` to the twelve custom properties and the per-mode fallbacks | `web/app/src/lib/design/tokens.test.ts` |
| every declared pair at its contrast bar, both modes | `web/app/src/lib/design/tokens.test.ts` |
| `tokens.json` to the six colour sets | `scripts/tests/test_tide_tokens.py` |
| every screen class a declared role, every card with a border class | `scripts/tests/test_tide_screens.py` |
| the head script mode and its listed hash | `web/app/src/lib/settings/colour-mode.test.ts` |
| every view through the Tide extension, and no scheme override | `scripts/tests/test_tide_native.py` |
