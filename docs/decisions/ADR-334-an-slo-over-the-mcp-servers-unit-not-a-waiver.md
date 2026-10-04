---
status: accepted
date: "2026-10-04"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# An SLO over the MCP server's unit, not a waiver

## Context and Problem Statement

The MCP server's unit (ADR-332) runs an HTTP service, and the observability probe refuses an HTTP
service whose unit no SLO in `deploy/slo.json` covers. The server's router already writes the
response event the SLO evaluator counts, so the unit is measurable from its first start (#157).

How is the unit covered, at what objective, and what does the host-budget prose say?

## Considered Options (the alternatives it was chosen against)

- D1, one more SLO in the API entry's shape at objective 0.95: chosen, because the counted event exists, the page and ticket windows are the API's, and at about one call an hour the page's 6h window holds 6 events, so one failure cannot page (#157).
- D1, a recorded waiver or deferral: rejected, because the router already writes the counted event, and the wiring's deferral names a closed issue (#157).
- D1, the API's 0.99: rejected, because at this traffic one failed call in six hours would page (#157).
- D1, one budget across the API and MCP units: rejected, because the evaluator reads one unit per SLO and a shared budget hides which failed (#157).
- D1, a latency SLO: rejected, because it is out of scope for one owner (#157).
- D2, the README sentence names all four daemons: chosen, because the share test sums all four (#157).
- D2, leaving the two-daemon sentence: rejected, because it understates the sum the test enforces (#157).
- D3, the bot's unit keeps no SLO here: chosen, because its traffic waits for the host (#157).

## Decision Outcome

Chosen option: `mcp-availability` over `deck-streak-mcp.service`, 0.95 over 28 days, the API's
indicator and alert windows, no `low_traffic` key, and the host-budget sentence naming the API's,
the bot's, the replicator's and the MCP server's `CPUQuota=`.

### Consequences

- Good, because the probe's finding for the MCP unit is gone and a failing server pages.
- Good, because one failed call among a handful an hour cannot page.
- Bad, because at 0.95 a server failing one call in twenty spends no budget visibly faster than it
  could be noticed; the owner tightens it after a measured month.

### Confirmation

SPEC-333's A1 and A2, and the observability pack's `obs.slo-declared`, `obs.burn-rate-alerts`,
`obs.low-traffic` and `obs.slo-measurable` over the tree.

## What would make this wrong

- A measured traffic well above one call an hour, which would make 0.95 loose.
- An evaluator that cannot read a window holding no event, which would make the SLO page falsely.

## More Information

Issue #157; SPEC-333; ADR-031 (the SLO and its burn rates); ADR-332 (the unit).
