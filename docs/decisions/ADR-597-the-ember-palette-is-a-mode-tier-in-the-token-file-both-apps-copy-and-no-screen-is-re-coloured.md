---
status: "accepted"
---

# ADR-597: the Ember palette's six role values live in the token file as a mode tier both apps copy, and no screen is re-coloured by it

Decides SPEC-578. It builds on ADR-028 (the token file and its in-repo compile), keeps SPEC-028's
Telegram mapping of the `color` tier unchanged, and adds the colour sets to the iPhone and iPad
app's asset catalog that ADR-598 creates.

## Context and Problem Statement

DeckStreak's product palette is Ember: warm charcoal with an amber accent, as six role values, a
background, an ink and an accent for each of a dark and a light mode. The values come from the
DeckStreak brand source. The design system's first principle, as amended (`docs/DESIGN_SYSTEM.md`
lines 78 to 80), asks for "a dark-first, colour-rich palette with its own reference tier and role
mappings, WCAG 2.2 AA in both modes", and says the Mini App keeps its Telegram mapping until it
retires. Measured at dev `57cbb5e1`:

- The web app holds every colour in one DTCG file, `web/app/src/lib/design/tokens.json` (81 lines):
  a `ref` tier of five raw colours (lines 3 to 21: white, black, `#707579`, `#3390ec`, `#f4f4f5`)
  and a `color` tier of eight roles (lines 22 to 67), each naming the Telegram theme parameter that
  paints it and aliasing its fallback from `ref`.
- `web/app/scripts/build-tokens.ts` (181 lines) compiles it when a build or the dev server starts:
  each `color` role becomes `--<name>: var(--tg-theme-<parameter>, <hex>)` (line 151) and
  `--color-<name>` in Tailwind's `@theme inline` (line 152); a role naming no Telegram parameter
  fails the compile (lines 145 to 149). The `ref` tier is never emitted.
- `web/app/src/routes/layout.css` imports the compiled `tokens.css` (line 8). The page's scheme is
  `color-scheme: var(--tg-color-scheme, light)` (line 28): inside Telegram the chat chooses the
  mode and its theme paints every role; outside Telegram the page is light. A `.dark` variant is
  declared (line 13) and nothing in the tree sets it.
- `web/app/src/lib/design/tokens.test.ts` (167 lines) holds SPEC-028's A8 (lines 106 to 123) and A9
  (lines 125 to 166): every declared text pair meets 4.5:1 in Telegram's light and dark palettes
  and in the fallbacks.
- No raw colour literal sits in `web/app/src` outside `tokens.json`, and none in
  `ios/App/Sources/` (one `.foregroundStyle(.secondary)` at `VoicePickerView.swift:45` is a system
  style, not a colour).
- The iPhone and iPad product app has no asset catalog (`ios/app.yml` lines 30 to 33 list only
  `App/Sources` and the privacy manifest); the only catalog in the tree,
  `ios/Harness/Assets.xcassets`, belongs to the test harness.

The Ember values, from the brand source: dark background `#151210`, ink `#faf6f2`, accent
`#ff9a3c`; light background `#faf6f2`, ink `#151210`, accent `#a54d06`. Four distinct colours make
the six roles.

Contrast, computed by the WCAG 2.2 relative-luminance formula `tokens.test.ts` already uses
(lines 91 to 103): ink on background 17.35:1 in both modes; accent on background 5.33:1 light and
8.83:1 dark. All four pass AA for normal text (4.5:1). The token file's own grey, `#707579`, on the
Ember light background measures 4.33:1, under 4.5:1.

What a device shows (a colour on a real screen, an appearance switch) is not measured here.

## Decision Drivers

- One source of the six values that both apps copy, so a value cannot drift between them.
- Every kept check stays as strict: the Telegram mapping and its AA proof (SPEC-028 A8, A9).
- No colour value beyond the six, no new dependency, no change to how either app chooses a mode.
- No screen's layout changes; applying the palette to screens is its own round.
- The logo delivery never waits on this one.

## D1, the colour surfaces measured: what it was chosen against

