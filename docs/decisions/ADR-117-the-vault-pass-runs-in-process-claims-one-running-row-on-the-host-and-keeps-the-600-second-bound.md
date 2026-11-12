---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The vault pass runs in-process, claims one running row on the host, and keeps the 600-second bound

## Context and Problem Statement

#54 asks for the vault duties nightly and on the owner's trigger; #155 names the trigger
`/vaultops` and describes it as one configured external command, run with no arguments through a
single exact-command privilege rule, under a 600-second timeout. In DeckStreak the pass is its own
code (SPEC-116), reachable from two processes: the `job` role for the nightly run and the `bot` role
for the command. Two passes at once could both create tomorrow's daily note, and SPEC-042 R2's
rename would let the second replace the first. How does the trigger run the pass, and what keeps
it to one at a time?

## Decision Drivers

- The pass is DeckStreak's code: nothing in it needs another service user or a privilege.
- One at a time must hold on the host, not only within a process.
- A crashed pass must not block every later one.
- The owner's reply must say what happened, and nothing of an error's text.

## Considered Options (the alternatives it was chosen against)

- An in-process pass that claims one `running` row: chosen, because both roles share the ledger,
  and a row carries the instant that lets a stale claim be abandoned. The pass runs in the
  triggering role, and a partial unique index in `vault_passes` holds the claim.
- The issue's shape, an exact-command privilege rule: rejected because a shell-out would add a privilege
  and a user for work the process already does, since the pass is DeckStreak's own code.
- Start the nightly job's unit from the bot: rejected because controlling a unit needs a privilege
  rule of its own, and the reply could see the outcome only by polling.
- Queue a request row that the job role polls: rejected because the reply would wait on a poll
  interval, and polling needs a process that is always up.
- A lock file beside the vault: rejected because a lock file left by a crashed process carries no
  instant to judge it stale, and the vault is the owner's folder, not DeckStreak's scratch.
- ADR-113's one run per process alone: rejected because the `job` and `bot` roles are separate
  processes, so it would allow two passes on the host.

## Decision Outcome

Chosen option: "the pass runs in-process in the triggering role, and claims one `running` row in
`vault_passes` under a partial unique index", because it is the only option that holds on the host
with no privilege, and recovers from a crash by an instant rather than by hand.

- **The claim.** One `BEGIN IMMEDIATE` transaction abandons a `running` row older than 660 seconds,
  drops rows finished more than 90 days earlier, and inserts the new row; the index refuses a
  second `running` row with `already_running`.
- **The bound.** 600 seconds for either trigger, the predecessor's `bot.py:_VAULT_OPS_TIMEOUT_SECS`;
  an apply of a staged run that has begun is never cancelled.
- **The reply.** The predecessor's placeholder and its completed, timed-out and failed lines; its
  exit-status line is not ported, because an in-process pass has none.

### Consequences

- Good, because a nightly pass and an owner's request can never write the same note twice.
- Good, because the partial index is in the migration, where a script-mutation row proves it.
- Bad, because a pass that crashes blocks `/vaultops` for up to 660 seconds. The reply says a pass
  is running, which is what the ledger holds until the claim abandons it.

### Confirmation

SPEC-117's A2 to A5, A11 and A14, and its rows S11703 to S11706.

## What would make this wrong

- A pass that must run longer than 600 seconds (a large inbox): the bound and the abandon margin
  would move together, and the caps of SPEC-116 with them.
- A second DeckStreak writing the same vault from its own ledger: two ledgers would mean two
  claims, and the claim would have to move into the vault or a shared service.

## More Information

SPEC-117, SPEC-116, SPEC-027 R11 (the `job` role), SPEC-042 R2 and R4, ADR-113, ADR-059, and the W6
schematic `docs/schematics/w6-duty-run-and-its-degradation.md`.
