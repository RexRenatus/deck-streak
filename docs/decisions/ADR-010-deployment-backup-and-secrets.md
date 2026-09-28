---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Deployment, backup and secrets: hardened systemd units from a tagged release, credentials never in the environment

## Context and Problem Statement

The backend runs as native systemd units (no containers) on one small host shared with other
services, within a stated budget (CHARTER 3). The owner decided deploys run from the maintainer's
box, from a tag on `main` only, never from CI; the host never compiles. DeckStreak's database needs
continuous replication, a daily backup and a weekly restore drill.

## Decision Drivers

- The durable-services and rust-service packs: hardened units, `Type=notify` with a watchdog, a SIGTERM drain, `MemoryHigh` below `MemoryMax`, credentials by `LoadCredential`.
- No secret on disk in plain text, and never in an environment variable (systemd.exec: the environment is not for secrets).
- Every first-time change on the VM is an owner gate; a GCS bucket is an owner gate and follows the anti-enumeration rule.

## Considered Options (the alternatives it was chosen against)

- One release binary `deckstreakd`, units per role (API, bot, scheduled jobs as timers), installed side by side in a releases directory with an atomic `current` switch, deployed by `deploy/deploy.sh` from a verified release artifact of a `main` tag; secrets as systemd encrypted credentials — chosen: the durable-services practice, rollback by redeploying the previous tag.
- Deploy from GitHub Actions — rejected by the owner: a public repository's CI must hold no deploy credential.
- Containers — rejected because the radar holds containers at assess.
- Secrets in an environment file — rejected because the environment is readable from `/proc` for the life of the process and the packs refuse it.

## Decision Outcome

Chosen option. Units live in `deploy/systemd/` as templates parameterised by environment
variables with placeholder values: `deck-streak-api.service`, `deck-streak-bot.service`, one
`.service`/`.timer` pair per scheduled job, the alert template unit and the memory watch. Each runs
under its own system user with `ProtectSystem=strict`, `StateDirectory=`, `NoNewPrivileges=`,
`SystemCallFilter=@system-service`, `MemoryHigh`/`MemoryMax` from `deploy/host-budget.json`, and
`OnFailure=` the Telegram alert unit. Secrets are fetched from Secret Manager by a private deploy
rail and stored with `systemd-creds encrypt` (host key or TPM), and units read them with
`LoadCredentialEncrypted=` under `$CREDENTIALS_DIRECTORY`. `deploy/deploy.sh <tag>` proves the tag
is on `main`, installs the artifact beside the previous releases, switches `current` with
`mv -T`, and restarts; `deploy/rollback.sh` redeploys the previous tag. Backups: a Litestream 0.5
stanza for DeckStreak's database (global snapshot retention inside the declared backup window),
a daily backup and a weekly restore drill that cover it, and the offsite bucket chosen with the
owner (gate 7).

### Consequences

- Good, because a deploy is reproducible from a tag and reversible by redeploying the previous one.
- Bad, because the first deploy and every new unit are owner gates; the wave that ships them waits on the owner.

### Confirmation

The durable-services, rust-service, observability and release-ops rows over `deploy/`; the restore drill's first green run on the VM.

## What would make this wrong

- The host's systemd version lacks `systemd-creds` or `LoadCredentialEncrypted` (systemd 250 or later has both; re-check on an OS change).
- Disk headroom on the host falls below DeckStreak's budget.

## More Information

docs/schematics/deployment.md; the private deploy rail design in the maintainer's private operations notes; ADR-007; ADR-015.

**Superseded in part by ADR-038.** ADR-038 replaces this ADR's credential storage: no credential is
stored on the host with `systemd-creds encrypt` or loaded with `LoadCredentialEncrypted=`. Each unit
loads each credential at every start as `LoadCredential=<id>:/run/deck-streak-credentials/socket`,
fed from the secret manager by the private rail's fetch helper, so no value is ever written to the
host's disk; the first item under "What would make this wrong" gives way to ADR-038's own (a systemd
that cannot load a credential from a socket). The rest of this ADR stands: the units per role, the
hardening, the budget, the deploy from a tag, the rollback and the backups.

Amendment (2026-09-28): passages describing the host's other services and its capacity, in the
context, a considered option, the outcome and a condition that would make this wrong, were redacted
under the public-prose rule (ADR-059).
