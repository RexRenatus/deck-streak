---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# One timer serves a block and the focus rows, and a block shows only its own time left

## Context and Problem Statement

The study clients, the web app and the iPhone and iPad app, run a review as a focus block (#924).
The focus parity rows of #98 and #99 already need a timer: SPEC-079 planned one row on the
service's clock, armed by the `api` and `bot` roles and claimed once by the block's generation
(ADR-079). If the block kept a timer of its own beside it, the same minutes would be counted as
study minutes and again as focus minutes. The owner decided that the Mini App and the bot are
deprecated and gain no feature, so ADR-079's role list no longer fits. And the PRD's third
non-goal forbids a countdown on DeckStreak's game, public or AI surfaces: may a block in a study
client show how long it has left?

## Decision Drivers

- One credit per block, across a restart, whatever client started it (ADR-079's rule).
- Focus minutes and study minutes pay XP once (the owner's decision).
- A block shows its own time left and its progress, never a date, a day count or days left (the
  owner's decision).
- Vivid in session, calm out of session: the timer's copy never urges (the owner's design
  principle).
- The study clients are the web app and the iPhone and iPad app; nothing is built on the Mini App
  or the bot (the owner's decision).
- The predecessor's numbers stay proved by their goldens (ADR-012).

## Considered Options (the alternatives it was chosen against)

- One timer row on the service's clock serves the block and the focus rows, claimed once by its generation as ADR-079 has it, armed by the `api` role alone — chosen: one credit, one count of minutes, and both clients agree on one end.
- A timer in each client on the device clock — rejected because two clients would disagree on the end, a device clock moves it, and the focus rows would need a second credit.
- A study timer beside the focus timer — rejected because the same minutes would be counted twice, which the owner's decision on the economy forbids.
- Keep ADR-079's role list, the `api` and `bot` roles — rejected because the bot is deprecated and gains no feature.
- Show the block's end as a time of day or a date — rejected because the owner's decision allows the block's own time left and its progress, never a date.
- Hide the time left and show only progress — rejected because the owner decided a block may show its own time left, and the third non-goal as amended does not forbid it.
- End the block hard at its length, cutting the card in hand — rejected because #924 asks for a soft end that finishes the card in hand.
- Candidate for paying a shared minute once, the block's minutes paid only by its reviews' own XP while they still count for the focus goals, streak and badges: left open, because this ADR states the rule and not its mechanism.
- Candidate for paying a shared minute once, the focus curve paying the block while the reviews inside it pay no per-review XP: left open, because this ADR states the rule and not its mechanism.
- Candidate for paying a shared minute once, each shared minute paying the greater of the two: left open, because this ADR states the rule and not its mechanism.
- Candidate for paying a shared minute once, the focus curve paying the block less the per-review XP its reviews earned, never below zero: left open, because this ADR states the rule and not its mechanism.

## Decision Outcome

Chosen option: "one timer row on the service's clock serves the block and the focus rows, claimed
once by its generation, armed by the `api` role", because it counts every minute once, credits
every block once, and lets both clients show the same time left.

- **One timer.** Every block in a study client is a focus block on the service's one `focus_timer`
  row, and its minutes are the focus log's minutes. Overlapping focus minutes and study minutes
  pay XP once; this ADR states that rule and leaves its mechanism open.
- **The service's clock.** The row holds the end instant. Both clients count down from it and from
  the service's own current instant in the same response, never from the device clock alone, and
  ask the service again when their count reaches zero.
- **The claim.** ADR-079's rule stands: one conditional update keyed by the block's generation, and
  the claim, the block's `focus_log` row and the timer's next state in one write. A block counted
  in cards is claimed when the client reports its count reached, by the same update, and is
  certified as a block that runs to its end is.
- **The roles.** This replaces ADR-079's role list: the long-running `api` role, which serves both
  study clients, arms the block's timer when it starts and whenever it writes the row, and stops it
  at shutdown. The deprecated bot role arms none, and the Mini App gains no block screen.
- **The time left.** The owner decided that a block may show its own time left in the block and its
  progress, the minutes or cards done of its length, and never a date, a day count or days left.
  The owner's reading of the PRD's third non-goal, as the app-surfaces ruling amends it, is that it
  does not forbid a block's own time left.
- **The copy.** The block's copy never urges: it holds none of the urging words the ux-laws pack's
  fake-urgency row lists, and names no deadline. That row reads the block's time-left element as
  its advisory `countdown-pressure` reading; the element is the learner's own block, not a
  deadline, and stays advisory.
- **What a block writes.** Nothing to a card beyond the learner's own answers: it never changes
  which cards are due, any card field, the scheduler or its weights.

### Consequences

- Good, because a minute is counted once and a block is credited once, whichever client started
  it.
- Good, because both clients show the same time left, and a device clock set apart from the
  service's moves nothing.
- Good, because ADR-079's claim is kept whole, so its tests and mutation rows carry over.
- Bad, because a block armed by the `api` role alone ends only when that role runs: if it is down
  at the end, the block ends at its next start, as a catch-up, while the client shows the end from
  the service's end instant.
- Bad, because the ux-laws pack reports the time-left element as an advisory on every walk.
- Bad, because the mechanism that pays a shared minute once is still to be chosen.

### Confirmation

SPEC-607's acceptance tests: a minute both rows count pays XP once, a second claim wins nothing, a
stale timer claims nothing, a block counted in cards is claimed once, a restart mid-block credits
the block once, only the `api` role arms the timer, both clients count down to the service's end
whatever the device clock says, and the block shows only its own time left with no urging word.

## What would make this wrong

- The owner decides a block may show a time of day or a date: the time-left decision is then
  amended.
- A second long-running role serves a study client: it then arms the timer too, under the same
  claim.
- DeckStreak gains a durable task scheduler that holds one-off deadlines: the timer then moves
  there.

## More Information

SPEC-607; ADR-079 (the claim, kept); ADR-012 (the parity oracle); ADR-072 (derived XP); ADR-629
(the rest guard); `docs/schematics/focus-blocks.md`; #924, #98, #99.
