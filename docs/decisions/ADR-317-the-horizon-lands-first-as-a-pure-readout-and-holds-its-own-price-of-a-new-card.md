---
status: accepted
date: "2026-10-03"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The horizon lands first as a pure readout, and curriculum holds its own price of a new card

## Context and Problem Statement

SPEC-091 (planned) ports two families of readout into `deck-streak-curriculum`: memory health
(#90) and the obligation horizon (#91). The horizon is a pure function of each card's queue, type,
due and borrowing deck, plus the collection's day number. Memory health needs each card's memory
state and desired retention (SPEC-077, not yet landed), the rollups of SPEC-071 and the parse of
SPEC-092. The predecessor's `horizon.py:build_readout` also reads the price of a new card,
`divest.py:NEW_CARD_LIFETIME_REVIEWS`, which SPEC-091 names nowhere; curriculum may not depend on
insights (`docs/CONTEXT-MAP.md`), and insights holds no such constant at dev. The predecessor's
`horizon.py:compute_horizon` also reads two queue facts the SPEC words loosely: a card in a queue
the port does not know, and a borrowed card with a negative due, are both owed now.

Which part of SPEC-091 can land now, where does the price of a new card live, how are the
horizon's constants proved when three of them are sets, and how does the readout format its counts
and its percentage as the predecessor does?

## Decision Drivers

- #91 asks for the horizon alone, and its three acceptance boxes are all decided by goldens of
  `compute_horizon` and `build_readout`.
- A builder's slice of a SPEC moves only the criteria it delivers (SPEC-082 section 3c is the
  house shape); the rest stay on the SPEC, named, for the part that delivers them.
- A constants golden holds JSON values, and the generator writes a set as nothing it can compare.
- The readout's text is the predecessor's, byte for byte: `{n:,}` groups digits and `{pct:.0f}`
  rounds a binary float's exact value, a half-way tie going to the even digit.
- The context map is binding: a new edge is an ADR, never a fix to make code compile.

## Considered Options (the alternatives it was chosen against)

