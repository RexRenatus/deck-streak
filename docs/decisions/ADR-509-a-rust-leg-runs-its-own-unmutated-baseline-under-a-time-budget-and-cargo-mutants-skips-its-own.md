---
status: "accepted"
---

# ADR-509: a Rust leg runs its own unmutated baseline under a time budget, and cargo-mutants skips its own

Decides SPEC-480 section 9. It admits `--baseline=skip`, which ADR-057 and ADR-373 refused, only
after the same leg has run its own baseline green, and it bounds that baseline outside the one
`--timeout` ADR-328 reads as bounding both.

## Context and Problem Statement

The release pull request (#822) ran its Rust legs in run 39033655190, whose rust class read VOID.
cargo-mutants has one `--timeout`, which bounds a baseline's tests as well as each mutant's (ADR-328),
and 107 of the run's 192 Rust legs' baselines read Timeout at 2300.0 s to 2300.1 s of test (run 39033655190).
So no mutant of those legs was judged. The other 85 legs were whole, and their largest baseline,
mutation-rust-shard-121 of run 39033655190, took 2562.7 s: build 300.8 s, test 2261.9 s (run 39033655190).
The owner's ruling bounds each leg's own unmutated baseline by a time budget of 3845 s, that
largest whole baseline times SPEC-362 R7's margin 1.5, rounded up, and keeps every mutant at 2300 s.
It refuses a skipped baseline that no leg runs and any change that narrows the gate.

## Decision Drivers

- A baseline over its budget, or red, reads VOID with its reason, never a pass, and no mutant is
  skipped or filtered.
- cargo-mutants offers no baseline-only timeout: its `--timeout` bounds the baseline's tests and
  every mutant's alike, and its baseline build has no bound of its own.
- The release plan deals 304 Python legs against its ceiling of 305: the verdict file refuses the
  whole plan at 17 more listed mutants, and a new `scripts/*.py` adds a leg at its first mutant.
- The plan's baseline charge moves every pull request's projection, and rows S32702 to S32704 pin it.
- A mutant whose own package's tests run past 2300 s reads Timeout, which the rust class scores as
  caught (ADR-328): the residual a baseline budget alone leaves.

## D1, where the bounded baseline runs: what it was chosen against

- Chosen: a step of the `mutation-rust` job before cargo-mutants, because the same leg that judges its mutants then runs `scripts/mutation-baseline.sh` inside the memory scope, under `timeout --kill-after=60 3845`, and records its exit and its log under its own report directory; cargo-mutants runs only after a green record, with `--baseline=skip`, so the leg never trusts a baseline it did not see (ADR-057) and never mutates a build it did not prove green (ADR-373).
- cargo-mutants' own baseline at the budget: rejected because its one `--timeout` would bound every mutant at 3845 s too.
- A raised per-mutant `--timeout`: rejected because it narrows the gate, which the owner's ruling refuses, and it would charge each hanging mutant more of its leg's bound.
- A skipped baseline that no leg runs: rejected because the owner's ruling refuses it: each leg would judge its mutants against a baseline nothing ran.
- The step only on release-marked plans: rejected because it makes two paths for one leg, where the owner's budget is one bound for every run, pull requests into dev included.
- The baseline outside the memory scope: rejected because the scope bounds every test run a leg makes (SPEC-196), and the base's baseline ran inside it.
- A Python runner: rejected because a new `scripts/*.py` takes the release plan's last Python leg at its first mutant; a shell script adds none.
- A deal read from `cargo mutants --list` in the step: rejected because the plan's listing, `listed.json`, is the population the verdict already holds the legs to (SPEC-362 R8), and A14 pins the round-robin deal over it.

## D2, how the verdict reads the baseline: what it was chosen against

- Chosen: `baseline_void` reads the leg's own `baseline/baseline.exit` and `baseline/baseline.log` for every leg the plan deals a mutant, because a missing record, exit 124 and any other non-zero exit each then read VOID by name, and a green record is held to SPEC-362 R7 over its log as before.
- The `Baseline` outcome of cargo-mutants' report: rejected because under `--baseline=skip` the report holds none, so every leg would read VOID.
- The call only for a leg with a whole report: rejected because a leg whose baseline is red runs no mutant and leaves no report, so its reason would be hidden under its missing report.
- A seconds record beside the exit: rejected because exit 124 already carries the budget, and the extra record lists 2848 verdict mutants against this shape's 2845, which deals the release plan's last Python leg.

## D3, the plan's charge: what it was chosen against

- Chosen: the base's charge, `BASELINE_SECONDS` 2141 plus the census term, because over the release listing it deals 192 legs of 192, and with each baseline at its whole budget the slowest leg projects at 10690 s of the 10800 s bound (run 39151539183's plan).
- The budget charged flat, 3845 s: rejected because it deals the same 192 legs (run 39151539183's plan), moves every pull request's projection, and edits rows S32702 to S32704.
- The budget plus the census term, 5366 s: rejected because its 192 legs project 12211 s, past the 10800 s bound, so the release plan would be refused whole (run 39151539183's plan).

## D4, the false-kill residual: what it was chosen against

- Chosen: SPEC-362 R7 stays the guard, and the verdict job prints each package's Timeout count beside its baseline test time, because a package with a Timeout whose baseline tests, times 1.5, pass 2300 s, or that no log times, is then marked READ BY NAME with each Timeout by leg and name, and its reader reads them by name before ruling the run complete, with no Python leg added.
- A per-package guard in the verdict file: rejected because its mutants would refuse the release plan whole past its Python ceiling.
- Counting such Timeouts as anything but caught inside the gate: rejected because it narrows the gate.
- The package's slowest single test as its baseline test time: rejected because R7 already reads VOID every leg whose slowest test passes 2300/1.5 s, so the mark would never fire on a judged leg; the sum of the package's test times is the larger reading, so more packages are read by name.

## D5, the committed derive: what it was chosen against

- Chosen: `tools/rust-baseline-budget/size_budget.py`, because the derive the owner's ruling sized the budget by then runs from the public tree, its code lines byte for byte under a public header, outside the Python mutant population.
- A copy under `scripts/`: rejected because it joins the Python population and adds a release leg at its first mutant.
- A figure re-derived, rounded or sized another way: rejected because the budget is the owner's ruling's figure, and a second method would be a second figure.

## Decision Outcome

D1 to D5 are built by SPEC-480 section 9 in one pull request into dev; its closing run is the next
push-triggered run of #822 after the land, read by name (SPEC-480 section 11).

### Consequences

- A leg runs its baseline once, outside cargo-mutants, and cargo-mutants' first mutant build
  reuses its target directory, since both run in the same environment.
- A leg whose baseline is red, past its budget or unrecorded runs no mutant and reads VOID with its
  reason; the rust class reads VOID while any dealt leg does.
- ADR-057's and ADR-373's refusals still hold for a leg that has not run its own baseline.
- ADR-328's premise still holds of cargo-mutants itself; the baseline's bound now lives outside it.
- ADR-497 left the rust class reading its shards' red baselines VOID; this decision bounds them.
- `scripts/mutation-verdict.py` lists 2845 mutants, one more than at the base, within the 2847 that
  add no Python leg.

### Confirmation

SPEC-480 section 9's criteria in the pull request's checks, its rows S48008 to S48038 read KILLED
by id, and its closing run read by name: each leg's baseline line, the rust class's VOID lines by
reason, and the verdict job's Timeouts-by-package lines.

## What would make this wrong

- A mutant judged in a package its leg's baseline did not test: the deal and cargo-mutants' shards
  disagree.
- A Timeout read by name in the closing run that a test past 2300 s caused, not a hanging mutant:
  the residual ADR-328 names reached a kill.
- A whole baseline of the same tree reaching its 3845 s budget, or a release leg stopped by its
  job's bound with its baseline at its whole budget.

## More Information

Built by SPEC-480 section 9; drawn in section 12 of `docs/schematics/mutation-testing.md`. Refers
#822.
