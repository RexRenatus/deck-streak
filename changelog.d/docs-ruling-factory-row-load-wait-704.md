### Changed

- Each card factory row is gated by its own load wait, by a ruling the owner signs in
  `docs/rulings/` (#704). The ruling also admits one more fix round beyond the round budget and,
  by name, one test-only keeper view under the existing warm-up bound. This pull request adds only
  that ruling and changes no test, row, wait or assertion; nothing is skipped or deferred.
