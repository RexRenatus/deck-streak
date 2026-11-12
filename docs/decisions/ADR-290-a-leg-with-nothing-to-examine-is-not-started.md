---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A mutation leg with nothing to examine is not started, decided from the plan's listing

## Context and Problem Statement

Every pull request and every push starts all five mutation jobs, because SPEC-039 R3 says none is
ever skipped and `ci` accepts only `success` from each need. When the plan's listing gives
`mutation-rust` no mutant, its leg still starts and examines nothing. When the plan selects no row
and no retirement check is due, `mutation-rows` does the same (#435). Which fact, known when a job's
condition is evaluated, says a leg has nothing to examine, and how does the verdict keep refusing a
leg that should have run and did not?

## Decision Drivers

- The decision is the listing's, never a leg's absence: a leg that never uploaded looks the same
  whether it was not started or its runner stopped.
- Every refusal the verdict makes today stays: a shard the listing promises and that reported
  nothing is VOID by name, and so is an examined count the listing does not account for.
- No weakening: no test, mutant, row, timeout, baseline, listing or scan is removed or narrowed, and
  the examined count over a recorded run's inputs is the same before and after.
- A skipped job reads as passing to a required check, so a skip may be admitted only where a job
  that must succeed has checked it.

## Considered Options (the alternatives it was chosen against)

- Keep the placeholder legs: rejected, because each examines nothing and still takes a runner, a job
  set-up and a checkout on every event whose listing gives it nothing (#435).
- Decide the rust leg from `rust == 'false'`: rejected, because `rust` is `true` for a Rust diff whose
  listing is empty (run 36604153634, #362), so that leg would start and run cargo-mutants over no
  mutant; only the listing says a leg has nothing to examine.
- Decide from the leg's absence in the verdict: rejected, because a shard that uploaded nothing then
  reads correct whether it was not started or its runner died, and the verdict could no longer
  refuse a missing shard by name.
- An empty matrix alone: rejected, because an empty `fromJSON` matrix is an error when the job's
  strategy is evaluated, a job that errors is not a job that was not started, and it would change
  the matrix the shards are sized for.
- Shape (B), which skips the rows leg on every diff with no row selected: rejected, because the
  retirement check (SPEC-039 R3 and R11) runs in that leg on every diff, and skipping it there would
  move the check out of its job, a change this issue does not ask for (#435).
- Chosen, because the listing is known when a job's condition is evaluated and is the very thing a
  leg would examine, and the verdict keeps every refusal: a `listed` output from `shards`, a
  job-level condition on each leg that reads the plan's outputs alone, a not-started reading only
  for a shard the listing gives nothing, a refusal of any examined sum the listing does not account
  for, a `legs` check of each skipped leg, and `ci` admitting a skip from those two legs only.

## Decision Outcome

Chosen option: the listing decides. `mutation-verdict.py shards` writes `listed`, the number of
mutants its shards hold (`0` when the Rust class does not apply), and `mutation-plan` maps it.
`mutation-rust` runs `if: ${{ needs.mutation-plan.outputs.listed != '0' }}`, and `mutation-rows`
runs `if: ${{ needs.mutation-plan.outputs.rows == 'true' || needs.mutation-plan.outputs.scope ==
'diff' }}`. The matrix is untouched. The verdict reads a shard that the listing gives no mutant and
that left no artifact as `not started`; it refuses a sum of the reports' mutants that differs from
the listing's count; and its new `legs` verb refuses, by name, a skipped leg the plan owed work.
`ci` admits `skipped` from those two legs alone, one admission for each leg that was not started,
beside a verdict that must succeed.

### Consequences

- Good, because a pull request that lists no Rust mutant starts no `mutation-rust` leg, and a push
  that merges a pull request starts no `mutation-rows` leg.
- Good, because no refusal is lost and one is gained: a report whose counters disagree with its
  listing used to read whole and is now VOID.
- Good, because an unset `listed` output reads as the empty string, which is not `'0'`, so the leg
  starts: the condition fails toward running.
- Bad, because SPEC-039 R3's "never skipped" no longer holds for two jobs, which an insert-only
  amendment of SPEC-039 records, and `ci`'s step grows from one line to an admission loop.
- Bad, because the rows leg still starts on a diff that selects no row: its retirement check is
  work, so it is not a placeholder, and this delivery leaves it where it runs.

### Confirmation

SPEC-290's A1 to A7 (`scripts/tests/test_not_started_legs.py`), the edited job-condition guard in
`test_mutation_workflows.py`, the rows S29000-S29099, and the pull request's own run, whose diff
lists no Rust mutant: `mutation-rust` not started, `mutation-verdict` and `ci` passing.

## The design questions, answered with their evidence

1. **Where the listing lives when a job's condition is evaluated.** In `mutation-plan`'s outputs:
   the `shards` step reads the listing and writes the plan, so its new `listed` output is the
   listing's own count. A dependant reads it through the `needs` context, which "contains outputs
   and execution results from all jobs defined as a direct dependency of the current job", with
   `needs.<job_id>.outputs` for the values (GitHub Actions docs, "Contexts reference"). Both
   directions measured: the Rust class applying with an empty listing is run 36604153634 (#362),
   whose plan applies the class and lists shard 0 with no mutant. The reverse, a class that does not
   apply with a listing that is not empty, cannot occur: both listing steps run only when
   `steps.plan.outputs.rust == 'true'`, and `shards` reads no listing when the class does not apply;
   A1 plants a listing beside a plan with no Rust change and reads `listed` as `0`.
2. **The matrix.** The lever is the job-level `if:` alone. The docs say `jobs.<job_id>.if` "is
   evaluated before matrix strategies are applied", so a leg skipped by its condition never reads its
   matrix, which stays `[0]` for an empty listing. The docs say nothing of an empty matrix; GitHub's
   community discussion #27096 records the job failing with "Error when evaluating 'strategy' ...
   Matrix vector 'cfg' does not contain any values". It is not measured here because this design
   never builds one; the pull request's own run measures the skip.
3. **The verdict's reading of a not-started leg.** `whole_reports()` promises every shard `0` to
   `n-1` of the plan and reads each one's `mutants.out/outcomes.json` and `cargo-mutants.exit`;
   `partial_reason()` makes one with no report VOID by name, and `partition()` makes a listed mutant
   no report holds VOID. The new reading sits before `partial_reason()`: no report, no recorded exit
   and no mutant listed for the shard reads `not started`, and nothing else does, so a skipped leg
   whose listing is not empty stays VOID by name exactly as a missing report is. `legs` names the
   skipped leg itself against the plan.
4. **The required `ci` check.** It runs under `if: ${{ always() }}` and reads
   `join(needs.*.result, ' ')`. Each result is one of "success, failure, cancelled, or skipped"
   (the `needs` context), and "A skipped job reports its status as 'Success' ... even if the job
   was configured as a required check" (GitHub Actions docs, "Using conditions to control job
   execution"), which is why `ci` itself is never skipped and why it admits a skip only from
   `mutation-rust` and `mutation-rows`, once each, and only when that leg's own result says so. A
   skipped `mutation-verdict` is never admitted, and the verdict fails on a skip the listing owed.
5. **What stays byte-identical, and how it is shown.** The two listing commands and their
   conditions, the shards and the matrix for every listing, the shard job's `cargo mutants` line with
   `--timeout 300 --build-timeout 600` and its baseline, the equivalence records, the rows
   selection, and the retirement step and its condition. The existing workflow guards pin each
   (`test_mutation_workflows.py` and `test_verdict_download.py`), the pull request's diff of
   `ci.yml` touches none of those lines, and the verdict's examined count over two recorded runs'
   inputs, and the rows census, read the same at `dev` and at the head (`docs/red-first/SPEC-290.md`).

## What would make this wrong

- A listing that is not empty for a leg that has nothing to examine, or the reverse: `listed` would
  then start or skip the wrong legs. A1 ties it to the shards the plan writes.
- GitHub evaluating a job's `if:` after its matrix: an empty-listing leg would still expand `[0]`
  and run no step; nothing would break, and nothing would be saved.

## More Information

SPEC-290, issue #435, SPEC-039 R3, R4, R11 and R18, SPEC-038, ADR-057.

## Amendment, 2026-09-30: the recorded runs belong to two pull requests (#455)

Made after the delivery, insert-only: every earlier byte is kept in order. The Rust class applying
with an empty listing is cited twice above as "run 36604153634, #362". #362 is an issue. The run
was on the branch `fix/agent-gate-362`, pull request #388, at 24077ccbcb1e. Read both citations as
run 36604153634 of #388. SPEC-290 section 9 carries the same correction for its run 36624231257,
which ran on pull request #410, not on issue #396. The decision, its alternatives and its
consequences are unchanged.