- D1, the horizon alone as its own pull request, with the rest of SPEC-091 moved into a section 3c for CU3 (#90): chosen, because nothing in R6 waits on SPEC-077, SPEC-085 or SPEC-092, so it lands and is proved now (#91).
- D1, waiting for #90's chain and landing SPEC-091 whole: rejected, because the horizon would sit behind three unmerged SPECs it does not read (#91).
- D1, a new SPEC carrying R6 in SPEC-302's form: rejected, because it forks one contract into two documents and a second band for one readout (#91).
- D1, delivering R2 (the desired retention) with the horizon: rejected, because R2 reads each card's desired retention, which SPEC-077 has not landed (#91).
- D2, `horizon.rs` declaring `NEW_CARD_LIFETIME_REVIEWS = 8` itself, proved equal to the predecessor's by a golden: chosen, because curriculum then reads nothing outside its declared edges and the golden names `divest.NEW_CARD_LIFETIME_REVIEWS` as the source (#91).
- D2, an edge from curriculum to insights: rejected, because the context map forbids it and insights holds no such constant at dev (#91).
- D2, the caller passing the price in: rejected, because the readout's own text would then depend on a number the caller can get wrong, and A9 could not prove the text alone (#91).
- D2, the constant in the kernel: rejected, because the kernel is the shared kernel and knows no context's price (#91).
- D3, a separate `horizon.constants` golden for the horizon's three numbers and the price: chosen, because the horizon's constants are proved in the horizon's own part and `memory.constants` stays whole for #90 (#91).
- D3, one `memory.constants` golden proved in two halves: rejected, because a golden half-proved in one pull request and half in another has no single test that holds it equal (#91).
- D3, a generator that sorts a set into a list: rejected, because editing `generate.py` stales every golden in the tree; the three queue sets are proved by A8's cases in every queue instead (#91).
- D4, a private grouping helper in `horizon.rs` for `{n:,}` and `format!("{:.0}", pynum::round(pct, 0))` for `{pct:.0f}`: chosen, because the tie is then decided by the kernel's proved `round`, not by Rust's formatter (#91).
- D4, a grouping and fixed-point function added to the kernel's `pynum` now: rejected, because grouping is text and ADR-090's module holds arithmetic (#91).
- D4, Rust's own `{:.0}` on the float: rejected, because the half-way tie 92.5 and 93.5 would be decided by the formatter, not by CPython's rule (#91).

## Decision Outcome

Chosen option: the horizon alone, as a pure readout in `crates/curriculum/src/horizon.rs`, with
these rulings.

1. **The slice (D1).** This delivery is SPEC-091's R6, the horizon's constants of R12, criteria
   A8, A9 and the new A19, and the goldens `horizon`, `horizon_readout` and `horizon.constants`.
   CU3 (#90) delivers R1 to R5, R7 to R11, R12's memory constants, A1 to A7, A10 to A18 and the
   box run's B1 and B2. SPEC-091 section 3c holds the rest, and section 10 records the amendments.
2. **The price of a new card (D2).** `NEW_CARD_LIFETIME_REVIEWS` is 8 in `horizon.rs`, held equal to
   `divest.NEW_CARD_LIFETIME_REVIEWS` by `goldens/horizon.constants.json` (A19).
3. **The constants golden (D3).** `horizon.constants` names `horizon.HORIZON_DAYS`,
   `horizon.WINDOW_DAYS`, `horizon.FLAT_PEAK_REVIEWS` and `divest.NEW_CARD_LIFETIME_REVIEWS`.
   The queue sets are proved by A8's cases in each queue.
4. **The readout's two format specs (D4).** Five counts are grouped by a private helper; the
   percentage is `format!("{:.0}", pynum::round(pct, 0))`. The `horizon_readout` golden holds the
   ties 92.5, 93.5 and 0.5 and counts of 1,000, 12,345 and 1,234,567.
5. **The input type.** `compute_horizon(cards: &[Card], today: i64)` and
   `build_readout(cards, today, desired_retention_pct)` take ingest's card (the declared edge);
   `today` is the collection's day number.

### Consequences

- Good, because #91 lands whole and proved while #90 waits on its own prerequisites.
- Good, because curriculum gains no dependency edge.
- Bad, because the price of a new card now lives in two places, the predecessor's and this crate's;
  A19 holds them equal, and the later insights port (SPEC-098) must read this constant or prove its own.
- Bad, because a golden of a set is not possible, so the three queue sets rest on A8's cases.

### Confirmation

`horizon_goldens::the_horizon_matches_the_predecessors_golden` (A8) holds every field of every
case of `goldens/horizon.json`; `the_horizon_readout_matches_the_predecessors_golden` (A9) holds
`goldens/horizon_readout.json`, floats by their bits; and `the_horizon_constants_equal_the_predecessors`
(A19) holds `goldens/horizon.constants.json`. The mutation rows of band `S09100-S09199` prove the
queue set, the 365-day length, the overdue clamp, the beyond-horizon edge, the new-card
disjunction, the flat-peak edge, the forward scan's start, the window and the price.

## What would make this wrong

- A later change to the predecessor's price of a new card: A19 reads its golden, which is current
  only at `27ee2bc`; the golden's generator digest would stale it.
- A card field that curriculum reads which ingest's card does not carry: the horizon reads only
  queue, kind, due and the borrowing deck.
- An insights constant for the same price landing first: the two would then need one home, which is
  a new ADR.

## More Information

Issue #91; SPEC-091 R6 and R12 and its section 3c and section 10; ADR-012 (the parity oracle proves
the math), ADR-029 (the parity registry), ADR-090 (CPython's numeric semantics are ported once,
into the kernel) and ADR-091 (curriculum's readouts are computed in the current study day's step).
