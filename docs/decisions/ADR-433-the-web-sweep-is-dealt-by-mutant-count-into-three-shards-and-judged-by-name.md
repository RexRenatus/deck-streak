---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-433: the pull request's web sweep is dealt by StrykerJS's own mutant count into three shards, and one job of the leg's name judges them by name

Decides SPEC-419.

## Context and Problem Statement

`ci.yml`'s `mutation-web` job runs StrykerJS over every web production file a pull request
changes, whole (ADR-057 D3), in one job bounded at 100 minutes by ADR-390's rule. The release pull
request's one run, 38242723211, ran that job for 100m13s (job 114789042867): StrykerJS finished
inside the bound, but the job did not, so it reads cancelled, and a required leg at 100.2 percent
of its bound is cut by any growth. ADR-390 named the next step: a leg cut at 100 is evidence for
#693's sharding, not for a second raise. The sweep has to fit its bound with room to grow, without
one fewer mutant and without moving any verdict.

## Decision Drivers

- No fewer mutants and no verdict moved in the permissive direction (ADR-199's first driver).
- Every shard keeps the leg's bound of 100 minutes; the bound is not this decision's lever.
- A shard that is missing, cut or not whole reads VOID, never ok; nothing is judged around it.
- The verdict's reading of StrykerJS's report, `scripts/mutation-verdict.py`'s `judge`, the
  equivalence records and the survivors table, stays byte-identical.
- The check that reports `mutation-web` keeps that name, because the aggregate `ci` job reads it.

## Considered Options (the alternatives each was chosen against)

D1, how many shards:

- Chosen: three, because the slowest shard is projected at 35.2 minutes under the count model and
  41.2 under the tests-weighted one (job 114789042867), so the leg keeps more than half its bound
  for growth under every model the measure ran.
- Two: rejected because the slowest of two is 51.5 minutes under the count model and 57.4 under
  the tests-weighted one (job 114789042867), so the leg would cross two thirds of its bound, ADR-390's
  signal to re-measure, after a modest growth.
- Four: rejected because it buys 27.1 minutes under the count model and 35.9 tests-weighted (job
  114789042867) for a fourth repeat of the 162.7 s set-up and initial test run, and one more job
  per pull request, where three already holds the bound with room.

D2, how files are dealt:

- Chosen: whole files by mutant count, largest first, each onto the shard with the fewest mutants
  so far, equal counts by path and equally light shards by the lowest index, because a mutant's
  cost is the measured unit (0.7494 s, job 114789042867), and the deal is then a pure function of
  the counts that any job can recompute.
- An even split by file count: rejected because one file holds 1464 of 7805 mutants (job
  114789042867), so equal file counts can put most of the sweep on one shard.
- A deal weighted by the tests each mutant ran: rejected because the measure saw two shards of
  equal mutant count read 45.5 and 57.4 minutes under that weighting (job 114789042867), a skew of
  about 12 minutes the count model does not show, and its inputs exist only after a run.
- Splitting a file across shards: rejected because ADR-057 D3 mutates every touched file whole, and
  a survivor on a touched file is the pull request's own; a file stays in exactly one shard.

D3, where each file's count comes from:

- Chosen: StrykerJS's own mutation server, `stryker serve stdio`, asked `configure` then `discover`
  in `web/app` with `--mutate` set to exactly the plan's files, because it instruments with the
  run's own configuration and file set, so its count is the run's count; measured over one Svelte
  file, discover read 33 mutants, the count the release report holds for it (job 114789042867).
- Lines, bytes or the number of tests a file has: rejected because each is a proxy for the mutant
  count, and a proxy that misjudges one large file undoes the deal.
- A stale report's per-file counts, with a guess for new files: rejected because a changed file's
  count changes with its code, and the guess is exactly what the deal must not rest on.
- StrykerJS's `--dryRunOnly`: rejected because it runs the initial test run and writes no report,
  so it costs a shard's set-up and names no per-file count.

D4, how the shards are judged:

- Chosen: one job named `mutation-web`, not a matrix, that needs every shard, runs always, reads
  each shard's record and report by name through `web_mutation_shards.py union`, and hands the
  merged report to the unchanged judge step, because the name the aggregate reads stays reported
  and a missing, cut or partial shard reads VOID by name before any mutant is judged.
- A verdict in each shard: rejected because a matrix leg reports as `mutation-web-shard (shard=<k>)`,
  never under its bare name, and three partial verdicts each judge some mutants while no check
  says the whole sweep was judged.
- Teaching `scripts/mutation-verdict.py` to read N reports: rejected because the judge then
  changes, and the union can hand it one report it already reads; the verdict's code stays
  byte-identical.

D5, the alternatives main refused:

- (B) Raise the bound again: rejected because ADR-390 ruled that a leg cut at 100 is evidence for
  sharding, not for a second raise, and a longer bound holds a hung test's runner longer.
- (C) Fold the web leg into the Python legs' pricing: rejected because those legs price
  `scripts/mutation_python.py`'s mutants and are their own work; the web leg's unit is a StrykerJS
  mutant.
- (D) A narrower `mutate` set, or `ignoreStatic`: rejected because each examines fewer mutants,
  a weakening by ADR-199's first driver.
- (E) Raise StrykerJS's `concurrency`: rejected because more test-runner processes contend for the
  same hosted runner, more mutants reach their per-test timeout, and StrykerJS scores a timed-out
  mutant as detected, so a verdict can move in the permissive direction.

## Decision Outcome

Three shards, dealt by StrykerJS's own count, judged by one job of the leg's name. The projection's
arithmetic, each figure with its job:

- a mutant's cost: 5849.24 s over 7805 mutants, 0.7494 s (job 114789042867);
- a shard's fixed cost: set-up 34 s + initial test run 122 s + tail 6.7 s = 162.7 s (job 114789042867);
- the deal of the release report's per-file counts, 7805 mutants in 106 files (job 114789042867),
  into three shards: 2602, 2602 and 2601 mutants;
- the slowest shard: 162.7 + 0.7494 x 2602 = 2112.6 s = 35.2 minutes (job 114789042867);
- the same deal weighted by tests, 21484 tests at 0.2723 s each, slowest 41.2 minutes (job 114789042867);
- against two shards, 3903 mutants: 162.7 + 0.7494 x 3903 = 3087.6 s = 51.5 minutes (job 114789042867);
- against four shards, 1952 mutants: 162.7 + 0.7494 x 1952 = 1625.5 s = 27.1 minutes (job 114789042867).

### Consequences

- Good: the release's sweep is projected at about a third of each shard's bound, so the leg holds
  the app's growth for longer than a raised bound would, and no mutant is dropped.
- Good: a shard that is missing, cut or partial reads VOID by name, so a sharded sweep can never
  read ok on part of the app.
- Bad: each shard repeats the set-up and StrykerJS's initial test run, 162.7 s three times (job
  114789042867), and a pull request runs two more jobs than before.
