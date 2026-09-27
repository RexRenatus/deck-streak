---
status: proposed
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The agent core: a shell runner of DeckStreak's own, the output gate run by the packs' own probes, and caps declared per duty

## Context and Problem Statement

ADR-015 decided that the agent is headless Claude Code on the host, reaching the owner's
subscription proxy over a reverse tunnel with its own device key, fail closed. Three things it did
not settle decide whether that holds in code. The subscription-proxy pack's reference runner cannot
be vendored (it and its scanner name the maintainer's private secret), and its scanner reads a
runner written in anything but shell as no launch at all. The packs' blocking classes, which the
charter requires on every output before delivery, are standard-library Python probes. And the caps
each run carries need values with a source.

## Decision Drivers

- The proxy client rows must examine a real launch, or they read VOID.
- Every AI output is gated by the packs' blocking checks before delivery (constraint 17), and the
  model never grades its own output (the ai-content-safety pack).
- One source of truth for each check: a class copied into Rust would drift from its pack.
- Caps are a per-run safety bound, never a daily cap on readings (the owner's reading rules).

## Considered Options (the alternatives it was chosen against)

- A shell runner of DeckStreak's own in `agent/run-headless.sh`, reading the device key with `gcloud secrets versions access` at launch and passing it only in `claude`'s environment; the output gate running every class a task's gate names by spawning the vendored probes on the output; caps declared per duty with the reference defaults and the predecessor's per-topic deadline for the daily reading — chosen: the box scanner judges the launch, the packs remain the one definition of each check, and every cap has a source.
- Spawn `claude` directly from Rust with no shell runner — rejected because the proxy client scanner reads it as no launch (VOID), so the credential, loopback and cap rows would judge nothing.
- Copy the reference client and edit out the private secret's name — rejected because a vendored file must stay byte-identical to its source (ADR-004), and an edited copy is neither vendored nor ours.
- Re-implement the gate's classes in Rust — rejected because two copies of each of about twenty classes would drift from the packs, which the packs themselves forbid ("reuses them and never copies them").
- Gate only in CI, over golden outputs — rejected because the text actually delivered each night would go unchecked.
- Read the device key from a systemd credential file — rejected because the scanner's `credential-from-secret-manager` row accepts only a secret-manager read, and ADR-015 reads the key from Secret Manager at launch.

## Decision Outcome

Chosen option. The runner is a shell script with the reference client's exits and refusals and no
private name; the unit that starts a job sets the secret's name, project, proxy URL and caps in
its environment from the private rail. The agent crate spawns the runner, parses its JSON result,
and runs the task's gate classes as subprocesses of `python3` on the output directory, so `python3`
becomes a declared runtime dependency of the host. Caps: 30 turns, 5 USD and 1800 seconds by
default (the subscription-proxy pack's reference defaults), and 620 seconds of wall clock for the
daily reading (the predecessor's per-topic deadline). The daily-reading task holds no tool.

### Consequences

- Good, because a launch that drifts (a remote URL, a key on an argv, an uncapped run) turns the box
  scan red before it merges.
- Good, because a pack fix reaches the gate by re-vendoring, with no Rust change.
- Bad, because the host must keep `python3` after the predecessor is retired, and a reading's gate
  costs a few dozen short processes.
- Bad, because the runner's tests use fakes of `claude`, `gcloud` and `curl`; the live proof is W2's.

### Confirmation

SPEC-043's tests; `python3 scripts/no-apikeyhelper-scan.py --root .` in the gate's scrub stage; the
subscription-proxy client rows run by `scripts/box-packs.sh`; the ai-content-safety rows over
`ai-safety.json`.

## What would make this wrong

- The open-source pack runner (ADR-004) ships a vendorable proxy client with no private name (then
  `agent/run-headless.sh` is replaced by it).
- The proxy client scanner learns to read a launch written in Rust (then the shell layer can go).
- Gate latency measured in `agent_runs` becomes a noticeable share of a reading's run.

## More Information

SPEC-043; ADR-015; the subscription-proxy pack (client rules 1 to 10), the ai-content-safety pack
("The gate", "Agency"); docs/schematics/agent-duty-run.md.
