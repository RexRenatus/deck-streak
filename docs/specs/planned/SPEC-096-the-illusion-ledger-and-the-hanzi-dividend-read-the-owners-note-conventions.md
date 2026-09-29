# SPEC-096: the Illusion Ledger and the Hanzi Dividend read the owner's note conventions

- **Wave:** W4. **Issues:** #137, #140 (epic #5). **Context(s):** `deck-streak-ingest` (the
  direction counts, and the transfer reads with their bounds); `deck-streak-insights` (the
  direction classifier and ledger, the variant fold and the transfer report, their registry rows);
  `deck-streak-coordination` (each deck's language and the tracked roots, passed to both
  instruments); the Mini App (two sections of the insights screen).
- **Decided by:** ADR-012 (the parity oracle proves the math), ADR-094 (the frame), ADR-095 (the
  reads keep SPEC-023's scope, and only a read of answers keeps its window) and ADR-096 (the owner's
  note conventions are one private file the kernel loads).
- **Prerequisites:** SPEC-029, SPEC-045 (the readings taxonomy's writing roots), SPEC-071 (each
  card's course), SPEC-092 (the law root setting), SPEC-094 (the frame, the conventions, the
  structure reads) and SPEC-095 (the Echo's pooling, which the transfer report reuses).
  **Mutation band:** `S09600-S09699`.
- **Status:** planned (in `docs/specs/planned/`) until the delivery that builds it moves it to
  `docs/specs/` with its tests and `docs/red-first/SPEC-096.md` (ADR-016).

## 1. The problem, measured

- **Nothing exists yet.** SPEC-094 (planned) loads the conventions and reads templates and fields;
  nothing classifies a card's direction or reads a note's Han characters.
- **What is ported.** The Illusion Ledger (`direction.py:classify`, `_resolve_bucket`,
  `_resolve_language` and `build_ledger`, over `anki_reader.py:read_direction_card_counts` and
  `read_direction_answer_counts`), and the Hanzi Dividend (`transfer.py:build_transfer_report`,
  `fold`, `parse_note`, `_snippet_for` and its reads `read_reviewed_note_stats`,
  `read_note_fields`, `read_reviewed_home_dids`, `read_gap_reviewed_nids` and `read_gap_census`).
- **The weekly report is W5.** Both issues depend on #130; W4 stores the reports and shows them on
  the insights screen (ADR-094).
- **Traps a hand port falls into.**
  - A direction comes only from the conventions' curated rules, over names NFC-normalised,
    casefolded, with `->` rewritten to an arrow and whitespace collapsed: the (note type, template)
    pair first, then the template alone, then a note-type substring, else unknown; the four
    outcomes are recognition, cued recall, production and unknown. There is no
    default bucket, and the words recall, recognition and recognize are never evidence (SPEC-094
    R2 refuses a rule that would use them).
  - An unresolved cloze ordinal falls back to ordinal 0, as the Echo's cells do, and a card is unknown
    only when ordinal 0 has no template either.
  - A card's language is its course (SPEC-071 R2); a deck under a readings writing root whose
    second segment is a language's display name takes that language (SPEC-045 R2). Anything else is
    counted as unmapped, never folded into a bucket.
  - The ledger's answers are the first review-type answer of each card on each study day, since the
    window's start; mature means a last interval of 21 days or more going into the answer.
    Suspended cards leave the card and mature counts and are counted apart. The window is 90 days,
    held between 30 and 400.
  - A gap needs both arms answered and 60% of the language's cards classified; otherwise it is
    absent, never zero.
  - The Hanzi Dividend takes Han characters (U+4E00 to U+9FFF and U+3400 to U+4DBF) from item
    fields only: never a field on the conventions' excluded list, a choice field, the rank field or
    a meaning field. Characters are compared through the vendored variant fold, so a traditional
    and a simplified form are one identity while each keeps its written form.
  - Its window is 180 days back from the collection's own latest answer, never the clock, and it
    reduces answers to per-note aggregates while it reads. Coverage gaps (decks with reviewed Han
    items that no language claims) are found over the whole scoped log, because a deck's existence
    is not a windowed fact; decks under a writing root or the law root are tracked, never gaps.
  - The reads hold at most 400 notes' text at once, 100,000 notes in all and 200,000 notes in the
    gap census; past a bound the report says its numbers are lower bounds.
  - The subsidy pools rank bands of 300 through the Echo's Mantel-Haenszel pooling and its 20 per
    arm; fewer than 2 bands is null. The candidates are ranked by evidence, at most 5, and each
    snippet is 15 characters either side of the shared character inside the field that holds it.
  - Anki collates the name columns of fields and note types `unicase`; as SPEC-023's read does, no
    read registers a collation or orders, groups or seeks on such a column.
  - The wording is "shares a character with", never "same word" or its kin.
- **What the parity oracle proves.** The classifier and the language attribution over synthetic
  rules, names and decks; the ledger; the direction counts over a synthetic collection; the fold,
  the note parse, the snippet and the transfer report over synthetic notes; and the constants.
  Every golden's rules, fields and decks are synthetic: the adapter patches them in where the
  predecessor's module reads them, so no name of the owner's enters a golden.
- **Prerequisites.** SPEC-029, SPEC-045, SPEC-071, SPEC-092, SPEC-094 and SPEC-095, as the header
  names them.

## 2. Requirements

The Illusion Ledger (#137)

R1. A (note type, template) pair's direction equals `goldens/direction_classify.json`
    (`direction.py:classify`), over the rules of the conventions file (SPEC-094 R1).
R2. `crates/ingest/src/direction_reads.rs` reads each (home deck, note type, ordinal) cell's cards,
    mature cards and suspended cards, and its first-answer-per-day answers, passes, mature answers
    and mature passes since a given instant, by a given study-day rule, keeping SPEC-023's scope, equal to
    `goldens/direction_counts.json` (the two `anki_reader.py` reads over a synthetic collection).
R3. Coordination gives each home deck its language: its course, or a writing root's language,
    equal to `goldens/direction_language.json` (`direction.py:_resolve_language`).
R4. The ledger equals `goldens/illusion_ledger.json` (`direction.py:build_ledger`). It is a weekly
    instrument over a 90-day window, and its constants (90, 30, 400, 60.0% and 30 answers) equal
    `goldens/direction.constants.json`.

The Hanzi Dividend (#140)

R5. `crates/ingest/src/transfer_reads.rs` reads the reviewed notes' per-note aggregates over 180 days
    back from the latest answer, reduced to the first answer per card and study day while reading;
    the text of the source languages' notes 400 at a time, stopping at 100,000; the decks ever
    reviewed; and the gap census, stopping at 200,000 notes. Each read keeps SPEC-023's scope,
    returns the name of every read that failed and states when it stopped at a bound.
R6. The variant fold equals `goldens/transfer_fold.json` (`transfer.py:fold`), a note's parse equals
    `goldens/transfer_parse.json` (`transfer.py:parse_note`), and a candidate's snippet equals
    `goldens/transfer_snippet.json` (`transfer.py:_snippet_for`), with the field names of the
    conventions file.
R7. The report equals `goldens/transfer_report.json` (`transfer.py:build_transfer_report`). The two
    source languages are the courses coded `zh` and `ja`; without both, the instrument reports that
    it is not configured. Its constants (180, 5, 5, 3, 300, 15, 2, the Han ranges and the three
    bounds) equal `goldens/transfer.constants.json`.
R8. The report's stored JSON holds note snippets and meanings; `privacy.json`'s
    `research-instruments` category and its `PRIVACY.md` line say so.

The surfaces

R9. The insights screen shows the Illusion Ledger per language with its buckets, its gaps and the
    caveat that the gap is a lower bound, and omits the section when nothing is classified. The
    Hanzi Dividend section shows the subsidy or its named null, up to 5 candidates with both
    snippets and meanings, up to 3 coverage gaps with a count of the rest, and a lower-bound line
    when a read stopped at a bound. It says "shares a character with", never "same word", and marks
    each Han snippet with its language.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the classifier equals the golden over synthetic rules | `the_direction_classifier_matches_the_predecessors_golden` |
| A2 | the direction counts over a synthetic collection equal the predecessor's reads | `the_direction_counts_match_the_predecessors_reads` |
| A3 | each deck's language equals the golden | `each_decks_language_matches_the_predecessors_golden` |
| A4 | the ledger equals the golden | `the_illusion_ledger_matches_the_predecessors_golden` |
| A5 | the direction constants equal the predecessor's | `the_direction_constants_equal_the_predecessors` |
| A6 | the transfer reads hold 400, 100,000 and 200,000 and say when they stopped | `the_transfer_reads_hold_their_bounds` |
| A7 | the note-stats read reduces answers while it reads, over 180 days from the latest answer | `the_note_stats_reduce_at_the_read_from_the_latest_answer` |
| A8 | the fold equals the golden | `the_variant_fold_matches_the_predecessors_golden` |
| A9 | a note's parse and a snippet equal the goldens | `the_note_parse_and_snippet_match_the_predecessors_goldens` |
| A10 | an excluded field never creates a pair | `an_excluded_field_never_creates_a_pair` |
| A11 | the transfer report equals the golden | `the_transfer_report_matches_the_predecessors_golden` |
| A12 | the transfer constants equal the predecessor's | `the_transfer_constants_equal_the_predecessors` |
| A13 | a tracked root is never a coverage gap | `a_tracked_root_is_never_a_coverage_gap` |
| A14 | the Illusion section is omitted when nothing is classified | `is omitted when nothing is classified` |
| A15 | the Hanzi section says shares a character with, never same word | `says shares a character with and never same word` |
| A16 | each Han snippet carries its language | `marks each han snippet with its language` |
| A17 | the Illusion Ledger is passed its reads: the direction counts since its window's start, 90 days of 86,400,000 ms before the run's instant and never before 0, with the kernel's study-day rule, and a window of 90 days to record | `the_illusion_ledger_is_passed_its_window_and_study_day_rule` |
| A18 | the Hanzi Dividend is passed its reads: the transfer reads' window of 180 days back from the latest answer, the first answer per card and study day by the kernel's study-day rule, and the tracked roots | `the_hanzi_dividend_is_passed_its_window_and_study_day_rule` |

```acceptance
A1: cargo test -p deck-streak-insights --test illusion -- --exact the_direction_classifier_matches_the_predecessors_golden
A2: cargo test -p deck-streak-ingest --test direction_reads -- --exact the_direction_counts_match_the_predecessors_reads
A3: cargo test -p deck-streak-coordination --test instrument_languages -- --exact each_decks_language_matches_the_predecessors_golden
A4: cargo test -p deck-streak-insights --test illusion -- --exact the_illusion_ledger_matches_the_predecessors_golden
A5: cargo test -p deck-streak-insights --test illusion -- --exact the_direction_constants_equal_the_predecessors
A6: cargo test -p deck-streak-ingest --test transfer_reads -- --exact the_transfer_reads_hold_their_bounds
A7: cargo test -p deck-streak-ingest --test transfer_reads -- --exact the_note_stats_reduce_at_the_read_from_the_latest_answer
A8: cargo test -p deck-streak-insights --test hanzi -- --exact the_variant_fold_matches_the_predecessors_golden
A9: cargo test -p deck-streak-insights --test hanzi -- --exact the_note_parse_and_snippet_match_the_predecessors_goldens
A10: cargo test -p deck-streak-insights --test hanzi -- --exact an_excluded_field_never_creates_a_pair
A11: cargo test -p deck-streak-insights --test hanzi -- --exact the_transfer_report_matches_the_predecessors_golden
A12: cargo test -p deck-streak-insights --test hanzi -- --exact the_transfer_constants_equal_the_predecessors
A13: cargo test -p deck-streak-coordination --test instrument_languages -- --exact a_tracked_root_is_never_a_coverage_gap
A14: pnpm exec vitest run web/app/src/lib/insights/IllusionSection.test.ts -t "is omitted when nothing is classified"
A15: pnpm exec vitest run web/app/src/lib/insights/HanziSection.test.ts -t "says shares a character with and never same word"
A16: pnpm exec vitest run web/app/src/lib/insights/HanziSection.test.ts -t "marks each han snippet with its language"
A17: cargo test -p deck-streak-coordination --test instrument_languages -- --exact the_illusion_ledger_is_passed_its_window_and_study_day_rule
A18: cargo test -p deck-streak-coordination --test instrument_languages -- --exact the_hanzi_dividend_is_passed_its_window_and_study_day_rule
```

## 3a. What the box run judges

The box run (`scripts/box-packs.sh`, ADR-069) judges these over the committed tree and posts its
verdict on the pull request as the `box/packs` status. They have no line in the acceptance fence,
because no public test can run a pack's row. The privacy-gdpr, accessibility and cjk-typography
packs stay enforced; no check is deferred or lifted for this delivery, so the private wiring does
not change when it merges.

| id | criterion | decided by |
|---|---|---|
| B1 | over `privacy.json`, `PRIVACY.md` and `crates/coordination/src/data_rights.rs`: the `research-instruments` category names the note snippets and meanings its reports hold, and export and erase still cover `instrument_reports` | the privacy-gdpr pack |
| B2 | over `web/app/src/lib/insights/IllusionSection.svelte` and `HanziSection.svelte`: both sections pass the accessibility audit in both Telegram colour schemes | the accessibility pack |
| B3 | over `web/app/src/lib/insights/HanziSection.svelte`: Han text is marked with its language so each renders in its own glyph forms | the cjk-typography pack |

## 4. File manifest

| file | context | change |
|---|---|---|
| `crates/ingest/src/direction_reads.rs` | `deck-streak-ingest` | added: the direction counts |
| `crates/ingest/src/transfer_reads.rs` | `deck-streak-ingest` | added: the transfer reads and their bounds |
| `crates/ingest/src/lib.rs` | `deck-streak-ingest` | changed: the modules |
| `crates/ingest/tests/direction_reads.rs` | `deck-streak-ingest` | added: A2 |
| `crates/ingest/tests/transfer_reads.rs` | `deck-streak-ingest` | added: A6, A7 |
| `crates/insights/src/direction.rs` | `deck-streak-insights` | added: the classifier and the ledger |
| `crates/insights/src/transfer.rs` | `deck-streak-insights` | added: the fold table, the parse, the pooling and the report |
| `crates/insights/src/registry.rs` | `deck-streak-insights` | changed: two weekly rows |
| `crates/insights/src/lib.rs` | `deck-streak-insights` | changed: the modules |
| `crates/insights/tests/illusion.rs` | `deck-streak-insights` | added: A1, A4, A5 |
| `crates/insights/tests/hanzi.rs` | `deck-streak-insights` | added: A8 to A12 |
| `crates/coordination/src/instruments.rs` | `deck-streak-coordination` | changed: each deck's language and the tracked roots, passed to both instruments, the Illusion Ledger's window start and study-day rule (A17), and the Hanzi Dividend's window and study-day rule (A18) |
| `crates/coordination/tests/instrument_languages.rs` | `deck-streak-coordination` | added: A3, A13, A17, A18 |
| `privacy.json` | repo | changed: the `research-instruments` category names note snippets and meanings |
| `PRIVACY.md` | repo | changed: the category's line |
| `web/app/src/lib/insights/IllusionSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/IllusionSection.test.ts` | miniapp | added: A14 |
| `web/app/src/lib/insights/HanziSection.svelte` | miniapp | added |
| `web/app/src/lib/insights/HanziSection.test.ts` | miniapp | added: A15, A16 |
| `web/app/src/lib/insights/insights.ts` | miniapp | changed: the two reports' types |
| `.sqlx/` | workspace | changed: the offline cache for the new queries |
| `tools/parity-oracle/registry/spec_096.py` | repo | added: this SPEC's registrations (SPEC-029) |
| `tools/parity-oracle/goldens/direction_classify.json` | repo | added: the golden of `direction.py:classify` (adapter; synthetic rules patched in) |
| `tools/parity-oracle/goldens/direction_counts.json` | repo | added: the golden of `anki_reader.py:read_direction_card_counts` and `read_direction_answer_counts` (adapter; a temporary synthetic collection) |
| `tools/parity-oracle/goldens/direction_language.json` | repo | added: the golden of `direction.py:_resolve_language` (adapter; synthetic courses and writing roots) |
| `tools/parity-oracle/goldens/illusion_ledger.json` | repo | added: the golden of `direction.py:build_ledger` (adapter) |
| `tools/parity-oracle/goldens/direction.constants.json` | repo | added: the direction constants (constants) |
| `tools/parity-oracle/goldens/transfer_fold.json` | repo | added: the golden of `transfer.py:fold` (function) |
| `tools/parity-oracle/goldens/transfer_parse.json` | repo | added: the golden of `transfer.py:parse_note` (adapter; synthetic fields patched in) |
| `tools/parity-oracle/goldens/transfer_snippet.json` | repo | added: the golden of `transfer.py:_snippet_for` (function) |
| `tools/parity-oracle/goldens/transfer_report.json` | repo | added: the golden of `transfer.py:build_transfer_report` (adapter; synthetic notes, decks and census) |
| `tools/parity-oracle/goldens/transfer.constants.json` | repo | added: the transfer constants (constants) |
| `scripts/mutation-rows.d/S09600-S09699.json` | repo | added: the rows of §9 |
| `docs/specs/SPEC-096-the-illusion-ledger-and-the-hanzi-dividend-read-the-owners-note-conventions.md` | docs | moved from `docs/specs/planned/` |
| `docs/red-first/SPEC-096.md` | docs | added |
| `changelog.d/` fragment | repo | added |

## 5. What this does NOT do

- It sends no weekly report; the weekly report reads the stored reports (#130).
- It offers no command for the Illusion Ledger's other windows; the predecessor never wired one
  (#137).
- It serves neither report to the agent's machine read tool (#157).
- It edits no rule of the conventions file from the Mini App (#57).
- It imports none of the predecessor's reports (#61).

## 6. Risks

- **An owner's rule or field name reaches a golden or a test.** Prevented by the adapters patching
  synthetic rules and fields, and caught by the public scrub over the tree.
- **A gap is read as a measured zero.** Detected by A4, whose golden holds a language under the
  60% coverage gate.
- **A choice or meaning field creates a false pair.** Detected by A10.
- **The note-text read holds a whole collection.** Detected by A6.

## 7. Parity goldens

Registered in `tools/parity-oracle/registry/spec_096.py` and generated on the owner's checkout of the
predecessor at `27ee2bc` (SPEC-029). Every case is synthetic.

| golden | the predecessor's function | kind | the adapter builds |
|---|---|---|---|
| `direction_classify` | `direction.py:classify` | adapter | patches the three rule tables with synthetic rules, including an arrow and a casefold case |
| `direction_counts` | `anki_reader.py:read_direction_card_counts`, `read_direction_answer_counts` | adapter | a temporary collection file with suspended, mature and borrowed cards and several answers a day |
| `direction_language` | `direction.py:_resolve_language` | adapter | patches the course and writing-root tables with synthetic decks |
| `illusion_ledger` | `direction.py:build_ledger` | adapter | synthetic count rows, names and decks, under, at exactly 60% and over the coverage gate, one-armed and unmapped |
| `transfer_fold` | `transfer.py:fold` | function | none: every folded character and a character outside the table |
| `transfer_parse` | `transfer.py:parse_note` | adapter | patches the field-name constants with synthetic names; excluded, choice, rank and meaning fields |
| `transfer_snippet` | `transfer.py:_snippet_for` | function | none: a character at a field's start, middle and end |
| `transfer_report` | `transfer.py:build_transfer_report` | adapter | synthetic stats, notes, fields and a census, null for each reason and resolved, truncated and not, and a pool that uses exactly 2 rank bands |
| `direction.constants`, `transfer.constants` | the modules' constants | constants | none |

## 8. Tables and the v9 import

This SPEC adds no table; both reports are stored in SPEC-094's `instrument_reports`, whose
`research-instruments` category now names the snippets and meanings they hold. None is imported
(#61).

## 9. Mutation rows

| row | target | what it guards | killer |
|---|---|---|---|
| `S09601-PAIR-BEFORE-TEMPLATE` | `crates/insights/src/direction.rs` | the pair rule is tried before the template rule | `illusion::the_direction_classifier_matches_the_predecessors_golden` |
| `S09602-COVERAGE-GATE` | `crates/insights/src/direction.rs` | a gap needs 60% classified | `illusion::the_illusion_ledger_matches_the_predecessors_golden` |
| `S09603-SUSPENDED-APART` | `crates/ingest/src/direction_reads.rs` | a suspended card leaves the card count | `direction_reads::the_direction_counts_match_the_predecessors_reads` |
| `S09604-EXCLUDED-FIELD` | `crates/insights/src/transfer.rs` | an excluded field contributes no character | `hanzi::an_excluded_field_never_creates_a_pair` |
| `S09605-MIN-BANDS` | `crates/insights/src/transfer.rs` | fewer than 2 rank bands is null | `hanzi::the_transfer_report_matches_the_predecessors_golden` |
| `S09606-NOTE-BATCH` | `crates/ingest/src/transfer_reads.rs` | the batch of 400 notes | `transfer_reads::the_transfer_reads_hold_their_bounds` |
| `S09607-TRACKED-ROOT` | `crates/coordination/src/instruments.rs` | a writing or law root is never a gap | `instrument_languages::a_tracked_root_is_never_a_coverage_gap` |
