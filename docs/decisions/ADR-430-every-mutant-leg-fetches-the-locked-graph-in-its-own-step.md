---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-430: every mutant leg fetches the locked graph in its own step before its baseline

Decides SPEC-416.

## Context and Problem Statement

A release pull request runs `ci.yml`'s `mutation-rust` legs over its whole diff. Each leg restores
the Rust cache by key, then runs cargo-mutants, whose first act is the unmutated baseline: the
workspace's tests, unmutated. One of those tests, the progression census
(`crates/progression/tests/xp_census.rs:714-716`), asks `cargo metadata --locked --offline
--all-features` for the graph. Offline, cargo can only read the packages already in the registry
cache.

A pull request into `main` restores no cache written on `dev`, so every leg started on an empty
registry. A build downloads only the packages it compiles, and the census needs every target's.
So the census failed, the baseline was red, and cargo-mutants examined no mutant in any leg:

- job 114529706626 (`mutation-rust`, shard 0): `Cache not found` (log line 252); 49 mutants
  listed; `FAILED   Unmutated baseline in 345s build + 935s test` (line 387); the census test
  failed in 3.548 s on `failed to download` a package (`accessory`, line 6120), and fail-fast
  stopped three others; `cargo-mutants exited 4` (line 6137). The step took 1283 s, and the job
  concluded success.
- job 114529617445 (`rust`): the same miss (line 287); its fetch step ran 36 s and downloaded that
  package (line 339); the census passed and the job was green.
- job 114529617364 (`release`): its fetch step ran 39 s; green.

