---
status: "accepted"
---

# ADR-429: the Python mutation plan refuses a listing past its 32 legs whole, by name, and never caps it

Decides SPEC-415.

## Context and Problem Statement

A pull request's Python mutants run in the `mutation-python` matrix: each leg is one hosted job
with `timeout-minutes: 60`, and runs `mutation_python.py run --plan ... --shard k/n --failfast`
over its share of the diff's listing. `scripts/mutation-verdict.py`'s `python_shards` sizes that
matrix at 20 listed mutants a leg (SPEC-362 R14) and then clamps the count to 32
(`count = min(PYTHON_MAX_SHARDS, max(1, -(-len(listing) // PYTHON_SHARD_MUTANTS)))`). Below 641
listed mutants the clamp never binds. Above it, the clamp is a silent cap: the legs keep their
count of 32 and each takes more mutants than the 60-minute job was sized for, and the plan prints
no refusal. The Rust plan sizes its legs from a cost model and refuses whole, by name, above the
run's ceiling ("never capped", `fewest_shards`). The Python plan does neither.

The release pull request measured it. Its plan listed 5423 Python mutants and printed
`mutation: shards: 32 python shard(s) for 5423 listed mutant(s)`
(plan job 114529617353, which ran 24 seconds), so each leg held 169 or 170 mutants where the
rule holds 20. Every one of the 32
legs ended without a report: 28 were cancelled at the job's 60-minute bound (shard 0, job
114529704945, its run step 60 minutes 6 seconds; shard 14, job 114529704978, 60 minutes 8
seconds), and 4 failed with exit 143, "The runner has received a shutdown signal", after 33 to 54
minutes (shard 28, job 114529704969, 33 minutes 45 seconds; shard 27, job 114529704958, 44
minutes 3 seconds; shard 31, job 114529704961, 50 minutes 37 seconds; shard 21, job
114529704919, 53 minutes 12 seconds). No leg uploaded a report ("No files were found"), and the
runner writes its report only at the end of its run, so the verdict can only read every leg VOID.
Up to an hour of each of 32 legs bought no verdict.

## Decision Drivers

- No mutant is sampled, filtered, dropped or given a raised per-mutant timeout, and no verdict
  moves in the permissive direction (ADR-199's first driver, read as ADR-390 D3 reads it).
- A run's 256 jobs bound one run (ADR-373); `LEG_CEILING` and every row that pins it hold today.
- A bound is raised only by measurement, by a stated rule, inside a band (ADR-390).
- A run that cannot be judged is refused before it starts, by name, as the Rust plan refuses.
- The verdict's reading of reports is not touched: a missing or cut leg still reads VOID.

## Considered Options (the alternatives each was chosen against)

D1, where the shortfall is. Chosen against:

- Chosen: the shortfall is 272 legs needed against 32 run, because at the rule's 20 a leg the
  5423 listed mutants need `ceil(5423 / 20)` = 272 legs, and the clamp ran 32 of 169 or 170 each.
- A measured per-mutant cost for these 5423 mutants: rejected because no record holds one; the
  runner prints nothing per mutant and writes its report only at its end, so the cut legs left no
  cost, and the scheduled battery's Python leg was skipped in every run read, so none was measured
  there either.
- A cost guessed from the cut legs: rejected because they bound it only from below (a leg of 170
  did not finish in 3606 seconds, so more than 21.2 seconds a mutant, its per-file control and
  sentinel runs included), which sizes nothing.
- The tree's one per-mutant figure, about 96.5 seconds (34 mutants in 54 minutes 42 seconds, in
  SPEC-362 R14), as this listing's cost: rejected because it comes from another diff's files, and
  each file's bound is five times its own control run's time, so the cost varies by file.

D2, the shape. Chosen against:

- Chosen: the Python plan refuses whole above 32 legs, by name, and never caps, because it is the
  Rust plan's own rule (SPEC-362 R3) applied to the Python legs and it needs no figure nobody has
  measured; 20 a leg, the 60-minute job, `--failfast`, the round-robin deal, `LEG_CEILING` and
  the weekly 16 legs all stay as they are.
- Sizing the Python legs by measured per-mutant cost, as `fewest_shards` sizes the Rust legs:
  rejected for this delivery because no per-file cost is measured (D1); it stays the follow-up
  the issue tracks, priced from the per-file control seconds a completed leg's report records.
- Packing more mutants into each of the 32 legs up to the job's own 60 minutes before refusing:
  rejected because the only figure that would set the packing is the 96.5 seconds of D1, from
  another diff, and 37 a leg at that figure leaves 30 seconds of the hour for setup.
- Raising the Python job's `timeout-minutes`: rejected because the precedent's band reaches 120
  minutes (ADR-390 D2) and no shape fits this listing at 120 (half the job, 37 a leg, needs 147
  legs; the whole job, 74 a leg, needs 74; the run has 49 jobs to spare beside 192 Rust legs);
  a bound past that band is a signed ruling's to make, not this decision's.
- A joint budget, where the Python and Rust legs share the 241 jobs the run's other 15 jobs leave:
  rejected for this delivery because it re-derives `LEG_CEILING` and every row and test that pins
  209 and 234, and at 20 a leg it still holds 980 of the 5423 mutants beside 192 Rust legs.
- Keeping the cap: rejected because it spent 32 legs of up to an hour each on a run that read
  VOID, and gives no sign in the plan that the run cannot be judged.
- Sampling or filtering the listing to fit 32 legs: rejected because it drops mutants, a weakening.

D3, the proof of no weakening. Chosen against:

