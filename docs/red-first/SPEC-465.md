# Red-first record: SPEC-465

The SPEC, ADR-482 and the schematic were committed with the red commit. The tests of A1 to A4,
A6 to A8, A10 and A14 were committed alone (d5d3b541) against the unchanged runner; the whole
test class set ran against the base and each red failed by assertion. The refusal of a
misshapen retired list and the run state's signal latch (490aa7b4) turned them green; the cover
stamp (3a6847d2), the vectors (2bb9df2f) and the rows (d654763c) followed. Between the red
commit and the green tip, the four commits d654763c, 2bb9df2f, 3a6847d2 and 490aa7b4 change no
test file: `git log d5d3b541..HEAD -- scripts/tests` prints nothing.

```red-first
A1: red at d5d3b541: a top-level list in the retired file ended in a Traceback and exit 1, not the named refusal: 1 != 2 : Traceback (most recent call last):
A1: green at d654763c
A2: red at d5d3b541: a retired value of 1 ended in a Traceback and exit 1, not the named refusal: 1 != 2 : Traceback (most recent call last):; a retired value of {} was read as no entries and the row was refused as unapproved instead: 1 != 2 : retired: S00030-DOUBLE: REFUSED: it left while its target scripts/fixmod.py stays, and scripts/mutation-rows.retired.json records no reason and approval for it
A2: green at d654763c
A3: red at d5d3b541: an entry that is not an object ended in a Traceback and exit 1, not the named refusal: 1 != 2 : Traceback (most recent call last):
A3: green at d654763c
A4: red at d5d3b541: an entry without a string id ended in a Traceback and exit 1, not the named refusal: 1 != 2 : Traceback (most recent call last):
A4: green at d654763c
A6: red at d5d3b541: a SIGHUP of the runner left the killer running: True is not false : the killer 119074 still runs after a SIGHUP of the runner
A6: green at d654763c
A7: red at d5d3b541: a SIGTERM while the killer starts left the killer running: True is not false : the killer 119079 still runs after a SIGTERM while the killer starts
A7: green at d654763c
A8: red at d5d3b541: a second SIGTERM during the cleanup left the killer running: True is not false : the killer 119066 still runs after a second SIGTERM during the cleanup
A8: green at d654763c
A10: red at d5d3b541: main left its own handler on SIGTERM after a return: <function end_on_sigterm.<locals>.<lambda> at 0x7f2dc455d440> is not <function MainLeavesTheProcessAsItFoundIt.setUp.<locals>.on_term at 0x7f2ddc7bd6c0> : a return
A10: green at d654763c
A14: red at d5d3b541: the reader crashed where its proof refuses: Tuples differ: ('crashed', 'AttributeError') != ('refused', 'scripts/mutation-rows.retired.json must hold a top-level object')
A14: green at d654763c
A5: not red: its tests pin what a well-formed retired list admits and what a repeated key refuses, which the base already did; they read green at the red commit and their rows read KILLED at the green tip
A9: not red: it pins that a runner started with SIGHUP ignored keeps it ignored, which the base already did; it read green at the red commit and its row reads KILLED at the green tip
A11: not red: it pins that main leaves no signal state behind on the paths the base already left clean; it read green at the red commit and its row reads KILLED at the green tip
A12: not red: it pins that a signal held while a killer fails to start still ends the run, which the base already did for the path the test drives; it read green at the red commit and its row reads KILLED at the green tip
A13: not red: the census of every committed row and the group tests pin the base's rows against drift; they read green at the red commit and S02506, the one row re-anchored, reads KILLED at the green tip
```

Rows S46500 to S46504 and S46510 to S46519 are companions of the killing tests of A1 to A4,
A6 to A8, A10 and A14, and S02506 is re-anchored in place; each was proved KILLED by its full id.
