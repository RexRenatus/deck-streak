### Changed

- The gate's `audit-web` stage audits every package the lockfile resolves, the development
  dependencies too, prints how many it examined, and refuses a run that examined nothing. It fails
  on any advisory at `low` or above, the bar `audit-rust` holds.
