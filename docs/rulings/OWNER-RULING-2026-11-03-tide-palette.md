The owner moves DeckStreak's product palette to Tide, in the owner's own words: "lets transition to the tide pallete instead of ember". The app palette in the design token file and in the iPhone and iPad app's colour sets is Tide, with the six role values of the brand repository's ADR-0003 (a deep blue-black with a teal accent, in a dark and a light mode), and the palette it replaces is dropped from both; SPEC-578, ADR-597, their schematic and records, and the changelog fragment name Tide in its place.

# OWNER RULING 2026-11-03: the Tide palette

## What was held

SPEC-578, as first written, held another of the brand source's palettes in the token file's `modes` tier and in the
app's three colour sets. No screen read it, so no page or screen changed colour by it:

| where | the text held | what replaces it |
|---|---|---|
| `web/app/src/lib/design/tokens.json`, its `ref` and `modes` tiers | the earlier palette's four raw colours and six role values | Tide's four raw colours and six role values |
| `ios/App/Assets.xcassets`, three colour sets | the earlier palette's names and values | `TideBackground`, `TideInk` and `TideAccent`, with Tide's values |
| SPEC-578, ADR-597, their schematic, the changelog fragment, and the amendments to SPEC-028 and `docs/DESIGN_SYSTEM.md` | the earlier palette's name and values | Tide's name and values, in place |

## What replaces it

- **The palette.** DeckStreak's product palette is Tide, the palette the brand repository's ADR-0003 chose for the
  logo: a deep blue-black with a teal accent, in a dark and a light mode. Its six role values, a background, an ink and
  an accent for each mode, live in the token file's `modes` tier as `modes.tide`, each an alias of a raw colour in
  `ref.tide`, and the compile emits each as `--tide-<mode>-<role>`.
- **The colour sets.** The iPhone and iPad app's colour sets are `TideBackground`, `TideInk` and `TideAccent`, written
  by program from the token file and held equal to it by the census.
- **The earlier palette.** It is dropped: the token file keeps no second palette, and no colour set carries it.

## Why it is admitted

It is the owner's own answer. It re-points one delivery that no screen reads yet, so no page or screen changes colour;
the `color` tier, its Telegram mapping and SPEC-028's A8 and A9 are untouched. The app's palette now matches its logo,
which the brand repository draws in Tide.

The rejected alternatives:
- **Keep the earlier palette beside Tide as a second palette in `modes`.** The owner asked for a transition, Tide
  "instead of" the earlier palette, and two palettes would leave the screens round two sources to choose from.
- **A new delivery for Tide.** The earlier palette never reached a release, so the same specification, decision and
  numbers carry Tide in its place.
