---
status: "accepted"
decision-makers: "the owner, the DeckStreak architect"
---

# The web catalogs are held whole by one standard-library census, and every gap it finds is filled in all seven locales

## Context and Problem Statement

The web app ships seven locales: English, Spanish, French, Japanese, Korean, Simplified Chinese and
Traditional Chinese (`web/app/project.inlang/settings.json:7-8`). Every message must exist in all
seven at launch. At dev `4e600dbb7565bd7cd6becb8f1c724b5e502c3d4a`:

- `web/app/messages/en.json` holds 315 message keys; each of the six other catalogs holds 215. The
  same 100 keys are missing from all six (`en.json:4-169`: the shell's and the about screen's
  lines, the score, level, streak, insights, dark fields, quick capture and remote screens).
- No value is empty, no placeholder differs from English's, no locale holds a key English lacks,
  no file repeats a key, and every value is in Unicode NFC. 55 English messages hold placeholders.
- 14 values equal their English text (`XP` and `Telegram` in six locales, `Source` and `Action` in
  French): names and cognates, not gaps.
- The message compiler does not refuse a gap. A message missing from a locale is compiled as the
  base locale's own, so the 100 keys render in English inside the six other locales today.
- Five tests read the catalogs, each for one screen's keys (`passkey-messages.test.ts:44-67`,
  `ai-decks-locales.test.ts:72-88`, `sync-locales.test.ts:99-111`,
  `mastery-about-locales.test.ts:28-54`, and the locale list in `smoke.test.ts:14-16`). None reads
  every key, so the 100 gaps pass every check.

The house has refused an English fallback beside a translated screen before
(`docs/decisions/ADR-422-each-mastery-measure-keeps-its-own-plain-sentence-shown-as-text-beside-its-figure.md:100-101`,
`docs/decisions/ADR-423-one-tap-runs-the-clients-own-sync-once-a-late-review-settles-and-the-daily-sync-stays-as-it-is.md:83`).
This ADR makes that rule whole-catalog and checked.

## Decision Drivers

- A gap must be refused when it is made, by a check CI already runs, never found by a person
  reading a screen in another language.
- Three open changes edit all seven catalogs; the census must hold for whichever lands first, so it
  counts the catalogs at the tree it reads and pins no key count.
- No new package or tool: a local run and CI run the census with what they already have.
- The bot's and the Mini App's own strings, every screen's look and the palette stay unchanged.

## D1. The strings, measured

The measurement above is the round's population: 100 keys missing in each of six locales (600
values), 0 empty, 0 placeholder mismatches, 0 keys a locale holds that English lacks, 0 values out
of NFC. The locale is chosen at run time by the message runtime's default order (a cookie, then a
global variable, then the base locale): `web/app/vite.config.ts:19-23` names no other order, and no
file in the tree calls the runtime's locale setter. The page language follows the active locale
(`web/app/src/hooks.client.ts:8-10`). How a device draws each script and how well each translation
reads are not measured by any of this.

## D1. Chosen against

- Counting only the screens a later change touches: the 100 gaps sit on screens no open change
  edits, so a per-screen count would leave them standing.

## D1. Consequences

- The census's population is every key of every catalog at the tree it reads.

## D2. One census, run locally and in CI, refuses a missing, empty or mismatched message

`scripts/locale_census.py`, standard-library Python, reads `web/app/project.inlang/settings.json`
and the catalogs it names, and refuses, one finding per line:

- a locale set other than the seven, a base locale other than English, or another path pattern;
- a listed locale with no catalog, and a catalog no listed locale names;
- a catalog that is not a JSON object of messages, or that repeats a key;
- a key English holds that a locale lacks (`missing`);
- a value that is blank, in any locale, English included (`empty`), or that is not text
  (`not-text`);
- a value whose `{name}` placeholders, as a multiset, differ from English's (`placeholder`);
- a key a locale holds that English lacks (`stale`);
- a value not in Unicode NFC (`not-nfc`).

It prints `examined <n> locales, <m> keys`, exits 0 when green, 1 on a finding and 3 when it
examined nothing. `scripts/tests/test_locale_census.py` runs it over planted trees and over the
repository itself; CI's Python stage already discovers every `scripts/tests/test_*.py`, so a later
change that adds an English key without its six translations is refused there.

