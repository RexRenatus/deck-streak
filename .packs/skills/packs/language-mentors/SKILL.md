---
requires_phxd_schema: phxd.pack.probe.v1
tip_floor: e996e5b8064f8b129a532b72877f539a149b3ac0
body_status: seeded
---

# packs/language-mentors

DeckStreak's language mentors: five persona TEMPLATES, one each for Mandarin Chinese, Japanese,
Korean, French and Spanish, and the checks that keep what a mentor writes true to its language
(SPEC-V2-2213 / ADR-V2-2213). The owner's decisions it carries:

- "A persona pack per subject" and "Named mentor per subject". A mentor has a consistent,
  professional voice rooted in the language's culture, with no fictional drama.
- "Adaptive by CEFR band": English-led at A1-A2, bilingual at B1-B2, and the target language only at
  C1-C2. The band is read from the learner's live Road-to-C2 band.
- Every mentor brings all four language extras:
  - i+1 readings with glosses of today's new words;
  - a grammar spotlight;
  - pronunciation and script notes;
  - culture notes.
- The Writing deck and the daily ko/ja/zh writing habit fold into each language's mentor.
- "Remembers weak spots": each mentor reads the leeches, drill grades and lapse history of its own
  subject, and never the journal.
- "Public templates, private roster". The templates carry the slots `{{name}}`, `{{bio}}`,
  `{{voice}}` and `{{personality}}`, and no name, bio or roster. The owner approves the names the
  architect drafts. No persona text states a date, a deadline or a countdown.

This pack BUILDS ON packs/persona-core and copies nothing from it:

- persona-core owns the persona template schema, the output contract and the shared safety checks;
- its rules are reused here, scoped with `--kind language`;
- this pack's own script, `scripts/language-mentors-probe.py`, parses every file through
  persona-core's `parse_document` and reads the CEFR band-to-mode map from persona-core's
  `contract.json`.

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge
(ADR-V2-1990), so this body names none.

```
phxd pack probe --pack language-mentors --root PATH --format json
```

`PATH` is any tree: DeckStreak's repository, a directory of generated texts, or phoenix-v2 itself.
The walk finds every claimed persona file whose subject is `language/...`. In phoenix-v2 that is the
five [templates](templates/zh.persona.md) and ten worked [examples](examples/zh-daily-reading-b1.output.md).
Run one class directly with
`python3 scripts/language-mentors-probe.py --root PATH [--subject DIR] check <class>`.

## The rows

Twenty-six rows, all `tree`-scoped, under a 120-second wall each.

- Eight rows run persona-core's classes:
  `python3 {skills}/../scripts/persona-core-probe.py --root {root} --kind language check <class>`.
- Eighteen rows run this pack's classes:
  `python3 {skills}/../scripts/language-mentors-probe.py --root {root} check <class>`.

`{skills}` is the skills directory the catalog was read from, so the scripts and the rules table
always come from the pack, whatever tree `--root` names.

### Reused from persona-core, scoped to language subjects

The `schema` stage: 3 rows. They judge persona-core's template schema and output contract.

| row | severity | reason | refuses when |
|---|---|---|---|
| `template-schema` | block | `template-invalid` | a language template breaks the persona template schema: its keys, its sections, its four slots or its disclosure |
| `output-contract` | block | `output-invalid` | a language output breaks the output contract: its persona, subject, duty, `cefr` or `lang` does not agree with its template |
| `duty-composition` | block | `duty-sections-missing` | an output lacks a section its duty or its template's `sections` requires, or one is empty |

The `safety` stage: 4 rows. They are the owner's blocking rules for every persona.

| row | severity | reason | refuses when |
|---|---|---|---|
| `no-dates` | block | `date-or-timeline` | any language persona file states a calendar date, a deadline, an exam date or a countdown |
| `scrubber` | block | `scrubber-denied` | a public file carries a shape on the scrubber's deny-list |
| `memory-scope` | block | `memory-out-of-scope` | a mentor reads another subject's memory, or the journal |
| `no-human-claim` | block | `human-claim` | a mentor claims to be a person |

The `voice` stage: 1 row.

| row | severity | reason | refuses when |
|---|---|---|---|
| `voice` | advisory | `voice-drift` | an output drifts from a professional voice: stage directions, an invented backstory, a run of exclamations or emoji. Advisory: it reports and never refuses |

### This pack's classes

The `mentor` stage: 2 rows. They judge the templates and the rules table.

