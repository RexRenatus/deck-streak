---
status: "accepted"
---

# ADR-432: the Python mutation legs are dealt by each file's measured control cost under the run's fixed leg shares, and a listing they cannot hold is refused whole

Decides SPEC-418.

## Context and Problem Statement

A pull request's Python mutants run in the `mutation-python` matrix: each leg is one hosted job
with `timeout-minutes: 60` (`.github/workflows/ci.yml`, the `mutation-python` job), and runs
`mutation_python.py run --plan ... --shard k/n --failfast`. SPEC-415 sizes that matrix by a count
rule, 20 listed mutants a leg, and refuses a listing that needs more than the run's 32 Python legs,
whole and by name. A count rule treats every mutant alike, and they are not alike: the runner
bounds each file's mutants by five times that file's own control run (`BOUND_FACTOR`), so one
file's mutant can cost many times another's. Issue #825 asks for the legs to be sized by a
measured cost, under a budget the Python and Rust legs share, with every listed mutant examined or
the whole plan refused by name, no sampling, no silent cap, and no leg bound above 120 minutes.

What was measured, on the release pull request's cancelled run (its plan job 114529617353, 24
seconds; its Python legs each about 60 minutes):

- The plan listed 5423 Python mutants in 14 files and dealt them into 32 legs of 169 or 170.
- 30 of the 32 Python leg logs were read; two logs are gone. Every one shows exactly 0 lines
  printed by the runner between the start of its run step and the job's end: leg 0
  (job 114529704945) ran 60 minutes 6 seconds and printed nothing, and the four legs lost to a runner
  shutdown (jobs 114529704969 at 33 minutes 45 seconds, 114529704958 at 44 minutes 3 seconds,
  114529704961 at 50 minutes 37 seconds, 114529704919 at 53 minutes 12 seconds) printed nothing.
  Each leg's upload step ran after the cut and found no file.
- The runner (`scripts/mutation_python.py`) times each file's control run and keeps it as
  `files[].control.seconds` in its report, with the file's per-mutant `bound`, but writes the
  report only when the run ends, and prints only its closing counts. A cut leg therefore leaves no
  cost anywhere: not in its log and not in an artifact.
- So the run holds no per-file control second. Its legs bound the cost from below only: a leg of
  170 that ran 3606 seconds in job 114529704945 without finishing costs more than 21.2 seconds a mutant, its files'
  control and sentinel runs included. The tree's one per-mutant figure, about 96.5 seconds (SPEC-362
  R14), came from another diff's files.

## Decision Drivers

- Every listed mutant is examined once, in a leg the verdict counts, or the plan is refused whole
  and by name; nothing is sampled and nothing is capped.
- A leg's projection stays inside its own job by the margin the Rust legs keep: half the job.
- No bound loosens in this delivery: not a job's `timeout-minutes`, not a per-mutant bound, not a
  leg ceiling. A figure that would loosen one is put to the maintainer as a question.
- A price is a measurement with its source, never a guess and never a lower bound.
- The workflows stay byte-identical, so the delivery moves no job, no matrix and no bound.

## Considered Options (the alternatives each was chosen against)

D1, the cost source. Chosen against:

- Chosen: a per-file price table in `scripts/mutation-verdict.py`, `PYTHON_CONTROL_SECONDS`,
  shipped empty, filled only from a recorded run, because the cancelled run holds no per-file
  control second (30 logs read, 0 runner lines), so no figure in it can be named with its job.
- Pricing each file from the cancelled run's legs: rejected because no leg printed or uploaded a
  control second; the ruling's source carries no figure, and a table that names one would be false.
- Pricing from the legs' lower bound, 21.2 seconds a mutant: rejected because a lower bound
  under-prices, and a leg dealt by an under-price runs past its job, which is the defect itself.
- Pricing from the tree's 96.5-second figure: rejected because it was measured on another diff's
  files, and each file's cost differs by its own control run.
- Running each file's control in the plan job to price it live: rejected because the plan job's
  15-minute bound would have to grow to hold 14 controls, which loosens a bound, and the plan
  would run tests it does not judge.

D1, how a later run refreshes it. Chosen against:

