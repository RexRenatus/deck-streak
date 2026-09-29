### Added

- The Python mutation runner, `scripts/mutation_python.py`, and its population map
  `scripts/mutation-python.json` (SPEC-087; #218, #219). It lists the mutants of the guard scripts
  and the parity oracle's generator, runs each against its mapped test modules, restores every file
  checked by sha256, and reads a hung mutant, a red control and a failed restore as VOID by name.
