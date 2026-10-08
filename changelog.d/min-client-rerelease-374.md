### Added

- A client below the sync service's stated minimum stops before any sync and says why (SPEC-374,
  ADR-385). The service states the oldest client level it accepts at `GET /api/sync/minimum-client`,
  the engine core refuses every sync until a statement that admits its level is read, and the
  static library the iPhone and iPad app links reads the statement before every sync login.
