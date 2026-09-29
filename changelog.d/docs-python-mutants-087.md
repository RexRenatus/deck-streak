### Added

- A plan for generated mutants over the repository's own Python: the guard scripts under
  `scripts/` and the parity oracle's generator (SPEC-087, planned; #218, #219). ADR-073, proposed,
  chooses a runner of the repository's own, `scripts/mutation_python.py`, against mutmut 3.8.0 and
  cosmic-ray 8.7.0, each measured on four targets: mutmut ran on none of them as the tree stands,
  and cosmic-ray records a hung mutant as killed and reads a run that examined nothing as complete
  with no survivor. The runner mutates in place and restores each file checked by sha256, reads a
  hung mutant and an empty run as VOID by name, leaves out a test that fails on any change to the
  file's bytes, and names every killer. A pull request judges the mutants on its changed lines,
  and the weekly battery the whole population in sixteen shards; a survivor is killed by a test or
  recorded equivalent with its argument.
  - The schematic `docs/schematics/mutation-testing-python.md` adds the Python job to the mutation
    jobs of `docs/schematics/mutation-testing.md`, and draws one file's run.
