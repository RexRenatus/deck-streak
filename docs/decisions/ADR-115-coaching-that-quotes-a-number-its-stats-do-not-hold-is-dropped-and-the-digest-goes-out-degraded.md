---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Coaching that quotes a number its stats do not hold is dropped, and the digest goes out degraded

## Context and Problem Statement

#53 adds an AI coaching paragraph to the daily digest SPEC-101 (planned, W5; #129) builds, and asks
that the coaching quote no number its stats input does not hold. The nudge-duties pack checks that
only as an advisory row (`coaching-numbers`), which reports and still lets the message go. Its
blocking row `digest-degraded` accepts two states, coaching `ok` with a coaching part and coaching
`unavailable` with a plain notice. ADR-054 adds a third state the pack cannot name: with the route
absent, no coaching line and no unavailable line. What happens to a coaching paragraph that quotes a
number the stats do not hold, and how is each state proved?

## Decision Drivers

- No dishonest copy (CHARTER 10): a number the owner reads in a digest must be one the product
  measured.
- A model's text is advisory until a deterministic rule accepts it.
- The digest goes out every study day; a duty's failure never blocks it.
- ADR-054 is accepted and binds the absent state.

## Considered Options (the alternatives it was chosen against)

- A deterministic check of every numeral: chosen, because the rule is exact and a test can kill
  it; on a miss the coaching is dropped and the degraded digest is sent, so the owner still gets
  the stats and a plain notice.
- Send it and report the advisory row, as the pack alone would: rejected because a wrong number
  would reach the owner, and a report nobody reads does not stop it.
- Retry the coaching once with a repair instruction: rejected because a retry doubles the run's
  cost and wall clock on the digest's path, and the second answer can miss too.
- Remove the offending number from the sentence: rejected because the edited sentence can say
  something the model never wrote, which is worse than no sentence.
- Mark the absent state `unavailable` with a notice, to fit the pack: rejected because ADR-054 says
  nothing failed, so a notice would be dishonest in the other direction.

## Decision Outcome

Chosen option: "a deterministic check of every numeral against the rendered stats, dropping the
coaching and sending the degraded digest on a miss", because it is the only option that keeps both
the numbers and the schedule honest.

- **The check.** Every run of digits (with an optional decimal part and percent sign) in the
  coaching equals a value of the stats input as the stats part renders it.
- **The states.** `ok` (stats, then coaching); `unavailable` (stats, then the notice) for any
  failure, a cap or a miss; and, with the route absent, the stats alone.
- **The proof.** The two pack states are golden digests the box run judges; the absent state is a
  public test, outside the pack's population, because the pack's contract has no value for it.

### Consequences

- Good, because the advisory row can never fire on a sent digest: the engine stops the miss first.
- Good, because the owner sees one plain notice on a degraded day and nothing on a no-AI day.
- Bad, because a number written in words ("three reviews") passes the check. The coaching rules
  forbid any unheld number, and the pack's advisory row still reports it on the goldens.

### Confirmation

SPEC-115's A1 and A4 to A8, and its rows S11502 to S11506.

## What would make this wrong

- A nudge-duties contract that gains an absent value: the absent digest would then join the pack's
  population, and this ADR's third state would move into the pack.
- Coaching that needs a derived number (a week's change): the stats step would compute it and put it
  in the input, never the model.

## More Information

SPEC-115, SPEC-101 (planned, W5; #129), ADR-054, SPEC-043 R12, and the W6 schematic
`docs/schematics/w6-duty-run-and-its-degradation.md`.
