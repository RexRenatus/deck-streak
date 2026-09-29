### Added

- DeckStreak's database is replicated to an offsite bucket by its own Litestream unit, copied daily
  with an integrity check that keeps three copies, and restored every week into a private temporary
  path by a drill that checks both the replica and the newest daily copy. A failure pages through the
  alert unit. The replica and daily copies keep an erased row for at most three days, the declared window.

### Changed

- DeckStreak's memory share is 704 MiB, to hold the replicator's ceiling.
