---
status: "proposed"
---

# ADR-373: a release is judged in one run of round-robin legs sized to the hosted job, bounded by the run's job budget

Decides SPEC-362.

## Context and Problem Statement

A release pull request into `main` is judged on its own merge diff, every mutant of the range
listed, sharded, every shard counted, and the verdict a need of `ci` (SPEC-039 R3 and R18). The
first release pull request after the first tag carries the whole tree: #691 listed 8367 mutants,
projected at 1,023,555 s, and `shards` refused because more than 256 shards of at most 3600 s each
would be needed. The refusal is correct under its rules, and the rules are what has to change,
without capping, sampling or dropping a mutant, and without deferring any mutant or row to the
verdicts the merged pull requests already read: a later change can delete the test that killed an
earlier change's mutant, and only the combined diff shows it (#220).

The same sizing also decides whether the scheduled battery can judge the whole tree at all: today
it deals the tree into 32 legs whose projections each exceed their job's timeout several times.

## Decision Drivers

- Every listed mutant runs once, in a leg the verdict counts, and the verdict is a need of `ci`.
- The run must finish inside one release's working time, never hold the runners for longer.
- A workflow run generates at most 256 jobs, counted per run (GitHub Actions' limits page), and a
  hosted job runs for at most 360 minutes.
- The pull request's token is read-only, and the repository runs only on hosted runners
  (ADR-017).
- ADR-034's model: only `dev` reaches `main`, and `base-is-dev` refuses any other head.
- The smallest change to the sizing that `dev`'s pull requests, the release and the battery
  already share.

## Considered Options (the alternatives it was chosen against)

- **Chosen: one run, one matrix of round-robin legs, bounded at half of the hosted job's limit, at
  most the run's job budget.**
- Several matrix jobs in one run, each of 256 legs or fewer: lost because the 256 jobs are per
  run, so a second matrix in the same run shares the first one's budget and adds no leg.
- Several runs, each of 256 legs or fewer, with one verdict counting every leg: lost because a job
  in another run cannot be a need of `ci` in the release's run; a verdict that waits for other runs
  is itself a job bounded by the hosted limit; starting runs from a pull request needs a token
  that can write actions, which the read-only token refuses; and `workflow_run` reads the default
  branch's workflow, which lacks the change until a release lands. Throughput is bounded by how
  many jobs run at once, not by the matrix, so more runs finish no sooner.
- Matrices nested through reusable workflows: lost because the per-run limit still binds the run
  that calls them, and reported uses of the pattern to exceed it have met the service's abuse
  protection.
- Stepwise release merges along `dev`'s first-parent history, each step a release of its own: lost
  because a step judges only its own merge diff, so a later step's change that deletes the
  test killing an earlier step's mutant is never judged with that mutant (#220's class, at step
  size); a step's head is not `dev`'s tip, so `base-is-dev` and ADR-034's model would have to
  admit a second head into `main`, which the no-back-merge model does not survive; and the steps
  run one after another, so the range takes longer than one run.
- Per-package legs (each leg one package's slice, its baseline that package's tests): lost because
  at today's tree it needs 153 legs against round-robin's 144, and the ceiling is the binding
  limit; it saves about 6% of runner time and adds a deal, a per-leg package word and per-leg
  checks. It is the fallback if the census comes to dominate every round-robin leg's baseline.
  Both counts charge the census once in a leg's baseline. The sizer as built also charges each
  census package's term on top of `BASELINE_SECONDS` (SPEC-327 R4), so `shards` reads 200
  round-robin legs for the same listing; the first release run's leg times decide which reading
  holds.
- `--baseline=skip` on the legs: lost because a leg's own build is then never proven green before
  its mutants are judged, and a pre-existing failure would read as a kill.
- `-j` parallel jobs inside a leg: lost because it conflicts with `--in-place` (cargo-mutants
  refuses the pair), and copies of the tree per worker cost the leg's disk.
- A nextest filter that leaves the census out of a mutant's run: lost because SPEC-327 R7 runs the
  census for every progression mutant, and a census-only kill would survive.
- Larger hosted runners, or self-hosted runners: lost because larger runners are billed for a
  public repository too, and a public repository refuses self-hosted runners (ADR-017).
- A one-time admission of the release past its mutation gate: lost because it lands a tree whose
  mutants were never examined, the next whole-range release meets the same refusal, and the
  scheduled battery could not judge the tree either.
