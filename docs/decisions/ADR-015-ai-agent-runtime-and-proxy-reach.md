---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The AI agent: headless Claude Code on the VM, reaching the subscription proxy over a reverse tunnel

## Context and Problem Statement

The owner decided the daily digest's coaching, the readings and every other duty run as Claude
Code through the owner's subscription proxy, "as if it is operating here", on the VM for data
locality with the vault replica and the database. The proxy is bound to loopback on the
maintainer's machine. The co-hosted second brain's current agent routes through a local LLM proxy
to a third-party model; it is replaced, side by side.

## Decision Drivers

- Never expose the proxy, and give the VM no credential to the maintainer's machine.
- Never an `apiKeyHelper`: the proxy's client discipline refuses it.
- A dedicated device key, read into the process environment at launch, never on disk.
- Fail closed and say so; cap every run in turns and time.

## Considered Options (the alternatives it was chosen against)

- A reverse SSH tunnel opened FROM the maintainer's machine by a supervised user unit, so the proxy appears at the VM's own loopback port; the agent sets the same base URL as sessions on the maintainer's machine — chosen: no firewall rule, no bind change, no credential on the VM that reaches the maintainer's machine.
- Bind the proxy to the internal address behind a source-restricted VPC rule — rejected because it exposes the proxy on a network interface.
- A tunnel initiated by the VM — rejected because the VM would hold a key to the maintainer's machine.
- Keep the local LLM proxy to a third-party model — rejected by the owner.
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