- Bad: the deal depends on the mutation server's protocol; a change there makes every shard's
  count read VOID until it is followed.
- Neutral: the judge, its thresholds, the equivalence records and the survivors table read exactly
  as before; only the number of StrykerJS runs behind the one report changes.

### Confirmation

SPEC-419's A1 and A2 read the workflow's shape and plant each defect into a copy of `ci.yml`; A6 to
A13 pin the deal, the count, the empty record and the union over fixed fixtures and a fake mutation
server. Rows `S41900` onward put the shard count back to 1, 2 or 4, raise a shard's bound past 100,
read a missing shard as ok, deal onto the heaviest shard, split a file across shards and leave an
empty shard absent; each is killed by name. `web/app/stryker.config.json`,
`scripts/mutation-verdict.py`, `scripts/mutation-equivalent.d/` and
`.github/workflows/mutation-weekly.yml` are byte-identical to the base.

## What would make this wrong

- A completed shard past two thirds of its bound (66:40): re-measure, and re-derive the count by
  this rule before any raise.
- A run in which a shard's report holds other than its discovered count: the union reads VOID, and
  the count source is re-measured against the run before any shard is trusted.
- A StrykerJS change that ties one mutant's result to another file's mutants: then a file's
  shard would decide its verdict, and whole-file dealing no longer holds.

## More Information

- SPEC-039 R3, amended by a section naming SPEC-419; ADR-057 D3 (whole files on a pull request),
  unchanged and kept; ADR-390 (the bound and its band), whose `mutation-web` bound of 100 stands on
  the verdict job and on each shard.
- ADR-199 (the first driver); #693 (the web sweep's sharding).
