---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner, through the maintainer at gate 6), the DeckStreak architect"
---

# The AI route is optional, and no-AI mode is the default and a first-class path

## Context and Problem Statement

ADR-015 runs DeckStreak's AI duties, the flagship readings among them, as headless Claude Code
through the owner's subscription proxy, with a dedicated device key. At gate 6 the maintainer made
that key conditional:
- the owner must first confirm that use of the subscription, and an API key is the alternative;
- the first deploy must not depend on it;
- ADR-015's deterministic fallback must ship as a first-class path, tested, and be the default when
  the proxy route is absent;
- the AI route becomes an add-on at gate 3, and nothing in W0 or W1 may assume the proxy.

The W1 plan built readings only through the agent. Without it, every topic ended `failed` with
`agent_unavailable` and an alert (SPEC-046 R7, SPEC-053 R4), and only the digest had a deterministic
form. How does DeckStreak run, and look honest, with no AI route?

## Decision Drivers

- The first deploy must be whole and healthy with no AI route configured.
- An absent route is a configuration, not a failure: it must not page anyone, and must not look
  broken.
- No stand-in text: a reading is a mentor's primer that passed every gate, or it is not shown
  (SPEC-046 R8).
- Whichever route the owner chooses later (the proxy, or an API key), the change is an adapter, not
  a rewrite.

## Considered Options (the alternatives it was chosen against)

- An optional AI route with no-AI mode as the default: chosen, because the first deploy is whole
  without any AI route, the owner's later choice is one adapter, and every surface states plainly
  what is off. The route is `absent` unless configured; the proxy adapter is ADR-015's, and an
  API-key adapter would follow an amendment of ADR-015. With the route absent each AI duty records
  `ai_route_absent`: no reading is generated, no alert is raised, the surfaces say readings are not
  enabled, and the digest and every game feature run fully.
- Proxy-only, as the W1 plan had it: rejected because it makes the first deploy depend on a key the
  owner has not approved, and turns every night into a failure and an alert.
- A deterministic reading, such as a formatted list of the day's new cards: rejected because it is
  stand-in text in a reading's place, which SPEC-046 R8 and the no-placeholder rule forbid, and the
  owner defined a reading as a mentor's primer.
- Choose an API key now: rejected because the choice between the subscription and an API key is
  the owner's, and is still open.

## Decision Outcome

Chosen option.
- **The port.** The agent context exposes an AI route port: `Absent`, `Proxy` (ADR-015's runner,
  unchanged) and, only after an ADR-015 amendment, `ApiKey`. The route comes from configuration,
  and an unconfigured host is `Absent`.
- **An absent route is not an alert.** Every duty checks the route before its caps and its runner.
  - With `Absent`, the outcome is `ai_route_absent`: recorded in the duty's run, never alerted,
    never retried.
  - It is shown as "readings are not enabled" on the Mini App, in the bot and on the status panel.
  - A configured route that fails keeps the existing `agent_unavailable:<cause>` outcome and its
    one alert.
- **The digest.** It has one deterministic form. With the route absent it carries no coaching line
  and no "unavailable" line, because nothing failed.
- **What the deploy needs.** Tests cover `Absent` as the default path, and each W1 SPEC proves its
  duty's `Absent` outcome. The first deploy (#42) needs no device key, no tunnel and no proxy. The
  agent's path (#43) and the first live reading (#45) wait for gate 3 (#162) or the owner's API-key
  choice.

This amends ADR-015: the proxy is one adapter of an optional route, not a precondition.

### Consequences

- Good, because the first deploy is complete and quiet without any AI, and the owner's choice
  between the subscription and an API key changes one adapter.
- Good, because an absent route and a failing route are told apart: one is a setting, the other an
  alert.
- Bad, because the flagship readings do not appear until the owner enables a route. The Mini App
  says so plainly rather than showing something lesser.

### Confirmation

- In SPEC-043, the runner refuses nothing and alerts nothing when the route is `Absent`.
- In SPEC-046 and SPEC-053, a night with the route absent records `ai_route_absent` for every topic,
  raises no alert, and stores nothing.
- The first deploy's rehearsal runs with no AI route configured.

## What would make this wrong

- The owner confirms the subscription route and asks for readings at the first deploy. Gate 3 then
  precedes the deploy, and `Absent` stays the tested default for any host without a key.
- The owner wants a card list when no primer exists. That is a new product decision, with its own
  SPEC and its own label, never a reading.

## More Information

ADR-015 (amended), ADR-019 (the readings surface), SPEC-043, SPEC-045, SPEC-046, SPEC-050,
SPEC-051, SPEC-052, SPEC-053; the ai-content-safety and study-duties packs.
