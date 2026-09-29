---
status: "proposed"
date: "2026-09-29"
decision-makers: "the DeckStreak architect (the W5 turn)"
---

# A discipline verdict is provisional until its evidence settles, and a fine is refunded when its evidence fails

## Context and Problem Statement

The discipline wave fines on evidence: an open of a distracting app before the day's reviews or
inside a committed window (#110), a failed sprint (#110), a contract's breached day (#113). It also
pays on evidence: the free spin of the instant loop banks only when reviews follow the open (#105).

The predecessor judges each at its deadline on the reviews it has read, under a sync every few
minutes, and treats a fine as final unless the open was closed within the grace
(`pipeline_layers/tripwire.py:TripwireLayer._resolve_sprints_and_fines`, `book_defection_fine`,
`_grace_defection`). It grants the free spin at the open and sets its amount to 0 when no review
follows (`_instant_loop_ack`, `_confirm_free_spin`, predecessor `27ee2bc`).

DeckStreak reads reviews once a study day, at the rollover, plus the owner's own `/sync` (ADR-037).
Judged at its deadline on the reviews DeckStreak has read, nearly every open before the owner syncs
would count as idle, and every sprint would fail for want of its reviews: a false fine each time.
The grant port never lowers an amount (ADR-040), so the predecessor's spin cannot be ported as it
is. And the rail's replies need a kind in the router's policy (SPEC-041): the existing kinds either
ignore quiet hours or keep one message a study day.

When does a verdict that reads reviews settle, what may change it afterwards, and under which kind
does the rail speak?

## Decision Drivers

- A false fine is worse than a missed fine: every verdict that fines is time-boxed and revisable,
  with an automatic refund, and a sensor's silence never fines.
- ADR-037's cadence is the owner's decision at cutover (#164): the rail must be right at any cadence.
- ADR-040: an XP grant is written once and never lowered.
- ADR-071: a settled study day's value never falls without a recorded reason.
- Every message goes through the one router, with quiet hours and an owner-facing setting.

## Considered Options (the alternatives it was chosen against)

- Settle each verdict at the first sync cycle that begins after its evidence window closes, and re-judge a fine for 7 closed study days, refunding it when its evidence fails — chosen, because it is correct at any sync cadence and never keeps a fine the evidence no longer supports.
- Settle at the deadline on the reviews already read, as the predecessor does — rejected because under ADR-037's one daily sync those reviews are the previous morning's, so nearly every open would fine and every sprint would fail.
- Raise the sync cadence for the rail — rejected because ADR-037 leaves the cadence to the owner at cutover (#164); this rule settles sooner whenever the owner syncs, and needs no cadence of its own.
- Final verdicts, reversed only by the grace, as the predecessor does — rejected because a review uploaded late by another client would leave a fine standing on an open that was not idle.
- Revise a fine for as long as it exists — rejected because a fine's standing would never settle, and the smoke bomb (#279), the digest (#129) and the de-escalation read a closed set.
- Revise the rung, the chest lock and the surcharge with the fine — rejected because the grace and the pardon give coins back and keep the tier, the predecessor's rule, and a lock already consumed cannot be un-consumed.
- Grant the free spin at the open and lower it when no review follows — rejected because the grant port never lowers an amount (ADR-040).
- Speak under the `alert` kind — rejected because an alert is exempt from quiet hours and has no owner setting, so a defection's reply could wake the owner.
- Speak under a nudge kind of the policy, such as `habit` — rejected because its dedupe of one message a study day would drop the reply to a second defection.

## Decision Outcome

Chosen option: "settle after the evidence window, revise within 7 closed study days", with a kind
of the rail's own.

- **Settlement.** A verdict that reads reviews settles at the first sync cycle (SPEC-023 R12) that
  begins after its evidence window closes: a sprint's deadline plus 15 minutes, an unanswered
  defection's instant plus 45 minutes, a free spin's instant plus 60 minutes. Each deadline is
  registered with the obligation port (SPEC-023 R10), so that cycle recomputes. At settlement an
  idle defection whose study reviews timestamped before its instant reach 15 is cleared and fines
  nothing. Until then nothing is fined for it, and its reply asserts nothing the rail cannot yet
  know.
- **Revision.** At each sync cycle, every unreversed fine of the current study day or the 7 closed
  study days before it is judged again on the evidence as it stands, and reversed through SPEC-103's
  port with the reason `revision` when it no longer holds. SPEC-104 and SPEC-106 name each
  fine's rules. The rung, a lock and a surcharge stand.
- **Rewards.** A reward that waits on evidence is drawn and stored when it is offered, and granted
  once, only when the evidence confirms it; unconfirmed, it closes ungranted.
- **The kind `discipline`.** The policy gains `discipline`: class `nudge`, tier T2, no budget,
  dedupe `per-incident`, setting `discipline_enabled`. Quiet hours withhold it like any nudge. Its
  deviation `kinds.discipline` cites this ADR.

### Consequences

- Good, because no fine stands on reviews DeckStreak had not yet read.
- Good, because the rule holds unchanged if the owner raises the cadence at cutover.
- Good, because a refund is automatic and exact (ADR-103).
- Bad, because a fine may arrive the next morning rather than within the hour, which weakens the
  loop's immediacy; the owner's `/sync` settles sooner.
- Bad, because a refund may follow a fine by days, and the message that announces it must not read
  as a reward.

### Confirmation

SPEC-104's A8, A17, A18 and A19, SPEC-105's and SPEC-106's revision criteria, and the hand-proved
rows `S10412` and `S10413`.

## What would make this wrong

- The owner raises the sync cadence to minutes at cutover: settlement would then come within the
  hour, and the revision window could shrink, though the rule would still hold.
- A verdict whose evidence can arrive later than 7 closed study days: no such source exists at W5.

## More Information

Cites ADR-037 (the cadence), ADR-040 (the grant's key and amount), ADR-071, ADR-103 (the fine port)
and SPEC-041 (the router and its policy). The schematic is `docs/schematics/fine-verdict-and-refund.md`.
