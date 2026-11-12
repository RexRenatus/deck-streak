---
status: "accepted"
date: "2026-10-02"
decision-makers: "the DeckStreak architect seat"
---

# A band-up is keyed by its lowercase band, pays curriculum's own XP, and owes its celebration on its milestone

## Context and Problem Statement

SPEC-077 R7 pays a course's band-up once: 500 XP through SPEC-040's grant with scope once, the band
badge `band_<code>_<band>` through SPEC-073's award port, and one budget-exempt T5 celebration with
the event `band_up` through the router. Migration `007701_curriculum_road_to_c2_and_law.sql` creates
`band_milestones`, whose key on the course and the band records each band once. Three questions
stood between the SPEC's text and that migration: how the band is spelt in the grant's source and
the celebration's dedupe key, where the 500 XP amount lives, and where the band-up's owed
celebration is kept between the fold's write and the router's answer.

## Decision Drivers

- The kernel spells the bands in uppercase (`CEFR_BANDS`: `A1` to `C2`), and the band badge's key
  and SPEC-073's award port accept the band only in that spelling (`band_<code>_<band>`, R3).
- SPEC-040's grant source grammar (`^[a-z0-9][a-z0-9:._-]{0,127}$`, ADR-040) and the router's dedupe
  key grammar (`^[a-z0-9][a-z0-9:._-]*$`, ADR-041) refuse an uppercase letter, so
  `bandup:<code>:B1` is refused by both before any write.
- The predecessor spells the band in uppercase in the band-up's XP source and in its celebration's
  event key (`bandup:<code>:<BAND>`, `pipeline.py:GamifyPipeline._celebrate_band_up` at `27ee2bc`,
  as SPEC-077's `band_up` golden records it).
- The predecessor keeps the band-up's 500 XP outside its economy file, beside its progress rules,
  and the game-economy reference `economy.json` follows has no key for it; its `economy-declared`
  check refuses a key the reference does not name.
- ADR-303: an award commits in the fold's write and is celebrated between the fold's writes, so its
  celebration is owed on its own row, marked when the router answers, and offered again until it is.
- A course's first sighting records its current band as a silent baseline that owes no celebration,
  and that row shares the key on the course and the band with every later band-up.

## Considered Options (the alternatives it was chosen against)

- The band lowercased in the grant's source and in the celebration's dedupe key, `bandup:<code>:<band in lowercase>`, while the badge key keeps the uppercase band: chosen because both grammars hold unchanged and the key still names the course and the band.
- Widening SPEC-040's source grammar and the router's key grammar to admit uppercase letters: rejected because it reopens ADR-040's ledger key and the router's key rule for every source and every key, to spell one band.
- Spelling the badge key's band in lowercase too: rejected because SPEC-073's award port accepts a band key only with the band as the kernel spells it, so it would widen that rule as well.
- The 500 XP as curriculum's own constant `XP_BONUS_BAND_UP`, in `crates/curriculum/src/progress.rs` beside the other Road to C2 constants, as streaks keeps its relight amount: chosen because it gives the amount one home, the one the predecessor gave it (ADR-047's rule for an amount kept outside the economy file).
- The 500 XP as a key of `economy.json`: rejected because the game-economy reference has no such key and its `economy-declared` check refuses one it does not name.
- `band_milestones` with a `baseline` flag and a nullable `celebrated_at` mark in ADR-303's form, a band-up written with its mark unset and marked when the router answers, a baseline written marked, which a check on the table holds (`baseline = 0 OR celebrated_at IS NOT NULL`): chosen because the owed celebration lives on the row the band-up already writes.
- A separate table of owed band-up celebrations: rejected because it adds a table, its data-rights rows and a second writer for a fact one nullable column on the milestone already holds, as ADR-303 found for the badges and the records.

## Decision Outcome

Chosen options: "the band lowercased in the grant's source and the dedupe key, the badge key
uppercase", "the 500 XP as curriculum's constant `XP_BONUS_BAND_UP`" and "`band_milestones` with
`baseline` and `celebrated_at` in ADR-303's form", because each keeps an existing grammar, home or
protocol unchanged and adds no table.

- The phase-4 progress step reads each course's stored band before it rewrites the course's row.
  A course with no stored band records its current band as a baseline, written marked. A course
  whose current band comes later in `CEFR_BANDS` than its stored one records that band; only a
  record written for the first time grants `XP_BONUS_BAND_UP` through progression's `grant_on`,
  inside the day's write, with source `bandup:<code>:<band in lowercase>`, track language and scope
  once.
- The phase-7 band badge step awards `band_<code>_<BAND>` for each band-up recorded on the day,
  inside the same write; the award port writes a band badge marked, and the step pays no XP.
- The band-up's celebration (event `band_up`, dedupe key `bandup:<code>:<band in lowercase>`) is
  offered between the fold's writes from each band-up whose `celebrated_at` is unset, and the mark is
  set when the router answers (ADR-303). Its wiring is a later part of this delivery.
- The v9 import maps the predecessor's uppercase band in a band-up's XP source and in its
  celebration's event key to the lowercase band (#61).

### Consequences

- Good, because the grant's source, the ledger's key and the router's key keep the grammars every
  other award uses, and a source or key with an uppercase band is refused before any write.
- Good, because a band-up and its grant commit in one write: a write that rolls back leaves neither,
  and a band reached again after a drop finds its milestone held and pays nothing.
- Good, because a band-up whose celebration was not answered is offered again from its milestone
  row, and a baseline is never offered.
- Bad, because the band is spelt two ways, lowercase in the source and the dedupe key and uppercase
  in the badge key and the milestone row, so a reader joining them lowercases the band first.
- Bad, because the v9 import must rewrite each band-up's XP source and event key (#61).

### Confirmation

SPEC-077's tests: a course seen for the first time records a silent baseline and pays nothing
(A8); the band-ups of every case equal the predecessor's golden and each grant, badge and
celebration is written once over two recomputes (A7); the constants equal the predecessor's (A5,
`XP_BONUS_BAND_UP`); and the migration's key and check, which refuse a second row for a course and
a band and an unmarked baseline.

## More Information

SPEC-077 (R6, R7, R18, R19 and its section 10); SPEC-040 and ADR-040; SPEC-041 and ADR-041;
SPEC-073; ADR-047; ADR-303; the predecessor's `pipeline.py:GamifyPipeline._persist_progress` and
`_celebrate_band_up` at `27ee2bc`; #61; #85.
