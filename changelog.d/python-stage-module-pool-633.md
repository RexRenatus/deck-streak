### Changed

- The gate script's python stage now runs each test module in a process of its own, several at
  once (`PYTHON_STAGE_JOBS`, by default the CPU count), and then, one by one, the modules that spawn
  cargo or are not yet measured; its last line and its exit are as before, and
  `PYTHON_STAGE_JOBS=1` runs every module one after another.
