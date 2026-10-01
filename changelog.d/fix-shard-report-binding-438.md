### Fixed

- The Python mutation verdict now counts a shard's work only from the report bound to that
  shard: the report's own shard field must equal the slot it sits in, and the mutants it
  examined must equal the plan's listing for that shard. A copied, swapped, trimmed or padded
  report, or one with a missing or malformed shard field, is refused by name (#438).
- The same verdict reads a shard's report through one reader that the binding and the judge
  share: a container of another type, an outcome the runner does not produce, or a mutant filed
  under a path the class does not read is refused by name rather than crashing or passing (#438).
- The Python mutation lane runs the six test modules of the verdict script in a cheaper order, the
  module that kills most mutants first; the same modules and the same mutants run, so a slow shard
  stops costing about as long as the longest job (#438).
- The shard-report tests plant a dropped, a constant and a swapped value in every interpolated field
  of every asserted message, so a drift tail with its counts swapped or fixed is refused (#438).
