---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# The comeback is the owner's one-reading sequence under the predecessor's cadence

## Context and Problem Statement

In a lapse the predecessor sends, in the morning brief's place, at most three re-entry messages
an episode, at least three days apart (`telegram.py:comeback_gate`, predecessor `27ee2bc`):
an acknowledgement that names the days of silence, an intention hung on a landmark weekday or the
first of the month, and an explicit welcome. #123 asks to keep that text.

SPEC-049, planned and decided in W1, already builds a comeback: in a lapse, one reading chosen once
per lapse (ADR-049), offered in one of three variants by the sends of the episode, with one URL
button to the reading and no date, count or streak. The router already holds its cap and gap
(`comeback.max_per_episode`, `comeback.min_gap_days`). SPEC-049 R5 refuses a date, a count of missed
days and any streak or loss wording, and R1 sends nothing when no reading is ready. Which comeback
does DeckStreak send?

## Decision Drivers

- One comeback a morning, never two re-entry messages on the same lapse day.
- SPEC-049's rules: no date, no count of missed days, no streak or loss wording and one button (R5),
  and one reading per lapse (R2).
- #123's cadence: at most 3 an episode, 3 days apart, then silence, a new lapse starting a new
  count.
- A skip day or quiet hours withholds a re-entry message, as the predecessor's comeback morning
  does.

## Considered Options (the alternatives it was chosen against)

- SPEC-049's variants under the predecessor's cadence, proved by the golden of `comeback_gate` — chosen, because it keeps #123's cadence and SPEC-049's rules, and sends one message a lapse morning.
- The predecessor's three templates verbatim — rejected because they name the days of silence and a landmark weekday, which SPEC-049 R5 refuses.
- A second sequence beside SPEC-049's — rejected because a lapse morning would then carry two re-entry messages, up to six an episode, against the rule of one reading per lapse.
- A comeback without a reading when none is ready — rejected because SPEC-049 R1 sends nothing without a ready reading, and whether to send one is the owner's question.

## Decision Outcome

Chosen option: "SPEC-049's variants under the predecessor's cadence".

- The comeback stays SPEC-049's, raised by the morning readings job, with its three variants and its
  one button to the reading.
- The router's cap and gap are proved equal to the golden `comeback_gate`, and a new lapse id
  restarts the count.
- An active skip day declines the comeback with `skip_day`, as quiet hours already withhold it.
- The policy's `comeback.landmarks` stays declared and read by no rule; no landmark and no
  silent-day count is ported.

### Consequences

- Good, because the owner receives one re-entry message a lapse morning, with a reading to open.
- Good, because no comeback says how long the owner was away.
- Bad, because #123's "keep the message text" is not kept, and the landmark framing is lost.
- Bad, because a lapse with no ready reading sends no comeback at all.

### Confirmation

SPEC-100's A25, A26, A38 and A39.

## What would make this wrong

- The owner rules that the predecessor's words return: SPEC-049 R5 would then need amending for the
  silent days and the landmark.
- The owner wants a comeback when no reading is ready: a reading-less variant would then join, under
  the same cadence.

## More Information

Cites SPEC-049 (the comeback reading), ADR-049 (the reading chosen once per lapse), SPEC-041 (the
router's cap and gap) and SPEC-083 (the skip set). SPEC-100 builds it.
