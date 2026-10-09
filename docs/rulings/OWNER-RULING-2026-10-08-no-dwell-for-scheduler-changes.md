The owner rules that DeckStreak's scheduler changes (a preset's refit, a preset's desired retention, and the order in which cards already due are shown) wait no dwell time between changes, in the owner's own words: "remove the dwell, this ensure we are able to replicate the continuous improvement/ ML techniques". This ruling changes only the dwell in ADR-301 (c) (line 181), and only for a class whose changes are these alone and move no card's due date. Every other part of ADR-301, and ADR-037 and ADR-089 as ADR-301 amends them, stands as written.

# OWNER RULING 2026-10-08: no dwell between scheduler changes

## What was held

ADR-301 (c) (`docs/decisions/ADR-301-deckstreak-writes-to-the-collection-only-through-declared-write-classes.md:181-184`)
has each write class's ADR state its change budget, its dwell time between changes, and the band a result must clear
before a change is reversed. It leaves the dwell's length to each class.

A class ADR could state a fixed dwell between changes, so that each change would wait out a clock before the next
one. A fixed dwell slows the continuous improvement the owner wants: a refit, a retention change or a reorder that
the evidence supports would wait out the clock instead.

## What it rules

Removing the dwell weakens ADR-301 (c)'s change control. This signed ruling is its authority.

- **The dwell.** A class whose only changes are a preset's refit (its fitted scheduler parameters), a preset's desired
  retention, or the order in which cards already due are shown, moving no card's due date (ADR-339), states its dwell
  time between changes as none. Any class that makes any other change states its dwell as ADR-301 (c) requires. No
  class ADR, SPEC or plan adds a dwell for these changes without the owner's new word.
- **How often the owner is asked.** Such a class may propose a change as often as its evidence supports. Each
  proposal is still a batch of that one class, and on the approval rung the owner approves it on its own (ADR-301 (b),
  (c)). Removing the dwell can raise how often the owner is asked; it changes nothing about how a batch is approved.
- **Judging each change.** Each refit is still judged on its hold-out (ADR-301 (c)), and each change against its
  class's band. A change replaced before that judgment is recorded as unjudged, never as passed.
- **What stays, unchanged:**
  - The ladder (ADR-301 (c)). This ruling changes no class's rung or ceiling: a class climbs exactly as ADR-301 (c)
    says, and removing the dwell never stands in for the owner's approval of a batch.
  - The change budget and the band, which each class's ADR still states.
  - The hold-out. A score that the class's own fit produced is never its guard. A fitted parameter is still judged on
    reviews made after the fit (a hold-out split by time), never on the reviews it was fitted to.
  - The trial freeze. While a trial runs, no class changes what the trial's arms share. A change that must run anyway
    is recorded as a ledger change point, and the analysis takes it as a covariate (ADR-301 (c)).
  - The guard metrics, their automatic undo, and the owner's kill switch.
  - Every other part of ADR-301 as written, including (a) the never-list, (b) the backup and batch, (d) the single
    writer, (e) the objective, (f) card text, derived data and persona review, and the amended clauses with ADR-037 (b)
    and the zero-upload proof on every other path.

## What it was chosen against

- **A fixed dwell between changes.** ADR-301 (c) leaves room for it. The owner removed it so that the scheduler can
  follow a continuous-improvement cadence.
- **A dwell for a class's first change only.** It would keep the clock for one change, and the owner wants none.
- **Signing later, when a refit is first proposed.** Signing now means no refit waits on a signature.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
