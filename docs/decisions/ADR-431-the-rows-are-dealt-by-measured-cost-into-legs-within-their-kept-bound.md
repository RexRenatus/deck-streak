---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-431: the rows are dealt into legs sized by their measured cost, within their kept bound

Decides SPEC-417.

## Context and Problem Statement

`ci.yml`'s `mutation-rows` job proves, one at a time, every hand-written row the plan selects
(`scripts/mutation_rows.py prove`, `:1191-1222`). It is one job with a 90-minute bound
(`ci.yml:579`). A release's diff selects every row, 2512 of them, and the runner writes its report
only after the last row (`:1217-1218`), so a job cut at its bound leaves no report at all.

Job 114529704482 (`mutation-rows`, the release pull request #822's run) read `Cache not found`
(log line 250), fetched the graph in 38 s, and proved rows from S02001 to S04909 for 5357 s:
331 KILLED, 0 SURVIVED. Then it was cancelled at its bound (line 1216), with 331 of 2512 rows
examined and no report. Its per-row cost, from the same log: mean 16.2 s, median 4.3 s, 90th
percentile 22.9 s, largest 198.5 s (S04150). The 88 rows of the band S03900-S03999 took 3369 s in job 114529704482
together. At the mean, 2512 rows need about 40,694 s, about 678 minutes. That is more than a
hosted job's whole 360-minute bound (SPEC-362 R2), so no raise of one job can hold them.

The run's `mutation-plan` job (job 114529617353, 24 s) already sizes the Rust and Python legs from
their listings (`scripts/mutation-verdict.py`'s `shards`, `:1008`). The rows have no such plan.

## Decision Drivers

- No row is sampled, dropped or skipped: every selected row is proved, or the verdict is VOID.
- No job's bound is raised: `mutation-rows` keeps its 90 minutes (`ci.yml:579`; SPEC-039 R18).
  Raising a job's bound is the maintainer's own decision, and this delivery takes none.
- A leg's projection stays within half its job's bound, the house margin (SPEC-039 R18, SPEC-362
  R2), so a slower runner or a costlier row than measured does not cut a leg.
- The verdict reads one rows report by one path, as it does now (`ci.yml:689-694`); the judge and
  the spans `formal/tla/EveryLegCounted` covers stay unedited.
- The run's jobs share one budget (`RUN_JOBS` in `scripts/tests/test_dispatch_shards.py`), and
  SPEC-362 pins a release's 200 Rust legs inside it; every rows leg is a job of that budget, so
  each costs one Rust leg of the ceiling.

## Considered Options (the alternatives each was chosen against)

D1, what makes the rows fit (the rows job keeps its 90-minute bound):

- Chosen: deal the rows into legs, each sized by the measured cost within half of the kept
  90-minute bound, because 2512 rows then take 16 legs of at most 157 rows, each projected at
  2669 s of the 2700 s half, every row is proved, and no job's bound moves.
- Raise the rows job's bound to 360 minutes and deal fewer legs (4 of at most 628 rows): rejected
  because raising a job's bound is the maintainer's own decision, which this delivery does not
  hold, and legs within the kept bound prove every row without it.
- Raise it to 180 minutes (8 legs of at most 314 rows): rejected for the same reason.
- Raise the one job's bound alone, with no legs: rejected because 678 minutes at the measured mean
  is more than the 360 minutes a hosted job may run, so no raise of one job holds a release's rows.
- Sample the rows, or prove only the rows a release changed: rejected because each examines fewer
  rows, which weakens the battery, and a row on an unchanged line still guards its code.

D2, the price, the bound and the most legs:

- Chosen: `ROW_SECONDS = 17`, the measured mean rounded up to the second; `ROWS_BOUND_SECONDS =
  2700`, half of the rows job's 90 minutes; `ROWS_MAX_LEGS = 17`, because a leg's time is the sum
  of its rows, which the mean governs, and 17 legs of at most 158 rows hold 2686 rows: the 2512
  measured, and #822's one re-run after SPEC-416 and this delivery land, at least 2539 rows (2512, SPEC-415's 8,
  SPEC-416's 3 and this delivery's 16), at a cost of 17 jobs of the run's budget.
- Price each row at the 90th percentile, 22.9 s: rejected because it prices a leg's sum by its
  slow tail: a leg then holds 117 rows, 2512 rows take 22 legs, and the ceiling falls to 188,
  below #822's 192 Rust legs. The half-bound margin already holds a leg up to 2.1 times the mean
  (158 rows in 5400 s is 34.2 s a row).
- Price each row at the largest, 198.5 s: rejected because one row's cost is not a leg's.
- A per-row price table: rejected because one job has priced 331 rows, the first in id order, so
  most rows would carry a guessed price.
- Sixteen legs at most, 2528 rows: rejected because #822's one re-run comes after SPEC-416's 3
  rows and this delivery's 16 land, at least 2539 rows, so `legs` would refuse it whole by name.
- Eighteen legs at most, 2844 rows: rejected because the eighteenth costs one more Rust leg of the
  run's ceiling, 192 against #822's 192 Rust legs with none to spare, and 17 hold the 2539; a
  population past 2686 re-derives the most legs by this decision, with the ceiling.

D3, where the rows are sized:

- Chosen: a `legs` verb in `scripts/mutation_rows.py`, run in the plan job after `shards`, which
  writes `rows_legs` and `rows_matrix` as the plan's outputs, because the rows runner already owns
  the selection (`chosen`, `:1169-1188`), so the count it sizes is the population it proves.
- In `scripts/mutation-verdict.py`'s `shards`: rejected because `shards` is a span the formal
  model covers, and the Python plan's own delivery edits that file and that verb.
- A fixed matrix of 17 legs: rejected because an ordinary pull request that selects a few rows
  would pay 17 jobs, and a population that outgrows it would be cut again with no refusal; `legs`
  refuses past `ROWS_MAX_LEGS` by name instead.

D4, which rows a leg proves:

- Chosen: `prove --leg k/n` proves the chosen rows `[k::n]`, in plan order, because dealing round
  robin spreads each band's rows over every leg, and a band's rows cost alike.
- Contiguous blocks: rejected because a slow band lands in one leg; the 88 rows of S03900-S03999
  cost 38.3 s each against a mean of 16.2 s.
- Dealing by cost: rejected because no per-row price exists (D2).

D5, how the verdict reads the legs:

- Chosen: a `merge` verb in the rows runner writes the one report the judge reads, from the leg
  reports the verdict's job downloads, because the judge, its path and its VOID for an id
  "selected and never proved" (`scripts/mutation-verdict.py:1176`) stay as they are. A missing
  leg is named, and its rows stay out of the merged report. The judge then reads each as VOID.
  A row in two legs, or in a leg the deal did not give it, is refused with no report written.
- The judge reads each leg's report itself: rejected because it edits the verdict's covered code
  and a file the Python plan's delivery also edits.
- The merge fails on a missing leg: rejected because a failed step skips the judge step after it,
  so the run would lose every class's verdict line while the judge would VOID those rows anyway.

D6, the run's ceiling for Rust legs:

- Chosen: `LEG_CEILING["ci"]` falls from 209 to 193, the rows job counted at `ROWS_MAX_LEGS`,
  because the ceiling is fixed before a plan exists and must fit the most rows legs a plan can
  make. #822's 192 Rust legs still fit, with 1 to spare. SPEC-362's release listing at #691, 200
  legs, no longer fits the pull request's run, and its plan refuses it whole by name; SPEC-417
  section 10 amends SPEC-362's pins.
- Count the rows legs a plan actually makes: rejected because the ceiling is a constant the plan
  reads, set before any rows count exists.
- Leave 209: rejected because 209 Rust legs beside 17 rows legs and the other jobs exceed the
  run's budget by 16.

D7, the retirement check:

- Chosen: it runs in leg 0 only, because it compares the rows file at two commits and needs no
  leg's rows.
- In every leg: rejected because it repeats one check up to 17 times in a run.
- Its own job: rejected because it costs one more job of the run's budget.

D8, a formal model:

- Chosen: no new entry, and `formal/tla/EveryLegCounted` is not extended, because the legs share
  no written object (each uploads its own named report) and the verdict's job reads them only
  after every leg has ended (`needs`), so the merge is a total function of the reports present.
  Tests with planted reports hold it (SPEC-417 A6 and A7). `shards`, which the model covers,
  reads `LEG_CEILING`, whose value reaches no variable of the model.
- Extend `EveryLegCounted` with a rows population: rejected because it adds covers to an entry
  and restarts its promotion streak, for a function with no interleaving to explore.
- A proof of the deal's partition: rejected because `[k::n]` over `k` in `0..n-1` is a partition
  by the slice's definition, and A4 pins it on a fixture.

## Decision Outcome

The plan job writes `rows_legs` and `rows_matrix`. `mutation-rows` becomes a matrix over
`rows_matrix` with `fail-fast: false`, and keeps `timeout-minutes: 90`, now each leg's. Each leg
proves its deal into `rows-leg-<k>.json` and uploads it as `mutation-rows-leg-<k>`. The verdict's
job downloads `mutation-rows-leg-*` merged into `reports/mutation-rows`, runs `mutation_rows.py
merge` there under the same `if:`, and then judges exactly as before.

The arithmetic, each figure with its job:

- price: 5357 s over 331 rows is 16.2 s a row (job 114529704482); 17 s, rounded up;
- bound: 90 minutes is 5400 s, kept; half is 2700 s; a leg holds at most 158 rows (158 x 17 =
  2686 s; 159 x 17 = 2703 s);
- 2512 rows: 42,704 s at 17 s; 15 legs hold 2370 rows, too few, so 16 legs of at most 157 rows;
  157 x 17 = 2669 s, about 44 minutes;
- #822's re-run, at least 2539 rows: 16 legs hold 2528, too few, so 17 legs of at most 150 rows;
  150 x 17 = 2550 s;
- room: 17 legs hold 2686 rows; 2687 is refused by name;
- ceiling: 47 other jobs at the base, less the one rows job, plus 17 rows legs, is 63; the run's
  budget of 256 less 63 is 193.

### Consequences

- Good: a release's rows are all proved, each leg projected at most 2686 s within its 2700 s
  half, in place of one job cut at 90 minutes; no job's bound is raised.
- Good: a leg cut by a cost the measurement did not see leaves its rows unproved, and the verdict
  reads them as VOID by id, never as a pass.
- Neutral: an ordinary pull request selects few rows and runs one leg, as now.
- Bad: each leg pays its own cache restore, fetch and build, up to 17 times on a release. The
  measured mean already holds one cold start, spread over 331 rows; a leg of at most 158 rows
  spreads it over fewer, inside the half-bound margin.
- Bad: the Rust legs' ceiling falls by 16, to 193, and a release listing of 200 Rust legs no
  longer fits one pull request run. The Python plan's deliveries share that budget, so their leg
  counts and this one's are read together (SPEC-417 section 6).

### Confirmation

SPEC-417's A1 to A12, each red first where the base lacks the behaviour, and rows S41700 to
S41715. The rows job keeps its 90-minute bound, held by A9 and row S41712. The first release run
after it shows each rows leg's time, read by job id; the slowest leg within 1.3 times the mean
leg says the deal balances.

## What would make this wrong

- A rows leg past two thirds of its bound, 3600 s of 5400 s: the mean has moved; re-measure and
  re-derive by D2.
- A rows leg cut at 90 minutes: its rows read VOID; the price, not the bound, is re-derived, by
  measurement.
- A population past 2686 rows: `legs` refuses by name; the most legs is re-derived together with
  the run's ceiling. Every delivery adds rows, this one 16, so the population is counted again at
  each cut.

## More Information

- SPEC-039 R3, amended by a section naming SPEC-417 (its `mutation-rows` becomes one job per rows
  leg; R18's 90 minutes stands, now each leg's); SPEC-362 R3's ceiling and A1, amended by SPEC-417
  section 10 and by a section naming SPEC-417; SPEC-332, the rows job's fetch, unchanged; SPEC-290
  R3, the job's start condition, unchanged.

## Amendments: D2's most legs is 18 and D6's ceiling is 189, read from the release's plan (`docs/rulings/OWNER-RULING-2026-10-18-rows-legs-822.md`)

The release pull request's plan selected 2688 rows, two more than the 2686 that 17 legs hold, and
`legs` refused them whole by name (run 39151539183, job 117746024485; the same at run 39141497433,
job 117717787744). This is the case "What would make this wrong" names: the most legs is
re-derived together with the run's ceiling. The signed ruling
`docs/rulings/OWNER-RULING-2026-10-18-rows-legs-822.md` admits both figures, and SPEC-417's section 11
carries them.

D2, the most legs, as amended:

- Chosen: `ROWS_MAX_LEGS = 18`, because 18 is the fewest most legs that deal the 2688 rows read
  within the kept bound, and 18 legs of at most 158 rows hold 2844. `ROW_SECONDS = 17` and
  `ROWS_BOUND_SECONDS = 2700` stand.
- The largest of 18 legs holds 150 rows, projected at 2550 s of the 2700 s half (job 114529704482's price).
- Eighteen legs at most was rejected at the base because the eighteenth cost one of the release's
  192 Rust legs, with none to spare. That reason no longer holds: the release run's Rust legs are
  held to `RUST_RELEASE_CEILING` in a matrix of their own (SPEC-423), not to `LEG_CEILING["ci"]`,
  so the eighteenth costs one leg of the pull request run's ceiling only.
- Keeping 17: rejected because the release's plan refuses its 2688 rows whole, so the release
  cannot be judged at all.
- Re-measuring `ROW_SECONDS` so that 2688 rows fit 17 legs: rejected because it narrows the margin
  each leg keeps under its bound, and the owner named the most legs as the figure to sign.
- Nineteen legs or more: rejected because 18 is the fewest that deal the population read, and
  headroom above it admits legs no plan read.

D6, the run's ceiling for Rust legs, as amended:

- Chosen: `LEG_CEILING["ci"]` falls from 190 to 189, the rows job counted at `ROWS_MAX_LEGS`, 18,
  because the ceiling is fixed before a plan exists and must fit the most rows legs a plan can make.
  `LEG_CEILING["battery"]` stays 234, since the weekly workflow deals no rows legs.
- Keeping 190 by counting the rows legs outside the pull request run's jobs: rejected because the
  ceiling would no longer hold every job a pull request's run can generate (SPEC-417 R11).
- Raising the rows job's `timeout-minutes` so that 17 legs hold more rows: rejected for D1's reason,
  because raising a job's bound is the maintainer's own decision, and the kept bound proves every
  row at 18 legs.

Consequences, as amended:

- Good: the release's 2688 rows are all dealt and proved, each leg projected within its half bound.
- Bad: a release's rows pay up to 18 cold starts in place of 17.
- Bad: the pull request run's Rust ceiling falls by 1, to 189, and a pull request whose Rust mutants
  need 190 legs is refused whole by name, where it was dealt before.

Confirmation, as amended: SPEC-417's A13, red at a commit whose `ROWS_MAX_LEGS` is 17 and green at
18, with A2, A10 and A12 as section 11 moves them, and rows S41702, S41716 and S36201. The
release's next run shows 18 rows legs, each read by job id.

What would make this wrong, as amended:

- A population past 2844 rows: `legs` refuses by name, and the most legs is re-derived together
  with the run's ceiling, under a new signed ruling.
