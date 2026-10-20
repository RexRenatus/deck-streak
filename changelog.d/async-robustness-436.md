### Added

- The workspace denies clippy's `await_holding_lock` and `await_holding_refcell_ref` by name, so
  a lock guard held across an await fails the lint stage whatever the groups say. An audit refuses
  a missing or lowered entry, and any lowering of either lint or of the `suspicious` group in a
  source attribute, a member manifest or a flag file. Two token censuses start at warn: one lists
  the eleven blocking calls that sit in async bodies today, and one finds no panic in the
  request-handler crates (SPEC-436).