## D2. Chosen against

- A Vitest file beside the five per-screen tests: it would hold the web catalogs only, and the
  app's String Catalog census needs the same standard-library reader in a local run.
- The message project's lint plugins: a new package dependency for a check twenty lines of
  standard library make.
- Refusing a value equal to its English text: 14 such values are names and cognates, and a
  refusal would push translators to change a correct word.
- Extending each per-screen test: five files would each keep a copy of the rule, and a key no
  screen test names would still pass.

## D2. Consequences

- The five per-screen tests stay as they are; the census adds the whole-catalog rule beside them.
- A message with variants (a value that is not text) is refused until a later SPEC teaches the
  census to read variants; the tree holds none.

## D3. Every gap the census finds at the cut is filled in its locale

The delivery fills every `missing` finding at its cut with that locale's text. Each filled value
keeps English's placeholders, the locale's own terms for words its catalog already translates, its
punctuation conventions and NFC, and adds no pressure, countdown or loss wording English does not
carry. The existing lines of each catalog stay byte-unchanged; the filled keys are appended in
English's order. `en.json` is not edited. The filled keys are listed per locale in the delivery's
red-first record, for a fluent reader's review tracked on #1053.

## D3. Chosen against

- Leaving the gaps to fall back to English: a reader of another locale meets English lines inside
  a translated screen, which the house has refused before.
- Removing the 100 keys' screens from the six locales: it hides screens the product ships.

## D3. Consequences

- The fill is data only; no component, route or style changes.

## D4. Tests red first; the census's literals pinned by rows; formal not applicable by surface

Each refusal is a test over a planted tree, seen red against a census stub that accepts everything,
then green; a complete planted tree reads green as the control; the census over the repository is
red at the cut until the fill lands. Rows in `scripts/mutation-rows.d/S57100-S57199.json` pin the
census's literal constants (each of the seven locales, the base locale, the path pattern, the
placeholder pattern, the normal form and the void exit), each killed by a named test. The Python
mutation lane maps `scripts/locale_census.py` to its test module. Formal: not applicable by surface
(no protocol, no interleaving, no pure algorithm a proof would hold).

## D4. Chosen against

- Rows anchored in the catalog files: three open changes edit every catalog line range, so each
  would break a row that is not its own.

## D4. Consequences

- One push: every red runs locally.

## D5. Production or test

Production: the six non-English catalogs gain values. Test and tooling: the census, its test
module, its rows and the Python mutation map. Byte-unchanged: `en.json`, the settings file, every
Svelte file, every style, the palette, the bot's and the Mini App's own strings, and the five
per-screen tests. No kept check is loosened. How the web app picks and keeps the learner's locale
is unchanged (a change there is a separate decision, tracked on #1054).

## D5. Chosen against

- Teaching the app to follow the browser's language in this delivery: it changes how the locale is
  kept, which this delivery does not decide.

## D5. Consequences

- A learner reaches a locale other than English only as the runtime's order allows today.

## D6. Shape: this delivery first, the app's String Catalog second

This delivery comes first: its census sets the reader and the refusal names the app's census reuses,
its fill completes the web catalogs the app's shared keys are compared with, and every red runs
locally, in one push. The app's String Catalog follows under its own SPEC and ADR.

## D6. Chosen against

- One delivery for both apps: a census, 600 values and a Swift change with reds read only in CI
  exceed one builder's size, and a Swift red would hold the web fill behind a second push.
- The app's String Catalog first: its shared-key check would compare against catalogs still
  missing 100 keys, and its two-push form would hold the larger web fill.

## D6. Consequences

- No SPEC, ADR or schematic states a rule this delivery changes: every per-screen rule that each
  string exists in all seven locales is extended by the census, none replaced. No amendment.

## Consequences

- Every web message exists in all seven locales, and a gap is refused by name where it is made.

## What would make this wrong

- A catalog format the census cannot read (variants, nested messages) entering the tree: the census
  refuses it as `not-text`, and a later SPEC must teach it before such a message lands.
- A translation that is present but wrong: the census sees presence and shape, never meaning; the
  fluent reader's review on #1053 is the only check of meaning.

## More Information

- SPEC-571 holds this decision's acceptance criteria and its file manifest.
