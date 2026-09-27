---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The branch model and CI: dev takes pull requests, main takes releases, GitHub-hosted runners only

## Context and Problem Statement

The repository is public. The owner chose a `dev` branch and a protected `main`, required checks
instead of reviews (one account cannot approve its own pull request), and GitHub-hosted CI. The
maintainer's box runs self-hosted runners for private work; a fork's pull request must never run
code there.

## Decision Drivers

- main changes only through a release pull request from dev.
- No self-hosted runner on a public repository; no `pull_request_target`; read-only tokens; actions pinned by full SHA; no secret in a pull-request workflow.
- CI runs the same `scripts/check.sh` builders run locally.

## Considered Options (the alternatives it was chosen against)

- Rulesets on `main` (pull request required, merge commit only, required checks, no force-push, no deletion) and on `dev` (the same, any merge method), a `base-is-dev` job that fails a pull request into `main` whose head is not `dev`, and one CI workflow on `ubuntu-24.04` that runs `scripts/check.sh` with an aggregate required check — chosen: the owner's model, and the release-ops and cyber-pipeline practices.
- Protect only `main` — rejected because builders merge into `dev`, which must stay green too.
- Require a review — rejected because GitHub refuses self-approval for a single-account project.
- Self-hosted runners — rejected because a fork's pull request could run code on the maintainer's box.

## Decision Outcome

Chosen option. `.github/rulesets/main.json`, `dev.json` and `release-tags.json` declare the
rulesets; `scripts/github-setup.sh` applies them idempotently with `gh api`, with the default
branch (`main`, the branch visitors see), labels, milestones, secret scanning with push protection,
Dependabot alerts and security updates, and private vulnerability reporting. The required checks
are the aggregate `ci`, `fragment` (a changelog fragment per pull request) and `base-is-dev` on
`main`. Releases merge `dev` into `main` with a merge commit, are tagged `vX.Y.Z` on `main`, and
`main` is merged back into `dev` (RELEASING.md).

### Consequences

- Good, because the gate is the same locally and in CI.
- Bad, because a ruleset cannot be applied to a private repository on the owner's plan; the script records the refusal and runs again after the repository is public.

### Confirmation

The release-ops rows over `.github/rulesets/` and the workflows; cyber-pipeline's workflow rows on the box.

## What would make this wrong

- GitHub changes ruleset semantics (release-ops' `ro.*` rows would read red).

## More Information

CONTRIBUTING.md; RELEASING.md; the release-ops and cyber-pipeline packs.
