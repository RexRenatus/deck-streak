---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A re-price takes effect from the next study day, and the grant port applies it, so no closed day's XP moves

## Context and Problem Statement

#281 revives the predecessor's per-source XP re-pricing: a multiplier per bucket, clamped to
`[0.1, 5.0]`, applied as `max(1, round(base * multiplier))` (`exchange.py:apply_multiplier`,
`exchange.py:clamp_multiplier`, `pipeline_layers/economy.py:EconomyLayer.grant_priced_xp`). The
predecessor priced each grant by the multiplier current at the call. ADR-072 holds that a closed
day's settled XP never falls, and XP moves only through the grant port (SPEC-040). When does a new
price take effect, and where is it applied?

## Decision Drivers

- Only future grants change: a granted row is never restated (the W7 plan's binding decision 5).
- A recompute of a day meets the price its first settle met, so ADR-072 holds.
- XP moves only through the grant port, and no caller prices an amount.
- The rule is idempotent: writing the same price twice changes nothing.

## Considered Options (the alternatives it was chosen against)

- Price inside the grant port, in force from the next study day: chosen, because a day's price is
  fixed before its first grant, so a replay and a recompute meet the same price; `GrantPort::grant`
  and `settle` price by the multiplier in force on the request's study day.
- Price at the call, as the predecessor did: rejected because a change in the middle of a day would
  price its later grants unlike its earlier ones, and a recompute of that day could change it.
- Each caller prices its amount before the port: rejected because a caller could forget, and the
  port is where XP moves (SPEC-040).
- Re-price past grants as well: rejected because a restated ledger breaks ADR-072's closed day and
  the port's replay answer.
- A change in force from a date the owner picks: rejected because a past date would restate closed
  days, and a far date would be a forward-looking date on the screen.

## Decision Outcome

Chosen option: "priced inside the port, in force from the next study day", because it is the only
option under which every study day has one price for its whole life.

- **The table.** `xp_price_changes` (bucket, multiplier, effective study day), with one pending
  change per bucket at most; the multiplier in force on a day is the latest row on or before it,
  or 1.0.
- **The write.** One `BEGIN IMMEDIATE` transaction: an unchanged price writes nothing, a new one
  replaces the pending row, and a change back to the price in force removes the pending row.
- **The port.** `grant` and `settle` price the amount; a key already granted answers
  `AlreadyGranted` with its stored amount, whatever the price now is.
- **The rounding.** Python's `round` is half to even, so the port rounds with
  `f64::round_ties_even`, and the goldens hold it.

### Consequences

- Good, because a price change cannot touch today's grants, so the owner sees today's XP unchanged
  and tomorrow's priced.
- Bad, because a change never applies to the rest of the current study day; the screen says so,
  and shows the pending price apart from the one in force.

### Confirmation

SPEC-138 §3 (the pricing goldens, the effective-day and replay criteria) and its rows in
`S13800-S13899`.

## What would make this wrong

- A price that must apply at once (a correction of a mistaken price): it would be a new decision,
  since it would price one day two ways.
- A grant source whose study day is not the kernel's: the effective day would need that source's
  own day.

## More Information

SPEC-138, SPEC-040, SPEC-072, SPEC-075, ADR-072, and the schematic
`docs/schematics/xp-grant-port.md`.
