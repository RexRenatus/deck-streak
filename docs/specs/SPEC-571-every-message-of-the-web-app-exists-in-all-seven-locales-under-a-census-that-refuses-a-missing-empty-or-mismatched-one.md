# SPEC-571: every message of the web app exists in all seven locales, under a census that refuses a missing, empty or mismatched one

- **Issue:** #1051 (every message of the web app in all seven locales at launch).
  **Context(s):** the web app's message catalogs (`web/app/messages/`, read through
  `web/app/project.inlang/settings.json`) and the repository's Python tooling (`scripts/`).
- **Decided by:** ADR-590 (this SPEC's: the census, the fill, the tests, the shape), ADR-005 (the
  web front end and its message compiler, unchanged).
- **Mutation band:** `S57100-S57199` (§7).
- **Status:** one pull request, cut after the release pull request merges. Every path it writes is
  under `web/app/messages`, `scripts` or `docs`, plus its changelog fragment.

## 1. The problem, measured

Read at dev `4e600dbb7565bd7cd6becb8f1c724b5e502c3d4a`, by `git show <dev>:<path>` and a
standard-library count over the parsed catalogs.

- The project lists seven locales, base `en`, path pattern `./messages/{locale}.json`
  (`web/app/project.inlang/settings.json:5-8`).
- `web/app/messages/en.json` holds 315 message keys (318 lines); `es.json`, `fr.json`, `ja.json`,
  `ko.json`, `zh-Hans.json` and `zh-Hant.json` hold 215 each (218 lines). One set of 100 keys is
  missing from all six: 10 of the shell and about screen, 9 of the score, 20 of the level screen,
  15 of the streak screen and its governor, 6 of insights, 6 of dark fields, 8 of quick capture
  and 26 of the remote screen (`en.json:4-169`).
- 0 empty values, 0 values whose placeholders differ from English's, 0 keys a locale holds that
  English lacks, 0 repeated keys, 0 values out of NFC, 0 values that are not text. 55 English
  messages hold placeholders.
- 14 values equal their English text (`level_exchange_xp` and `methods_telegram` in six locales,
  `level_exchange_source` and `study_mapping_action` in French): names and cognates.
- The message compiler does not refuse a gap: a message missing from a locale is compiled as the
  base locale's own, so the 100 keys show English in the six other locales.
- Five tests read the catalogs, each for one screen's keys (`web/app/src/lib/passkey-messages.test.ts:44-67`,
  `web/app/src/lib/study/ai-decks-locales.test.ts:72-88`, `web/app/src/lib/sync/sync-locales.test.ts:99-111`,
  `web/app/src/lib/mastery-about-locales.test.ts:28-54`, `web/app/src/lib/smoke.test.ts:14-16`).
  None reads every key.
- The locale is chosen by the message runtime's default order (a cookie, a global variable, the
  base locale): `web/app/vite.config.ts:19-23` names no other order, and no file calls the
  runtime's locale setter. `web/app/src/hooks.client.ts:8-10` sets the page's `lang` and `dir`.
- CI's Python stage discovers every `scripts/tests/test_*.py` (`scripts/check.sh:185-186`, run by
  the hygiene job, `.github/workflows/ci.yml:351`).
- Every `scripts/*.py` is named in `scripts/mutation-python.json`
  (`scripts/tests/test_mutation_python.py:435-446`).

## 2. Requirements

R1. `scripts/locale_census.py` reads `web/app/project.inlang/settings.json` and refuses a locale
    set other than `en`, `es`, `fr`, `ja`, `ko`, `zh-Hans` and `zh-Hant`, a base locale other than
    `en`, and a path pattern other than `./messages/{locale}.json`.
R2. It refuses a listed locale with no catalog, a catalog under `web/app/messages/` that no listed
    locale names, a catalog that is not a JSON object, and a catalog that repeats a key.
R3. It refuses each key English holds that a locale lacks (`missing <locale> <key>`).
R4. It refuses a value that is blank after trimming, in any locale, English included
    (`empty <locale> <key>`), and a value that is not text (`not-text <locale> <key>`).
R5. It refuses a value whose `{name}` placeholders, counted as a multiset, differ from English's
    (`placeholder <locale> <key>`).
R6. It refuses a key a locale holds that English lacks (`stale <locale> <key>`).
R7. It refuses a value not in Unicode NFC (`not-nfc <locale> <key>`).
R8. It prints one line per finding, then `examined <n> locales, <m> keys`; it exits 0 when green,
    1 on a finding and 3 when it examined no catalog. The `$schema` member is not a message.
R9. `scripts/tests/test_locale_census.py` runs the census over planted trees and over the
    repository; the repository's test asserts no finding and a count above 0, and pins no key
    count.
R10. Every `missing` finding at the cut is filled with its locale's text, keeping English's
    placeholders, the locale's existing terms and punctuation, and NFC, and adding no pressure,
    countdown or loss wording English does not carry. Each catalog's existing lines stay
    byte-unchanged; the filled keys are appended in English's order. The filled keys are listed
    per locale in `docs/red-first/SPEC-571.md`.
R11. `web/app/messages/en.json`, `web/app/project.inlang/settings.json`, every file under
    `web/app/src`, every style and the five per-screen tests are byte-unchanged.
R12. `scripts/mutation-python.json` maps `scripts/locale_census.py` to `test_locale_census`, and
    the band's rows (§7) pin the census's literal constants.

## 3. Acceptance criteria of SPEC-571

