### Added

- A refused owner `/sync` is recorded with its reason and the time it was refused, beside the
  pending flag, which the same write clears. The owner is told the reason code instead of "still
  running", the notification router is not flushed, and the next job run does not serve the same
  request again. A new request clears the record. An erase clears it, and an export carries it.
