# The web app's seven message catalogs, and the census that holds them whole

The data flow of SPEC-571 (ADR-590): where each web message lives, how the web app chooses the locale
it shows, and what the census reads and refuses. Every `path:line` below was read at
`4e600dbb7565bd7cd6becb8f1c724b5e502c3d4a`.

## Data flow

```mermaid
flowchart LR
  subgraph project["web/app/project.inlang"]
    settings["settings.json :5-8<br/>seven locales, base en,<br/>./messages/{locale}.json"]
  end
  subgraph catalogs["web/app/messages"]
    en["en.json<br/>every key, the reference"]
    six["es, fr, ja, ko,<br/>zh-Hans, zh-Hant .json<br/>filled by this delivery"]
  end
  subgraph build["the build and the check script"]
    compile["message compiler<br/>vite.config.ts :19-23<br/>package.json :12"]
    out["src/lib/paraglide<br/>generated, ignored"]
  end
  subgraph runtime["the browser"]
    pick["getLocale: cookie,<br/>global variable, base locale"]
    page["hooks.client.ts :8-10<br/>html lang and dir"]
    screens["components call m.key"]
  end
  subgraph census["scripts"]
    reader["locale_census.py<br/>reads settings and catalogs"]
    findings["one line per finding<br/>examined n locales, m keys<br/>exit 0, 1 or 3"]
  end
  settings --> compile
  en --> compile
  six --> compile
  compile --> out --> screens
  pick --> screens
  pick --> page
  settings --> reader
  en --> reader
  six --> reader
  reader --> findings
```

## How a message reaches a screen

1. A component calls the compiled message function for its key.
2. The runtime picks the locale in its default order: the locale cookie when one is set, then a
   global variable, then `en`. No file in the tree sets either, so a visitor sees `en` unless a
   cookie names another locale. This delivery does not change that order.
3. The compiled function returns the locale's text. A key missing from that locale returns
   English's text; after this delivery the census refuses such a gap, so none ships.
4. `hooks.client.ts:8-10` sets `<html lang>` and `dir` to the active locale, so the CJK
   stylesheet's `:lang()` rules and assistive technology read the page's language.

## What the census reads and refuses

```mermaid
flowchart TB
  start["--root tree"] --> rs{"settings: a JSON object?"}
  rs -->|no| f0["unreadable settings"]
  rs -->|yes| s{"settings: seven locales,<br/>base en, the pattern?"}
  s -->|no| f1["locale-set"]
  s -->|yes| files{"one catalog per locale,<br/>no stray catalog?"}
  files -->|no| f2["file-missing, file-stray"]
  files -->|yes| parse{"JSON object,<br/>no repeated key?"}
  parse -->|no| f3["unreadable"]
  parse -->|yes| keys["for each key of en.json<br/>and each locale"]
  keys --> m["missing"]
  keys --> e["empty, not-text"]
  keys --> p["placeholder"]
  keys --> st["stale"]
  keys --> n["not-nfc"]
  start --> v{"examined 0 catalogs?"}
  v -->|yes| void["exit 3"]
```

## Each step, its test, its red and its row

