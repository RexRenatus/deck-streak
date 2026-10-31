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
  start["--root tree"] --> s{"settings: seven locales,<br/>base en, the pattern?"}
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
