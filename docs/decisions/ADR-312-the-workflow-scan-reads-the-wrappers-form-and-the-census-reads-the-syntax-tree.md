---
status: accepted
date: "2026-10-02"
decision-makers: "the DeckStreak architect"
---

# The workflow scan reads the wrapper's form from the guard's text, and the stand-in census and the finder's copy check read the syntax tree

## Context and Problem Statement

ADR-306 gave `test_mutation_workflows.py` a recognizer of wrapped commands that, in its own words,
"names no wrapper and reads no file: for a command whose program is not cargo, the command is the
words after its first standalone `--`". The guard of `test_dispatch_shards.py` recognizes only its
wrapper's own form. The two readings disagree on every other program before a `--`: `echo`, `env`,
`timeout`, `git` and a `python3` script are a found command to the scan and a refusal to the guard
(#533).

The census of stand-ins that ADR-306 added reads text: the word `plant` in the `try`, one pattern
for a real-program call, a 12-line window after the handler, and the top-level files only. It misses
`os.system`, a call 13 lines away, a plant spelled without `plant`, and a stand-in in a
subdirectory. The census of the finder's definitions matches the finder's name, so a renamed copy
escapes it (#532).

How should the scan know the wrapper, and how should the two censuses read the test tree, while
`test_mutation_workflows.py` and the census stay runnable where the wrapper's own tests are not?

## Decision Drivers

- The scan must read a wrapped command as the guard reads it, or it disagrees with the guard.
- `test_mutation_workflows.py`, `test_stand_in_census.py` and `_stand_in_census.py` must stay
  runnable where the wrapper's tests are not: none may import, run or load anything that reaches the
  wrapper, and reading another module's text is allowed.
- `test_dispatch_shards.py` and `_mutants_finder.py` are not edited: the guard's tests run in CI
  only, so an edit there could not be seen red or green before CI.
- Two copies of one fact drift; ADR-306 records that the two scans had already drifted once.
- A census that can miss a shape by a word or a window is not a census of the shape.

## Considered Options (the alternatives it was chosen against)

- Read `WRAPPER` from `test_dispatch_shards.py`'s text, by `ast.parse` and `ast.literal_eval` of its one module-level assignment, and recognize only that form: chosen, because the scan then has the guard's own words, cannot drift from them, and loads nothing.
- Import the guard's `wrapped`: lost, because it loads the wrapper's module to read its parser, so `test_mutation_workflows.py` would no longer run where the wrapper's tests do not.
- A second literal copy of the wrapper's form in `test_mutation_workflows.py`: lost, because a copy drifts when the guard changes, and nothing would tie the two together.
- Keep the every-program reading and add a test that the two readings agree on the workflows in the tree: lost, because the over-find of #533 stays, and a parity test over today's workflows says nothing about the next one.
- Read the stand-ins with `ast.parse`, resolve real-program calls through the file's import aliases, and reach every later statement of the enclosing blocks: chosen, because a word gate and a line window are exactly what missed, and the syntax tree has neither.
- Widen the census's patterns and its window: lost, because a wider window is still a window and a wider word list is still a word gate; each misses the next shape the same way.
- Parse the files but not their string constants: lost, because the guard holds its stand-in's source in string constants, so the one stand-in in the tree would be outside the census.
- Narrow reach (only the statements after the `try`): lost, because a handler that runs the real program itself, or a `finally` block that does, is the same fallback.
- Compare each definition's body, normalised by `ast.dump` with its docstring dropped and its names renamed in order of first appearance, against every function of the finder: chosen, because a rename or a re-wrap leaves the normalised body equal.
- A similarity threshold between bodies: lost, because the threshold is tuned to the tree it was measured on and a small edit drops under it silently.
- Compare source text: lost, because a rename or a re-wrap changes the text and the copy escapes.
- Replace the name match with the body comparison: lost, because a copy with changed logic under the finder's own name would then escape, and the name match catches it at no cost.

## Decision Outcome

Chosen options: the scan reads the wrapper's form by text, the census reads the syntax tree and the
string constants with wide reach, and the copy check compares normalised bodies beside the name
match.

- **This amends ADR-306.** The sentence of its Decision Outcome that says `test_mutation_workflows.py`
  passes a recognizer "that names no wrapper and reads no file: for a command whose program is not
  cargo, the command is the words after its first standalone `--`" is replaced: the recognizer reads
  one file, `test_dispatch_shards.py`, as text, and returns a command start only after the guard's
  `WRAPPER` words and the first standalone literal `--` after them. ADR-306's consequence that names
  the broader reading ("it also finds commands the guard refuses") no longer holds; its other
  decisions stand.
- The scan cannot read the options the wrapper's parser declares. Two readings follow, both named
  in SPEC-129 section 12 and pinned: it over-finds where the guard refuses a word before the
  separator (`--cap 1`, `--report $OUT`, `$SEP`, and a lone `--` taken as `--report`'s value),
  and it refuses a separator given as the value of `--report`. Each fails closed: an over-found
  command is held to the bounds, and a refusal carries none.
- Measured at the base over the tree's 69 files, the census of the syntax tree with wide reach
  lists 0 arms, and parses 16613 string constants and skips 8123. Each of the four shapes of #532
  planted alone is listed once, and the same text with a handler that leaves is not.
- Measured at the base over the tree's 68 files other than the finder, the body comparison with the
  name match beside it finds one copy: `_indent` in `test_ci_workflows.py`, equal to the finder's
  `indent_of` by accident. It is the floor, held in one tuple. A renamed and re-wrapped copy of each
  of the finder's 29 functions is caught.

### Consequences

- Good, because the scan and the guard read a wrapped command the same way, outside two named
  limits, and a population test generates the members that would show a new disagreement.
- Good, because the census reads any depth, any spelling of the plant and any distance, and a
  stand-in held in a string constant.
- Good, because a renamed or re-wrapped copy of the finder is caught.
- Bad, because the scan now reads one file it did not read, and depends on `WRAPPER` staying one
  literal module-level assignment; a test pins that it is.
- Bad, because the census does not follow a call, does not evaluate a computed name, does not parse
  a string assembled at run time, and counts a handler that leaves on some paths as leaving. Each is
  stated and pinned.
- Bad, because a copy of the finder with changed logic under a name of its own is not caught.

### Confirmation

`TheScanReadsTheWrappersForm` and `EveryMutantsSpellingIsFound` in `test_mutation_workflows.py`,
`TheCensusOfStandIns` in `test_stand_in_census.py`, and the mutation rows from S12913 whose killers
are those tests.

## What would make this wrong

A second module-level assignment to `WRAPPER`, or one that is not a literal, would leave the scan
without a wrapper; the pin test fails first. A wrapper whose parser declares a second option that
takes a value would widen the second named limit. A stand-in that runs its real program through a
helper function would be outside the census; a census that follows calls would answer it.

## More Information

Issues #532 and #533; SPEC-129 sections 10 to 13; ADR-306.
