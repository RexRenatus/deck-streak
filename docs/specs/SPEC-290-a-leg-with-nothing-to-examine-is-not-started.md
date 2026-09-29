# SPEC-290: a mutation leg with nothing to examine is not started, decided from the plan's listing

- **Wave:** W4. **Issue:** #435. **Context(s):** `repo` (`.github/workflows/ci.yml`,
  `scripts/mutation-verdict.py`, their tests and `docs/`).
- **Decided by:** ADR-290 (this SPEC's own: the listing decides, and what it was chosen against),
  ADR-057 and SPEC-039 R3, R4, R11 and R18 (the mutation jobs, the verdict, the retirement check and
  the shards; SPEC-039 takes an insert-only amendment). SPEC-038's rule that `ci` needs every job
  still holds and is unchanged.
- **Status:** delivered. It holds `docs/red-first/SPEC-290.md`.

## 1. The problem, measured

- **A plan that lists no Rust mutant still starts a `mutation-rust` leg.** The shard job's matrix is
  the plan's, `0` to `n-1`, and `mutation-verdict.py shards` gives one shard when the listing is
  empty, so a pull request that changes no Rust starts shard 0. Every Rust step of that leg is
  skipped by its step condition `needs.mutation-plan.outputs.rust == 'true'`: it checks the tree
  out, downloads the plan, prints its case and uploads nothing (`ci.yml` at `dev` c1bbc6d).
- **A Rust diff whose listing is empty starts it too, and runs cargo-mutants over nothing.** Run
  36604153634 (#362): the plan's Rust class applies, cargo-mutants' `--list --json --in-diff`
  printed nothing because no mutant overlaps the diff, shard 0 lists no mutant, and the leg ran
  cargo-mutants, which exited 0 and wrote no report. The verdict read `mutation-rust-shard-0: no
  mutant listed, and cargo-mutants reports none`. So the `rust` output alone cannot say a leg has
  nothing to examine: it is `true` here.
- **A plan that selects no row still starts `mutation-rows`.** On a push that merges a pull request
  the scope is `not-applicable`: no row is selected and the retirement check's step condition
  (`scope == 'diff'`) is false, so the leg examines nothing. On a diff with no row selected the leg
  still has work: the retirement check (SPEC-039 R11) runs on every diff.
- **Why every leg starts today.** SPEC-039 R3: each job "is never skipped, because `ci` reads a
  skipped need as failed". `ci`'s one step accepts only `success` from each need.
- **Why a leg's absence cannot decide.** The verdict counts every shard the plan promised, and a
  shard that uploaded nothing is VOID by name. A leg that was not started also uploads nothing, so
  only the listing can tell "had nothing to examine" from "its runner stopped". And GitHub reads a
  job skipped by its condition as passing: "A skipped job reports its status as 'Success' and will
  not prevent a pull request from merging, even if the job was configured as a required check"
  (GitHub Actions docs, "Using conditions to control job execution"). So a skip may read as correct
  only where the verdict, which must succeed, has checked it against the listing.

## 2. Requirements

R1. `mutation-verdict.py shards` writes a step output `listed`: the number of Rust mutants its
    plan's shards hold, `0` when the Rust class does not apply and `0` for an empty listing.
    `mutation-plan` maps it as the job output `listed`. The shard count, the matrix and each
    shard's mutants are unchanged for every listing.
R2. `mutation-rust` takes the job-level condition
    `${{ needs.mutation-plan.outputs.listed != '0' }}`: it is not started when the plan's listing
    gives no shard a mutant, whether the Rust class does not apply or applies with an empty
    listing. The condition reads the plan's `listed` output and nothing else, never `rust`. Its
    matrix is unchanged, so a listing that is not empty starts the same legs as before.
R3. `mutation-rows` takes the job-level condition
    `${{ needs.mutation-plan.outputs.rows == 'true' || needs.mutation-plan.outputs.scope == 'diff' }}`:
    it is not started only when the plan selects no row and no retirement check is due, the
    `not-applicable` scope. On a diff it starts with or without a selected row, and runs the
    retirement check as before.
R4. The verdict reads a shard that the plan lists no mutant for and whose artifact is absent (no
    report and no recorded exit) as `not started: the plan lists no mutant for it`, and it adds
    nothing to the examined count. The class's examined count is then the rows' alone, as it is today
    for such a shard, and an applying class that examined nothing is still VOID. A shard the plan
    lists mutants for whose artifact is absent is still VOID by name.
R5. When every shard's report is whole, the verdict refuses as VOID a sum of the reports' mutants
    (caught, missed, timed out and unviable) that differs from the number of mutants the plan's
    shards list, and names both numbers.
R6. A new verb, `mutation-verdict.py legs --plan <plan> --rust-leg <result> --rows-leg <result>`,
    judges each leg's result against the plan. A `skipped` `mutation-rust` is correct only when the
    plan's shards list no mutant. A `skipped` `mutation-rows` is correct only when the plan selects
    no row and its scope is not `diff`. Any other skip is VOID, naming the leg and what the plan
    owed it. A result other than `success`, `failure`, `cancelled` or `skipped` is VOID, and a plan
    that is not a mutation plan, or names no shards beside a skipped `mutation-rust`, is VOID. A leg
    that started is judged by its own job, its reports and `ci`. It examines the two legs.
R7. `mutation-verdict`'s step passes `needs.mutation-rust.result` and `needs.mutation-rows.result`
    through `env:` and runs `legs` after its two judge lines, which stay byte-identical. The step
    exits with the first status that is not 0: the Rust judge, the oracle's, then `legs`.
R8. `ci` admits `skipped` from `mutation-rust` and `mutation-rows` only, one admission for each of
    those legs that was not started, and prints each one. Every other result that is not `success`
    still fails it: a cancelled or failed leg, a skipped `mutation-verdict`, any other skipped need.
    A leg skipped while the listing owed it fails `mutation-verdict` (R6), whose `success` `ci`
    requires, so `ci` cannot pass with it.
R9. These stay byte-identical: both cargo-mutants listing commands and their conditions, the
    `shards` computation and its matrix for every listing, the shard job's `cargo mutants` line
    (both timeouts, `--timeout 300 --build-timeout 600`, and the baseline it runs), the
    equivalence records, the rows selection, the retirement step and its condition, and the weekly
    battery. The verdict's examined count over a recorded run's inputs is the same at `dev` and at
    this delivery's head, and so is the rows census.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | `shards` writes `listed`: `0` for a plan whose Rust class does not apply, even beside a listing that is not empty, `0` for a Rust diff whose listing is empty, and the listing's count otherwise, with the shard count and matrix of each unchanged; `mutation-plan` maps it as a job output | `test_not_started_legs.py` `the_plan_writes_how_many_rust_mutants_its_listing_holds` |
| A2 | `mutation-rust`'s and `mutation-rows`' job-level conditions read the plan's outputs alone: evaluated over every combination of `listed`, `rust`, `rows` and `scope`, the rust leg starts exactly when `listed` is not `0` and the rows leg exactly when a row is selected or the scope is `diff`; the matrix line is unchanged, and no other mutation job gains a condition | `test_not_started_legs.py` `each_legs_job_condition_reads_the_plans_listing_alone` |
| A3 | on a recorded plan whose Rust class applies and whose listing is empty, a shard with no artifact reads `not started`, the rows carry the examined count and the verdict is ok; with no row it is VOID because nothing was examined; `legs` reads a skipped rust leg there, and a skipped rows leg on a `not-applicable` plan, as not started and correct | `test_not_started_legs.py` `a_leg_the_listing_gives_nothing_reads_not_started` |
| A4 | the first plant: on a recorded plan whose listing holds mutants, the shard with no artifact is VOID by name; `legs` refuses by name a skipped rust leg there, a skipped rows leg beside a selected row, a skipped rows leg on a diff with no row selected, and a result that is not a job's | `test_not_started_legs.py` `a_listed_leg_that_is_missing_or_not_started_is_refused_by_name` |
| A5 | the second plant: the recorded whole report with one caught mutant and one total added reads VOID, naming the reports' count and the listing's; so does a report in a shard the listing gives no mutant; the recorded report itself reads ok | `test_not_started_legs.py` `an_examined_sum_that_differs_from_the_listing_is_refused` |
| A6 | `ci`'s own step, run under `bash -e` with each need's result: a not-started `mutation-rust` or `mutation-rows`, or both, passes and is named; a skipped `mutation-verdict`, a skipped other job, a cancelled leg and a failed verdict beside a skipped leg each fail it | `test_not_started_legs.py` `ci_admits_a_not_started_leg_and_no_other_skip` |
| A7 | `mutation-verdict`'s own step, run with a recording interpreter, passes each leg's result to `legs` and fails with `legs`' status when both judges passed, and keeps a judge's status when one failed | `test_not_started_legs.py` `the_verdict_step_fails_on_the_legs_check` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k the_plan_writes_how_many_rust_mutants_its_listing_holds
A2: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k each_legs_job_condition_reads_the_plans_listing_alone
A3: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k a_leg_the_listing_gives_nothing_reads_not_started
A4: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k a_listed_leg_that_is_missing_or_not_started_is_refused_by_name
A5: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k an_examined_sum_that_differs_from_the_listing_is_refused
A6: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k ci_admits_a_not_started_leg_and_no_other_skip
A7: python3 -m unittest discover -s scripts/tests -p test_not_started_legs.py -k the_verdict_step_fails_on_the_legs_check
```

The recorded inputs live in `scripts/tests/fixtures/not-started-legs/`, laid out as the verdict's
job downloads them. They are the plan, the rows' report and, for one, the shard's report, of two
pull request runs, reduced to the fields the verdict reads: no timing, no log, no path of the
machine that ran them. `listed/` is a plan whose listing holds mutants, with its shard's whole
report (run 36624231257, #396); `empty/` is a plan whose Rust class applies and whose listing is
empty (run 36604153634, #362). The two plants are built from `listed/` at run time and replay with
`mutation-verdict.py judge` alone: the first removes the shard's artifact, as a leg that never ran
leaves it; the second raises the report's caught count and its total by one, so the report still
reads whole and every mutant it names is listed once. Each test prints how many cases it examined
and refuses zero.

A change that lists Rust mutants runs its legs exactly as before, and the pull request's own CI shows
both readings: its first run, whose diff lists no Rust mutant, starts no `mutation-rust` leg, and
its verdict and `ci` pass. The weekly battery does not read the plan and is unchanged.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/ci.yml` | repo | changed: the plan's `listed` output (R1), the two legs' conditions (R2, R3), the verdict's `legs` line (R7), `ci`'s admission (R8) |
| `scripts/mutation-verdict.py` | repo | changed: `listed` (R1), the not-started reading (R4), the examined sum (R5), `legs` (R6) |
| `scripts/tests/test_not_started_legs.py` | repo | added: A1 to A7 |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: its job-level condition guard admits R2's and R3's conditions by job name, and only them |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-plan/plan.json` | repo | added: recorded, reduced |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rows/rows.json` | repo | added: recorded, reduced |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/cargo-mutants.exit` | repo | added: recorded |
| `scripts/tests/fixtures/not-started-legs/listed/mutation-rust-shard-0/mutants.out/outcomes.json` | repo | added: recorded, reduced |
| `scripts/tests/fixtures/not-started-legs/empty/mutation-plan/plan.json` | repo | added: recorded, reduced |
| `scripts/tests/fixtures/not-started-legs/empty/mutation-rows/rows.json` | repo | added: recorded, reduced |
| `scripts/mutation-rows.d/S29000-S29099.json` | repo | added: section 7's rows |
| `docs/specs/SPEC-290-a-leg-with-nothing-to-examine-is-not-started.md` | repo | added |
| `docs/decisions/ADR-290-a-leg-with-nothing-to-examine-is-not-started.md` | repo | added |
| `docs/schematics/mutation-testing.md` | repo | changed: an insert-only section 6, the legs from the plan to `ci` |
| `docs/specs/SPEC-039-every-change-proves-its-tests-kill-its-mutants.md` | repo | changed: an insert-only amendment section at its end |
| `docs/red-first/SPEC-290.md` | repo | added |
| `changelog.d/ci-not-started-legs-290.md` | repo | added |

## 5. What this does NOT do

- It does not skip the retirement check: the rows leg starts on every diff, with or without a
  selected row, and SPEC-039 R11's check runs where it runs today (#435).
- It removes no test, mutant, row, timeout or baseline, and narrows no listing and no scan: every
  leg that has something to examine starts and runs exactly as before (#435).
- It does not change `mutation-web`, `mutation-plan` or `mutation-verdict`, which still start on
  every event, nor any job of the gate itself (#435).
- It does not change the weekly battery, which sizes its own shards with `size` and counts its
  reports with `battery`, and reads no plan output (#435).
- It gives the gate's own Python no generated mutants: its new lines are proved by the hand rows of
  section 7 (#218).

## 6. Risks

- **An output that is missing or misspelled.** `listed` unset reads as the empty string, and
  `'' != '0'` holds, so the rust leg starts: the condition fails toward running, never toward a
  skip. A1 and A2 pin the output's name and its mapping.
- **A plan that fails.** Its dependants are skipped by the implicit `success()` of their
  conditions, and `ci` fails on the plan's own `failure`, as today.
- **A skipped leg read as a pass.** Only `ci` is the required check, it runs under `always()`, and
  it admits a skip from the two legs alone, beside a `mutation-verdict` that must succeed; A6 and
  A7 run both steps as written.
- **A report whose counters disagree with its names.** R5 refuses it; before this delivery such a
  report read whole, and its counters decided the examined count.

## 7. The mutation rows (S29000-S29099)

Each row mutates one line this delivery adds and names the one test that kills it; they are proved
KILLED on a committed tree with `python3 scripts/mutation_rows.py prove --band S29000-S29099`.

| row | mutates | its killer |
|---|---|---|
| S29001 | `mutation-rust` loses its condition: the placeholder leg returns | A2 |
| S29002 | `mutation-rows`' condition loses its `scope == 'diff'` term: the retirement check skipped | A2 |
| S29003 | the plan's `listed` output unmapped | A1 |
| S29004 | the not-started reading decided from the artifact's absence alone, not the listing | A4 |
| S29005 | the not-started reading also taken with a report present | A5 |
| S29006 | the examined-sum refusal dropped | A5 |
| S29007 | `legs` decides the rust leg from the `rust` class, not the listing | A4 |
| S29008 | `legs` drops the retirement check from what the rows leg owes | A4 |
| S29009 | `ci` admits a skip without a leg that was not started | A6 |
| S29010 | the verdict step drops `legs`' status | A7 |

## 8. References

Issue #435; ADR-290; ADR-057; SPEC-039 R3, R4, R11 and R18; SPEC-038; the GitHub Actions docs on
`jobs.<job_id>.if`, the `needs` context and skipped required checks, as ADR-290 quotes them.
