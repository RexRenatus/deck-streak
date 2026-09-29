---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The owner's courses are private configuration, and the predecessor's functions receive them through their own parameters

## Context and Problem Statement

Several W3 features need to know which course a card belongs to: the per-language statistics
(#67), Road to C2 (#85), the reading and writing habits (#93 to #96), the focus subjects (#98,
#99) and the quest that asks for several languages (#100). The predecessor hard-codes the owner's
course table (predecessor `27ee2bc`): top-level deck names mapped to a code, a display name and a
flag (`curriculum.py:LANGUAGE_DECKS`), one-letter aliases (`curriculum.py:LANG_ALIASES`), the
languages that carry a writing habit (`curriculum.py:WRITING_LANG_CODES`), each course's
unit-to-band table (`curriculum.py:CEFR_UNIT_BANDS`) and the further focus subjects
(`curriculum.py:FOCUS_SUBJECT_DISPLAY`). These are the owner's personal defaults: the deck names and
the choice of courses are the owner's study plan, which CHARTER 11 keeps out of the repository.

The readings context already reads a private taxonomy file that names the language decks with
their codes (SPEC-045, ADR-045). How does DeckStreak learn the owner's courses with no personal
literal in the repository, and without two sources that can silently disagree?

## Decision Drivers

- CHARTER 11: a personal default is configuration with a neutral example value.
- ADR-012: the goldens drive the predecessor's own functions; a private constant is replaced with a
  synthetic one in the adapter, never re-derived.
- ADR-002: analytics, curriculum, habits and focus may not depend on each other or on readings, so
  what they all read lives in the shared kernel, and a card's course is translated where Anki's
  deck names are, in ingest.
- The predecessor's own functions already take the course codes as a parameter whose default is the
  hard-coded table (`habits.py:writing_day_xp`, `habits.py:compute_reading`,
  `focus.py:subject_breakdown`), so the configured codes can be passed without changing a rule.

## Considered Options (the alternatives it was chosen against)

- A private courses file read once by the kernel, with ingest translating each card's course — chosen: one source for the owner's courses, no personal default in the repository, and the predecessor's functions receive the configured codes through their own parameters.
- The predecessor's hard-coded table and deck names — rejected because they are the owner's personal defaults, which CHARTER 11 says are configuration.
- Reuse the readings taxonomy file — rejected because it is the readings context's schema (ADR-045), which analytics, curriculum, habits and focus may not read.
- Environment variables — rejected because a list of records with names, flags, deck roots, aliases and unit bands does not fit one variable without a second parser.
- A table the owner edits — rejected because there is no settings screen until #57, and a table would put configuration into the owner's export and erase.
- A course list in `economy.json` or another public file — rejected because the list is personal.

## Decision Outcome

Chosen option: "A private courses file read once by the kernel, with ingest translating each
card's course", because it keeps the owner's plan out of the public tree while every consuming
context reads one typed value, and the predecessor's functions run unchanged over it.

- **The file.** `DECKSTREAK_COURSES_FILE` names a private JSON file, schema
  `deckstreak.courses.v1`, outside the repository. Each course carries a `code`, a `name`, a `flag`,
  the top-level deck name it roots (`deck_root`), a one-letter `alias`, whether it carries the
  writing habit (`writing`), and its `unit_bands`: for each CEFR band from A1 to C2, an inclusive
  range of unit numbers. A top-level `focus_subjects` list names the further focus subjects, each
  with a code, a name and an alias. `deploy/config/courses.example.json` shows the shape with
  neutral values.
- **The kernel** loads the file once at start into `Courses`, a shared-kernel value, and refuses
  start, naming the setting and never a value, when the file is unreadable or malformed, when two
  entries share a code, an alias or a deck root, or when a course's unit bands overlap, run
  backwards or come out of the A1 to C2 order. With the setting unset there are no courses, and the
  start says so once.
- **Ingest translates.** A card's course is the course whose `deck_root` equals the top-level name
  of the card's home deck exactly, the predecessor's `progress.py:language_of`; the read's scope
  (SPEC-023, a prefix rule) is unchanged. Only the code travels with the card.
- **The unit bands** live in the file. The private deploy rail generates them from the owner's
  syllabus source (#41); a public test holds the loaded bands contiguous, ordered and
  non-overlapping.
- **One truth across two files.** The readings taxonomy keeps its own file (ADR-045). When both are
  configured, a language deck that the two files map to different codes refuses start, naming both
  settings.
- **A change takes effect at the next start.** The kernel records the file's digest with the
  settings generation (SPEC-020 R19) and bumps the generation when the digest changed, so the next
  cycle's change gate recomputes (SPEC-023); each study day's roll-up fingerprint carries the
  digest, so that recompute re-rolls every day of the window under the new courses.
- **The goldens.** An adapter replaces the predecessor's private tables with synthetic courses in
  the module where the function reads them, which for a name bound by `from ... import` is the
  importing module (for example the `LANGUAGE_DECKS` that `progress.py` imports), and its note
  records the synthetic table. A function that takes the codes as a parameter receives the
  synthetic codes through it.

### Consequences

- Good, because no course, deck name, alias or unit band of the owner's enters the repository.
- Good, because the predecessor's functions run unchanged over any configured set of courses, and
  their goldens prove them over synthetic ones.
- Bad, because the operator keeps two private files while the readings taxonomy stays separate; the
  start-up check catches a code that disagrees, not a course one file forgot.
- Bad, because a course added or renamed re-rolls every day of the window at the next recompute.

### Confirmation

SPEC-071's criteria on the courses: the file's refusals, a card's course from its home deck's
top-level name, the disagreement with the readings taxonomy refused at start, and a changed file
bumping the settings generation once. Each later SPEC's golden notes name the synthetic courses its
adapters use.

## What would make this wrong

- A later decision merges the readings taxonomy and the courses file into one file; the start-up
  check then becomes a schema rule.
- The owner wants to edit courses in the Mini App; the settings screen (#57) then writes the file
  or a table, and this record is amended.

## More Information

ADR-002; ADR-012; ADR-045; SPEC-020; SPEC-023; SPEC-045; SPEC-071, which builds the loader; CHARTER
11. The predecessor's `curriculum.py` tables and `progress.py:language_of`.