The `rust` job (`ci.yml:65-66`), the `release` job (`:187-188`) and the `mutation-rows` job
(`:622-624`, SPEC-332) each run `cargo fetch --locked` after their cache restore. `mutation-rust`
(`:447-524`) has no fetch step. SPEC-332 excluded it in so many words (its section 5: a job that
runs an offline cargo call behind a restore-only cache without running `prove`, #606).

## Decision Drivers

- The census keeps every offline flag it has, and runs in every leg's baseline: a test that
  checks the graph must not be the thing a fix weakens.
- One rule, one judge: SPEC-332's test already holds a job to "fetch after the last restore and
  before the first offline call". The new population reuses that judge.
- Narrow: while the release pull request #822 is open, `dev` takes only narrow fixes to the
  mutation legs, each by measurement.
- The verdict, not a leg's colour, decides. A red baseline must reach the verdict as VOID.

## Considered Options (the alternatives each was chosen against)

D1, what gives the baseline every package offline:

- Chosen: one step in `mutation-rust`, after its cache restore and before cargo-mutants runs,
  that runs `cargo fetch --locked --manifest-path Cargo.toml` under the same `if:` as the
  cargo-mutants step, because it is the step the `rust` and `release` jobs already run, measured
  at 36 s and 39 s on a cold cache (jobs 114529617445 and 114529617364), and it changes no test.
- Remove `--offline` from the census's `cargo metadata`: rejected because it weakens the test
  this delivery exists to let run; an online census also passes on a graph the lock does not hold.
- Skip or filter the census out of a mutant's run: rejected because a mutant the census would
  kill then survives, and the baseline judges less (an `--exclude`, a feature or a test filter
  alike).
- Restore the cache by exact key only: rejected because a pull request into `main` restores no
  `dev` cache at all (job 114529706626, `Cache not found`), so no key choice supplies the packages.
- Fetch in `rust-cache.yml`'s warm-up: rejected because that workflow saves on `dev`, which a pull
  request into `main` cannot restore, and its warm-up runs no fetch, so its entry can lack the
  packages the census needs.
- Fetch inside cargo-mutants' run: rejected because it runs once per mutant as well as once per
  baseline (a wrapper, or a `cargo fetch` in the test command), and it hides the fetch's failure
  inside a test outcome.

D2, which jobs the rule holds:

- Chosen: every job in `ci.yml` that runs a cargo-mutants baseline (a step that runs
  `cargo mutants` without `--list`), which is `mutation-rust` today, because those are the legs
  of a pull request's run and the release's re-run reads them.
- Every workflow, the weekly battery's `rust` leg and its rehearsal included: rejected because
  the weekly legs run on `dev`, where the `rust` job's saved cache holds every target's packages,
  and neither is part of the release's re-run; the gap is recorded as open (SPEC-416 section 5).
- Only the one job by name: rejected because a second leg added later would escape the rule; the
  population is found by what a job runs, as SPEC-332's is.

D3, whether `mutation-verdict` reads a red-baseline leg as examined (measured, not assumed):

- Chosen: no change. `partial_reason` refuses any exit outside `WHOLE_EXITS = {0, 2, 3}`
  (`scripts/mutation-verdict.py:133`, `:1203-1206`), and exit 4 is listed as a red baseline
  (`:134-140`); `whole_reports` records it with `verdict.void` (`:1260`); `Verdict.close` returns
  `EXIT_VOID` (`:1134`); the judge step keeps that status (`ci.yml:689`) and exits with it
  (`:699`); the aggregate `ci` job then fails (`:904`). A red baseline is VOID already, because it
  fails closed at every step from the leg's report to the aggregate.
- Make the leg's step exit with cargo-mutants' code: rejected because the leg would turn red while
  the verdict already reads the same report as VOID, and the verdict's leg check would read a
  failed leg where it now reads a report; the leg's green colour is noted, not changed.
- A baseline-specific VOID message in the verdict: rejected because the message already names the
  exit and its meaning (`TOOL_EXITS[4]`), and the edit would move a span a formal model covers.

D4, how it is tested:

- Chosen: SPEC-332's module, `scripts/tests/test_rows_jobs_fetch_graph.py`, gains a population of
  baseline jobs and gives its judge the anchor step as a parameter, because the rule is the same
  and one judge cannot drift from itself. The test reads the workflow as text; it runs no cargo
  and no network.
- A new module with its own judge: rejected because two judges of one rule drift apart.
- A test that runs the census offline against a cold registry: rejected because it measures the
  network and the runner, not the workflow, and its outcome would vary with the cache.

D5, the shape of the record:

- Chosen: two deliveries. This one fixes the baseline; SPEC-417 and ADR-431 deal the rows leg into
  legs that fit their bound, because the two share no line and no test, and the release's re-run
  needs both.
- One delivery for both: rejected because the rows sizing changes the plan, the rows runner, the
  verdict's downloads and the run's leg ceiling, so one red-and-green cycle would hold a one-step
  fix behind a multi-file change.

## Decision Outcome

`mutation-rust` gains one step, placed right after its cache restore:

```yaml
      - name: the locked dependency graph, every target's packages (the census reads it offline at the baseline)
        if: ${{ needs.mutation-plan.outputs.rust == 'true' }}
        run: cargo fetch --locked --manifest-path Cargo.toml
```

The step's `run:` line is the `rust` and `release` jobs' own. It is not the rows job's
`run: cargo fetch --locked` line, which rows S33200 and S33201 anchor on and which stays unique.

### Consequences

- Good: every leg's baseline sees every target's packages, so a baseline is red only when a test
  is red, and the release's re-run examines its mutants.
- Good: the census keeps `--locked --offline --all-features`.
- Neutral: each leg spends about 40 s more (the measured fetches above), inside the half-bound
  margin each leg's projection already keeps (SPEC-362 R2).
- Bad: one more network call per leg. A registry outage now fails the leg's fetch step. The leg
  then uploads no report, which the verdict reads as VOID, never as a pass.

### Confirmation

SPEC-416's A1 reads the workflow and is red at the base, where `mutation-rust` holds no fetch. A2
plants each wrong shape into a copy of the job and is refused by name. Rows S41600 to S41602
remove the fetch, swallow its exit, and invert its `if:`; each is killed by A1. The release's
re-run then shows every `mutation-rust` leg past its baseline, read by job id.

## What would make this wrong

- A leg whose fetch passes and whose baseline still fails on a download: then the census needs a
  package `cargo fetch --locked` does not fetch, and the fix is in the lock, not the workflow.
- A baseline red for a test the fetch does not touch: that is a red test, and the verdict's VOID
  is right.

## More Information

- SPEC-332 and ADR-333, the same rule for the rows jobs (#606), amended by a section naming
  SPEC-416. SPEC-362 R2, each leg's projection within half its job's bound. SPEC-196 and ADR-199,
  the memory scope the leg's cargo-mutants call runs inside, unchanged.