- Chosen: the measurements above, each cited by file and line at dev `57cbb5e1`, because every later decision reads them and a step the reads cannot show (a device's rendering) is named as not measured
- Inferring the web app's mode from the `.dark` variant: rejected because nothing sets the class, so the page has one mode outside Telegram and the chat's mode inside it
- Treating the harness catalog as the app's: rejected because `ios/project.yml` builds it into the harness target only, and the product target in `ios/app.yml` names no catalog

## D2, the token home: what it was chosen against

The six values land in `web/app/src/lib/design/tokens.json`, the file SPEC-028 R5 already names as
the one source of design tokens, in two additions:

- `ref.ember`, the raw colours, by the brand source's own scale names: `neutral.50` `#faf6f2`,
  `neutral.950` `#151210`, `accent.400` `#ff9a3c`, `accent.700` `#a54d06`, each a DTCG sRGB colour
  whose components agree with its hex (the compile's existing check, `build-tokens.ts:90-112`).
- `modes`, a new tier: `modes.ember.dark` and `modes.ember.light`, each holding exactly
  `background`, `ink` and `accent`, each an alias of a `ref.ember` colour (dark: 950, 50, 400;
  light: 50, 950, 700).

`build-tokens.ts` compiles the tier: for each palette under `modes`, each of `dark` and `light`, and
each of the three roles, `--<palette>-<mode>-<role>: <hex>;` in `:root` and
`--color-<palette>-<mode>-<role>: var(--<palette>-<mode>-<role>);` in `@theme inline`. A palette
missing either mode, a mode missing a role, or a role outside the three fails the compile, naming
the token. The `color` tier, its Telegram mapping and its fallbacks are byte-unchanged, so no page
pixel moves: no role reads the Ember tier in this delivery.

The census, `scripts/tests/test_ember_tokens.py`, refuses a role missing from either mode, an app
colour set whose value differs from the token's, and a raw colour literal outside the token route
in the files this delivery changes (`build-tokens.ts`, `layout.css`) and in every Swift file of
`ios/App/Sources/`.

- Chosen: a `modes` tier beside `color` in the one token file, compiled to its own six custom properties, because it keeps SPEC-028's single source, adds no value beyond the six, and moves no pixel
- Re-pointing the web roles' fallbacks at Ember now: rejected because `muted-foreground` (`#707579`) on the Ember light background measures 4.33:1, which A9 refuses, so it needs a seventh value or a role remap, and either is the screens round's decision
- A second token file at the repository root for the shared palette: rejected because SPEC-028 R5 makes one DTCG file the source, and two sources are what ADR-028 rejected ("two sources of truth drift")
- Ember roles inside the `color` tier with Telegram parameters: rejected because inside Telegram the chat's theme would paint them, so they would not be Ember there, and the deprecated Mini App gains no colour
- DTCG `$extends` to derive the light mode from the dark: rejected because the compile's alias resolver does not read it and every role differs between the modes anyway
- A transform tool such as a token build library: rejected because it is a new dependency for six values, and ADR-028 keeps the in-repo compile until the set outgrows one file

## D3, the colour sets in the app's asset catalog: what it was chosen against

Three colour sets join `ios/App/Assets.xcassets`, the catalog ADR-598 creates and wires into the
product target: `EmberBackground.colorset`, `EmberInk.colorset` and `EmberAccent.colorset`. Each
`Contents.json` holds two entries: the universal entry with no appearance carries the light value,
and the entry with the `luminosity` appearance `dark` carries the dark value, each as sRGB
components written as 8-bit hex strings (`"red" : "0xFA"`) with alpha `1.000`, so no float rounds.
No Swift file changes, and the target sets no global accent colour: no screen reads the sets in
this delivery.

- Chosen: three colour sets with a light and a dark entry in the one catalog the logo delivery creates, because the system picks the appearance, the app's mode choice stays the system's, and the native screens copy them later by name
- One catalog per delivery: rejected because two catalogs in one target collide on the compiled `Assets.car` and split the app's assets
- Setting the colour sets as the target's global accent colour: rejected because it re-tints every control, which is applying the palette to screens
- Float components such as `0.980`: rejected because a float may round to a neighbouring byte; the hex form is the byte

## D5, the copy route and production or test: what it was chosen against

The six values are typed once, into `tokens.json`, by the builder from the brand source's
measured values; the three colour sets are written by a program the delivery runs once, which reads
`tokens.json` and writes each `Contents.json`, and the census then holds them equal. Byte-unchanged:
every screen's layout and behaviour, the `color` tier and its Telegram mapping, `layout.css`, the
harness's catalog and project spec, the bot and the Mini App. No kept check is loosened: A8 and A9
run unchanged over the unchanged `color` tier.

- Chosen: one hand entry in the token file, a program for the copies, a census over both, because the token file is the source and a program never mistypes a byte
- A committed generator script for the colour sets: rejected because it adds a production script and its mutation entry for a six-value copy the census already holds
- Writing the colour sets in CI at build time, as the harness icon is: rejected because it changes the workflows and leaves the census nothing committed to read

## D6, the shape: what it was chosen against

Two deliveries, each sized for one builder. ADR-598's logo delivery (SPEC-579) is cut first and
creates the app's asset catalog; this delivery (SPEC-578) is cut after it merges and adds the colour
sets into that catalog. The logo never waits on this delivery, and this delivery changes no screen's
layout. The amendments are insert-only: SPEC-028 (R5's rule reads the `color` tier) and
`docs/DESIGN_SYSTEM.md` (the third tier).

- Chosen: the logo first, then the tokens, because both add to the app's first catalog and only this order keeps the logo free of any colour change
- One delivery for both: rejected because they share only the catalog's root and its wiring, not every file, and one builder would carry a gate change and a token compile at once
- The tokens first: rejected because the catalog would then arrive with the colour change and the logo would wait on it

## D7, tests and rows: what it was chosen against

Red first per criterion, every red run locally: Vitest over `tokens.test.ts` (A1 to A3) and the
census module run by name (A4 to A7), each with a planted red over a fixture and a planted complete
set as the green control. `build-tokens.ts` is web code with no rows table: StrykerJS proves it
whole at `thresholds.break` 100, run locally before the push and again in CI's `mutation-web`. The
band S57800-S57899 holds script rows on the data a Python killer reads: an Ember hex in
`tokens.json`, a colour set's light value and a colour set's dark value. One push: no red needs CI.
FORMAL: not applicable (no interleaving; the census is an equality of two finite maps, and the
compile is a web surface no `covers` line can name).

- Chosen: Vitest and a Python census run locally, StrykerJS for the TypeScript, script rows for the data, because each red is read where it runs and no Swift exists to need CI
- A Swift test that loads each colour set by name: rejected because no screen uses the sets yet, it needs the two-push form, and CI's existing archive step already compiles the catalog
- Hand rows on `build-tokens.ts`: rejected because the rows table takes no TypeScript killer and StrykerJS covers the file

## D8, not this round: what it was chosen against

The palette applied to any screen (the web roles re-pointed, the native screens' colours), a mode
switch the apps do not have, the bot and the Mini App, and what only a device shows. Each is listed
in SPEC-578 section 5 with its issue.

- Chosen: land the source and its copies, and leave every screen as it is, because applying the palette needs values beyond the six and a design round of its own
- A web dark mode outside Telegram by `prefers-color-scheme`: rejected because it changes how the page chooses a mode, which this round does not decide

## Decision Outcome

D2, D3 and D5 are built by SPEC-578 in one pull request, cut after SPEC-579's lands (D6); D7 names
its proof; D1 and D8 bound it.

### Consequences

- Good, because the six values have one home and every copy is checked against it on every run.
- Good, because no page and no screen changes, so no visual review is owed by this delivery.
- Bad, because the web app carries six custom properties no rule reads until the screens round.
- Bad, because the app's colour sets are proved by a project-spec census on Linux; their
  rendering is first seen when a screen uses them.

### Confirmation

SPEC-578's acceptance criteria A1 to A7, its red-first record, its rows S57801 to S57803, and
StrykerJS's report over `build-tokens.ts`.

## What would make this wrong

- A device showing a colour set's dark entry in light appearance, or the reverse.
- A screens round that needs a role the three cannot express, which would add values to the tier,
  each a new decision.
- The token file growing past what one file holds readably (ADR-028's own limit).

## More Information

Built by SPEC-578; drawn in `docs/schematics/ember-tokens.md`. ADR-028 (the compile), SPEC-028 R5,
A8 and A9 (the Telegram mapping and its proof), ADR-598 (the catalog this delivery adds to).
