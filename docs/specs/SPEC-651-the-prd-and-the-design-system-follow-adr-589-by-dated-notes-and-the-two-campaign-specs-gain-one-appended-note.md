# SPEC-651: the PRD and the design system follow ADR-589 by dated notes, and the two campaign specs gain one appended note

- **Issue:** none; ADR-589 decided the change, and this delivery carries it into the product documents.
  **Context(s):** the product documents (`docs/PRD.md`, `docs/DESIGN_SYSTEM.md`), the two campaign
  SPECs (`docs/specs/`) and the repository's own guard tests (`scripts/tests/`).
- **Decided by:** ADR-673 (the product documents follow ADR-589 by dated notes that keep their text,
  held by a section census); ADR-589 (the surfaces, the deprecation and the scheduler).
- **Schematic:** `docs/schematics/product-docs.md`, new.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-651.md`. **Models:** none (ADR-673: four documents gain notes and marks, and a
  standard-library census reads them).

## 1. The problem, measured

ADR-589 decided that DeckStreak's surfaces are the web app and the universal iPhone and iPad app,
that the Telegram Mini App and the bot are deprecated, and that Rex is to be DeckStreak's only
scheduler. The delivery that landed it changed four files: the charter, the ADR, the owner's signed
ruling and a changelog fragment. Each figure below was read at this delivery's base, `dev` at
`0756b82`, by the command beside it.

- **The design system names no deprecation.** `grep -c -i deprecat docs/DESIGN_SYSTEM.md` prints 0,
  and `grep -c ADR-589 docs/DESIGN_SYSTEM.md` prints 0. Its lead opens "How the Mini App looks and
  behaves". Split at its `#` and `##` headings as A1 splits it, it holds 8 sections; 6 of them name
  a Telegram surface and none says the surface is deprecated: the lead, Principles, Tokens,
  Accessibility, Theming and Amendments (A1's red, `docs/red-first/SPEC-651.md`).
- **The PRD restates nothing.** `grep -c ADR-589 docs/PRD.md` prints 1, inside its identity note
  (lines 73 to 78). That note marks four statements as naming a deprecated surface (the problem's
  last sentence, line 13; the second and third goals, lines 25 and 26; the first open question,
  line 58) and restates none of them. The PRD holds no paragraph opening
  `Amendment (2026-10-29, ADR-589)` (A2's red). Its 10 sections hold 4 that name a Telegram
  surface, each already beside the word deprecated.
- **The campaign specs name no ADR-589.** SPEC-001 (433 lines) and SPEC-334 (256 lines):
  `grep -c ADR-589` prints 0 in each, and `grep -c -i deprecat` prints 0 in each.
- **No test holds any of it.** `git grep -l -E 'PRD\.md|DESIGN_SYSTEM'` finds 15 files, none under
  `scripts/`, `web/`, `crates/` or `ios/`: no test reads either product document by name.

## 2. Requirements

R1. `docs/DESIGN_SYSTEM.md`: under each of its headings `# Design System`, `## Principles`,
    `## Tokens`, `## Accessibility` and `## Theming`, directly after the blank line that follows the
    heading, one marked line opening `Amended below (ADR-589):` says that the Telegram surface the
    section names is deprecated, and one blank line follows it. One blank line and a dated note
    opening `Amendment (2026-10-29, ADR-589)` are appended at the end of the file, the end of its
    Amendments section.
R2. `docs/PRD.md`: one blank line and a dated note opening `Amendment (2026-10-29, ADR-589)` are
    appended at the end of the file, the end of its Amendments section. The note restates, for the
    web app and the universal iPhone and iPad app, the four statements the identity note marks.
R3. SPEC-001: one dated bullet marked `(2026-10-29, ADR-589)` is appended at the end of the file with
    no blank line before it, continuing section 14's last list. Its 433 lines at the base stay its
    first 433 lines, byte for byte.
R4. SPEC-334: one blank line and a new section,
    `## 12. Amendment (2026-10-29, ADR-589): the surfaces, the cutover and the scheduler`, are
    appended at the end of the file. Its 256 lines at the base stay its first 256 lines, byte for
    byte.
R5. No line of the four documents is removed or changed: `git diff --numstat` over the base reads
    `15 0` for the design system, `9 0` for the PRD, `10 0` for SPEC-001 and `11 0` for SPEC-334.
    No requirement, acceptance criterion or parity row of either campaign spec changes.
R6. `scripts/tests/test_product_docs.py` holds R1 to R4 by three tests, standard library only
    (`unittest`, `re`, `hashlib`, `pathlib`) plus the suite's own helper (`from _support import
    REPO, examined`). Each test first asserts that its checker refuses a planted member, an inline
    text and never a file, then runs the checker over the documents, and refuses an examined count
    of zero. Every failure names a document by its repository-relative path.
    - A1 splits `docs/PRD.md` and `docs/DESIGN_SYSTEM.md` at the lines matching `^#{1,2} ` (the
      text before the first heading, if any, is its own section). Every section whose text matches
      `(?i)mini[\s-]?apps?|telegram|\bbots?\b|initdata` also matches `(?i)deprecat`, and a failure
      names each offending document and heading. Its planted member is a two-section text whose
      second section names the Mini App without the word.
    - A2 reads each of the two documents' paragraphs (the lines between blank lines, whitespace
      normalised): exactly one opens `Amendment (2026-10-29, ADR-589)`, and it names `web app`,
      `iPhone and iPad` and `deprecated`. Its planted members are a text with no such paragraph and
      a text with two.
    - A3 takes a text, a line count n and a sha256 pin: the sha256 of the text's first n lines,
      each with its newline, equals the pin, and the text after them holds `(2026-10-29, ADR-589)`
      exactly once and names `web app`, `iPhone and iPad`, `deprecated` and `Rex` (whitespace
      normalised). SPEC-001 is read with 433 and
      `8f532904fcee5400ed7f08243e37acef99f3959c244b1bd88708d616108741cb`, SPEC-334 with 256 and
      `f187f6a5f68de754f8a9e75fd29f3634ca592de161814ecb9f91672f4acd5dab`, the hashes of their text
      at the base. Its planted members, made before any document is read, are a few inline lines
      whose pin the test computes itself: those lines with one character changed and a note after
      them (refused by the hash), and those lines alone (refused for no note).

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every section of the PRD and the design system that names a Telegram surface also says it is deprecated, and a planted section naming the Mini App without the word is refused (R1, R2, R6) | `test_product_docs.py` |
| A2 | each of the two product documents holds exactly one paragraph opening the dated ADR-589 note, naming the web app, the iPhone and iPad app and the deprecation; a planted text with no such paragraph, and one with two, are refused (R1, R2, R6) | `test_product_docs.py` |
| A3 | each campaign spec's earlier lines hash to their pin and the text after them holds the dated mark exactly once, naming the web app, the iPhone and iPad app, the deprecation and Rex; planted lines with one character changed, and the same lines with no note, are refused (R3, R4, R6) | `test_product_docs.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_every_section_naming_a_telegram_surface_says_deprecated
A2: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_each_product_doc_holds_one_dated_adr_589_note
A3: python3 -m unittest discover -s scripts/tests -p test_product_docs.py -k test_each_campaign_spec_keeps_its_text_and_gains_one_dated_note
```

Each criterion is red, on an assertion, at the commit that adds the tests with the four documents
unchanged, and green at the commit that adds the notes and marks, with the tests unchanged
(`docs/red-first/SPEC-651.md`).

## 4. File manifest

| file | context | change |
|---|---|---|
| `docs/specs/SPEC-651-the-prd-and-the-design-system-follow-adr-589-by-dated-notes-and-the-two-campaign-specs-gain-one-appended-note.md` | this SPEC | added |
| `docs/schematics/product-docs.md` | the documents, their notes and marks, and the census that reads them | added |
| `docs/decisions/ADR-673-the-product-documents-follow-adr-589-by-dated-notes-that-keep-their-text-held-by-a-section-census.md` | the decision | added |
| `changelog.d/product-docs-adr-589-notes-651.md` | the changelog fragment | added |
| `scripts/tests/test_product_docs.py` | the census (A1 to A3) | added |
| `docs/red-first/SPEC-651.md` | the red-first record | added |
| `docs/DESIGN_SYSTEM.md` | five marked lines and the dated note (R1) | changed |
| `docs/PRD.md` | the dated note (R2) | changed |
| `docs/specs/SPEC-001-campaign-prd-parity-and-waves.md` | one appended dated bullet (R3) | changed |
| `docs/specs/SPEC-334-app-campaign-prd-web-and-ios-clients-over-the-engine.md` | an appended section 12 (R4) | changed |

## 5. What this does NOT do

- It removes no Telegram code and leaves the Mini App's Telegram mapping in the web app, as the marked sections say; the removal delivery takes it out with its own ADR and adds its own note beside these marks (#1046).
- It builds no native push: APNs and web push carry the one router in their own delivery, which the removal waits for (#640).
- It changes no line of the README, `AGENTS.md` or the charter; their Telegram lines change with the retirement of the bot and the Mini App (#1046).

## 6. Risks

- **A parser of SPEC-001 reading the note as a row.** `scripts/tests/test_parity_matrix.py` splits
  SPEC-001 at its `## ` lines and reads the first table under Appendix A and Appendix B. The note
  adds no `## ` line and no table row, and that module runs whole at the green commit.
- **A whole-folder reader.** `scripts/tests/test_deploy_templates.py` reads every file under `docs/`
  for a password hash in its PHC form. No note or mark holds one; that module runs a deploy script,
  so CI runs it.
- **A later change to a pinned spec.** A3 pins the hash of each campaign spec's earlier lines. A
  delivery that must change one of those lines (a redaction under the public-prose rule) re-pins A3
  in its own change; until it does, A3 reads red by the spec's path.
- **A wrapped sentence.** A line-level census would refuse a sentence that names a surface on one
  line and the word deprecated on the next, so the section is the unit (ADR-673).
- **A new section naming a surface.** A later section of either product document that names a
  Telegram surface without the word reads red by its heading; A1 is the detector.
