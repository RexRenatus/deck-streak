### Added

- The XP ledger and its grant port. Every XP grant is written once through one port: once per study
  day, source and track, or once ever for a source and track, so a replayed grant writes nothing
  and answers with the amount already granted. An amount cannot be negative, a grant's source is a
  short token the port checks before any write, and the level is read from the ledger's total each
  time, by the predecessor's integer level curve. The ledger is exported and erased with the rest
  of the owner's data.
