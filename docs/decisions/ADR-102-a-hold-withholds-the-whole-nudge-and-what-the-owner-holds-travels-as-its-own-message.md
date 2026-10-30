---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# A hold withholds the whole nudge, and what the owner holds travels as its own message

## Context and Problem Statement

The nudge ablation (#132) holds out a deterministic share of the morning brief, the stakes preview
and the last chance, and compares the days the owner studied after a sent nudge with the days after
a held one. The comparison is only as honest as its arms: a held arm must be an occasion the owner
did not receive.

The predecessor's morning brief carries, besides the due count, the quest offer's pick and last
night's vaulted chests. Holding the brief would strand both, so the predecessor draws the arm on
every morning and applies a hold only when the brief carries neither
(`pipeline_layers/nudges.py:NudgesLayer.run_morning_brief`, predecessor `27ee2bc`). The offer is
pending nearly every morning, so most held arms were delivered in full, and the readout compares a
"held" arm that was sent. #122 carries the rule as "never held when it carries a quest offer or
vaulted chests". How does DeckStreak hold the brief without stranding what it carries or
contaminating the readout?

## Decision Drivers

- A held arm is an occasion the owner never received, or the readout claims an effect it has not
  measured (learning-science).
- Nothing the owner must act on is withheld by an experiment: the offer's pick and the vaulted
  chests.
- The draw happens on every holdout occasion, or the ledger has empty days (the predecessor's
  "second hole").
- One router, one decision per message.

## Considered Options (the alternatives it was chosen against)

- The offer and the chests travel as their own messages, and a hold withholds the whole brief — chosen, because every held arm is then an occasion the owner never received, and nothing to act on is ever held.
- Draw always and still send the brief when it carries an offer or a chest, as the predecessor does — rejected because such a held arm was delivered, and the readout then compares arms that are not what they say.
- Skip the draw when the brief carries an offer or a chest — rejected because the offer is pending nearly every morning, so the ledger would stay nearly empty, the hole the predecessor closed.
- Mark each line of the brief holdable or not, and hold only the holdable lines — rejected because a partial brief is neither arm: the owner still received a morning message.
- Send a shortened brief on a hold — rejected because any delivered message is a sent arm, so a shortened brief is not the held arm the readout counts.

## Decision Outcome

Chosen option: "the offer and the chests travel apart, and a hold withholds the whole brief".

- The morning job raises the quest offer (`quest_offer`), the brief (`morning`) and the vaulted
  chests (`chests_vaulted`), in that order, as three occasions.
- The holdout decides the brief alone; a hold withholds it with `ablation_hold`. The offer and the
  chests are digests, outside the holdout and outside a lapse's suppression.
- The same rule holds for the stakes preview and the last chance: a hold withholds the whole part,
  and its buttons with it.
- A failed push deletes its `send` arm, so the readout never counts a message the owner did not
  receive.

### Consequences

- Good, because every held arm is an occasion the owner never received, and every sent arm one they
  did.
- Good, because the offer and the chests arrive on a held morning and in a lapse.
- Bad, because a morning can bring three messages where the predecessor sent two.
- Bad, because the arms are not comparable with the predecessor's, whose held arms were mostly
  delivered; the import carries them as they are.

### Confirmation

SPEC-100's A8, A9 and A32.

## What would make this wrong

- The owner prefers one morning message, and accepts a readout of the brief's hold that excludes the
  mornings it carried an offer or a chest.
- The ablation ends: the rule then only decides the shape of the morning.

## More Information

Cites SPEC-041 (the router), SPEC-080 (the offer and its pick), SPEC-081 (the vaulted chests) and
ADR-080 (the draw). SPEC-100 builds it; its schematic is
`docs/schematics/nudge-coordinator-and-holdout.md`.
