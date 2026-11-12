---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# The repository's Python is mutated by a runner of its own, restored by digest

## Context and Problem Statement

SPEC-039 generates mutants for the Rust crates and the Mini App, and none for Python. The gate's
own guard scripts (`scripts/*.py`: the public scrub, the web audit's verdict, the mutation runner
and verdict, and the issue writers) decide verdicts, and nothing proves their tests can fail on a
wrong guard beyond the rows a SPEC writes by hand (#218). The parity oracle's generator
(`tools/parity-oracle/generate.py`) is production code whose invariants are rows only, because
mutmut 3.8.0 did not fit it (SPEC-039 §1, ADR-057 D1), and ADR-057 left cosmic-ray unmeasured for
#219 to measure. Which tool generates the mutants of both, and how does each run keep SPEC-039's
rules: a run that examined nothing is VOID, never green; a mutant that does not build or parse is
never a kill; and the tree is restored byte for byte?

## Decision Drivers

- **The VOID rule holds without a reader's guesswork.** A run that examined nothing, a mutant whose
  tests hung, and a test set that selected nothing must each read VOID by name, from the tool's own
  report.
- **The tests load the scripts as they are.** The guard tests load a script by its path under a
  module name of their own (`importlib.util.spec_from_file_location`), import it by name from
  `scripts/`, or run it as a child `python3`; the oracle's tests load the generator by path as
  `parity_generate`. A tool that needs another layout would change the code it measures.
- **Diff scope.** A pull request mutates the lines it changed, as cargo-mutants' `--in-diff` does.
- **The killer is known.** #220's killer map needs, per mutant, the tests that killed it.
- **No vacuous kill.** A test that fails on any change to the file's bytes proves nothing about a
  mutant, and must not count as its killer.
- **One reader.** `scripts/mutation-verdict.py` judges every class (ADR-057, ADR-070).

## Considered Options (the alternatives it was chosen against)

Measured on `dev` 26263de, in a scratch export of the tree, on four targets that span the shapes:
`scripts/public-scrub.py` (loaded by path, and run as a child), `scripts/audit-web-verdict.py` (a
command with `main()`, run only as a child), `scripts/mutation_rows.py` (imported by name; its
census reads its own text), and `tools/parity-oracle/generate.py` with its tests. SPEC-087 §1
tables every count.

**D1, the tool.**

- A runner of the repository's own, `scripts/mutation_python.py`: chosen, because it is the one
  option that met every driver on every target. It generates mutants from Python's `ast` with a
  fixed operator set, installs each in place and reuses `tracked_changes` and `sha256` from
  `scripts/mutation_rows.py` and the same restore-then-compare pattern as its `prove_row` (the
  refusal of a dirty tree, the parse check and the restore checked by sha256),
  and runs the file's tests in a child of its own that prints every failing test's id. Its
  prototype ran on all four targets, restored every byte (sha256 equal before and after, the tree
  clean), reported each hung or unparsable mutant apart from a kill, and named the killers of every
  killed mutant, down to the sub-test. It needs no dependency: the scripts and the gate stay
  standard-library Python.
- mutmut 3.8.0 (the current release): rejected by measurement, because it ran on none of the four
  targets as they stand. It stopped on the scrub and on the oracle, whose tests load the file under
  a module name that is not the file's path; its clean run failed on the web audit's verdict,
  whose child `python3` imports mutmut's rewritten copy and cannot import mutmut; and its clean
  run failed on `mutation_rows.py`, whose rewritten copy repeats every function, so the rows census
  found an anchor 31 times where it must occur once. With the scrub's test changed to load the file
  under mutmut's name, it reported 592 of 684 mutants "no tests" and exited 0, the vacuous green
  SPEC-039 §1 found on the oracle (415 of 415). It mutates a copy, so a test that reads the file's
  bytes fails its clean run; it scopes by file, never by line; and it records no killer.
- cosmic-ray 8.7.0: rejected by measurement, because its own verdict breaks the VOID rule twice. A
  mutant whose tests hang is recorded `killed` (12 of the 23 mutants of a looping fixture, each with
  the output `timeout`). A diff filter that leaves nothing to run reports `complete: 23 (100.00%)`,
  `surviving mutants: 0 (0.00%)`, a survival rate of 0.00 and exit 0, so a run that examined nothing
  reads green; only its session database says `skipped`, and its survival rate divides by every job,
  the skipped included (26 survivors of 60 run on #288's diff read 2.35%). Its run exits 0 with
  survivors too (30 on the web audit's verdict, 141 on the scrub), so every verdict would be
  re-derived from its database by a reader of our own. It does mutate in place and restore (sha256
  equal after every run), and `cr-filter-git` scopes to a diff by line; but it records no killer,
  only the test command's output, and its operators mutate type annotations, which never run under
  `from __future__ import annotations` (every target holds it): 22 of its 30 survivors on the web
  audit's verdict were such, as were 22 of its 26 on #288's diff, and the population holds 53
  operators inside annotations, eleven mutants each. Skipping them takes a comment in the source,
  the kind of exclusion SPEC-057 R11 refuses.
- No generated mutants, rows only: rejected, because a row is one mutant a person chose. Over the
  diff of the shell parse check (#288), merged with its own rows, the prototype found 32 mutants on
  the changed lines, 4 of which no test of `test_mutation_rows.py` kills.

**D2, which tests a mutant runs.**

- A committed map with a census: chosen, because a reviewer reads it, and the census refuses a file
  of the population the map omits, a module that does not exist, a file the map names outside the
  population, and an entry with any key but the test directory and modules. The map,
  `scripts/mutation-python.json`, names each population file's test modules. A file whose list is
  empty has no test, and its mutants read `uncovered`: examined, and unexplained, as StrykerJS's
  no-coverage mutants are (SPEC-039 R4).
- Every module under `scripts/tests`: rejected, because some of them build Rust
  (`test_engine_pin.py`), and every mutant would pay for all of them.
- The modules whose text names the file: rejected, because naming is not exercising. The scrub's
  name occurs in six test modules, and the oracle's in `test_mutation_verdict.py`, which never
  runs it.

**D3, a test that reads the file's bytes.**

- A sentinel before the mutants: chosen. The runner appends one comment line to the file, which
  changes its bytes and never its behaviour, and runs the file's tests; each test that fails reads
  the bytes, and is named and left out of that file's mutants. On the oracle it named exactly the
  two tests of `test_goldens.py` that compare the committed goldens' `generator_sha256` with the
  generator's digest, and on the three scripts it named none.
- Count their kills: rejected, because they kill every mutant. With them the oracle read 165 of
  165 killed (the prototype) and 279 of 279 (cosmic-ray); with `test_generate.py` alone, 26 and 52
  survived, and each of the prototype's 26 was killed by those two tests and by no other.
- Leave `test_goldens.py` out by name: rejected, because a name is a list someone must keep; the
  sentinel finds the next such test by what it does.

**D4, a mutant whose tests do not finish.**

- VOID by name, never a kill: chosen, because it is the rows' rule (a row whose killer outlives
  its bound reads VOID, "it timed out") and SPEC-039 R4's for a partial report: a measurement that
  did not finish proves nothing. A mutant that hangs is killed by a test with a bounded wait of its
  own.
- A kill: rejected, because a hang from an unrelated cause, a slow runner or a test that waits on
  the network, would then read as proof. It is how cosmic-ray and cargo-mutants read a hung
  mutant, and how SPEC-039 R4 counts a Rust timeout. The cost is small and bounded: a Python test
  can bound its own wait (a child with a timeout, or an alarm), and none of the prototype's runs on
  the four targets met a hung mutant. The Rust class keeps SPEC-039 R4's reading; this decision is
  the Python runner's.

**D5, which killers a run records.**

- All of them weekly, the first on a pull request: chosen, because the verdict needs one killer,
  the pull request's wait is the cost, and the killer map (#220) reads the battery's full record.
  The weekly battery records every failing test, and a pull request runs with `--failfast`.
- Every failing test on every run: rejected, because a killed mutant would pay for its whole test
  set on each pull request, which one killer already decides.

**D6, where it runs and who judges it.**

- Its own jobs, judged by `scripts/mutation-verdict.py` under two classes: chosen. A pull request
  runs `mutation-python`, a matrix the plan sizes from the diff's listing; the weekly battery runs
  sixteen shards over the whole population. The verdict judges a new class `scripts`
  (`scripts/<name>.py`) and the existing class `oracle`, whose rows keep counting beside its
  generated mutants; an equivalent Python mutant is recorded in
  `scripts/mutation-equivalent.d/python.json`, bound as ADR-070 binds a Rust record.
- Inside `mutation-rows`: rejected, because a row is one chosen mutant with one named killer, and
  the rows' report, census and retirement check would have to learn a second shape.
- A verdict script of its own: rejected, because `mutation-verdict.py` is the one reader of every
  class, the records and the battery (ADR-057, ADR-070).

## Decision Outcome

Chosen: D1 to D6's first options, for both targets. SPEC-087 states them as requirements and
plans the rows that guard them.

This amends ADR-057 D1 (the oracle's Python gains generated mutants, and cosmic-ray is measured
and rejected) and SPEC-039 R1 and R2 (the oracle is judged by generated mutants beside its rows,
and `scripts/<name>.py` gains a class of its own while staying outside production code). It takes
effect when SPEC-087's delivery builds the runner and sets this ADR `accepted`; ADR-057 then gains
a note that points here, and SPEC-039 a dated amendment.

### Consequences

- Good, because every VOID state is the runner's own report, read by the one verdict.
- Good, because every killed mutant names its killers, which #220 needs.
- Good, because the tests keep loading the files as they do today: nothing moves to fit a tool.
- Bad, because the operator set is DeckStreak's own code, with its own tests and its own blind
  spots: it generates fewer mutants than cosmic-ray (55 against 89 on the web audit's verdict, 286
  against 545 on the scrub).
- Bad, because the whole population is large: about 3,070 mutants at `dev` 26263de, 1,850 of
  them in `mutation-verdict.py`, whose tests are the slowest, so the weekly battery shards it.
- Bad, because the first whole run will find survivors in files no pull request touches; the
  campaign that clears them is #322.

### Confirmation

SPEC-087's acceptance criteria, red first in its delivery; the delivery's own pull request, whose
`mutation-python` job judges its own new lines; and the first weekly battery after it lands.

## What would make this wrong

- A maintained Python mutation tool that reads a hung mutant apart from a kill, reports an empty run
  as a failure, and names each mutant's killers: the runner would then be replaced by it.
- A population that outgrows the battery's sixteen shards: the runner would then need a projection,
  as SPEC-039 R18 sizes the Rust shards.
- Survivors that are mostly equivalent by construction: the operator set would then be narrowed,
  by a note here, and never by a comment in the source.

## More Information

SPEC-087; SPEC-039 (§1, R1 to R4, §5); SPEC-057 (R4 to R11); ADR-057 (D1, D4); ADR-070; #218,
#219, #220, #322. mutmut: https://mutmut.readthedocs.io. cosmic-ray:
https://cosmic-ray.readthedocs.io.
