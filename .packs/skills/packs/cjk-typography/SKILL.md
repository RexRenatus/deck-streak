---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/cjk-typography

Chinese, Japanese and Korean typography for a web app, checked (SPEC-V2-2223 / ADR-V2-2223). The
pack follows the W3C's text layout requirements for the three scripts:

- JLREQ, Requirements for Japanese Text Layout;
- CLREQ, Requirements for Chinese Text Layout, simplified and traditional;
- KLREQ, Requirements for Hangul Text Layout and Typography;

and the CSS and HTML features that implement them: CSS Text 3 and 4, CSS Ruby, CSS Fonts 4,
HTML's `<ruby>` model and BCP 47 language tags. It is also the check for the CJK rules the owner
approved with the house stack (stack-selection's "CJK text: checkable guidance").

`scripts/cjk-typography-probe.py` is the check. It is standard-library Python and vendorable, it
judges any tree through `--root`, and it reuses `accessibility-probe.py`'s tree walk, markup
tokenizer and CSS reader rather than copying them. Which seats consume this pack is its catalog
row's `consumes`, the one record of that edge (ADR-V2-1990), so this body names none.

```
phxd pack probe --pack cjk-typography --root PATH --format json
```

## How it composes

- **language-mentors** judges the TEXT that DeckStreak's language mentors write: whether a
  furigana `<rt>` is kana over kanji, and whether a pinyin tone mark sits on the vowel the Hanyu
  Pinyin rules name. Its `furigana-ruby` and `pinyin-tones` classes run here as two rows, straight
  from `language-mentors-probe.py`, so one walk judges both the app's typography and the readings.
  They are never copied, and their rules stay in language-mentors' `languages.json`. The catalog
  row says so with `extends: packs/language-mentors`, as cyber-pipeline extends web-security, so
  installing this pack brings the mentors' probe with it. The two rows keep language-mentors' ids,
  because they are that pack's classes.
- **accessibility** maps WCAG 2.2's 3.1.2 Language of Parts to this pack's `cjk-lang` row: a CJK
  passage inside English copy carries its `lang`, so a screen reader switches voice and the browser
  picks the right glyphs.
- **stack-selection** records the house CJK rules for the i18n golden path (Paraglide 2). This
  pack is their check.

## The rows

Sixteen rows, all `tree`-scoped. Fourteen run
`python3 {skills}/../scripts/cjk-typography-probe.py --root {root} check <class>`, and the two
composed rows run `python3 {skills}/../scripts/language-mentors-probe.py --root {root} check <class>`,
each under a 60-second wall. `{skills}` is the skills directory the catalog was read from, so the
scripts always come from these packs, whatever tree `--root` names.

The classes read markup (HTML, Svelte, Astro, Vue), CSS (files and `<style>` blocks), an
inlang project's `settings.json`, and message files under `messages/`, `locales/`, `i18n/` and
similar directories. Tests and build output are skipped. A language counts as USED when a `lang`
attribute or a locale names it, or, for Japanese and Korean, when kana or Hangul appears in the
markup.

The `lang` stage: 2 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `cjk-lang` | block | `cjk-lang-untagged` | a text run or a text attribute (`alt`, `title`, `placeholder`, `aria-label`) holding Han, kana, Hangul or bopomofo has no `lang` on it or on an element around it, or kana sits under a non-Japanese `lang`, Hangul under a non-Korean one, or Han under a non-CJK one. A `lang` set by an expression, such as `lang={locale}` or Paraglide's placeholder, is trusted | WCAG 3.1.2; W3C i18n "Declaring language in HTML" |
| `zh-script-subtag` | block | `zh-without-script` | a `lang`, `xml:lang` or `hreflang`, a locale in `settings.json`, or a message file's name is Chinese with no script subtag (`zh`, `zh-CN`, `zh-TW`), or a `:lang(zh)` rule sets `font-family` for both scripts. The finding names `zh-Hans` or `zh-Hant`, from the region where there is one | BCP 47 script subtags; the house rule |

The `fonts` stage: 3 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `cjk-font-stack` | block | `cjk-font-stack-missing` | a used language among `ja`, `ko`, `zh-Hans` and `zh-Hant` has no `:lang(...)` or `[lang|=...]` rule of its own that sets `font-family`. Han unification makes glyph choice follow the language, so one shared stack draws one language's forms for all three | the house rule; CLREQ 3.1, KLREQ 4.1 |
| `cjk-font-subset` | block | `cjk-font-unsubset` | a font file over 1 MiB ships in the tree, or an `@font-face` that loads a file (`url()`) and feeds a CJK stack declares no `unicode-range` | the house rule; CSS Fonts 4 `unicode-range` |
| `cjk-font-display` | advisory | `font-display-missing` | an `@font-face` that feeds a CJK stack or covers CJK code points sets no `font-display`, so a slow slice hides the text | CSS Fonts 4 `font-display` |

The `breaking` stage: 4 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `kinsoku` | block | `kinsoku-broken` | a tree with CJK sets `line-break: anywhere` on anything but code, which ignores every line-start and line-end prohibition, or Japanese is used and no `:lang(ja)` (or global) rule sets `line-break: strict` | JLREQ 3.1.7-3.1.8, CLREQ 6.1.1, KLREQ 7.1.2-7.1.3; CSS Text 3 `line-break` |
| `korean-keep-all` | block | `korean-breaks-mid-word` | Korean is used and neither a `:lang(ko)` rule sets `word-break: keep-all` nor a Korean element carries Tailwind's `break-keep` | KLREQ 7.1.1 (breaking by word); CSS Text 3 `keep-all`; the house rule |
| `no-break-all` | block | `break-all-on-cjk` | a tree with CJK sets `word-break: break-all`, or carries Tailwind's `break-all`, on anything but code. It splits the Latin words inside CJK text; `overflow-wrap: anywhere` is the tool for long URLs | CSS Text 3; the house rule |
| `phrase-headings` | advisory | `headings-break-mid-phrase` | Japanese or Korean is used and no heading rule sets `word-break: auto-phrase`. It is a progressive enhancement, and elsewhere it behaves as `normal` | CSS Text 4 `auto-phrase` |

The `spacing` stage: 2 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `cjk-line-height` | advisory | `cjk-line-height-tight` | a CJK rule sets a unitless or percent `line-height` under 1.5, or a used language has no rule of its own and the body's is under 1.5 | JLREQ's line gap of half to a whole character; KLREQ 7.4.1 |
| `mixed-script-autospace` | advisory | `autospace-undeclared` | a CJK or global rule turns `text-autospace` off, or Japanese or Chinese is used and no rule sets it | CLREQ 6.3.3 and JLREQ 3.2 (the space between CJK and Latin text); CSS Text 4 `text-autospace` |

The `ruby` stage: 3 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `ruby-conformance` | block | `ruby-nonconforming` | the app's markup uses `<rb>` or `<rtc>`, which HTML lists as obsolete, places `<rt>` or `<rp>` outside a `<ruby>`, or has a `<ruby>` with no `<rt>` and no dynamic content | the HTML standard's `ruby` element and obsolete features |
| `ruby-legible` | advisory | `ruby-illegible` | an `rt` rule sets ruby text under half the base (under 50%, 0.5em or 10px), or bopomofo ruby is used and no rule sets `ruby-position: inter-character` | JLREQ 3.3.3; CLREQ 5.5.3; CSS Ruby `ruby-position` |
| `furigana-ruby` | block | `ruby-invalid` | language-mentors' class: a mentor text's furigana ruby is malformed, or its reading is not kana over kanji | JLREQ 3.3; language-mentors' `languages.json` |

The `pinyin` stage: 2 rows.

| row | severity | reason | refuses when | follows |
|---|---|---|---|---|
| `pinyin-tones` | block | `tone-mark-misplaced` | language-mentors' class: a pinyin syllable in a mentor text carries more than one tone mark, a mark on the wrong vowel, or ü where the Scheme writes u | the Scheme for the Chinese Phonetic Alphabet; GB/T 16159-2012 |
| `pinyin-nfc` | advisory | `text-not-nfc` | a line of markup, message or Markdown text is not in Unicode NFC, such as a tone mark typed as a combining accent. Decomposed pinyin renders unevenly in CJK fonts and fails to match an answer typed with an input method | W3C Character Model (content authors should use NFC) |

The two composed rows answer as language-mentors answers: VOID on a tree with no mentor document,
and green or a finding otherwise. Every class of `cjk-typography-probe.py` prints one line per
finding, `<class>: <finding>`, and ends with `examined N`, the files it read. It exits 0 when green,
1 on a finding, 2 on a usage error, and 3 when VOID. VOID means nothing was examined or
`accessibility-probe.py` could not be loaded, and it is never a pass. A tree that uses no CJK still
counts the files it read, so it is green, not VOID. The pack's card turns any non-zero exit of a
`block` row red, and prints an `advisory` row's failure without reddening the card.

## The house rules, in one stylesheet

[templates/cjk.css](templates/cjk.css) is the stylesheet every class accepts, for all four
languages at once. Import it globally. What it does:

- **One font stack per language**, system fonts first: `:lang(ja)`, `:lang(ko)`, `:lang(zh-Hans)`
  and `:lang(zh-Hant)`. A web font, when one is needed, is sliced by `unicode-range`, with
  `font-display: swap`. A whole CJK font runs to several megabytes, and a sliced one downloads
  only the characters a page shows.
- **Kinsoku.** Japanese sets `line-break: strict`, so no line starts with a closing bracket, a
  small kana or the prolonged sound mark.
- **Korean breaks by word.** `word-break: keep-all` breaks Korean at the spaces between words.
- **Headings keep their phrases.** `word-break: auto-phrase` on Japanese and Korean headings.
- **Only URLs and code break anywhere**, through `overflow-wrap: anywhere`. Prose never takes
  `word-break: break-all`.
- **Line height 1.7** for CJK text.
- **`text-autospace: normal`** puts the narrow space between CJK and Latin letters or numerals,
  so copy needs no hand-typed spaces.
- **Ruby** is half size and over the base; bopomofo sits beside the base
  (`ruby-position: inter-character`).

What the rows cannot see, and a builder still does:

- tag each passage with its language: Paraglide sets `<html lang>` per locale, and a CJK example
  inside English copy takes its own `lang`;
- write furigana as `<ruby>漢<rp>(</rp><rt>かん</rt><rp>)</rp></ruby>`: mono-ruby per character
  for teaching, group-ruby for a word that reads as one;
- put pinyin over the characters (`ruby-position: over`), and bopomofo beside them;
- format numbers and plurals with `Intl.NumberFormat` and `Intl.PluralRules`, and find word
  boundaries with `Intl.Segmenter`, never by hand;
- store every string in NFC.

## How a project adopts this

A SvelteKit Mini App with Paraglide, such as DeckStreak, takes four steps.

1. **Name the locales with scripts.** In `project.inlang/settings.json`, list `ja`, `ko`,
   `zh-Hans` and `zh-Hant`, and name the message files the same way.
2. **Import the house stylesheet** from `templates/cjk.css` into the global CSS.
3. **Tag every CJK passage** with its `lang`, down to a single example word inside English copy.
4. **Run the pack** against the tree, in CI and before each release:

   ```
   phxd pack probe --pack cjk-typography --root PATH --format json
   ```

   It refuses:
   - an untagged or mistagged passage, and Chinese without a script;
   - a language without its font stack, and a whole font shipped;
   - `line-break: anywhere` or `word-break: break-all` on prose, Japanese without
     `line-break: strict`, and Korean without `keep-all`;
   - obsolete or misplaced ruby;
   - a mentor text's furigana or pinyin that language-mentors refuses.

   Its advisory rows report headings that break mid-phrase, tight line height, missing autospace,
   tiny ruby, a slow font face and text not in NFC.

## References

- JLREQ: https://www.w3.org/TR/jlreq/ (3.1.7-3.1.8 line-start and line-end prohibition, 3.2 mixed
  text, 3.3 ruby)
