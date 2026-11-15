---
status: proposed
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-673: the product documents follow ADR-589 by dated notes that keep their text, held by a section census

## Context and Problem Statement

ADR-589 decided that DeckStreak's surfaces are the web app and the universal iPhone and iPad app,
that the Telegram Mini App and the bot are deprecated, and that Rex is to be DeckStreak's only
scheduler; the owner's signed ruling it carries asked for the PRD and the two campaign specs to
change with the charter. The delivery that landed it changed the charter, the ADR, the ruling and a
changelog fragment. At this delivery's base (SPEC-651 section 1), the design system still opens "How
the Mini App looks and behaves", and none of its eight sections names a deprecation, though six
name a Telegram surface; the PRD's identity note marks four statements as naming a deprecated
surface without restating them; SPEC-001 and SPEC-334 name ADR-589 nowhere; and no test reads
either product document. How do the product documents and the campaign specs follow ADR-589, and
what holds them?

## Decision Drivers

- Records stay records (ADR-589): a landed SPEC is history, and its text is kept.
- The product documents read in layers: each of their amendments says the text above is kept, as the charter does.
- ADR-589's Confirmation names a docs check that reads red on the earlier text.
- A wrapped sentence names a surface on one line and the word deprecated on the next.

## Considered Options (the alternatives it was chosen against)

- Chosen: one dated note at the end of each product document's Amendments, one marked line under each design-system heading whose section names a Telegram surface, and one appended dated note in each campaign spec, held by a standard-library section census, because it follows ADR-589 and keeps every earlier line.
- Restate the two product documents in place: rejected because each document's notes say the text above is kept, so restating would falsify them and break the documents' own amendment form.
- A line-level census: rejected because a wrapped sentence names a surface on one line and the word deprecated on the next, so a correct section would fail it.
- No test: rejected because ADR-589's Confirmation names a docs check that reads red on the earlier text, and nothing would hold the notes once they are written.

## Decision Outcome

Chosen option: dated notes and marks that keep every earlier line, held by a section census.

- **D1. The product documents.** The PRD and the design system follow ADR-589 by one dated note
  each, opening `Amendment (2026-10-29, ADR-589)`, at the end of their Amendments section, and no
  line of their text is removed or changed: both already read in layers, as the charter does and
  as the PRD's identity note did. The PRD's note restates, for the two apps, the four statements
  its identity note marks.
- **D2. The marks.** Each design-system section above its Amendments whose text names a Telegram
  surface (the lead, Principles, Tokens, Accessibility and Theming) gains one marked line under its
  heading, in the PRD's own form (`Amended below (...)`), saying that the surface is deprecated.
  The Amendments section, which names the Mini App's Telegram mapping, gains the dated note at its
  end. The PRD's sections already carry the identity note's marks.
- **D3. The campaign specs.** SPEC-001 and SPEC-334 are records: each gains one appended dated
  note, SPEC-001 a bullet that continues its section 14 and SPEC-334 a new section 12, and each
  keeps its earlier text byte for byte, held by a hash of its earlier lines.
- **D4. The census.** `scripts/tests/test_product_docs.py`, standard library only, holds D1 to D3:
  the section census (the section is the unit, because a wrapped sentence names a surface on one
  line and `deprecated` on the next), the one dated note in each product document, and the
  campaign specs' kept text with their one note. Each test refuses a planted inline text before it
  reads a document, and refuses an examined count of zero.

### Consequences

- Good, because the design system no longer presents the Mini App as a current surface without
  saying it is deprecated, and the PRD states what replaces each statement its identity note marks.
- Good, because no line of the four documents changes: history stays readable, and SPEC-001's
  parity matrix reads as before.
- Bad, because the two product documents read in more layers: a reader takes a marked section with
  the dated note below it.
- Bad, because A3 pins two hashes: a delivery that must change an earlier line of either campaign
  spec re-pins it in its own change.

### Confirmation

- `scripts/tests/test_product_docs.py` reads red by name at the earlier text (A1 on six
  design-system sections, A2 on both product documents, A3 on both campaign specs), and green once
  the notes and marks are in, with the test unchanged (`docs/red-first/SPEC-651.md`).
- `scripts/tests/test_parity_matrix.py` stays green: the SPEC-001 note adds no `## ` line and no
  table row.

## What would make this wrong

- The removal delivery takes the Telegram mapping out of the web app: the marks then describe
  history, and that delivery adds its own note.
- A ruling orders the product documents restated in place.
- Either campaign spec is redacted under the public-prose rule: A3 is then re-pinned in that
  delivery.

## More Information

- ADR-589 (the surfaces, the deprecation and the scheduler); SPEC-651 (the requirements and the
  criteria); SPEC-001 and SPEC-334, the two campaign specs.
- `docs/schematics/product-docs.md` (the documents, their notes and marks, and the census).
