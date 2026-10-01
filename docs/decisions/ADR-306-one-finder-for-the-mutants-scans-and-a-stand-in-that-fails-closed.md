---
status: accepted
date: "2026-10-01"
decision-makers: "the DeckStreak architect"
---

# One finder serves both mutants scans, and a stand-in that cannot plant its wrapper fails closed

## Context and Problem Statement

`test_mutation_workflows.py` found `cargo mutants` commands with a line pattern that needs a blank
after `mutants` and a job filter that needs the literal text `cargo mutants`. The dispatch-shard
guard of SPEC-129 section 8 finds more spellings: a toolchain selector, the hyphenated binary,
repeated blanks, a cargo option before the subcommand, and a command at the end of a line. A job
that spelled the command one of those ways escaped the bounds, the nextest-install and the
yaml-suffix assertions of the module (#418).

Separately, the stand-in the guard runs in place of the interpreter planted the wrapper and, when
planting raised, executed the real program with the words unchanged. A failed plant then read as a
passing run (#497).

Where should the one reader of the command live, and what should a stand-in do when its plant
fails?

## Decision Drivers

- A reader of an open grammar, written twice, drifts: the two scans had already drifted.
- `test_dispatch_shards.py` and `test_memory_scope.py` never run on the box, so the module that
  must stay locally runnable cannot import either of them.
- A fix to the finder must not change what it finds; the move has to be provable by a diff.
- A failure to plant is a failure of the test double, and must not pass for a measurement.

## Considered Options (the alternatives it was chosen against)

- Move the finder into one support module that both test modules import: chosen, because one definition cannot drift and a census of definitions can hold it to one.
- A second copy of the finder in `test_mutation_workflows.py`: lost, because it is what drifted before, and nothing would tie the two copies together.
- `test_mutation_workflows.py` imports `test_dispatch_shards.py` for the finder: lost, because it would put the locally runnable module under the ban on running that one and its wrapper.
- Import the real wrapper's option parser for the workflow tests' recognizer of wrapped commands: lost, because the support module would then read a file under `scripts/` by computation and the workflow tests would depend on the wrapper.
- A module attribute that each importer sets to its own recognizer: lost, because one process loading both modules would let the last importer's choice reach the other.
- Fall back to running the real program when the plant fails: lost, because the failure is then read as the program's own run and nothing names it.
- Exit non-zero naming the failure, and run nothing: chosen, because the failure is then the test's result.

## Decision Outcome

Chosen options: the finder moves, unchanged, to `scripts/tests/_mutants_finder.py` (a flat module,
because a package named `_support` would shadow `_support.py`), and the stand-in exits when its plant
fails.

- The finder's wrapper recognizer is an explicit parameter, `wrapped`, threaded through
  `mutants_of` and `mutants_in` and defaulting to a recognizer that sees no wrapper.
  `test_dispatch_shards.py` passes its own, which reads the wrapper's file.
  `test_mutation_workflows.py` passes one that names no wrapper and reads no file: for a command
  whose program is not cargo, the command is the words after its first standalone `--`.
- The only lines of the moved text that differ from the originals are the signatures and the call
  sites that pass `wrapped` on. A diff of the extracted text shows every other line byte-equal.
- Measured at the base over the six workflow files: the old scans found 9 commands and 6 jobs, the
  new ones find 9 and 6, and lose none. The null recognizer alone finds 5 and refuses 3 wrapped
  commands, which is why `test_mutation_workflows.py` passes its own.
- The stand-in prints `plant_wrapper failed: <the exception>` to standard error and exits non-zero.
  A text-only census lists every stand-in under `scripts/tests/` that falls back to `exec`,
  `subprocess` or `runpy` of a real program on a failed plant.

### Consequences

- Good, because both scans find every spelling the guard finds, and a planted copy of the finder is
  caught by a census of definitions.
- Good, because the failed plant is a visible failure, with no program run after it.
- Bad, because the support module holds one literal mention of the wrapper's file name, moved with
  the text it belongs to, so a search for that name lists it.
- Bad, because the workflow tests' recognizer is a second, narrower reading of what a wrapper is.

### Confirmation

`EveryMutantsSpellingIsFound` in `test_mutation_workflows.py`, the census of stand-ins in
`test_stand_in_census.py`, and, in CI only, `test_a_failed_plant_exits_non_zero_names_the_failure_and_runs_no_words`.

## What would make this wrong

A workflow command shaped so that the narrower recognizer reads it differently from the guard's own
one would make the two scans disagree again. A population test that generates such commands would
show it; the base population (9 commands) shows none.

## More Information

Issues #418 and #497; SPEC-129 sections 10 and 11.