| # | criterion | decided by |
|---|---|---|
| A1 | a key removed from one locale is refused by name | `test_locale_census.py`; red: the stub accepts every tree |
| A2 | a blank value is refused in any locale, English included | `test_locale_census.py`; red: the stub accepts every tree |
| A3 | a placeholder that differs from English's is refused, a reordered one is not | `test_locale_census.py`; red: the stub accepts every tree |
| A4 | a key English lacks is refused as stale | `test_locale_census.py`; red: the stub accepts every tree |
| A5 | a value out of NFC is refused | `test_locale_census.py`; red: the stub accepts every tree |
| A6 | a locale set other than the seven, another base or another pattern is refused | `test_locale_census.py`; red: the stub accepts every tree |
| A7 | a missing catalog, a stray catalog, a non-object catalog and a repeated key are refused | `test_locale_census.py`; red: the stub accepts every tree |
| A8 | a value that is not text is refused | `test_locale_census.py`; red: the stub accepts every tree |
| A9 | a planted complete tree of seven catalogs reads green with its examined count | `test_locale_census.py`; the control |
| A10 | a tree with no catalog exits 3 | `test_locale_census.py`; red: the stub exits 0 |
| A11 | the web app's own catalogs read green | `test_locale_census.py`; red: the tree before the fill |

```acceptance
A1: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_key_removed_from_one_locale_is_refused
A2: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_blank_value_is_refused_in_any_locale
A3: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_placeholder_that_differs_from_english_is_refused
A4: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_key_english_lacks_is_refused_as_stale
A5: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_value_out_of_nfc_is_refused
A6: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_locale_set_other_than_the_seven_is_refused
A7: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_an_unreadable_or_repeating_catalog_is_refused
A8: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_value_that_is_not_text_is_refused
A9: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_planted_complete_tree_reads_green
A10: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_tree_with_no_catalog_is_void
A11: python3 -m unittest scripts.tests.test_locale_census.TheWebTree.test_the_web_apps_catalogs_are_whole
```

Each planted tree is written by the test into a temporary directory: the settings file and seven
catalogs of four messages each, one with two placeholders, one with a composed accented letter, one
in each Chinese script. A1 to A8 assert the census's exact finding lines and exit 1; A9 asserts no
finding, exit 0 and `examined 7 locales, 4 keys`; A11 asserts no finding, exit 0 and an examined
key count above 0.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-571-every-message-of-the-web-app-exists-in-all-seven-locales-under-a-census-that-refuses-a-missing-empty-or-mismatched-one.md` | document | new |
| `docs/decisions/ADR-590-the-web-catalogs-are-held-whole-by-one-standard-library-census-and-every-gap-is-filled-in-all-seven-locales.md` | document | new |
| `docs/schematics/the-web-apps-seven-message-catalogs-and-the-census-that-holds-them-whole.md` | document | new |
| `docs/red-first/SPEC-571.md` | record | new |
| `scripts/locale_census.py` | tooling | new |
| `scripts/tests/test_locale_census.py` | test | new |
| `scripts/mutation-python.json` | tooling | changed: one entry maps the census to its test module |
| `scripts/mutation-rows.d/S57100-S57199.json` | rows | new |
| `web/app/messages/es.json` | web app | changed: the filled values, appended |
| `web/app/messages/fr.json` | web app | changed: the filled values, appended |
| `web/app/messages/ja.json` | web app | changed: the filled values, appended |
| `web/app/messages/ko.json` | web app | changed: the filled values, appended |
| `web/app/messages/zh-Hans.json` | web app | changed: the filled values, appended |
| `web/app/messages/zh-Hant.json` | web app | changed: the filled values, appended |
| `changelog.d/web-locales-571.md` | changelog | new |

## 5. What this does NOT cover

- No review of the filled values by a fluent reader of each locale: the census reads presence and
  shape, never meaning; the review is tracked on #1053.
- No change to how the web app picks or keeps the learner's locale: tracked on #1054.
- No String Catalog for the iPhone and iPad app: its own delivery, tracked on #1052.
- No locale beyond the seven and no right-to-left layout: tracked on #1056.
- No new screen: the native game and insight screens are their own round, tracked on #1057.
- No push or reminder copy: a later delivery adds it in all seven locales, and this census holds it
  once it lands, tracked on #640.
- No string of the Telegram bot or the Mini App's own surfaces is added or changed: tracked on
  #1046.
- No colour, palette or look changes: tracked on #1044.
- No measurement of how a device draws each script or breaks its lines: tracked on #1058.

## 6. Risks

- An open change that adds English keys lands between this delivery's cut and its land: the
  census, run in CI over the merge, refuses the gap by name, and the later change fills it.
- A message with variants enters the tree: the census refuses it as `not-text` until a later SPEC
  teaches it variants.
- A filled value is present and wrong: detected only by the fluent reader's review (#1053).
- A value holds a literal brace that is not a placeholder: the census counts it as one, and the
  refusal names the key; no such value exists at the measured dev.

## 7. Formal, rows and mutation

- **Formal: NOT APPLICABLE by surface.** The delivery is a file census and data: no protocol, no
  interleaving, no pure algorithm of the product a proof would hold.
- **Rows:** `scripts/mutation-rows.d/S57100-S57199.json`, table `SCRIPT_MUTATIONS`, rows
  `S57101` to `S57112`, each `[id, path, find, replace, description, killer]`, pinning the seven
  locales one row each, the base locale, the path pattern, the placeholder pattern, the normal
  form and the void exit. Each find occurs once in `scripts/locale_census.py`.
- **Mutation:** the Python mutation lane runs the census's changed lines against
  `test_locale_census`, through its entry in `scripts/mutation-python.json`.
