---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# One concurrency rule for every workflow with a pull-request trigger

## Context and Problem Statement

`ci.yml` cancels the run a newer push to a pull request supersedes and never cancels a push run
(SPEC-038 R8, ADR-055). `changelog.yml` sets no group and `engine-measure.yml` groups by ref without
cancelling, so their superseded runs finish and hold a job slot for a result nobody reads (#369).
Which concurrency block does a workflow with a pull-request trigger carry, so that only the newest
push's run is unchanged and no other run is ever cancelled?

## Decision Drivers

- The newest push's run of every workflow is unchanged, so every check examines what it did before.
- A push, tag, schedule or dispatch run is never cancelled, and GitHub keeps one pending run per
  group, so a shared group would replace a waiting run.
- The rule must be testable for every workflow, present and future, without an exception list.

## Considered Options (the alternatives it was chosen against)

- Chosen, because it is `ci.yml`'s block, already measured on pull requests: a workflow-level block
  with `group: <workflow name>-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}`
  and `cancel-in-progress: ${{ github.event_name == 'pull_request' }}`, on every workflow with a
  `pull_request` trigger.
- `cancel-in-progress: true` for every event: rejected, because a push run on dev or main would
  cancel the one before it and a land's full run would be lost, which is ADR-055's reason.
- Leaving `changelog.yml` alone because its run is short: rejected, because the rule would have an
  exception and a test could not hold a new workflow to it.
- A concurrency group set on a job: rejected, because SPEC-038 R8 says no job sets a group of its
  own and a job group leaves the workflow's other jobs uncancelled.
- `github.head_ref` in the group key: rejected, because it is empty on a push, so every push run
  would share one group and a waiting run would be replaced.

## Decision Outcome

`changelog.yml` gains the block and `engine-measure.yml`'s block becomes the rule's. `release.yml`
(tags only) keeps `cancel-in-progress: false`; `ci.yml` and `mutation-weekly.yml` already carry the
rule. `test_workflow_concurrency.py` holds every workflow file in the directory to it.

### Consequences

- Good, because a superseded run of each pull-request workflow stops at the next push and frees its
  slot, while the newest run and every non-pull-request run are unchanged.
- Good, because a new workflow with a pull-request trigger cannot land without the block.
- Bad, because a cancelled run reads as `cancelled` in the run list; the newest run still reports
  every required check.

### Confirmation

SPEC-190's A1 to A4, three hand-proved rows in band S19000-S19099, and the pull request's own run
history.

## What would make this wrong

- GitHub changing how a group treats pending runs: A1's scenarios model today's rule.

## More Information

SPEC-190, SPEC-038 R8, ADR-055, issue #369.
