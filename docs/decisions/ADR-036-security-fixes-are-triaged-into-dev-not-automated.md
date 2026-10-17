---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Security fixes are triaged into dev, never opened by automation against main

## Context and Problem Statement

Dependabot's automated security fixes open their pull requests against the default branch.
`.github/dependabot.yml`'s `target-branch: dev` governs version updates only. DeckStreak's default
branch is `main`, and `main` accepts only a release pull request from this repository's `dev`:
`base-is-dev` fails anything else, and the required `ci` fails with it (ADR-035). The maintainer
measured a security-update run on `main` minutes after `scripts/github-setup.sh security` enabled
the feature. It failed only because pnpm could not move a pinned transitive dependency. The next
fixable advisory would open a pull request into `main` that can never merge.

How does a security fix reach the code without breaking the dev-to-main release workflow
(SPEC-035)?

## Decision Drivers

- Every change reaches `main` only through a release pull request from `dev` (the owner's workflow).
- No automation holds write permission over pull requests (the hardened workflows).
- An advisory must still reach a person quickly.

## Considered Options (the alternatives it was chosen against)

- Alerts on, automated fixes off, triage into dev: chosen, because it keeps one path for every
  change, loses no signal, and keeps version updates flowing into `dev`. Automated security fixes
  are turned off, alerts stay on, and each fixable alert becomes a pull request into `dev`; a patch
  release ships a fix that cannot wait.
- A close-and-re-land runbook: rejected because every fix would arrive as a pull request into
  `main` that can never merge, and a person must copy it into `dev` by hand anyway. Automated fixes
  would stay on, and each security pull request would be closed and re-landed.
- Make `dev` the default branch, so security fixes target it: rejected because `main` is the branch
  visitors see (ADR-017), and the owner's measured configuration keeps `main` as the default.
- Retarget each security pull request to `dev` with a workflow: rejected because it needs a
  workflow with write permission over pull requests, triggered by pull requests from Dependabot,
  which the hardened workflows forbid (no `pull_request_target`, a read-only token).

## Decision Outcome

Chosen option.
- `security()` in `scripts/github-setup.sh` deletes `automated-security-fixes` and never puts it,
  so a re-run cannot turn it back on.
- It keeps Dependabot alerts, secret scanning with push protection, and private vulnerability
  reporting on.
- `SECURITY.md` and `docs/OWNER-SETUP.md` say how an alert is handled.

### Consequences

- Good, because no pull request is ever opened that the workflow must refuse.
- Good, because Dependabot still reports every advisory, and still sends its version updates to
  `dev`.
- Bad, because a fixable advisory waits for the maintainer's triage rather than arriving as a ready
  pull request. The alert notifies, and the maintainer's daily scan reads the lockfiles as well.

### Confirmation

- `scripts/tests/test_github_setup.py` (SPEC-035 A1 to A3).
- `gh api repos/RexRenatus/deck-streak/automated-security-fixes` reads `{"enabled": false}` after
  the script runs.

## What would make this wrong

- Dependabot learns a `target-branch` for security updates. The automated fixes could then be
  turned back on against `dev`.
- The project changes its default branch to `dev`. The same outcome would then follow.

## More Information

SPEC-035; ADR-017 (the branch model); ADR-034 (the release flow); ADR-035 (only this repository's
dev reaches main); the cyber-pipeline pack's supply-chain rows.
