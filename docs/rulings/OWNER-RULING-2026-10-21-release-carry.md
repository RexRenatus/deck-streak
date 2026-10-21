By signing the commit that adds this file, the owner admits one thing for the dev-to-main release pull request's mutation run, and only that run: a Python mutant's outcome may be carried under SPEC-493 and ADR-511 from one recorded release run of this repository, in place of running the mutant again, for a file whose whole read set is byte-identical at that run and here.

# OWNER RULING: a release run may carry a Python outcome from one recorded release run

## What it amends

One term of the release-legs ruling, "Every mutant runs, every shard is counted, and there is no skipped-mutant record", reads for every later release pull request's mutation run: "Every listed mutant is judged once: it runs in a leg the verdict counts, or its outcome is carried under SPEC-493 from one recorded release run on a byte-identical read set. Every shard is counted, and there is no skipped-mutant record." Every other term of that ruling stands, its ceilings and bounds included.

## What stays

- No VOID and no FAIL is waived: a carried survivor fails until it is killed or recorded.
- Every bound, price, timeout, ceiling and per-mutant budget stays as `dev` holds it, and the ceilings bound the legs the run deals.
- Rust, rows and web outcomes are never carried, and neither is a `timeout` or `void` outcome.
- The base is one run of this repository, chosen by run id without reading any outcome; a fork's run never qualifies.
- The equivalence records, the rows retirement check and the legs check are judged at the release head and never carried.

## The code change that applies it

- It admits every release pull request's release run in place of `CARRY_PULL_REQUESTS = frozenset({822})`, by the release run's own predicate.
- It cites this signed ruling and SPEC-422's amendment for SPEC-493.
- It lands only after the signed commit of this file is on `dev`.

## Why it is admitted

The admission is a weakening by the letter of "every mutant runs", because a carried mutant does not run again in that run. In effect it judges the same outcomes: a file carries only when everything its tests read or probe, absent paths and directory listings included, and the runner and its leg jobs are byte-identical at the recorded run and here, so the recorded outcome is the one a re-run gives, short of a flaky test, which is why a timeout never carries. It was chosen against these alternatives:

- **Running every listed mutant on every release run.** Each repeat after a small change re-runs mutants a recorded run already judged on the same bytes.
- **A base chosen by its outcomes.** Picking, among several recorded runs, one in which a mutant was killed passes more often than one draw of the same tests.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.
