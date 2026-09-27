---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The comeback reading: the last ready reading, chosen once per lapse and never generated, sent by the morning readings job, and off while the predecessor sends its own comebacks

## Context and Problem Statement

The owner decided that after two or more days without study the daily readings pause, and that the
owner gets one comeback reading per lapse, inside the three-message comeback cap. The pause belongs
to the readings (two study days); the lapse and its id belong to the governor (three study days).
Which reading is offered, how "one per lapse" holds across the several comeback messages the cap
allows, who sends it before the engagement wave's comeback protocol exists, and what happens while
the predecessor, which still sends its own comeback messages, runs beside DeckStreak?

## Decision Drivers

- One reading per lapse; the nudge-duties pack refuses comebacks of one lapse that name more than
  one reading (`comeback-shape`).
- No new generation for a comeback: the day set during a lapse is by nature the one the owner left.
- The three-message cap must hold across everything the owner receives, and both bots reach the
  owner during side by side.
- The first deploy should already give a returning owner a way back in.

## Considered Options (the alternatives it was chosen against)

- Choose once per lapse id the ready reading with the latest generation instant, store the choice, name it in every comeback message of that lapse, send it from the morning readings job with the nudge-duties variants, keep `comeback_enabled` off by default while the predecessor sends its own comebacks, and always offer it on the Mini App's Today — chosen: it keeps the owner's one reading and the cap, and reaches the owner from the first deploy.
- Generate a fresh comeback reading when the lapse begins — rejected because the owner decided on no new generation, and a lapse's day set is the one the owner left unstudied.
- Choose a reading afresh for each comeback message — rejected because one lapse would name several readings, which the owner's decision and nudge-duties' `comeback-shape` refuse.
- Send the comeback message from the new bot during side by side — rejected because the predecessor's own comeback messages already use the owner's cap, so the owner would get more than three.
- Wait for the engagement wave's comeback protocol before offering anything — rejected because the first deploy would give a returning owner no comeback reading at all.
- Wait for the W3 governor (#83) and deliver the comeback reading after it — rejected because the flagship comeback reading is an owner decision for W1 (ADR-019); W1 instead builds the one slice of the governor it needs, the open lapse episode and its id, in `streaks`, proven against the predecessor's lapse detection.

## Decision Outcome

Chosen option. `reading_comebacks` holds one row per lapse id. The morning readings job raises a
`comeback` occasion with the lapse id and the reading id while the lapse is open and the reading
unread; the router's cap and gap decide. The comeback protocol of the engagement wave later owns the
messages' words and timing, and still names this one reading. The comeback message's setting defaults
to `"0"` until the cutover checklist switches it on. The open lapse and its id come from a minimal
slice of the governor that SPEC-049 builds in `streaks`: a pure function over study days, their
qualifying review counts and the skip days its caller supplies, proven by a golden of the
predecessor's lapse detection. Coordination reads it, and the readings never depend on `streaks`;
W3's governor (#83) builds the rest and keeps this episode and its id.

### Consequences

- Good, because the reading a returning owner sees is the one they left, with its carried nights.
- Good, because no path can spend the owner's cap twice during side by side.
- Bad, because during side by side the comeback reading reaches the owner only in the Mini App.
- Bad, because W1 builds a slice of the governor (the open lapse episode and its id) ahead of W3's,
  which must keep that episode and id; the slice's golden holds both to the predecessor's.

### Confirmation

SPEC-049's tests; the nudge-duties, notifications-policy and telegram-platform message rows over the
golden comeback envelopes.

## What would make this wrong

- The owner wants the comeback reading from the new bot during side by side, accepting more than
  three messages across the two bots.
- The comeback protocol's delivery chooses its own reading (it must reuse this choice instead).

## More Information

SPEC-049; ADR-011; ADR-019; the nudge-duties pack's comeback template; the notifications-policy
pack's comeback section; docs/schematics/streaks-and-governor-state-machine.md.
