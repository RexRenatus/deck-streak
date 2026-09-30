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
  documentation ("Control the concurrency of workflows and jobs", read 2026-09-30) says: "By default,
  only one job or workflow run can be pending in a concurrency group at a time. To allow multiple runs
  to queue instead of being canceled, set queue: max. With queue: max, up to 100 jobs or workflow runs
  can wait in the concurrency group; once the queue is full, any additional runs are canceled." Its
  workflow syntax adds: "The combination of queue: max and cancel-in-progress: true is not allowed and
  will result in a workflow validation error." The group still serialises a tag's runs, so the first
  guarantee holds, and a run waits instead of being replaced, so the second holds up to a hundred
  waiting runs of one tag. A hundred-and-first is cancelled; the `release tags` ruleset lets a `v*` tag
  be pushed once and never moved, so only re-runs of that tag's one run can wait, far fewer than a
  hundred.
- A group keyed by the run's id (`release-${{ github.run_id }}`): rejected. Every run is its own group,
  so no run is dropped, but two runs of one tag run at once and can both write the draft release and
  publish it, which breaks the first guarantee. It drops the queue between runs of one tag.
- Keeping the tag group with the default single pending run (today): rejected. The first guarantee
  holds, and a third run replaces the waiting second without a trace but a `cancelled` mark, which
  breaks the second.

## Decision Outcome

`release.yml`'s workflow-level block becomes `group: release-${{ github.ref }}`,
`cancel-in-progress: false`, `queue: max`. `test_workflow_concurrency.py` derives from the directory
every workflow that runs for a tag, as GitHub's docs read it: a `release` or a `create` trigger, or a
`push` trigger with a `tags` or `tags-ignore` filter or with neither a branch nor a tag filter. It
prints how many it examined and closes that class by construction (SPEC-190 R11), never by rendering
a sample of another workflow's group: every key the workflow holds at the root, under `on:`, in a
push or release filter and in a job is one GitHub's parser defines there, read with case; its group
reads `github.ref` and nothing else (a leading `github.workflow` too, where no workflow can call it);
its `cancel-in-progress` is the literal `false`; `queue` is `max`; and no other concurrency block, a
workflow's or a job's, under any key spelling and in any workflow file, starts with text the release
group's start can also be, read without case, while a block with no literal text of its own is
refused. A workflow the reader cannot read is refused. Ten hand-proved rows (S19005 to S19014) prove
the killers. A key deeper than a job and an upper-case workflow file extension are follow-up #464's.

### Consequences

- Good, because a tag's re-run waits its turn and is never replaced.
- Good, because each part of the rule (keys, group, cancellation, other blocks) is closed from the
  group's own text, so a new tag-triggered workflow, a called workflow or a job's block cannot land
  with the default queue or share a release's group.
- Bad, because the key is newer than the default behaviour; a run beyond a hundred waiting is
  cancelled, and shows as cancelled.
- Bad, because a model of the queue's interleavings is owed until `covers` accepts a workflow file
  (#467).

### Confirmation

SPEC-190's A7, A8 and A9, rows S19005 to S19014, and the first real tag run's history.

## What would make this wrong

- GitHub changing the meaning or the limit of `queue: max`: the test reads the key, not the platform.

## More Information

SPEC-190 R10, R11, ADR-190, ADR-055, issue #377.
