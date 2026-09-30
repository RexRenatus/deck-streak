---
status: accepted
date: "2026-09-30"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A release tag's runs never replace a waiting run

## Context and Problem Statement

`release.yml` groups its runs by the tag's ref with `cancel-in-progress: false`, so a run of a tag is
never cancelled while it runs. GitHub keeps one pending run per group by default, though: when a third
run of one tag arrives while one runs and a second waits, the waiting second is cancelled and replaced
(ADR-055 rejected that shape for dev's pushes; SPEC-190 R5 named it as the one exception, #377). Which
shape does `release.yml` take, so a release's steps never run twice at once and no requested run is
silently dropped?

## Decision Drivers

- One tag's release steps never run twice at once: the job attaches to one draft release and publishes it.
- No requested run is silently dropped while it waits.
- The rule must be testable for every workflow that can hold two runs of one release, without a list.

## Considered Options (the alternatives it was chosen against)

- Chosen: keep the tag's group and `cancel-in-progress: false`, and add `queue: max`. GitHub's
  documentation (workflow syntax and "Control the concurrency of workflows and jobs") says: "By default,
  only one job or workflow run can be pending in a concurrency group at a time. Setting queue: max allows
  up to 100 jobs or workflow runs to queue and process sequentially based on when each run began
  waiting", and that `queue: max` cannot be combined with `cancel-in-progress: true`. The group still
  serialises a tag's runs, so the first guarantee holds, and a run waits instead of being replaced, so
  the second holds up to a hundred waiting runs of one tag (a hundred-and-first is cancelled, which is
  beyond any release's number of re-runs).
- A group keyed by the run's id (`release-${{ github.run_id }}`): rejected. Every run is its own group,
  so no run is dropped, but two runs of one tag run at once and can both write the draft release and
  publish it, which breaks the first guarantee. It drops the queue between runs of one tag.
- Keeping the tag group with the default single pending run (today): rejected. The first guarantee
  holds, and a third run replaces the waiting second without a trace but a `cancelled` mark, which
  breaks the second.

## Decision Outcome

`release.yml`'s workflow-level block becomes `group: release-${{ github.ref }}`,
`cancel-in-progress: false`, `queue: max`. `test_workflow_concurrency.py` derives every workflow with a
`push: tags` or a `release` trigger from the directory, prints how many it examined and holds each to
that shape: one workflow-level block, one group for two runs of one tag, no cancellation, `queue: max`.
A workflow the reader cannot read is refused. Two hand-proved rows (S19005, S19006) prove the killer.

### Consequences

- Good, because a tag's re-run waits its turn and is never replaced.
- Good, because a new tag-triggered workflow cannot land with the default queue.
- Bad, because the key is newer than the default behaviour; a run beyond a hundred waiting is cancelled.

### Confirmation

SPEC-190's A7, rows S19005 and S19006, and the first real tag run's history.

## What would make this wrong

- GitHub changing the meaning or the limit of `queue: max`: the test reads the key, not the platform.

## More Information

SPEC-190 R10, ADR-190, ADR-055, issue #377.
