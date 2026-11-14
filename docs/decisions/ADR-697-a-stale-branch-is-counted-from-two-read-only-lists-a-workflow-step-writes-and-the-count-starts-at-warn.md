---
status: "accepted"
---

# ADR-697: a stale branch is counted from two read-only lists a workflow step writes, and the count starts at warn

Decides SPEC-674. Path:
`docs/decisions/ADR-697-a-stale-branch-is-counted-from-two-read-only-lists-a-workflow-step-writes-and-the-count-starts-at-warn.md`.
Amends nothing. Follows ADR-450's Decision Drivers and its D6 promotion rule.

## Context

No check counts the repository's branches. A branch no pull request names, left after its work
merged or closed, stays listed until someone deletes it. The forge's own branches page calls a
branch stale when no one has committed to it in the last three months. The repository's test
stage runs with a token that reads contents only and exports none to its tests, so no existing
job can read the open pull requests.

## Decision Drivers

- No rule already in force is lowered, and nothing is deleted.
- A new rule starts at warn and is promoted only by its own delivery (ADR-450 D6).
- Every check prints what it examined and fails on an empty population, which is VOID.
- A census reads committed or given text and never the network, so every planted case runs
  anywhere.

## Decisions

**D1. The census reads one text a workflow step writes.** The step reads the branch list, one
GraphQL query of each branch's last commit time and the open pull requests, read-only, and writes
them as R4's lines with the read's own stamp. The module never reads the forge or the clock, so
its count is a function of the file alone.

**D2. A stale branch (SPEC-674 R2).** No open pull request of this repository names it as its
head, and its last commit is older than `WINDOW_DAYS`, 90 days, before the stamp, the forge's three months. A
fork's pull request naming a branch of the same name does not keep it.

**D3. The forms not counted (R3).** The default and the release branch, by name, and any branch
the list marks protected: the repository's rulesets refuse their deletion. Every other branch is
counted, a bot's and a merge queue's included, until a measurement supports leaving its form out.

**D4. Where it runs (R9).** Its own workflow, on a daily schedule, a dispatch and a pull request
that changes the census, never in `ci`: the count moves with the clock, so it judges no other pull
request.

**D5. The record and the levels (R6, R7).** `RECORD` is a count and its reason. At warn a count
past it prints a WARN line and passes; at both levels a record above the count fails, so the
record moves down when branches go. `LEVEL` moves to `fail` by D6 of ADR-450.

## What it was chosen against

- The census reading the forge from the test module: rejected because a test that reaches the
  network cannot run its planted cases offline and has no token in the test stage.
- A step in `ci`'s hygiene job: rejected because the clock moves the count, so every pull request
  would read a count no change of its own made.
- Leaving the default and release branches in the count: rejected because the rulesets keep them
  and their age measures releases, not leftovers.
- A 30-day window: rejected because many branches would count at once at the first run while
  their work may still be open in a closed draft's place; the forge's three months is its own idea.

## Consequences

- A stale branch shows as a count in the census's run; nothing is deleted.
- The record moves down only by a commit that lowers it, which names its run.

## What would make this wrong

- A branch kind the list cannot tell apart (one a tag or a release still needs) counted as stale
  often enough that the count stops meaning leftovers: then D3 gains a form by its own delivery.
- The forge's token losing read access to pull requests in this repository's runs: the step fails
  and the census reads nothing, which is VOID.