- Chosen: the runner appends one line per file's control, and one per mutant, to `costs.jsonl`
  beside its report as each happens, flushed, and a `prices` verb derives the table's entries from
  any run's recorded lines, because the legs' upload step already runs after a cut and keeps every
  file in the report's directory, so a cut leg still leaves its costs.
- Printing each control on the runner's standard output: rejected because the runner's stdout is
  pinned byte for byte by its tests (the `examined N` close), and a log line can only be read
  back by hand from a job log.
- Writing the report itself after every file: rejected because a partial report is a new state
  every verdict reader would have to refuse, and those readers are not this delivery's to change.
- Hand-typing the table from a run's report: rejected because a typed figure has no derivation a
  reviewer can repeat; the verb's output is the table text, each entry with its run and shard.

D2, the deal. Chosen against:

- Chosen: a priced file's mutants are cut into runs that each fit one leg with the file's control
  and sentinel, and the runs are dealt heaviest first onto the lightest leg, in the fewest legs
  whose slowest projection is within `PYTHON_LEG_BOUND_SECONDS` (1800, half the job's 60 minutes),
  because a file's control and sentinel are paid once per leg that holds it, and grouping by file
  pays them least.
- Round robin with a cost per mutant: rejected because every leg then holds every file and pays
  all of their controls and sentinels, 14 files' worth at the release, in every leg.
- One leg per file: rejected because the release's largest file lists 2488 mutants, which no
  single leg holds, and small files would each take a whole leg.
- A bound of the whole job, 3600 seconds: rejected because a projection is a price, not a
  measurement of this tree; the Rust legs keep half their job for the same reason.

D2, the joint budget. Chosen against:

- Chosen: the run's 225 leg slots (256 jobs less the 31 others) shared in fixed shares, 32 Python
  and 193 Rust, which bounds the run at 32 x 30 + 193 x 180 = 35700 projected runner-minutes, and
  the plan prints the joint line, because fixed shares move no ceiling: `LEG_CEILING` re-derives
  to `{"ci": 193, "battery": 234}`, unchanged, and no test or row moves.
- A floating split, the Python legs taking the slots the Rust plan leaves: rejected for this
  delivery because it raises the Python ceiling above 32, a loosening put to the maintainer as a
  question, and at the release it adds 17 legs, 49 x 1800 = 88200 seconds, still short.
- A budget in runner-minutes alone: rejected because the run's binding limit is its job count,
  not its minutes, and a minutes budget would admit more legs than the run can start.

D2, the fail-closed arm. Chosen against:

- Chosen: the plan refuses whole, by name, exiting 1 before any Rust leg is dealt, when a file's
  one-mutant run (control, sentinel and one mutant) projects past 1800 seconds, or when the priced
  and unpriced legs together need more than 32, because SPEC-415's refusal is the house's answer
  to a listing the run cannot hold.
- Capping at 32 legs and letting each leg run long: rejected because that is the defect.
- Dropping or sampling the dearest mutants: rejected because every listed mutant is examined.

D2, unpriced files. Chosen against:

- Chosen: a file with no price keeps SPEC-415's rule, 20 a leg, in legs of its own beside the
  priced legs, and a listing with no priced file is SPEC-415's plan byte for byte, because until a
  run records its control a file has no cost but the count rule's.
- Pricing an unpriced file at a default figure: rejected because a default is a guess.

D2, which names a leg runs. Chosen against:

- Chosen: when the plan carries a Python matrix, each leg runs exactly the names the plan dealt
  it, and refuses to mutate anything unless the plan's legs hold its own listing exactly once each
  and the plan's leg count is the count it was told, because a deal by cost is not the runner's
  `k::n`, and the runner's own listing is the independent check that nothing was dropped.
- The runner keeping its own `k::n`: rejected because each leg's report would then differ from
  the plan's list, and the verdict reads that as drift.
- A population check in the verdict instead: rejected because a leg that ran a short list has
  already spent its job; refusing before the first mutant costs nothing, and a refusing leg writes
  no report, which the verdict already reads VOID by name.

D3, the release listing. Chosen against:

- Chosen: say plainly that the release's 5423 listed mutants do not fit, because 32 legs of 1800
  seconds hold 57600 seconds, about 10.6 seconds a mutant, and the run measured more than 21.2
  (5423 x 21.2 = 114968 seconds, 64 legs at the bound); with no file priced the plan stays on
  SPEC-415's rule, which refuses it at 272 legs, so the re-run runs once and `mutation-python`
  reads VOID by name ("VOID the plan names no python shards", SPEC-087 R11).
- Claiming a fit at the job's whole 60 minutes: rejected because 32 x 3600 = 115200 seconds is
  within 0.2% of a strict lower bound, which proves no fit.
- Claiming a fit under a raised bound: rejected because a raise is a question, not this design's;
  even half of a 120-minute job, 3600 seconds a leg, holds the listing only at the lower bound, and
  at the 96.5-second figure it needs 146 legs.

D4, loosening figures. Chosen against:

- Chosen: none is decided; each is a question with its measurement and the alternative that needs
  none, because the issue admits no bound above 120 minutes and no loosening without the
  maintainer's answer.
- Raising the Python job to 120 minutes here: rejected because it lengthens a leg instead of
  sizing it, and the measured release does not fit even then.

D5, the tests. Chosen against:

- Chosen: one red-first test per criterion, structural (fixture listings and fixture prices as
  literals, the runner's `--control-seconds` fixing each recorded control figure), read red and
  green in CI with two pushes, the red commit alone with its draft pull request and the green and
  record commits after, because the modules that read the workflows run only in CI.
- A wall-clock assertion on a leg: rejected because it measures the machine; the deal is a pure
  function of the listing and the prices.
- Running the modules on a workstation to read the reds before a push: rejected because the
  workflow-reading modules run in CI only, and a third push is not admitted.

D6, the shape of the record. Chosen against:

- Chosen: one delivery under SPEC-418 and this decision, with insert-only amendment sections in
  SPEC-087, SPEC-362, ADR-373 and the Python mutation schematic, and a paragraph in the release
  runbook, because the deal is a new requirement on the plan and the runner, not a rewrite of
  either record.
- Rewriting SPEC-415's requirements: rejected because SPEC-415's count rule stays the rule for
  every unpriced file and for a plan with no price at all.

D7, the formal cover. Chosen against:

- Chosen: re-read `formal/tla/EveryLegCounted` against the cost deal and the runner's partition
  refusal, add covers for the deal and the runner's leg selection, and move the `shards` cover's
  digest, because a deal by cost is a new deal of legs the model's claims govern (every dealt leg
  counted, each listed mutant examined exactly once, a pass only when every leg reported whole).
- Leaving the Python legs outside the model: rejected because ADR-429 left their model to the
  delivery that adds a deal, and this one does.
- A separate entry for the Python legs: rejected because it would restate the same three claims
  over the same verdict, and an entry cannot extend another entry's module.

## Decision Outcome

Chosen: the Python legs are dealt by each file's measured control cost, under fixed leg shares, with
SPEC-415's refusal as the fail-closed arm:

- `PYTHON_LEG_BOUND_SECONDS = 1800`, half the `mutation-python` job's 60 minutes, pinned against
  the workflow by a test, as `SHARD_BOUND_SECONDS` is for the Rust legs;
- `PYTHON_CONTROL_SECONDS`, a per-file price in whole seconds, shipped empty, each later entry
  naming the run, attempt and shard it came from;
- a priced file's leg projects `(2 + m) x price` for its m mutants there: its control, its
  sentinel and each mutant at the control's cost;
- the deal, the refusals, the unpriced legs and the joint line as D2 states them;
- each Python leg runs its plan's names, after checking the plan's legs against its own listing;
- the runner records each control and each mutant's seconds in `costs.jsonl` as they happen;
- `mutation-verdict.py prices` derives the table's entries from recorded runs;
- `ci.yml`, `mutation-weekly.yml`, `LEG_CEILING`, `PYTHON_MAX_SHARDS`, `PYTHON_SHARD_MUTANTS` and
  every per-mutant bound are unchanged.

### Consequences

- Good: once a file is priced, its mutants are dealt by what they cost, a leg pays each of its
  files' control once, and a listing the run cannot hold is refused by name instead of cut.
- Good: a leg cut by its job, or lost to a runner shutdown, still leaves its files' costs, so a
  failed run measures the next plan.
- Bad: this delivery prices nothing, so the plan behaves as SPEC-415's until a pricing delivery
  commits figures; the release measured here is refused either way.
- Bad: the release cannot be judged in one run under any shape that loosens nothing; its range
  must shrink, or its tests grow faster, or the maintainer answers a question.
- Neutral: the joint line is derived from the shares; minutes do not bind, slots do.

### Confirmation

The acceptance criteria of SPEC-418, each red first in CI. On the first run after a pricing
delivery, the plan prints its cost line and its joint line, and each leg's `costs.jsonl` is in its
artifact.

## What would make this wrong

- A control run that is not a fair price for a file's mutants: if the recorded mutant seconds of a
  file run well above its control, the `prices` verb's dearer reading takes them, and a file whose
  mutants time out at five times the control needs a larger factor, measured, not guessed.
- A per-run limit that is lifted or counted per matrix: then the shares are conservative and the
  floating split is worth asking again.
- A runner that cannot flush before a cut: then the cut legs leave no line, and the cost source is
  the completed legs' reports only.

## More Information

- SPEC-415 and ADR-429 (the count rule and its whole refusal), SPEC-362 and ADR-373 (one run, legs
  sized to the hosted job, the leg ceiling), SPEC-087 (the Python runner and its plan legs).
- Issue #825 tracks the Python legs' fit; #822 is the release whose run measured it.

## Amendment: the price table's first fill, its lower bounds and its refresh (SPEC-420, ADR-434)

Insert-only; every earlier byte is kept. SPEC-420 fills `PYTHON_CONTROL_SECONDS` for the first time
from the cost records of battery run 38242806596. Each of that run's sixteen Python legs ran to the
job's 120-minute bound: two failed, and fourteen were cancelled. The amendment keeps D2's count rule
for every unpriced file, and every decision above.

**What it fills.** Exactly the seven lines `mutation-verdict.py prices --costs <dir>` prints over the
twelve `costs.jsonl` files the run kept. Legs 0, 5, 6 and 14 kept none. The verb reads 1017
records, none torn. The records are committed as a test fixture, and a test re-derives the table
from them byte for byte, so no figure is typed.

**Lower bounds, disclosed.** Every figure is a lower bound of its file's control cost, for three
reasons:
- twelve of the sixteen legs reported;
- every leg stopped at its bound;
- a leg's largest control is the largest that leg reached, not the largest the file has.

The largest control recorded sets `mutation-verdict.py`'s 224: 223.66 s, in leg 13 (job
114789331343, cancelled at its 120-minute bound). Two disclosures of the measurement behind
SPEC-420's scoped test set bind as well:
- `mutation_rows.py`'s and `public-scrub.py`'s prices are lower bounds, because their held modules
  were not measured;
