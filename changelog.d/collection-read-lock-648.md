### Fixed

- A collection read keeps its shared collection lock until the read has ended, even when its
  caller stops waiting or its runtime stops, so no exclusive holder of the lock runs beside a read
  that still runs (SPEC-648, #957).
