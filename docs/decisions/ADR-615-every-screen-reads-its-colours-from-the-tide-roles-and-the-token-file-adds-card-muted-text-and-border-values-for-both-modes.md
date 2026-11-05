---
status: "accepted"
---

# ADR-615: every screen reads its colours from the Tide roles, and the token file adds card, muted-text and border values for both modes

Decides SPEC-593 with ADR-616. It builds on ADR-597, whose title says no screen is re-coloured by the Tide tier: this record changes that clause for the delivery that re-colours the screens (#1044), and ADR-597 keeps its text as written. ADR-028 (the token file and its in-repo compile) stands.

## Context and Problem Statement

Measured at dev `6f7d05f9` by a fenced `git grep -n` over `web/app/src` and `ios/App`:

- The web screens already draw through roles: 32 lines that hold a role class in 23 `.svelte` files, three of them the off-role sites below (`bg-card` on 17 lines, 13 of them with `text-card-foreground`, `bg-foreground` with `text-background` on 8 controls, `bg-background`, `text-muted-foreground`, `border-foreground`). No screen source holds a colour literal: the literal scan hit 24 files, and every hit outside `tokens.json` and the tests is an issue number, a private field or the engine flag named `red`.
- Three sites name a role the token file lacks, or one it never draws as text: `bg-muted` (`AscendantChip.svelte:8`), `border-primary` (`CourseLadder.svelte:29`) and `text-link` (`TierAnimation.svelte:41`).
- Each `color` role compiles to `var(--tg-theme-<parameter>, <hex>)`, and every fallback is a raw colour of the Telegram default light palette, so outside Telegram the page is white in either mode.
- The native app has 14 SwiftUI views in `ios/App/Sources`, and none reads a colour set: one `.foregroundStyle(.secondary)` (`VoicePickerView.swift:45`) and one `.background(.bar)` (`AnswerBar.swift:41`) are system styles.
- The six Tide values cover a background, an ink and an accent. A card, muted text and a border have no Tide value, and the token grey `#707579` on the Tide light background measures 4.33:1, under 4.5:1.

## Decision

D1. The `modes` tier holds six roles in each mode: `background`, `ink`, `accent`, and the new `card`, `muted` and `border`, each an alias of a raw colour in `ref.tide`. The new raw colours are neutral steps between the two Tide neutrals, plus pure white for the light card, so no new hue enters the palette.

| mode | background | ink | accent | card | muted | border |
|---|---|---|---|---|---|---|
| dark | `#0f161d` | `#f4f7fa` | `#34c6c0` | `#21282f` | `#8f9499` | `#74797e` |
| light | `#f4f7fa` | `#0f161d` | `#0b6a67` | `#ffffff` | `#5f656a` | `#82868c` |

Text pairs, each at or above 4.5:1 (WCAG 2.2 1.4.3), dark then light: ink on background 16.94 and 16.94, ink on card 13.86 and 18.22, muted on background 5.95 and 5.49, muted on card 4.87 and 5.91, accent on background 8.66 and 5.97, accent on card 7.09 and 6.42, background on ink 16.94 and 16.94. Non-text pairs, each at or above 3:1 (1.4.11): border on background 4.15 and 3.40, border on card 3.39 and 3.66, the focus ring (the accent) on background 8.66 and 5.97 and on card 7.09 and 6.42. Each ratio is the WCAG relative-luminance formula over the 8-bit sRGB values, compared unrounded.

D2. Each `color` role names its Tide role in its `deck-streak` extension (`background` to `background`, `foreground` and `card-foreground` to `ink`, `card` to `card`, `muted-foreground` to `muted`, `border` to `border`, and `link` and `ring` to `accent`), and its `$value` aliases the light-mode value. The compile emits the light fallbacks in `:root` and the dark fallbacks under `.dark`, still inside each role Telegram variable, so the Telegram mapping holds inside Telegram and Tide paints every role outside it. A role with no Tide role fails the compile by name. The five Telegram raw colours in `ref` are no longer read, and leave the file.

D3. The three off-role sites take existing roles: `bg-muted` becomes `bg-card`, `border-primary` becomes `border-ring`, and `text-link` becomes `text-card-foreground` with its underline in `decoration-link`.

D4. The app gains three colour sets, `TideCard`, `TideMuted` and `TideBorder`, written by program from the token file like the first three, and every native view reads the six sets by name through one `Color` extension in one Swift file: the root of each screen takes `TideBackground`, its text `TideInk` or `TideMuted`, its panels `TideCard`, its rules `TideBorder` and its tint `TideAccent`.

## What it was chosen AGAINST

- Chosen: new values as neutral steps between the two Tide neutrals, with pure white for the light card, because they keep the palette to its two hues and every pair clears its bar in both modes
- Telegram default hint grey for muted text and borders: rejected because it measures 4.33:1 on the Tide light background, under 4.5:1
- A new `color` role per screen need (`muted`, `primary`): rejected because the compile requires a Telegram parameter for every role, and the two sites read the same surface and ring the other screens already use
- Re-pointing each screen class at `--tide-<mode>-<role>` directly: rejected because all 32 role-class lines would change where the role names already route them, and the mode would then live in every class
- A pure white light card: chosen over a tinted one because it gives the highest ink contrast, 18.22:1, and every card draws the border role, so its edge meets 1.4.11 by that border (3.66:1 on the card, 3.40:1 on the background) while the fill itself is decorative under 1.4.11

## Consequences

- Good, because outside Telegram every web screen takes Tide in both modes, with no class edit beyond three sites.
- Good, because one census holds the token file, the compile and all six colour sets equal.
- Bad, because the page and every native screen change colour, so a visual check of each mode is owed on a device.
- Neutral, because inside Telegram the chat theme still paints every role, as the Mini App keeps its Telegram mapping until it retires (#1046).

## What would make this wrong

- A rendered pair the token file does not declare, such as text on an accent fill, measured under its bar.
- A device showing a colour set entry in the other appearance.
- The screens needing a role the six do not cover, such as an error colour.