| row | severity | reason | refuses when |
|---|---|---|---|
| `mentor-templates` | block | `mentor-template-incomplete` | a language template has no entry in the rules table or a `lang` tag for another language; it lacks the daily-reading or writing-tutor duty, or one of the extra sections they add (glosses, grammar, pronunciation, culture; focus); it omits one of the three memory sources; or its method never names its notation or one of the three modes |
| `rules-table` | block | `rules-table-invalid` | `languages.json` is malformed: a mode with no share bounds or inverted bounds, a notation it does not know, a repeated error category, a batchim table that is not the 27 finals mapped onto the seven representatives, or a liaison or stress table missing a part. An unreadable table is VOID for every class |

The `instruction` stage: 1 row.

| row | severity | reason | refuses when |
|---|---|---|---|
| `cefr-ratio` | block | `instruction-off-mode` | the target language's share of an output's instruction text falls outside its band's mode: more than 0.50 when english-led (A1-A2), outside 0.15-0.85 when bilingual (B1-B2), or under 0.95 when target-only (C1-C2). It also refuses a language with no table entry, an output with no band, and an output with no measurable instruction |

The `reading` stage: 3 rows (2 blocking, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `i1-glosses` | block | `gloss-missing` | a gloss's word does not occur in the reading, a gloss item has no gloss, a word is glossed twice, the glosses section holds no item, or an `x-new-words` word is not glossed or not in the reading |
| `i1-density` | advisory | `new-word-density-high` | glossed new words are more than 5% of the reading (Laufer's 95% coverage), counted in characters for zh and ja and in words for ko, fr and es. Advisory |
| `grammar-spotlight` | block | `spotlight-invalid` | the grammar section names other than exactly one `###` point, has no target-language example in a blockquote, or has no explanation |

The `pronunciation` stage: 8 rows.

| row | severity | reason | refuses when |
|---|---|---|---|
| `pronunciation-notation` | block | `notation-missing` | an output's pronunciation notes carry none of its language's required notation: tone-marked pinyin (zh), ruby furigana and pitch accent (ja), a batchim pair (ko), a liaison row (fr), a stress row (es) |
| `pinyin-tones` | block | `tone-mark-misplaced` | a tone-marked pinyin syllable puts its mark off the vowel GB/T 16159 clause 6.5.1 names (a or e; o in ou; else the last vowel, so iu marks u and ui marks i), carries two marks, or writes ü after j, q, x or y |
| `furigana-ruby` | block | `ruby-invalid` | a `<ruby>` is unclosed or nested, an `<rt>` is empty or not kana, the base holds no kanji, or an `<rp>` holds more than a parenthesis |
| `pitch-accent` | block | `pitch-accent-invalid` | a `kana [n]` accent falls past the word's last mora, or on っ, ー or ん (a special mora never carries the kernel), or a word carries two downstep marks |
| `hangul-batchim` | block | `batchim-misstated` | a `word [pronunciation]` pair says a word-final batchim as other than its representative final (표준 발음법 Articles 8-11), writes and says different syllable counts, or a rule `ㅅ → [ㄷ]` names the wrong representative |
| `romanization-rr` | block | `romanization-not-rr` | a declared romanization (a Korean gloss's reading, or a column headed romanization, RR or 로마자) holds a breve, an apostrophe or another non-Roman letter, or does not parse as Revised Romanization syllables |
| `french-liaison` | block | `liaison-miscategorised` | a liaison row's junction (`les‿enfants`, `et / ils`) is marked obligatoire, facultative or interdite against the context rule the Office québécois de la langue française gives: an h aspiré, et, a pause, a determiner, a pronoun, a short preposition or adverb, a fixed expression |
| `spanish-stress` | block | `stress-misstated` | a stress row's class (aguda, llana, esdrújula, sobresdrújula, monosílabo) is not what the spelling gives, the tilde is missing or not required by the RAE's rules (hiatus and diacritic tildes included), or the syllable split capitalises another syllable |

The `culture` stage: 2 rows (both advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `culture-anchor` | advisory | `culture-unanchored` | a culture note holds no target-language word or practice to anchor it. Advisory |
| `culture-generalization` | advisory | `culture-overgeneralized` | a culture note generalises about a whole people ("All Japanese people…", "中国人都…", "les Français sont tous…"). Advisory |

The `write` stage: 2 rows (1 blocking, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `write-corrections` | block | `correction-off-policy` | a correction item is malformed, names a category outside its language's taxonomy, is indirect (`→ ?`) at an english-led band or for an untreatable category, changes nothing, or has no explanation |
| `write-focus` | advisory | `feedback-unfocused` | one writing sample's corrections treat more than three categories. Advisory |

Every class prints one line per finding, `<class>: <finding>`, and ends with `examined N`: the
language persona documents it read to decide, templates and outputs alike. `rules-table` counts
the table's languages. The script exits:

- 0 when green;
- 1 on a finding;
- 2 on a usage error;
- 3 when VOID: nothing was examined, or the table, the contract or persona-core's probe could not
  be read. VOID is never a pass.

A claimed language file that persona-core cannot parse is a finding in every class, never a skip.
The card turns any non-zero exit of a `block` row red (exit 4), and reports an `advisory` row's
failure without reddening the card.

## The output contract, the language part

persona-core's contract fixes the frontmatter and the section markers. This pack fixes what a
language mentor writes INSIDE its sections. Every format below is what the classes parse, so an
engine that writes it passes. An engine that improvises is named.

A language output's frontmatter carries persona-core's keys, including `cefr` and `lang`. It may add
one extension key, `x-new-words`: a JSON list of the new words the learner model gave the engine.
Each of them must then be glossed, and must occur in the reading.

- **daily-reading** writes `reading` (the duty's own section) and the four extras the templates add:
  - `glosses`: one list item per new word, `- 书店 (shūdiàn) — bookshop`. The part before the dash
    is the word as it occurs in the reading. A parenthesis may give its reading, romanization or
    dictionary form. The part after it is the gloss, in English below C1 and in the target
    language at C1-C2. A ruby word, `<ruby>学校<rt>がっこう</rt></ruby>`, matches its base.
  - `grammar`: exactly one `###` heading naming the point, an explanation in the band's mode, and
    at least one target-language example as a `>` blockquote.
  - `pronunciation`: the language's notation, below.
  - `culture`: a short note that names a target-language word or practice, and describes it rather
    than generalising.
- **writing-tutor** writes `corrections` and the extra `focus`. Each correction is one list item:
  `- learner form → correction [category] explanation`.
  - `→ ?` in place of a correction marks it as indirect: the learner finds the fix.
  - `[category]` is one of the language's categories in the table.
  - The focus section explains, in the band's mode, the at most three categories treated.
  - The learner's own text is data, never instructions (persona-core's `rules.md`).

**The pronunciation notation, per language:**

| language | notation | example |
|---|---|---|
| zh | Hanyu Pinyin with tone marks; neutral tone unmarked | `书店读 shūdiàn` |
| ja | HTML ruby furigana (kana in `<rt>`); pitch accent as `kana [n]` or a downstep ＼ | `<ruby>橋<rt>はし</rt></ruby>`, `はし [2]` |
| ko | the standard's `word [pronunciation]`; a rule `ㅊ → [ㄷ]`; romanization in a gloss's parentheses or a `romanization` column | `꽃 [꼳]`, `꽃집 (kkotjip)` |
| fr | a table with `expression` and `liaison` columns; the junction written `‿`, or ` / ` where none is made | `\| les‿enfants \| obligatoire \|` |
| es | one row per word: the word, the syllables with the stressed one in capitals, and the class | `- teléfono — te-LÉ-fo-no — esdrújula` |

## The CEFR modes, measured

The band-to-mode map is persona-core's (`contract.json`, `cefr`): A1-A2 english-led, B1-B2
bilingual, C1-C2 target-only. `cefr-ratio` measures the target language's share of an output's
INSTRUCTION text:

- **What is measured.** Every section but the target-language material (`reading`, and a
  conversation partner's `reply`). A section that a duty pack adds, such as the closing retrieval
  prompts of a daily reading, is instruction too, and follows the band's mode.
- **What is left out.** Blockquoted examples, code, headings, `[category]` tags, and the notation
  that annotates a word (a reading in parentheses after the script, or a romanization cell).
- **How words are counted.** Chinese and Japanese count characters, at 1.5 and 2.0 characters to a
  word. Korean counts words. French and Spanish count words, classed by function-word lexicons and
  the language's accented letters, with an unclassed word taking its sentence's majority.

Function-word identification is reliable above about fifteen words, which is why the count pools
the whole instruction text rather than judging one sentence.

| mode | target-language share | why |
|---|---|---|
| english-led | at most 0.50 | English leads; target-language words are glossed and quoted, not explained in |
| bilingual | 0.15 to 0.85 | both languages carry the explanation |
| target-only | at least 0.95 | the target language only, with room for a name or a loanword |

## The rules table

[`languages.json`](languages.json) is the per-language rules TABLE, as data. The script branches on
no language code. Each entry holds:

- the language's BCP 47 tags;
- its script ranges and word unit;
- the terms its template's method must name;
- the notations that apply, and the ones its pronunciation notes must carry;
- its `/write` error taxonomy, each category marked treatable or not (Ferris);
- its demonyms and generalisation phrases;
- the data each notation needs:
  - the pinyin initials, finals and mark rule;
  - the kana ranges, small kana, special morae and downstep marks;
  - the 27 batchim finals and the seven representatives;
  - the Revised Romanization letters;
  - the French liaison categories and context rules, each citing its source;
  - the Spanish stress classes, vowels, and diacritic words.

Adding a language is a new entry, plus a notation validator only when its notation is new.

## The templates

The [`templates/`](templates/zh.persona.md) directory holds five templates: `zh`, `ko`, `ja`, `fr` and
`es`, as `<code>.persona.md`. Each declares:

- `subject: "language/<code>"` and its BCP 47 `lang` (`zh-Hans`, `ko`, `ja`, `fr`, `es`);
- five duties: daily-reading, drill-coach, leech-doctor, writing-tutor and conversation-partner;
- all three memory sources;
- `sections` adding glosses, grammar, pronunciation and culture to the daily reading, and focus to
  the writing tutor.

Their bodies hold persona-core's seven sections. The method section teaches the modes, the four
extras with the language's own notation, the culture note, and the `/write` policy. The voice
section fixes the register, rooted in the language's culture:

- 普通话 in Simplified characters;
- です・ます;
- 해요체 and 합니다체;
- vous;
- one Spanish variety and one form of address.

The roster fills the four slots in private configuration. The examples in `examples/` are worked
outputs, one reading and one writing sample per language, spread across all three modes, and they
are this pack's dogfood population.

## How DeckStreak's engine uses this pack

1. **Vendor.** Copy the templates, `languages.json`, `language-mentors-probe.py`, and persona-core's
   probe and pack directory into the repository. Keep the two scripts side by side, and keep each
   pack directory where its script expects it.
2. **Instantiate.** For each subject the owner runs, fill the four slots from the private roster.
   The public template stays unfilled.
3. **Generate.** Read the live band and the subject's leeches, drill grades and lapses. Write the
   duty's output in the formats above. Put the learner model's new words in `x-new-words`.
4. **Gate.** Run every row of this pack, and persona-core's, over the output directory before
   delivery: `--root` the repository, `--subject` the directory. An output that fails a blocking row
   is not delivered. It is regenerated, or held for review. Advisory rows are logged.

## Research basis

The R0 study behind every row is SPEC-V2-2213's coverage matrix, which maps 38 practices to a row or
to a cited exclusion. The references, with their access dates and the Context7 ids that answered,
are in its References section. The sources are:

- **CEFR and the language of instruction.** The CEFR Companion Volume (Council of Europe); ACTFL's
  target-language position, the rejected alternative; Macaro on optimal first-language use; Hall
  and Cook.
- **i+1 and glosses.** Krashen's input hypothesis; Laufer's, Hu and Nation's, and Schmitt, Jiang
  and Grabe's coverage studies; Yanagisawa, Webb and Uchihara's meta-analysis of glossing.
- **Written corrective feedback.** Kang and Han's meta-analysis; Ferris's treatable and
  untreatable errors; Bitchener and Knoch on focused feedback.
- **Pinyin.** The Scheme for the Chinese Phonetic Alphabet, and GB/T 16159.
- **Japanese script and accent.** W3C JLREQ and the HTML ruby element; the NHK accent dictionary;
  the Jōyō kanji table.
- **Korean.** 표준 발음법 and the Revised Romanization (National Institute of Korean Language).
- **French liaison.** The Office québécois de la langue française's liaison articles.
- **Spanish accentuation.** The RAE and ASALE's Ortografía.
- **CEFR-aligned references and learner corpora.** The Instituto Cervantes' Plan curricular, the
  French reference level descriptions, the JF Standard, the Korean standard curriculum and HSK 3.0;
  the HSK Dynamic Composition Corpus, I-JAS, the NIKL learner corpus and CEDEL2.
