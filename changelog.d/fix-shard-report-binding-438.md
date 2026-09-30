### Fixed

- The Python mutation verdict now counts a shard's work only from the report bound to that
  shard: the report's own shard field must equal the slot it sits in, and the mutants it
  examined must equal the plan's listing for that shard. A copied, swapped, trimmed or padded
  report, or one with a missing or malformed shard field, is refused by name (#438).
