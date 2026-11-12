### Added

- A census in the python stage reads every tracked file's blob size and prints a warning for a
  file past 1 MiB that its listing does not name, or for a listed file grown past the size its
  entry records. It starts at warn with today's five parity goldens listed at their sizes, fails
  on a listing entry that names no current site, and fails on an empty population (SPEC-673,
  #1393).
