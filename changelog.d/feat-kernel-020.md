### Added

- The kernel every context builds on: the study day, which turns over at the rollover hour in a
  fixed UTC offset and renders as its ISO date; a clock tests drive by hand; typed settings that
  refuse start by name and never by value; credentials read only from systemd's credentials
  directory; one JSON log setup that opens each line with its journal priority and redacts every
  loaded secret and token shape; a bounded rail for blocking work; and the SQLite base (WAL,
  `BEGIN IMMEDIATE` writes, a read-only opener, embedded migrations) with the data-rights port.
