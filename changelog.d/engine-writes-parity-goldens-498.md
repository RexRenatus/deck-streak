### Added

- The engine's parity goldens have their readers before the one sandboxed run writes them: one committed golden per
  registered engine function, held to the root manifest's engine pin, the committed driver and registry, synthetic
  seeded inputs, and no date, id or clock of the run or stock line; a Rust test per golden that holds every answer to
  the harness's answer keys; and a schema snapshot writer whose output carries one `PRAGMA user_version` pin line
  (SPEC-498).
