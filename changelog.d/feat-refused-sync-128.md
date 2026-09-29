### Added

- A refused owner `/sync` is recorded with its reason and the time it was refused, beside the
  pending flag, which the same write clears. The owner is told the reason code instead of "still
  running", the notification router is not flushed, and the next job run does not serve the same
  request again. A new request clears the record. An erase clears it, and an export carries it.
- A refusal that comes after the owner's sync ran is answered beside the run: the sync's own line,
  then "Your scores were not recomputed (code), so they stand." Every code the owner cycle
  records is checked against the closed set, and the record's reason and instant are set together.