- `mutation_python.py` had no control in CI until SPEC-420 added one.

These seven figures depart from D2's driver that a price is never a lower bound. They are admitted
because the only alternative is to leave each file unpriced. Unpriced, a 224 s mutant is dealt
twenty to a leg, and the release reads 284 legs. Priced, it is dealt six to a leg, and the release
reads 557. An under-priced leg overruns its job and reads VOID by name, never a pass.

**Unpriced files.** A listed file the table does not price keeps the count rule of 20 a leg inside
the share (D2, unchanged). That covers:
- the eight release files no recorded leg reached (2272 of the 5670 mutants);
- the files of the four absent legs;
- any file the population gains later.

No file is priced at a default figure.

**Refresh.** SPEC-420's runner records every file's control and sentinel before any mutant of its
leg. So a later run's legs leave a control record for every file they hold, even when they are cut.
A later delivery refreshes the table by running the verb over that run's downloaded records, and
commits the output and the records together in one commit. The same test re-derives the table, and
each line keeps naming its run, attempt and shard.

Chosen against:
- (A) Battery substitution, reading the scheduled battery's Python results in place of the pull
  request run's legs: rejected because the battery runs from main, and its legs are cut at their
  bound (this run examined a fraction of the listing), so substituting it would pass mutants that
  nobody examined.
- A range split of the release into several runs: rejected because the base is dev. The release
  pull request is one range, main...dev. A split into sub-ranges examines no mutant the one range
  does not, and every sub-range run is a run that no release pull request names.
- An owner-signed leg-time raise: rejected because this delivery does not need it. It prices and
  scopes within the 1800 s legs, and a raise stays the owner's lever alone.
