### Added

- A mutants run now happens inside a memory scope (SPEC-196, issue #439). Each `mutation-rust`
  leg, each weekly rust leg and the `rehearsal` run `cargo mutants` through
  `scripts/memory_scope.py`, which holds the run in a transient scope at fifteen sixteenths of the
  machine with swap forbidden, so a mutant that allocates without end is stopped by the kernel
  and the rest of the leg goes on. The scope writes `memory-scope.json` beside the report. The
  verdict reads it: a leg with no record, or with a scope that was not in force, is VOID by name;
  a mutant the cap stopped fails its leg as `MEMORY-CAP <mutant>`, is neither caught nor a
  timeout, and is not examined; every other result in the leg stands. Thirty rows,
  `S19600` to `S19629`, hold the arms. A scheduled weekly run between this merge and the release
  that carries it reads its shards VOID, because the workflow comes from main and the scripts
  from dev.
