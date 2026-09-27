---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The repository layout

## Context and Problem Statement

Builders, probes and packs all read the tree by path. A layout decided once lets every pack's
row find its subject, and every builder know where a file belongs without asking.

## Decision Drivers

- The packs' conventions: `crates/*` for contexts, `deploy/**/*.service` for units, root config files (`stack.json`, `economy.json`, `notifications-policy.json`, `privacy.json`, `ai-safety.json`, `data-migration.json`, `methodology.json`).
- The public repository must hold no private file.

## Considered Options (the alternatives it was chosen against)

- A single workspace: `crates/` (Rust contexts), `web/app` and `web/site` (the Mini App and the landing page), `agent/` (the agent's public duty skills, prompts, settings template and runner), `deploy/` (unit and Caddy templates), `tools/parity-oracle/`, `.packs/` (vendored packs), `scripts/` (the gate), `docs/` — chosen: every pack's default paths hold, one clone builds everything.
- Separate repositories for the backend and the Mini App — rejected because a feature spans both and would need two coordinated pull requests.
- The agent in its own repository — rejected because its duties are gated by the same packs and ship with the same release.

## Decision Outcome

Chosen option, as in ARCHITECTURE.md's code map. Private material (the persona roster, the
private deny list, concrete infrastructure values, the owner's layout of the vault) never enters
this tree; the private deploy rail provides it on the VM.

### Consequences

- Good, because every pack runs with `--root .` and finds its subject at its default path.

### Confirmation

The packs' rows find their subjects (their examined counts are non-zero once each subject is built).

## What would make this wrong

- A pack's default path conflicts with another's.

## More Information

ARCHITECTURE.md; ADR-004.
