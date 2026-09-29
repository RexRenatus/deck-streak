---
status: accepted
date: "2026-09-27"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# Required checks come only from GitHub Actions, and a fork never reaches main

## Context and Problem Statement

The branch rulesets require `ci` and `fragment` before a pull request merges into `dev` or `main`.
Both entries leave `integration_id` unset. GitHub then accepts a check or a commit status with that
context name from any source: any app installed on the repository, or anyone who can post a commit
status.

`base-is-dev` keeps every head but `dev` out of `main`. It compares only the head's branch name,
and a fork can name its branch `dev`. The fork approval policy set when the repository went public
holds every outside workflow run for a maintainer's approval. That is a human step, and one
mistaken approval would let a fork's `dev` pass.

Who may produce a green required check, and whose `dev` may reach `main` (SPEC-034)?

## Decision Drivers

- A required check means "this repository's own CI ran and passed", not "something posted this
  name".
- `main` receives only this repository's `dev`.
- A rule that depends on a person not making a mistake needs a check behind it.

## Considered Options (the alternatives it was chosen against)

- Pin the checks to GitHub Actions and compare the head repository: chosen, because nothing but this
  repository's own workflows can then satisfy `ci` or `fragment`, and a fork's branch named `dev`
  fails `ci` even if its workflow run is approved by mistake. Measured on `dev`'s check runs, `ci`,
  `fragment` and `base-is-dev` all come from app id 15368 (`github-actions`).
- Leave the integration unset: rejected, because a same-named check from another app or a commit
  status would satisfy the ruleset.
- Rely on the fork approval policy alone: rejected, because it is one approval away from a green
  `ci` on a fork's `dev`.
- Require the head repository in the ruleset itself: rejected, because rulesets have no rule about
  a pull request's head repository.

## Decision Outcome

Chosen option.
- Every entry under `required_status_checks` in `.github/rulesets/main.json` and `dev.json` names
  `"integration_id": 15368`.
- `base-is-dev` reads `github.event.pull_request.head.repo.full_name` and `github.repository`
  through `env`. It fails a pull request into `main` unless the head is `dev` in this repository.
- Every agent follows the maintainer's rule, which `AGENTS.md`, `CLAUDE.md` and `CONTRIBUTING.md`
  state: never approve a workflow run from a fork, and never merge a pull request whose head
  repository is not `RexRenatus/deck-streak`. The maintainer re-lands an accepted outside
  contribution from a branch of this repository.

### Consequences

- Good, because a green required check is always this repository's own workflow.
- Good, because a fork's branch named `dev` fails `base-is-dev` even if its workflow run is approved
  by mistake.
- Bad, because an outside contribution is never merged as it stands. The maintainer re-lands it,
  which costs a step but keeps every merge on this repository's own branches.

### Confirmation

- `scripts/tests/test_rulesets.py` A3.
- `scripts/tests/test_ci_workflows.py` A5 to A7, which run `base-is-dev`'s own script.
- `scripts/tests/test_fork_rule.py` A8.

## What would make this wrong

- GitHub changes the Actions app's id. Every required check would then read as missing until the
  id is updated. That shows at once, because every pull request blocks on `ci`.
- The project accepts outside pull requests as they stand. A reviewed-fork path, with its own
  checks, would then replace re-landing.

## More Information

SPEC-034; ADR-017 (the branch model); ADR-034 (the release flow); the release-ops and
cyber-pipeline packs' ruleset and workflow rows.

## Note, 2026-09-29: the required `ci` is the pull request's own run

Recorded by SPEC-034's amendment (its section 8, R8, A14 and A15). This decision is unchanged; the
note decides which run satisfies the required `ci`.

The `ci` workflow runs on a pull request and on a push to `dev` or `main`, from the same app. The
aggregate job keeps its id `ci`, which every `needs` reader uses, and takes the name
`${{ github.event_name == 'pull_request' && 'ci' || 'ci (push)' }}`. A pull request's own run
reports as `ci`, the check both rulesets require, and a push run reports as `ci (push)`. No job
of any workflow reports a required name under an event that is not `pull_request`. GitHub's
contexts reference lists the `github` context among those available to a job's keys, and a job's
`name` is what its check run displays, so the name follows the event that started the run.

Chosen against:
- **A new pull-request-only required context in both rulesets.** Rejected: the name every document
  and tool reads would move, and the rulesets would have to change in step with the workflow.
- **Skipping the aggregate job on a push.** Rejected: a skipped job still reports under its name,
  and a push to `dev` would carry no aggregate verdict.
- **Leaving it to the maintainer's merge tooling.** Rejected: the ruleset, not a script, is the
  enforcement.

Confirmation: `scripts/tests/test_ci_workflows.py` A14 and A15, and the mutation rows S03401 to
S03403.
