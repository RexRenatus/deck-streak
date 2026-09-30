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
  waiting runs of one tag. A hundred-and-first is cancelled and lists as a cancelled run, so it is
  seen and not lost.
- A group keyed by the run's id (`release-${{ github.run_id }}`): rejected. Every run is its own group,
  so no run is dropped, but two runs of one tag run at once and can both write the draft release and
  publish it, which breaks the first guarantee. It drops the queue between runs of one tag.
- Keeping the tag group with the default single pending run (today): rejected. The first guarantee
  holds, and a third run replaces the waiting second without a trace but a `cancelled` mark, which
  breaks the second.
- Requiring `queue: max` only on a callee's block that a release run can enter twice, weighting each
  block by how often a release job calls it: rejected. GitHub's docs do not say how a called
  workflow's group queues within its caller's run, so the weighting decides from silence. One
  reading holds a callee called once with the default queue safe, since its caller's group already
  queues the tag's runs; another holds the same shape unsafe. The rule requires `queue: max` on every
  block a release run holds, which refuses the first reading's safe case as an advisory
  over-refusal and accepts neither reading's unsafe case.
- Reading the workflow with YAML 1.1's typing: rejected. It reads the key `on` as the boolean true,
  so no workflow's triggers are read and every workflow is refused, this repository's included. With
  that key repaired it still reads `no` and `off` as false and keeps the last of a key held twice, so
  it accepts shapes GitHub's parser refuses.
- Keeping round 3's rule alone (SPEC-190 R11): rejected. It read a quoted `false` as the boolean,
  read keys only at the root, under `on:`, in a push or release filter and in a job, counted spaces
  alone as indentation and never read a called workflow. A key in another event's mapping or under
  the root's `permissions` or `defaults`, a tab in the indentation and a remote call passed it;
  GitHub's parser refuses the first two, and the rule cannot read what the third runs.

## Decision Outcome

`release.yml`'s workflow-level block becomes `group: release-${{ github.ref }}`,
`cancel-in-progress: false`, `queue: max`. `test_workflow_concurrency.py` holds every workflow file
to SPEC-190 R12, never by rendering a sample of another workflow's group. A release workflow is one
a tag can start (a push whose filters admit a tag, a `create` or a `release`) and every workflow
such a workflow calls, at any depth; every other workflow is a reacher. Each file is read as
GitHub's parser reads it: YAML 1.2's core schema types each scalar, so a quoted `false` is a string,
and a tab in the indentation is refused. Every value a release workflow holds, down to a job's own
keys, is of a type the parser's schema defines there, read with case, and no string holds an
unclosed `${{`. Every call in a workflow that runs is to a workflow file of this repository that
takes `workflow_call`, walked with no cycle. Every block a release run holds, its own or a callee's,
has a group of text of its own followed by `github.ref` alone, a `cancel-in-progress` absent or the
boolean false, and `queue: max`; a release workflow holds no job-level block. No other block starts
with text a release group's start can be, read without case. A workflow the reader cannot read is
refused. The test prints how many it examined, and derives the class from the rule, never from a
name. Twenty-two hand-proved rows (S19005 to S19026) prove the killers. SPEC-190 R12 lists the
rule's advisory over-refusals and the remainder it does not read: the contents of a job's steps,
strategy, container and services and of its runs-on, environment and snapshot mappings, expression
grammar, function names and the contexts a bare `if:` reads, and a workflow file the directory scan
does not select, are follow-up #464's.

### Consequences

- Good, because a tag's re-run waits its turn and is never replaced.
- Good, because every workflow a tag can start, and every workflow it calls, is read as GitHub's
  parser reads it, holds only blocks that queue and never cancel, and makes no call the rule cannot
  read, while no other block can start as a release group does. A new tag-triggered workflow, a
  called workflow or a job's block therefore cannot land with the default queue or share a
  release's group, outside the remainder SPEC-190 R12 names (#464).
- Bad, because the rule refuses some shapes GitHub runs with a tag's runs kept, such as a
  workflow's block that can equal a release group only under an event that never holds a tag's
  ref; SPEC-190 R12 lists them as advisory.
- Bad, because the key is newer than the default behaviour; a run beyond a hundred waiting is
  cancelled, and shows as cancelled. The block sets `queue: max`, so the depth is a hundred where the
  default is one.
- Bad, because a model of the queue's interleavings is owed until `covers` accepts a workflow file
  (#467).

### Confirmation

SPEC-190's A7 to A10, rows S19005 to S19026, and the first real tag run's history.

## What would make this wrong

- GitHub changing the meaning or the limit of `queue: max`: the test reads the key, not the platform.

## More Information

SPEC-190 R10, R11, R12, ADR-190, ADR-055, issue #377.
