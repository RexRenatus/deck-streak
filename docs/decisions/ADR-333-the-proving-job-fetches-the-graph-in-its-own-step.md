---
status: "accepted"
date: "2026-10-04"
decision-makers: "the maintainer, the builder of #606"
---

# The proving job fetches the locked graph in its own step

## Context and Problem Statement

A job that proves mutation rows can restore an older Rust cache by key prefix, one whose registry
lacks a package the current `Cargo.lock` adds. The rows runner's first cargo call is offline, so it
fails and every row reads VOID (#606). Where does the dependency graph get made complete?

## Decision Drivers

- The cure must live in the job, because the cache is the job's and the failure is the job's.
- No new action, pin, cache key or environment setting (rulings 149 and 155).
- A fetch failure must fail the job at the fetch, named, not as VOID rows later.

## Considered Options (the alternatives it was chosen against)

- One step `cargo fetch --locked` after the cache restore and before `prove`, in each proving job - chosen, because it completes the registry from the lock and nothing else, and a failure fails that step.
- Make the census's offline call online - lost: the census measures the tree it is given, and its call is dev's.
- Restore the cache by exact key only - lost: a cold miss then rebuilds the whole build, and the cost lands on every dependency change.
- Fetch inside `mutation_rows.py` - lost: the runner is not the job, and a local run of the runner would then reach the network.

## Decision Outcome

Chosen option: "one fetch step per proving job", because it is the smallest change that makes the
offline call's premise true. `scripts/tests/test_rows_jobs_fetch_graph.py` enforces it over every
workflow: each job that runs `prove` holds the step after its last cache restore, before its first
prove, with no `continue-on-error`, `||` or `set +e`.

### Consequences

- Good, because a stale prefix cache no longer reddens `mutation-verdict`.
- Good, because a new proving job without the step fails a test, not a CI run.
- Bad, because a job now reaches the registry once; a registry outage fails it at the fetch.

### Confirmation

SPEC-332's A1 and A2, and the mutation rows S33200 and S33201.

## More Information

- SPEC-332; issue #606.
