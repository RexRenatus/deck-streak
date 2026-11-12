# ADR-518: The release run is named in base-is-dev, and ci admits its mutation results by value

| field | value |
|---|---|
| status | proposed |
| spec | SPEC-500 |
| builds on | ADR-057 D11, ADR-290, SPEC-422 R1 (none amended in kind) |
| measured at | this branch's base, `d3b09d5e` |

## Context

A change reaches `main` only through the release pull request, `dev` into `main` from this repository, and `ci` is its
required check. ADR-057 D11 has the release judged on its merge diff, every change `dev` carries since the last
release, and `ci`'s step fails on any mutation job's failure or cancellation (`.github/workflows/ci.yml:1141-1154`).
Each change that diff carries was already judged, mutation jobs included, on its own pull request into `dev`, where
`ci` keeps gating. On the release run the same jobs gate a second time, over the whole diff at once, so one red leg
holds every change, and the weekly sweep with them: GitHub runs `schedule` only from `main`, and `main` holds no
`mutation-weekly.yml` (SPEC-500 section 1). This ADR decides where the workflow names the release run, how `ci` reads
the mutation jobs there, where their results go, and what stays.

## D1. Where the release run is named

- **Chosen: a second step of `base-is-dev`, `id: release`, writing `release=true` or `release=false` to
  `$GITHUB_OUTPUT` by `release_run`'s predicate, and a job output `release` that `ci` reads.** `base-is-dev` already
  reads the five inputs the predicate needs, from the same five expressions, and already refuses every head into `main`
  but this repository's `dev`. A refused first step fails the job before the second runs, so the output is empty and
  `ci` reads the run as not the release run: the definition fails closed. The step runs as shell, so a test runs it
  over every context and compares it with `release_run` itself (SPEC-500 A1).
- **An expression in `ci`'s env** (`github.event_name == 'pull_request' && github.base_ref == 'main' && ...`): lost,
  because no local test can evaluate a workflow expression, so a second definition of the release run would be held
  by nothing but review.
- **The plan's release mark** (`plan.json`'s `release`, SPEC-422 R2): lost, because `mutation-plan` is itself one of
  the jobs whose result becomes advisory, and a gate that takes its own exemption from a job it exempts trusts that
  job twice.
- **A new job:** lost, because it adds a need and a check-run name to keep in step with the ruleset and the ceiling
  derivations (SPEC-362 R3), for one line of shell `base-is-dev` can hold.

## D2. How `ci` admits the mutation results

- **Chosen: on the release run, one non-success result of `RESULTS` is admitted, by value, for each mutation job whose
  result is not `success`; every other result must be `success`.** `RESULTS` stays `join(needs.*.result, ' ')`, a list
  of values without names, so the admission is by value: the multiset of results left is exactly the 9 gating needs',
  and a need added later is judged without an edit here. It generalises the skip counter of SPEC-290 R8 from one value
  to every value.
- **Judging the gating needs by name** (one env per gating need): lost, because a need added later would not be judged
  until someone named it, which is the failure `join(needs.*.result, ' ')` exists to prevent.
- **`continue-on-error` on the mutation jobs:** lost, because it turns a failed job's result into `success` for every
  reader, on every run, the pull requests into `dev` included, unless guarded by an expression no test can run.
- **Not starting the mutation jobs on the release run:** lost, because the run would then report nothing, and the
  release's whole-diff results and the `mutation-outcomes` artifact a later release run carries from (SPEC-493) would
  be gone.
- **Every other run keeps today's rule, byte for byte in its lines.** When the output is not `true`, the skip loop and
  the counter run as they do today. Their lines stay in the script, each once in `ci.yml`, so the hand rows anchored
  there (S29009, S29016, S42204) and their killer stand.

## D3. Where the advisory results go

- **Chosen: `ci` prints `ci: <job> reads <result>; advisory on the release run (SPEC-500)` for each of the 8 mutation
  jobs, in its needs' order, and appends the same lines to the step summary when `GITHUB_STEP_SUMMARY` is set.** The
  reader of the release pull request sees every mutation result on the check it opens, and each job's own log, its
  report artifacts and `mutation-outcomes` stay where they are.
- **An issue or a comment per red job:** lost, because it needs a token that writes, and the workflow's token stays
  `contents: read` (SPEC-034).

## D4. What stays

- Every mutation job, its condition, matrix, bound, report and upload; the rows job, which keeps gating every pull
  request that is not the release; the verdict; the plan's release mark; the weekly sweep. ADR-057 D11 still has the
  release's merge diff judged in full, in the run that reports it; only `ci`'s reading of those jobs changes on that
  one run.
- The ruleset: `ci` and `fragment` stay the required checks.

## Consequences

- A release pull request can merge while a mutation job of its merge diff is red; its results stay on the run, and
  the weekly sweep reaches `main` with the merge.
- Every pull request into `dev` is gated by the mutation jobs as before, and a release pull request from a fork or from
  any other branch is refused by `base-is-dev` and fails `ci`.
- `run_base_is_dev` cuts the first step's script up to the next step (SPEC-500 R5), so a later step in `base-is-dev`
  never runs inside it.
