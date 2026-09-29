---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The celebration reveal is its own delivery call, push_reveal, policed by the one-router check beside the other five

## Context and Problem Statement

SPEC-084 R8 renders a T3 celebration on the bot as a reveal: a placeholder, the golden's pause, then
an edit of the placeholder into the message, or the message sent anew when the edit fails. SPEC-041
R1 and ADR-041 hold every delivery to one router: the five delivery calls `notifications-policy.json`'s
`router.transport` names (`push_message`, `push_dice`, `push_reaction` and `push_pin` on the bot,
`push_in_app` in the Mini App) are called only inside `crates/notifications/src/router.rs`, and the
notifications-policy pack's `one-router` row and SPEC-041 A15's census refuse any call of them
elsewhere. The reveal is a delivery with its own steps and its own degraded outcome, so it must be
reachable through the port and held to the same rule.

## Decision Drivers

- One router (ADR-041, SPEC-041 R1): every celebration is chosen and rendered in the router module,
  and no holder of the bot's port can deliver around it.
- The port's existing implementers and `push_message`'s signature stay as they are.
- A transport that cannot reveal says so, and the router records that outcome by name as the
  ladder's degraded render instead of reporting a success it did not make.

## Considered Options (the alternatives it was chosen against)

- The reveal is its own delivery call, `push_reveal`, on the bot's port, with a default that answers an explicit unsupported outcome, named in `router.transport.bot` so the one-router check polices it beside the other five — chosen: the reveal is a delivery the policy names, and the check that holds the other five holds it.
- The reveal behind `push_message` — rejected because its payload would have to carry the tier, which changes every implementer's input, and the one-router check could not tell a reveal from a message.
- An unpoliced sixth call on the port, left out of `router.transport` — rejected because any holder of an `OwnerChat` could reveal outside the router, which breaks ADR-041.

## Decision Outcome

Chosen option: "the reveal is its own delivery call, `push_reveal`", because it keeps the one-router
rule whole: `notifications-policy.json` names `push_reveal` in `router.transport.bot`, so the
notifications-policy pack's `one-router` row and SPEC-041 A15's census refuse a call of it outside
the router module, exactly as they refuse the other five. The bot's port gains `push_reveal` beside
`push_dice`, `push_reaction` and `push_pin`, each with a default body that answers `Unsupported`,
never a silent success. The router records an unsupported reveal or pin by its call's name and
renders the celebration as a line at T2; an unsupported dice is recorded and the render goes on; an
unsupported reaction is recorded and the celebration renders at T0. The production transport
(`deck-streak-bot`'s `OwnerChat`) implements all four. `push_message` and its implementers are
unchanged.

### Consequences

- Good, because the reveal is held to one router by the same guards as every other delivery: the
  compiler holds the router's pass, the census holds the call's name, and the pack's row holds the
  policy's list.
- Good, because a transport that cannot reveal degrades to a line the decision ledger records at
  the tier it rendered, not to a reveal that never happened.
- Bad, because the bot's port now has five bot calls where SPEC-041 was written for four, and a new
  implementer of the port inherits four defaults it must override to render the ladder in full.

### Confirmation

SPEC-084 A7 and A14 (each tier's calls in the golden's order, and the reveal's edit and its
fallback on the bot); SPEC-041 A15's census over every shipped source, which names each of the port's
calls at its one call site; `the_policy_polices_the_reveal_beside_the_other_delivery_calls` in
`crates/notifications/tests/ladder_policy.rs`; and the notifications-policy pack's `one-router` row
in the box run, which reads the six names and goes red on a `push_reveal` call planted outside
`crates/notifications`.

## More Information

SPEC-084 (issue #120) R8 cites this decision. ADR-041 and SPEC-041 R1 hold the one router.
