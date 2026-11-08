# SPEC-629: no space is typed at a CJK and Latin boundary in the Japanese and Chinese web catalogs, under a census that refuses one

- **Issue:** #1053 (referenced: the fluent review of the seven locales; it stays open for that
  review). **Context(s):** the web app's message catalogs (`web/app/messages/`) and the
  repository's Python tooling (`scripts/`).
- **Decided by:** ADR-651 (this SPEC's: the refusal, the strip, the tests and the rows). ADR-590
  and SPEC-571 are unchanged: this SPEC adds one finding to their census.
- **Mutation band:** `S62900-S62999` (§7).
- **Status:** one pull request into dev. Every path it writes is under `web/app/messages`,
  `scripts` or `docs`, plus its changelog fragment.

## 1. The problem, measured

Read at the dev this delivery was cut from, by `git show <dev>:<path>` and a standard-library
count over the parsed catalogs, by the rule of §2 (R2 to R4).

- The house rule is `web/app/src/lib/styles/cjk.css:58-69`: `:lang(ja)` and `:lang(zh)` set
  `text-autospace: normal`, which draws the narrow space CLREQ 6.3.3 and JLREQ 3.2 ask for between
  CJK and Latin letters or numerals, "so no hand-typed spaces are needed".
- SPEC-571 R10 held that rule only for the values SPEC-571 itself wrote; the census it added
  judges no spacing (`grep -c -i -E 'spaced|autospace' scripts/locale_census.py` reads 0).
- `web/app/messages/ja.json` holds 59 typed-space runs at a CJK and Latin boundary, in 30 values;
  `zh-Hans.json` holds 72, in 40 values; `zh-Hant.json` holds 72, in 40 values; `ko.json` holds 0.
  A planted control of three values (`今日の XP`, `{count} 日` and `TelegramからDeckStreak`)
  counts 2 under the same rule.
- So the same gap renders as a full typed space in some messages and as the stylesheet's narrow
  gap in others, and nothing refuses a new typed space.

## 2. Requirements

R1. `scripts/locale_census.py` judges the spacing of three locales,
    `SPACED_LOCALES = ("ja", "zh-Hans", "zh-Hant")`. English, Spanish, French and Korean are never
    judged: Korean spaces its words, and Hangul is outside the class of R2.
R2. A CJK letter is a character whose code point lies in one of the ranges U+3005 to U+3007,
    U+3041 to U+3096, U+309D to U+309F, U+30A1 to U+30FA, U+30FC to U+30FF, U+31F0 to U+31FF,
    U+3400 to U+4DBF, U+4E00 to U+9FFF, U+F900 to U+FAFF and U+20000 to U+323AF, held as
    `CJK_RANGES`, one integer pair per line. CJK punctuation (U+3000 to U+3004, U+3008 to U+303F),
    the katakana double hyphen U+30A0, the katakana middle dot U+30FB, the full-width forms and
    Hangul are not CJK letters. The last range ends at U+323AF, the end of CJK Extension H.
R3. The Latin side is one ASCII letter, digit or percent sign (`[A-Za-z0-9%]`), or a placeholder
    brace: `{` when the CJK letter comes first, `}` when it comes last.
R4. A typed-space run is a maximal run of Unicode whitespace (U+0020, U+00A0, U+3000 and the rest)
    with a character on each side. It sits at a boundary when the character before is a CJK letter
    and the one after is on the Latin side, or the one before is on the Latin side and the one
    after is a CJK letter. A run at the start or the end of a value is never at a boundary.
R5. The finding is `spaced <locale> <key> <n>`: one line for each judged value holding n of 1 or
    more such runs, emitted after that value's `not-nfc` and `placeholder` findings, in catalog
    order, so a stale, non-text or blank value is not judged for spacing. It exits 1, as every
    finding does. A green census still prints only `examined <n> locales, <m> keys`.
R6. In `ja.json`, `zh-Hans.json` and `zh-Hant.json` every run at a boundary is deleted and both its
    neighbours kept; every other byte is unchanged (keys, order, line count, indentation,
    punctuation, placeholders, NFC). `en.json`, `es.json`, `fr.json` and `ko.json` are
    byte-unchanged.
R7. In `scripts/tests/test_locale_census.py` exactly four planted values lose their typed spaces,
    their assertions unchanged: the Japanese, Simplified Chinese and Traditional Chinese `progress`
    values of the planted tree, and the "added" case of
    `test_a_placeholder_that_differs_from_english_is_refused`, which plants `關閉{name}`. No other
    existing line changes but the module docstring's first line.
R8. The census's docstring lists the new finding among its refusals and names SPEC-629 and ADR-651
    beside SPEC-571 and ADR-590.
R9. The band's rows (§7) pin the census's new literals.

## 3. Acceptance criteria of SPEC-629

| # | criterion | decided by |
|---|---|---|
| A1 | a space typed between a CJK letter and a Latin letter, a digit or a percent sign is refused by name with its count, a no-break space and each range's low code point included | `test_locale_census.py`; red: the census judges no spacing |
| A2 | a space typed between a CJK letter and a placeholder brace is refused by name with its count | `test_locale_census.py`; red: the census judges no spacing |
| A3 | a space outside a CJK and Latin boundary is not refused: Korean, French, after full-width punctuation, between two Latin words, at the start or the end of a value, and beside each range's high code point plus one | `test_locale_census.py`; not red: it guards against over-refusal, and the census before this SPEC refuses no spacing |
| A4 | the planted complete tree, its Japanese and Chinese values written with no typed space, reads green | `test_locale_census.py`; not red: the control |
| A5 | the web app's Japanese and Chinese catalogs hold no typed space at a boundary, after a planted control is counted | `test_locale_census.py`; red: the control half before the refusal, then the tree half before the strip |

```acceptance
A1: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_latin_letter_digit_or_percent_is_refused
A2: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_space_typed_between_a_cjk_letter_and_a_placeholder_is_refused
A3: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_space_outside_a_cjk_and_latin_boundary_is_not_refused
A4: python3 -m unittest scripts.tests.test_locale_census.TheWebCensus.test_a_planted_complete_tree_reads_green
A5: python3 -m unittest scripts.tests.test_locale_census.TheWebTree.test_the_japanese_and_chinese_catalogs_hold_no_typed_space_at_a_boundary
```

A1 to A4 run the census over the planted tree of SPEC-571 with one value changed, and assert its
exact lines and exit code. A1 plants `学習デッキ XP` (`spaced ja title 1`), `3 个摘要` in Simplified
Chinese, `摘要 50% 完成` in Traditional Chinese (two runs), `閉じる` and `OK` joined by a no-break
space, a leading space before a boundary run, and each range's low code point before ` XP`; the
compatibility ideograph U+F900, which NFC maps to U+8C48, reads `not-nfc` first. A2 plants a run
after `}` and a run before `{`. A3 plants each case its row names and reads exit 0 with no finding.
The tests spell each code point as an integer of their own and never read the census's ranges, so
a mutated bound meets a case that kills it. A5 first counts a planted control
(`spaced ja title 1`), then runs the census by its default root, prints the total of its `spaced`
counts with the control's, and asserts no `spaced` line, an empty stderr and an examined key count
above 0.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-629-no-space-is-typed-at-a-cjk-and-latin-boundary-in-the-japanese-and-chinese-web-catalogs-under-a-census-that-refuses-one.md` | document | new |
| `docs/decisions/ADR-651-the-census-refuses-a-typed-space-at-a-cjk-and-latin-boundary-and-the-stylesheet-draws-that-gap.md` | document | new |
| `docs/schematics/the-web-apps-seven-message-catalogs-and-the-census-that-holds-them-whole.md` | document | changed: one section appended at the end |
| `docs/red-first/SPEC-629.md` | record | new |
| `changelog.d/cjk-latin-spacing-629.md` | changelog | new |
| `scripts/locale_census.py` | tooling | changed: the spacing refusal (R1 to R5, R8) |
| `scripts/tests/test_locale_census.py` | test | changed: the five criteria's tests, the four planted values of R7, the docstring's first line |
| `scripts/mutation-rows.d/S62900-S62999.json` | rows | new |
| `web/app/messages/ja.json` | web app | changed: typed spaces at a boundary removed (R6) |
| `web/app/messages/zh-Hans.json` | web app | changed: typed spaces at a boundary removed (R6) |
| `web/app/messages/zh-Hant.json` | web app | changed: typed spaces at a boundary removed (R6) |

## 5. What this does NOT cover

- No spacing rule for Korean, English, Spanish or French: Korean spaces its words, and the other
  three are Latin text; their wording stays with the fluent review, tracked on #1053.
- No String Catalog of the iPhone and iPad app: tracked on #1052.
- No review of wording by a fluent reader of each locale: the census reads shape, never meaning;
  tracked on #1053.
- No CJK text outside the web catalogs: the Telegram bot's and the Mini App's own strings are
  tracked on #1046, and the app's String Catalog on #1052.
- No code point past U+323AF: CJK Extension J of Unicode 17 (U+323B0 to U+3347F) and any later
  block are not CJK letters under R2, so a space beside one is not refused; how a device draws
  such a block is tracked on #1058.
- No gap where a browser lacks `text-autospace`: such a browser renders the boundary with no gap,
  which JLREQ and CLREQ accept; how a device draws each script is tracked on #1058.
- No gap beside a percent sign: `text-autospace: normal` draws its gap only beside letters and
  numerals, so a value stripped at a `%` side renders with no gap there; at the measured dev no
  Japanese or Chinese value holds such a run, and the rendering is tracked on #1058.

## 6. Risks

- An open change adds a Japanese or Chinese value with a typed space at a boundary: once this
  lands, the census in CI over the merge refuses it by name with its count, and the change strips
  the space.
- A boundary where a typed space is wanted (a code token, a product name): the census refuses it
  by name, and a later SPEC must teach the exception before such a value lands; none exists at the
  measured dev.
- A strip that changes more than the runs: the strip asserts the line count, the keys and their
  order, each placeholder multiset and NFC unchanged, and the census's own tests over the catalogs
  read every value again.

## 7. Formal, rows and mutation

- **Formal: NOT APPLICABLE by surface.** The delivery is a file census rule and catalog values: no
  protocol, no interleaving, no state machine a proof would hold.
- **Rows:** `scripts/mutation-rows.d/S62900-S62999.json`, table `SCRIPT_MUTATIONS`, rows from
  `S62901`, each `[id, path, find, replace, description, killer]`, pinning each of the three judged
  locales, the digit and the percent sign of the Latin class, each placeholder brace, the
  hiragana, katakana and unified Han ranges, and the finding word. Each find occurs once in
  `scripts/locale_census.py`, and each killer is an A1 or A2 test.
- **Mutation:** the Python mutation lane runs the census against `test_locale_census` through its
  existing entry in `scripts/mutation-python.json`, which is unchanged.
