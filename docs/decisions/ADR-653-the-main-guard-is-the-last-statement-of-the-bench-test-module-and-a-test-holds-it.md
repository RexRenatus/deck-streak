---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-653: the main guard is the last statement of the bench test module and a test holds it

Decides SPEC-637 (R1 to R4).

## Context and Problem Statement

The bench test module `scripts/tests/test_perf_benches.py` held its `if __name__ == "__main__":`
guard after its first class, and the class that holds SPEC-618's bound criterion was appended below
the guard so that no line above it moved. Test discovery imports the module whole and runs every
class, but a script run of the module reaches the guard first: `unittest.main()` runs the tests
defined so far and exits, so the bound criterion's test never runs that way. Where does the guard
stand, and what holds it there?

## Decision Drivers

- A script run of a test module runs every test the module defines.
- A row find or a line citation of the module stays true.
- A guard that drifts above a class again is refused by a test, never found by hand.

## Considered Options (the alternatives it was chosen against)

- Chosen: (a) the guard closes the module, and a test in the module parses the committed file and
  asserts that its one guard is its last statement, after every class, with one mutation row that
  puts a guard above the bound table again.
- (b) A test that runs the module as a script in a child process and reads the test names it
  prints: rejected because it starts a process from the test suite and runs the module's tests
  twice in every run.
- (c) A census over every module of `scripts/tests`: rejected for this change because four other
  modules hold a class after their guard, so it would read red until each of them moves, and those
  moves are changes of their own.
- (d) Remove the guard and run the module only by discovery: rejected because the other modules of
  `scripts/tests` keep their guards, and a script run is how one module is run by hand.
- (e) Leave the guard where it is: rejected because the guard then promises a script run that
  skips a class, which is the finding this decides.

## Decision Outcome

Chosen: (a). The guard's two lines leave their place above the bound table and close the module,
and no other line of the module's tests, tables, constants or bounds changes. The test
`test_the_main_guard_is_the_last_statement_after_every_class` parses the committed module with
`ast` and asserts that no class follows its first guard and that its one guard is its last
statement; row S63700 puts a guard above the bound table again and is killed by that test. No row
targets the module and no document cites one of its lines by number, so moving the guard moves
nothing a row or a citation reads.

### Consequences

- Good, because a script run of the module runs every class, the bound criterion's included.
- Good, because a class appended below the guard again reads red, so the guard stays last.
- Bad, because four other modules of `scripts/tests` still hold a class after their guard, which
  SPEC-637 section 5 leaves to changes of their own.

### Confirmation

SPEC-637 A1, red at the commit that adds the test while the guard stands above the bound table and
green at the commit that moves it; the filtered script run of SPEC-637 section 3, which finds no
test at the first of those commits and runs the bound criterion's test at the second; and row
S63700.

## More Information

- SPEC-637 (the measurements, the requirements and the file manifest).
- SPEC-618 (the benches, the bound criterion and the bound table this module holds).
- What would make this wrong: a runner that loads test modules other than by discovery or a script
  run, which is then a change of its own.