- Chosen: the proof is structural, because the only code that changes is the sizing and the plan's
  exit: `python_reports`, `read_python_shard`, `shard_listing_drift`, `judge_python`,
  `mutation_python.py`, `ci.yml`, `mutation-weekly.yml`, every bound and every timeout are
  byte-identical, and a refused plan writes a plan file byte-identical to one never sized, which
  the verdict already reads as "VOID the plan names no python shards".
- A run-time proof that re-runs the release: rejected because the release's run cannot be judged
  under any shape (D2), and a refusal is proven by its plan line, which a fixture listing produces.
- Changing `--failfast`: rejected because it decides only when a killed mutant's run stops, never
  whether a mutant is examined, and no measurement here bears on it.

D4, the tests and the record. Chosen against:

- Chosen: red-first tests over fixture listings, read red and green only in CI, in two pushes (the
  red commit alone, then the fix and the record), with the red-first record in the third commit,
  because no commit can hold its own sha, every red is observed only in the first push's run, and
  the house shape (SPEC-379) records it so.
- A wall-clock test of a leg's duration: rejected because it measures the machine, and the refusal
  is a pure function of the listing's length.
- Running the test modules on the box to read the reds before a push: rejected because the
  modules that read the plan run in CI only, and a third push to read them is not admitted.
- The record in the red commit with the expected failures: rejected because a record quotes the
  failure observed, and the red commit's failure is observed only in its own run.

D5, the shape of the record. Chosen against:

- Chosen: one delivery under SPEC-415 and this decision, with insert-only amendments to SPEC-087,
  SPEC-362, ADR-373 and the Python mutation schematic, because the refusal is a new requirement on
  the plan's exit, not a change to SPEC-087's runner or SPEC-362's one-run rule.
- An insert-only amendment of SPEC-362 alone: rejected because SPEC-362's R14 already amends
  SPEC-087 R9, and a third layer of amendment on one sizing rule hides it from the reader of either.
- Retiring SPEC-362 A12 in SPEC-056 R14's form: rejected because R14 retires a criterion whose test
  is removed, and A12's test stays, with every case: only its last case, 32 x 20 + 1, now reads
  refused, which SPEC-362's amendment states.

D6, the formal cover of `shards`. Chosen against:

- Chosen: re-read `formal/tla/EveryLegCounted` against the new refusal arm, name the arm among the
  model's abstractions, and move the `shards` cover's digest, with no property, action, constant or
  configuration changed, because the arm returns before any leg is dealt, as the Rust refusal in the
  same span already does, so it moves none of the model's variables and adds no pass.
- Leaving the cover stale: rejected because a stale cover reads on every property of the entry,
  and the re-read is done here.
- Modelling the Python legs in the entry: rejected because the model's claims are about legs the
  plan deals and reports the verdict reads, and the Python plan deals none it did not before; the
  Python legs' own model is the follow-up's, if sizing by cost adds a deal.

## Decision Outcome

- `python_needed(n)` is `max(1, ceil(n / 20))`, the legs a listing of n mutants needs at
  `PYTHON_SHARD_MUTANTS`. `python_shards` returns the matrix, dealt as today, when that need is at
  most `PYTHON_MAX_SHARDS` (32), and `None` above it.
- `shards --python-listed` over a refused listing prints `mutation: shards: REFUSED: <n> python
  mutant(s) need <legs> legs of at most 20, more than the pull request run's 32 python legs: split
  the change, since a run is never capped`, writes no step output and no plan change, and exits 1
  (`EXIT_FAIL`), before any Rust leg is dealt. The plan job fails; no Python or Rust leg starts;
  the verdict reads the run VOID; `ci` fails.
- A sized plan's line gains its headroom: `mutation: shards: <c> python shard(s) for <n> listed
  mutant(s); python legs <c> of ceiling 32`.
- The Python and Rust legs keep fixed shares of the run: Python at most 32, Rust at most 209
  (`LEG_CEILING["ci"]`, derived with Python counted at 32), beside 15 other jobs.
- The release measured here fits no shape: it is refused at its plan until the range is split or
  the Python legs are sized by a measured cost.

### Consequences

- Good: a listing the run cannot judge is refused in seconds at its plan, by name, instead of
  after 32 legs and an hour.
- Good: no verdict, bound, timeout or listing changes, so every pass the code reached before it
  still reaches, and nothing it failed now passes.
- Bad: a listing of 641 or more is refused where the cap ran it. A listing a little past 640 that
  the cap ran within the hour is refused too, until the Python legs are sized by cost.
- Bad: the release measured here stays refused until its range is split or the follow-up lands.
- Neutral: a listing that both the Python and Rust plans would refuse prints only the Python
  refusal, which comes first; the Rust headroom is read again after the range is split.

### Confirmation

SPEC-415's A1 to A4, red in the first push's run and green in the second's; the rows of band
S41500; the formal check of `tla/EveryLegCounted` reading no `STALE` on the `shards` cover.

## What would make this wrong

- A measured per-file Python cost: then the Python legs can be sized by it within the bound and
  the refusal moves to the run's real ceiling (the follow-up).
- A leg of 20 or fewer mutants cut at its 60-minute bound: then 20 a leg is no longer inside the
  job, and the rule is re-derived from the measured cost.
- A Python plan that deals legs the verdict does not count: then the model's claims are owed for
  the Python legs too.

## More Information

- SPEC-087 R9 and SPEC-362 R14 (the 20-a-leg rule this keeps), SPEC-362 R3 and ADR-373 (the run's
  ceiling and the Rust refusal this mirrors), ADR-390 (a bound raised only by measurement, and a
  job's `timeout-minutes` read as no per-mutant timeout), each named by an amendment of its own.
- The Python mutation schematic, `docs/schematics/mutation-testing-python.md`, amended insert-only.
