### Fixed

- The memory scope now reads its counts after the command only once its process list holds the
  script alone, so a kill of a process the command started is counted in the record or makes the
  record void, and never reads as a clean run. A TLA+ model holds the reads (SPEC-467, #469).