| step | test (`scripts/tests/test_locale_census.py`) | red | rows |
|---|---|---|---|
| the settings file is a JSON object | `TheWebCensus.test_an_unreadable_settings_file_is_refused` (A12) | the census stops on a traceback | the Python lane |
| the locale set, base and pattern | `TheWebCensus.test_a_locale_set_other_than_the_seven_is_refused` (A6) | the stub accepts every tree | S57101 to S57107 (each locale), S57108 (base), S57111 (pattern) |
| one catalog per locale, readable, no repeated key | `TheWebCensus.test_an_unreadable_or_repeating_catalog_is_refused` (A7) | the stub accepts every tree | the Python lane |
| a key English holds is in every locale | `TheWebCensus.test_a_key_removed_from_one_locale_is_refused` (A1) | the stub accepts every tree | the Python lane |
| no blank value, no value that is not text | A2 and `TheWebCensus.test_a_value_that_is_not_text_is_refused` (A8) | the stub accepts every tree | the Python lane |
| placeholders equal English's | `TheWebCensus.test_a_placeholder_that_differs_from_english_is_refused` (A3) | the stub accepts every tree | S57109 |
| no key English lacks | `TheWebCensus.test_a_key_english_lacks_is_refused_as_stale` (A4) | the stub accepts every tree | the Python lane |
| every value in NFC | `TheWebCensus.test_a_value_out_of_nfc_is_refused` (A5) | the stub accepts every tree | S57110 |
| nothing examined is not a pass | `TheWebCensus.test_a_tree_with_no_catalog_is_void` (A10) | the stub exits 0 | S57112 |
| a complete tree reads green | `TheWebCensus.test_a_planted_complete_tree_reads_green` (A9) | not red: the control | the killer of S57101 to S57108, S57110, S57111 |
| the web app's catalogs are whole | `TheWebTree.test_the_web_apps_catalogs_are_whole` (A11) | the tree before the fill | the census in CI over every later change |

## Where the census runs

- Locally: the builder runs `test_locale_census` by its module name over planted trees and over
  the repository.
- In CI: the Python stage of the hygiene job discovers every `scripts/tests/test_*.py`
  (`scripts/check.sh:185-186`), so a later change that adds an English key without its six
  translations reads red there, by the finding's name.
- In the mutation lane: `scripts/mutation-python.json` maps `scripts/locale_census.py` to
  `test_locale_census`.

## What is not changed

`en.json`, the settings file, every component, route and style, the five per-screen locale tests,
the runtime's locale order and the page-language hook keep their bytes.

## The CJK and Latin spacing refusal (SPEC-629)

SPEC-629 (ADR-651) adds one finding to the census. Every `path:line` below was read at the dev
SPEC-629 §1 was measured at.

```mermaid
flowchart LR
  value["a value of ja, zh-Hans<br/>or zh-Hant .json"] --> judged{"judged: stale, not-text,<br/>empty, not-nfc, placeholder"}
  judged -->|"stale, not-text or empty"| stop["no spacing judged"]
  judged -->|"text"| runs{"a whitespace run between<br/>a CJK letter and a letter,<br/>digit, % or brace?"}
  runs -->|"n runs, n of 1 or more"| spaced["spaced locale key n<br/>exit 1"]
  runs -->|"none"| green["no finding"]
  css["cjk.css :58-69<br/>text-autospace: normal<br/>for :lang(ja) and :lang(zh)"] --> gap["the browser draws<br/>the narrow gap"]
  green --> gap
```

1. The census reads each value of `ja`, `zh-Hans` and `zh-Hant`; English, Spanish, French and
   Korean are never judged for spacing.
2. A value that is stale, not text or blank gets its own finding and no spacing finding; any
   other value gets its `not-nfc` and `placeholder` findings first, then `spaced <locale> <key> <n>`
   when it holds n typed-space runs at a CJK and Latin boundary.
3. With no typed space, the stylesheet's `text-autospace` draws the gap at each boundary, so every
   Japanese and Chinese screen renders the same narrow gap.

| step | test (`scripts/tests/test_locale_census.py`) | red | rows |
|---|---|---|---|
| a space between a CJK letter and a letter, digit or `%` | `TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_latin_letter_digit_or_percent_is_refused` (A1) | the census judges no spacing | S62901 upward |
| a space between a CJK letter and a placeholder brace | `TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_placeholder_is_refused` (A2) | the census judges no spacing | the brace rows |
| a space outside a boundary | `TheWebCensus.test_a_space_outside_a_cjk_and_latin_boundary_is_not_refused` (A3) | not red: it guards against over-refusal | the Python lane |
| a complete tree reads green | `TheWebCensus.test_a_planted_complete_tree_reads_green` (A4) | not red: the control | the Python lane |
| the web app's Japanese and Chinese catalogs | `TheWebTree.test_the_japanese_and_chinese_catalogs_hold_no_typed_space_at_a_boundary` (A5) | the control half, then the tree before the strip | the census in CI over every later change |
