### Changed

- The mutation gate's per-mutant timeout may rise from 2200 to 2300 seconds, by a ruling the owner
  signs in `docs/rulings/` (#700). This pull request adds only that ruling and changes no workflow;
  no mutant, leg or row is skipped, capped or deferred. The raise itself lands with SPEC-362.