- Reading the release's mutants as judged by the verdicts of the pull requests `dev` merged: named
  only as excluded; it is never proposed in any variant, because the combined diff is where
  interactions show (#220).

## Decision Outcome

Chosen: one run, one matrix of round-robin legs, because it is the only option that keeps every
mutant in a leg counted by a verdict `ci` needs, inside the per-run job limit, with the least new
machinery:

- each `mutation-rust` leg declares `timeout-minutes: 360`, and `SHARD_BOUND_SECONDS` is 10800,
  half of it, as SPEC-039 R18 sets the bound;
- `LEG_CEILING` per workflow is 256 less what the workflow's other jobs can generate, derived from
  the workflow file by a test; beyond it the plan refuses whole, with its count and projection;
- the baseline and the census are re-measured, and the per-mutant timeout follows the census by
  SPEC-327 R2's rule, so a census-only kill can no longer read as a timeout;
- each leg proves from its own baseline log that the timeout covers its slowest test;
- the verdict reads the tool's listing as the population, and the plan's legs must partition it;
- nextest's `mutants` profile stops a mutant's run at its first failure, which drops no mutant;
- the scheduled battery is sized by the same function at its own ceiling, and its fixed 32 legs
  are retired.

### Consequences

- Good: the release that refused today sizes at 200 legs by `shards`' own projection, within its
  ceiling of 209, with every mutant counted. The battery judges the whole tree for the first time.
- Good: `dev`'s pull requests keep their path: a small diff still takes one leg, and a larger one
  takes fewer legs than at the old bound.
- Bad: a whole-tree release holds the hosted runners for a long run, and `dev` must
  merge nothing while it runs, because a push to `dev` cancels the release pull request's run.
  After the first release, a release's range is a small fraction of the tree.
- Bad: the tree can outgrow one run. The plan prints its headroom, and when the tree needs more
  legs than the ceiling the plan refuses; the remedy then is a measured re-pricing or a faster
  census, never a cap.
- Neutral: wall-clock follows the account's concurrent-job limit, and no `max-parallel` is set; the
  first battery's wall-clock decides whether one is owed.
- Neutral: the battery and a release each hold the runners while they run, so a release is never
  started while the battery runs.

### Confirmation

The acceptance criteria of SPEC-362: the release's listing at #691 sizes within the ceiling,
the bound and ceiling tests read the workflow files, the verdict's new VOIDs each have a test that
fails first, and the TLA+ entry `formal/tla/EveryLegCounted` holds its two properties with both
witnesses caught. On the first release pull request after the build lands, `mutation-plan` prints
its legs and ceiling and `mutation-verdict` counts every leg.

## More Information

- SPEC-039 R3 and R18 (the release's merge diff, the shards and their bound), SPEC-129 R4 and
  SPEC-327 R2, R7 and R8 (the census and the battery), each amended by SPEC-362.
- ADR-034 (only `dev` reaches `main`), ADR-017 (hosted runners only), ADR-057 (mutation testing on
  the diff in CI and on the whole tree by the scheduled battery).
- What would make this wrong: the per-run limit being lifted or counted per matrix (then the
  ceiling is conservative and can be raised by measurement); a measured census far below its
  price after R9 (then the bound can be re-measured downwards); or a tree that grows past the
  ceiling faster than re-pricing recovers headroom (then the per-package fallback, or a faster
  census, is owed).

## Amendment: D1, the Python matrix is sized to the hosted job

Insert-only; every earlier byte is kept. SPEC-362 R14 lowers `PYTHON_SHARD_MUTANTS` from 40 to 20
(`PYTHON_MAX_SHARDS` stays 32), under this decision's own principle that legs are sized to the
hosted job. Measured: a leg of 34 Python mutants took 54 minutes 42 seconds, and its sibling of 34
was cancelled at the 60-minute job cap, so a full shard of 40 needs about 64 minutes. Every listed
mutant is still examined and every shard still counted, so this is not a weakening: the same
population spreads over more legs (68 listed reads 4 shards of 17).

Chosen against:

- Raising the Python job's `timeout-minutes`: rejected, because it lengthens the leg instead of
  sizing it, and the next growth in per-mutant cost eats the margin again.
- Sizing the Python legs by measured per-mutant cost, as `fewest_shards` does for Rust: rejected for
  this delivery, because the Python listing carries no per-mutant cost; it is a follow-up idea.

## Amendment: D3, the per-mutant budget is 2300 seconds

Insert-only; every earlier byte is kept. The owner's signed ruling
`docs/rulings/OWNER-RULING-2026-10-06-mutation-timeout-2300.md` raises the per-mutant budget to 2300
seconds, the census term to 1521 and the baseline to 2141, from run 37438835490, so a release's
progression legs read as judged instead of VOID under R7. Every mutant is still examined: a mutant that
timed out at the old budget was scored examined, and at the new one it is caught or missed, never a
pass. The raise is a weakening by ADR-199's letter, admitted by that ruling.

Chosen against:

- Keeping the old budget: rejected, because R7 voids the census leg at the measured reading, so no
  release could be judged at all.
- A budget above 2300: rejected, because it is headroom beyond any measured census, and the budget is
  derived from the measured census, with R7 naming a later overrun.
- Admitting a release past its mutation gate once: rejected, because it lands a tree whose mutants were
  never examined.
- Waiting for the census test's own speed-up: rejected as the cure for this delivery, because no
  release can be judged until it lands; it stays the cure that can lower the budget again.
