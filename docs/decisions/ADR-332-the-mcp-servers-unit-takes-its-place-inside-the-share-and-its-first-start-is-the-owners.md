---
status: accepted
date: "2026-10-04"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The MCP server's unit takes its place inside the share, and its first start is the owner's

## Context and Problem Statement

SPEC-119 R2 names `deploy/systemd/deck-streak-mcp.service`, and its B3 judges the units under
`deploy/systemd/` with that unit among them. Part 157a landed the `mcp` role (ADR-329) with no unit,
so nothing starts the server on the host (#157).

A unit is not only a file. The deploy-template tests key their tables by unit, the host budget
names every unit's ceilings, and the share test sums the long-running units' memory ceilings with
the largest job's, and their CPU quotas, against DeckStreak's share of the host (ADR-032, ADR-064).
The four daemons' ceilings and the largest job's fill the memory share exactly once the server
joins, and the three daemons that run today already hold every CPU the share has. A new daemon
with no quota is refused by the share test.

`deploy/deploy.sh` installs every unit it ships and restarts only the API and the bot, by name; it
enables nothing. The role refuses start without its core token (SPEC-119 R6), which the private
rail must hold first (#167).

How is the unit shaped, where do its memory and CPU come from, and who starts it the first time?

## Decision Drivers

- The share is ADR-064's decision, an owner act on the host's capacity; a delivery does not move
  it.
- A daemon's ceilings are enforced limits, so a daemon that outgrows them fails where the alert path
  sees it, never silently.
- The bot's poll loop and the replicator have no bound of their own to shed load; the API sheds
  past its bound.
- A secret reaches a unit only as a credential from the private rail's socket (ADR-038).
- A deploy that starts a unit whose credential is not stored yet fails the role's start and rolls
  the release back.

## Considered Options (the alternatives it was chosen against)

- D1, one long-running unit in the API unit's shape: chosen, because `Type=notify` with the watchdog, the API's lifecycle, the full hardening set, the shared settings file and two `LoadCredential=` lines from the rail's socket are what every daemon here already proves (#157).
- D1, a socket-activated unit: rejected, because it is still a daemon while it runs, so the budget arithmetic is the same, and it adds a second unit to keep (#157).
- D1, the role inside the API's unit: rejected, because ADR-119 runs the server as its own role, and one unit for two roles would share one ceiling, one restart and one failure page between them (#157).
- D2, ceilings that fill the share exactly, and a CPU quota taken from the API's: chosen, because the share stays where ADR-064 put it, and the API is the one daemon that sheds load past its own bound (#157).
- D2, raising the share or its CPUs: rejected, because that is ADR-064's decision, an owner act on the host's capacity, which this delivery has no standing to take (#157).
- D2, taking the quota from the bot or the replicator: rejected, because neither has a bound of its own to shed load when it is throttled (#157).
- D2, leaving `MemoryMax=` or `CPUQuota=` off: rejected, because the share test refuses a daemon without either, and an unbounded daemon can take the host from the units it shares it with (#157).
- D3, the deploy installs the unit and never starts it, and the owner starts it once the core token is stored: chosen, because the role then starts only when its credential exists, and `deploy.sh` is unchanged (#167).
- D3, adding the unit to `deploy.sh`'s restart list: rejected, because a deploy before the core token is stored would fail the role's start and roll the release back, and it would put this delivery in the deploy class (#167).

## Decision Outcome

Chosen option: one unit, in the API's shape, inside the share, first started by the owner.

1. **The unit (D1).** `deck-streak-mcp.service` runs `deckstreakd mcp` as `Type=notify` with the
   watchdog, the lifecycle the daemons share (restart on failure, a start limit that trips, the
   stop drain), `OnFailure=` the alert template, the full hardening set, `AF_UNIX`, `AF_INET` and
   `AF_INET6` alone, the shared state directory and settings file, and `LoadCredential=` for
   `mcp-core-token` and `mcp-law-track-token` from the rail's socket. No `Environment=` line
   carries a token. The settings file gains `DECKSTREAK_MCP_LISTEN`, a loopback address, never a
   secret.
2. **The budget (D2).** The unit's memory ceilings are ADR-032's row of this date and
   `deploy/host-budget.json`'s entry; with them the long-running units and the largest job fill the
   memory share exactly, so the share's memory headroom is now zero and the next unit needs the
   share moved first, by ADR-064's decision. The unit's CPU quota is taken from the API's, whose
   quota is a ceiling and not a reservation: the API is held below it only when the two compete
   for the processor. The unit's task cap bounds a thread leak.
3. **The limits fail closed and visibly.** Past its `MemoryMax=` the kernel kills the unit, which
   fails with `OOMPolicy=kill`, restarts within its start limit and pages through `OnFailure=`, and
   the memory watch pages on the kill (SPEC-031). Past its `CPUQuota=` it is throttled, never
   given more. The share test still refuses a daemon that leaves either off, and names the unit.
4. **The first start (D3).** A deploy installs the unit with the others and starts nothing new.
   The owner stores the core token in the private rail and enables and starts the unit, the two
   steps `deploy/README.md` names (#167).

### Consequences

- Good, because SPEC-119 R2's unit exists and B3 judges it with the other units.
- Good, because the share, the bot's quota and the replicator's are unchanged.
- Good, because no deploy can start the server before its core token exists.
- Bad, because the memory share has no headroom left: the next unit waits for an owner decision on
  the share.
- Bad, because the API's processor ceiling is lower, so when the two compete the API is
  throttled sooner than before.
- Bad, because the server does not run until the owner starts it, and a deploy restarts it never:
  a new release reaches it only when the owner restarts it, until a later decision adds it to the
  restart list.

### Confirmation

SPEC-119's A46 to A52 (section 16): the template tests' budget, share, lifecycle, credential,
environment and hardening criteria, and the rail contract's census, each over the units with
`deck-streak-mcp.service` among them; and B3, the durable-services pack over `deploy/systemd/` on
the box.

## What would make this wrong

- A measurement of the role's resident memory under its held load above its `MemoryHigh=`: the
  ceilings would then have to move, and with no headroom that needs the share moved first.
- An owner decision that raises the share or its CPUs: the server's quota could then come from the
  share rather than from the API.
- A private rail that can hold the core token before the first deploy that ships the unit: the unit
  could then join the restart list, and the owner's first start would be one fewer step.

## More Information

Issues #157 and #167; SPEC-119 sections 15 and 16; ADR-032 and ADR-064, each with a note of this
date; ADR-038 (credentials from the rail's socket); ADR-119 (the server is its own role); ADR-329
(the role lands before its unit).
