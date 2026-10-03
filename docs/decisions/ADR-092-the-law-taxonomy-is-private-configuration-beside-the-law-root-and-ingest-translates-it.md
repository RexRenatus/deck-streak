---
status: "proposed"
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The law taxonomy is private configuration beside the law root, and ingest translates a card's law subject

## Context and Problem Statement

The strands (#88), the test-prep board (#135) and the leech board (#133) read a card's law
subject, and the runway (#144) groups unseen cards by it. The predecessor derives the subject from
the deck path (`leeches.py:_law_subject`) with the year bands and the test-prep subtree written as
literals beside its law root (`leeches._LAW_BANDS`, `lsat._LSAT_TRACK`, predecessor `27ee2bc`).
Those literals describe the owner's deck layout. SPEC-023 already reads the law root as a setting
(`DECKSTREAK_LAW_DECK_ROOT`). Where do the year bands and the test-prep subtree live, and which
context turns a deck path into a law subject?

## Decision Drivers

- CHARTER 11: a personal default is configuration with a neutral example value.
- ADR-002: curriculum and insights may not depend on each other; a deck path is Anki's language,
  which ingest translates (ADR-087 did the same for courses).
- ADR-012: the golden drives the predecessor's function with synthetic bands patched in.

## Considered Options (the alternatives it was chosen against)

- Two scope settings beside the law root, read by ingest, which gives each card its law subject — chosen: the layout stays private, it sits beside the root it refines, and every context receives a subject rather than a deck path.
- The predecessor's literals — rejected because they are the owner's deck names, which CHARTER 11 keeps out of the repository.
- The courses file — rejected because it lists language courses (ADR-087), and law is a track, not a course.
- The note conventions file (ADR-096) — rejected because it describes note types and fields, while the law taxonomy describes decks.
- Reuse the readings taxonomy file (SPEC-045), which already names the law roots and bands — rejected because ADR-045 makes it the readings context's schema, and ingest depends only on the kernel (ADR-002), so it may not read the readings context's code or schema.
- Curriculum parsing the deck path itself — rejected because the runway in insights reads the same subject and may not depend on curriculum (ADR-002), so insights would port `_law_subject` a third time; a deck path is Anki's language, which ingest translates once for both (ADR-087).

## Decision Outcome

Chosen option: "Two scope settings beside the law root, read by ingest", because the taxonomy is
deck layout, and deck layout is what ingest's scope settings already hold.

- `DECKSTREAK_LAW_YEAR_BANDS` (a comma-separated list) and `DECKSTREAK_LAW_TEST_PREP_DECK` (one
  deck name) are read with `DECKSTREAK_LAW_DECK_ROOT`; either set without the root refuses the cycle's step (the scope is read on each cycle), naming
  the setting and never a value.
- `crates/ingest/src/law_subject.rs` gives a card's law subject and, under the test-prep subtree,
  its section. The law subject equals `goldens/law_subject.json` (the golden of
  `leeches.py:_law_subject` with synthetic bands), and the test-prep section is the segment after the
  subtree's name, which `goldens/test_prep_board.json` holds.
- `.env.example` shows neutral examples.

### Consequences

- Good, because no law deck name of the owner's enters the repository.
- Good, because curriculum and insights read one subject for a card.
- Good, because the two ports of `_law_subject` (readings and ingest) are held to one golden.
- Bad, because a change to the bands takes effect at the next start, like every scope setting.
- **One truth across two files.** The readings taxonomy keeps its own file (ADR-045). When both are
  configured, ingest's settings and the readings taxonomy must name the same law roots and bands, and
  the daemon refuses to start when they disagree, naming both settings and neither value (SPEC-092 A17).
- Bad, because there are two ports of `_law_subject` and two settings files to keep equal, which one
  golden and the start refusal hold together.

### Confirmation

SPEC-092's A1, A2 and A17: the subject of every synthetic path equals the golden, a law setting
without the root refuses the cycle's step, and a root or band named differently by the two files refuses start.

## What would make this wrong

- The owner studies a second law syllabus with another layout; the settings then become a small
  file, as the courses did.

## More Information

ADR-002; ADR-012; ADR-087; SPEC-023; SPEC-092, which builds it; SPEC-093 and SPEC-098, which read it.
