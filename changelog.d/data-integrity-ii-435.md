### Added

- Three census tests hold today's data-integrity hazards in registers and refuse any other. The
  migration census reads every migration and finds each statement that removes a table, a column,
  an index, a view or a trigger, renames, or deletes rows; each one needs a register row whose SPEC
  file names the migration. The arithmetic census lists every unchecked `+`, `-`, `*`, compound
  assignment and `.sum(` or `.product(` call in the six crates that compute coins, prices, XP,
  levels, streaks and counters, by its function, with the bound that holds it. The escape-hatch
  census lists every `AssertSqlSafe(` call, every spawn of a shell or a configured program and
  every environment read in the crates' sources, build scripts and examples, with its reason. Each
  census prints its examined count, refuses an empty population, and refuses an unlisted, recounted
  or stale row (SPEC-435).
