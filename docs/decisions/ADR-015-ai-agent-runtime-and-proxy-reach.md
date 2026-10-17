---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The AI agent: headless Claude Code on the VM, reaching the subscription proxy over a reverse tunnel

## Context and Problem Statement

The owner decided the daily digest's coaching, the readings and every other duty run as Claude
Code through the owner's subscription proxy, "as if it is operating here", on DeckStreak's host,
for data locality with the database and the files it reads. The proxy is bound to loopback on the
maintainer's machine. An earlier agent path is replaced by this one, side by side; the agent reaches
its model only through the route ADR-054 names.

## Decision Drivers

- Never expose the proxy, and give the VM no credential to the maintainer's machine.
- Never an `apiKeyHelper`: the proxy's client discipline refuses it.
- A dedicated device key, read into the process environment at launch, never on disk.
- Fail closed and say so; cap every run in turns and time.

## Considered Options (the alternatives it was chosen against)

- A reverse SSH tunnel opened FROM the maintainer's machine by a supervised user unit, so the proxy appears at the VM's own loopback port; the agent sets the same base URL as sessions on the maintainer's machine — chosen: no firewall rule, no bind change, no credential on the VM that reaches the maintainer's machine.
- Bind the proxy to the internal address behind a source-restricted VPC rule — rejected because it exposes the proxy on a network interface.
- A tunnel initiated by the VM — rejected because the VM would hold a key to the maintainer's machine.
- Keep the earlier agent path to a third-party model — rejected by the owner.
- Call the model API directly with an API key — rejected because the owner's direction is the subscription proxy, and a key on the VM is a secret at rest.

## Decision Outcome

Chosen option. The runner is the subscription-proxy pack's reference client (`agent/run-headless.sh`)
under a settings template modelled on the owner's (permissions scoped per duty, attribution off,
no `apiKeyHelper`, hooks for the guards). At launch the private rail places the dedicated device
key in the runner's environment as `CLAUDE_CODE_OAUTH_TOKEN` with
`ANTHROPIC_BASE_URL` set to the proxy's loopback address; the key is read from Secret Manager and never
written to disk. Every run carries a maximum turn count and a wall-clock budget. If the tunnel,
the proxy or a gate fails, the duty fails closed: nothing is delivered or written, the reason is
logged and alerted, and the digest goes out deterministic with a line saying coaching was
unavailable. Adding the device key to the proxy's roster and opening the tunnel are owner gates
(3 and 2). The owner's guards run on the VM at a pinned version from the private rail.

### Consequences

- Good, because the agent behaves exactly as sessions on the maintainer's machine do.
- Bad, because the agent depends on the maintainer's machine being up; the deterministic digest and the fail-closed rule make that visible, not silent.

### Confirmation

The subscription-proxy pack's client rows (vendored) over `agent/`; the ai-content-safety rows over `ai-safety.json`; the W2 deployment's live proof (the tunnel up, a capped run delivering one gated reading).

## What would make this wrong

- The proxy's owner changes the device-key model or the tunnel is unreliable enough that the deterministic fallback fires most nights (the observability SLO shows it).

## More Information

The subscription-proxy pack; the ai-content-safety pack; the second brain's plan (private); ADR-010.

**Amended by ADR-054.** The proxy is one adapter of an optional AI route, not a precondition. The
route is `Absent` unless configured, and no-AI mode is the default and a first-class path: each duty
records `ai_route_absent` without an alert or a retry, the digest goes out in its deterministic form
with no coaching line and no "unavailable" line, and the first deploy needs no device key, no tunnel
and no proxy. The runner decided here is the `Proxy` adapter, unchanged, and its fail-closed rule
and single alert still apply to a configured route that fails. An `ApiKey` adapter would follow an
amendment of this ADR; the choice between the subscription and an API key is the owner's.

Amendment (2026-09-28): two passages describing co-hosted infrastructure, one in the context and one
in a considered option, were redacted under the public-prose rule.

Amendment (2026-09-28): a third passage describing co-hosted infrastructure, in the context, was
redacted under the public-prose rule (ADR-059).
