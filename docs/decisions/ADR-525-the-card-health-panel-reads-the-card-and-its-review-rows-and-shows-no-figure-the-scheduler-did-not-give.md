# ADR-525: The card health panel reads the card and its review rows, and shows no figure the scheduler did not give

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-507 |
| builds on | ADR-356, ADR-348 (none amended) |
| measured at | the delivery's base |

## Context

The learner wants Anki's Card Info during a review on the web: a card's reviews, lapses, answer time, what the
scheduler expects of it, and where it came from. Four choices shape it: where its figures come from, what "days
to 90%" means, whether the panel judges the card, and what happens to the FSRS memory state a card may still
carry from the previous scheduler.

## Decision

D1. **Sources.** The panel reads only the engine core's fixed reads: the card's counters by `SNAPSHOT_SQL` and its
review rows by a new `CARD_REVIEWS_SQL`. The core builds the record (`health.rs`); the web adapter exports it as
JSON over the Worker's one new operation. No adapter passes SQL, and nothing leaves the device.

D2. **The 60-second cap is the analytics cap.** Each answer counts at most 60,000 ms, as the analytics crate's
`ANSWER_TIME_CAP_SECONDS` caps each answer before it sums. A differential test holds the panel's total equal to
the analytics crate's over generated rows.

D3. **Days to 90% belongs to the scheduler.** It is the first day at which the scheduler's own recall estimate
falls to 90%, the scheduler-side meaning of FSRS stability. The panel shows it, and the recall chance, only from
the scheduler's readout in its Ready state, labelled "The scheduler estimates". In every other state the rows read
the state's line and no number.

D4. **No verdict on one card.** The panel shows counts, times and the labelled estimate. It never labels a card
hard, ambiguous or at risk from its lapses or its answer time: one card's lapse rate separates poorly, and no
source sets an answer-time threshold.

D5. **The stored FSRS state is not shown.** The scheduler does not use it, and a stored stability shown beside
the scheduler's rows could read as the scheduler's own estimate when the scheduler is not ready. The record does
not carry it at all, so no build can let it stand in.

D6. **Provenance is what the collection records.** Phase 1 has one honest value: the card came through the
collection, created at the instant its id records. A collection keeps no package of origin for a note.

## Considered options

- Read the review rows in the page through a generic query: refused; the core holds every read (ADR-356).
- Sum raw answer times: refused; it would disagree with every other time the product shows.
- Show FSRS stability as days to 90% while the scheduler has no readout: refused; it would be a stand-in figure.
- Show the stored FSRS state in a separate labelled group: not now; nothing schedules by it, and D5 keeps it out
  until a decision asks for it, in which case it is one labelled group and a new field.

## Consequences

- Good: the panel's time agrees with the analytics crate's by test; no figure appears that the scheduler did not
  give.
- Good: the record is integers and closed enums, ready for the native record.
- Bad: until the scheduler's port gains its readout, the recall rows read "Not available in this build."

## Confirmation

SPEC-507 A1 to A6: the core's `health` test target and the panel's Vitest files, with the study suite's axe run.
