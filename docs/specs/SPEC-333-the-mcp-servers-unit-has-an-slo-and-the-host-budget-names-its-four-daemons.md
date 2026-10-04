# SPEC-333: the MCP server's unit has an SLO and the host budget names its four daemons

- **Issue:** #157. **Context(s):** none (deployment declarations, not a bounded context).
- **Decided by:** ADR-334 (an SLO, not a waiver; 0.95; the host-budget sentence).
- **Status:** delivered by the pull request that adds this file, with its tests,
  `docs/red-first/SPEC-333.md` and mutation band S33300-S33399.

## 1. The problem, measured

Measured at dev `df2a4cdb`.

- The observability probe's `obs.slo-declared` reads: `deploy/systemd/deck-streak-mcp.service runs
  HTTP service deckstreakd, and no SLO in deploy/slo.json covers it`. `deploy/slo.json` declares one
  SLO, `api-availability`, over `deck-streak-api.service`.
- The MCP router writes the event the evaluator counts: `crates/mcp/src/server.rs:103` builds
  `TraceLayer::new_for_http()` with `.on_response(DefaultOnResponse::new().level(Level::INFO))` at
  line 120, as `crates/api/src/router.rs:240` and `:257` do, and `deploy/scripts/slo-evaluate.py:43`
  counts `RESPONSE = "finished processing request"` per unit. The SLO is measurable from the
  unit's first start.
- `deploy/README.md` "## The host budget" says the API's and the bot's `CPUQuota=` fit the share's
  CPUs. The share test sums four daemons': the API's 75%, the bot's 50%, the replicator's 50% and the
  MCP server's 25% (`deploy/systemd/*.service`, `CPUQuota=` lines 35, 43, 37 and 38).

## 2. Requirements

R1. `deploy/slo.json` declares ONE more SLO, `mcp-availability`, over `deck-streak-mcp.service`:
    the API's `sli` object unchanged, objective 0.95, `window_days` 28,
    `expected_events_per_hour` 1, and the API's two alerts (page 6h/30m at 0.05 consumed, burn 5.6;
    ticket 3d/6h at 0.1 consumed, burn 0.9333), both routed to telegram. It carries no
    `low_traffic` key: the page's 6h window holds 6 events at the expected traffic and one failure
    pages only below 1 / (5.6 x 0.05) = 3.57.
R2. The SLO's `rationale` says an agent's tool calls for one owner are retried by the agent, so a
    page means a failing server, not one failed call (ADR-334).
R3. `deploy/README.md` "## The host budget" names the daemons' `CPUQuota=`: the API's, the bot's,
    the replicator's and the MCP server's.
R4. Every SLO in the file names a unit that ships in `deploy/systemd/`, and every alert's burn rate
    equals budget consumed x window hours / long-window hours.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | the MCP server's unit has exactly one SLO, with the API's indicator, objective 0.95, 28 days, a page and a ticket routed to telegram at exact burn rates, and one failed call cannot page at the expected traffic | `python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k TheMcpSloBurnsAsDeclared` |
| A2 | every SLO names a shipped unit and burns as declared, over a counted, non-empty population (a guard, green at the base for the API entry) | `python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k EverySloIsDeclaredAgainstAShippedUnit` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k TheMcpSloBurnsAsDeclared
A2: python3 -m unittest discover -s scripts/tests -p test_slo_declaration.py -k EverySloIsDeclaredAgainstAShippedUnit
```

## 4. File manifest

| file | context | change |
|---|---|---|
| `deploy/slo.json` | deploy | the `mcp-availability` SLO |
| `deploy/README.md` | docs | the host-budget sentence |
| `scripts/tests/test_slo_declaration.py` | deploy | A1 and A2 |
| `scripts/mutation-rows.d/S33300-S33399.json` | gate | two rows |
| `docs/specs/SPEC-333-...md`, `docs/decisions/ADR-334-...md`, `docs/red-first/SPEC-333.md`, `changelog.d/mcp-slo-157.md` | docs | added |

## 5. What this does NOT do

- It gives the bot's unit no SLO: its traffic waits for the host (#157).
- It adds no latency SLO and no shared budget across the API and MCP units (#157).
- It records no waiver or deferral for the MCP unit (#157).

## 6. Risks

- At one call an hour the page's short window may hold no event; the evaluator's own handling of an
  empty window decides, and the owner reads the first weeks' journal before tightening anything.
- FORMAL: not applicable. A declaration plus an arithmetic identity that a unit test pins has no
  state machine and no concurrency.