- CLREQ: https://www.w3.org/TR/clreq/ (5.5.3 bopomofo ruby, 5.5.4 romanized ruby, 6.1.1
  prohibition rules, 6.3.3 Chinese and Western mixed text)
- KLREQ: https://www.w3.org/TR/klreq/ (7.1.1 line breaking, 7.4.1 line spacing)
- CSS Text 3: https://www.w3.org/TR/css-text-3/ and CSS Text 4: https://www.w3.org/TR/css-text-4/
- CSS Ruby 1: https://www.w3.org/TR/css-ruby-1/ and CSS Fonts 4: https://www.w3.org/TR/css-fonts-4/
- HTML ruby and its obsolete parts: https://html.spec.whatwg.org/multipage/text-level-semantics.html#the-ruby-element
  and https://html.spec.whatwg.org/multipage/obsolete.html
- W3C i18n: https://www.w3.org/International/questions/qa-html-language-declarations,
  https://www.w3.org/International/articles/language-tags/ and
  https://www.w3.org/International/articles/ruby/markup
- Character Model, NFC: https://www.w3.org/TR/charmod-norm/
- The Scheme for the Chinese Phonetic Alphabet (tone marks, the apostrophe, iu, ui, un):
  http://www.moe.gov.cn/jyb_sjzl/ziliao/A19/195802/t19580201_186000.html
- Browser support: MDN's browser-compat-data, https://github.com/mdn/browser-compat-data
- Context7 ids: `/mdn/content`, `/websites/w3_tr_wcag22`
