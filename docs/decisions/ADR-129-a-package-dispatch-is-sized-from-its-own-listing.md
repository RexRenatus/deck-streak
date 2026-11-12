---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A package dispatch is sized from its own listing, with the plan's own function

## Context and Problem Statement

A `workflow_dispatch` of `mutation-weekly.yml` that names a package still runs a fixed matrix of 32
legs, each building and testing the unmutated baseline before its first mutant, although a small
package gives most legs no mutant (#368). The per-pull-request plan already turns a listing into
the fewest shards within a time bound. How many legs should a package dispatch have, and where does
the number come from, when efficiency must never weaken a gate?

## Decision Drivers

- The same mutants, `--timeout`, `--build-timeout` and test tool: only the number of legs changes.
- One function decides a shard count, so the pull-request plan and the dispatch cannot drift.
- A count that is wrong must fail loudly, never quietly examine fewer mutants.
- The largest package must not exceed a leg's timeout.

## Considered Options (the alternatives it was chosen against)

- Chosen, because the count then follows the package as it grows: a sizing job lists the package
  with the listing job's command plus `--package`, projects it with the plan's `projected`, and
  writes the fewest shards within the bound as the matrix, which the legs and the battery read.
- (b) A fixed smaller count per package: rejected, because a table goes stale as packages grow and
  a leg could reach its timeout with nobody told.
- (c) One shard per dispatch: rejected, because the largest package would exceed the job's
  timeout.
- (d) Cache the baseline build across legs: rejected here, because it is a repository cache and
  storage decision that is not this lane's; the orchestrator proposes it separately.
- Size the scheduled sweep too: rejected here, because the whole tree at 32 legs is the weekly
  battery's measured shape and changing it is a separate finding.

## Decision Outcome

A `size` job runs before `rust` on every non-pull-request run. With a package it lists that
package's mutants and runs `mutation-verdict.py size`, which uses the function the plan's `shards`
uses; with none it writes 32. The `rust` legs read the matrix and pass `--shard <i>/<n>`; the
survivors job's battery reads the same n and refuses a report at or beyond it. A projection past
the matrix's limit is refused with its projection, as `shards` refuses, and is never capped.

### Consequences

- Good, because a small package's dispatch builds one baseline instead of thirty-two, with every
  mutant still examined once.
- Good, because a change to the bound or the cost table moves the plan and the dispatch together.
- Bad, because the dispatch gains a listing job of its own, which builds nothing.

### Confirmation

SPEC-129's A1 to A6, the equal-count proof in the pull request, and its live dispatch.

## What would make this wrong

- A run where the sized package's listing and the shards' listing disagree in order: the table's
  `never tested` refusal would name it, and this ADR would be revisited.

## More Information

SPEC-129, SPEC-057 R14 and R18, ADR-057 D7, issue #368.
