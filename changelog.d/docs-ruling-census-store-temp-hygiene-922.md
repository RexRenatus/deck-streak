### Changed

- The SPEC-030 temp-hygiene lint admits exactly one `std::env::temp_dir()` call, the one in `store_of` in
  `crates/progression/tests/xp_census.rs` that roots the settle census store, by a ruling the owner signs in
  `docs/rulings/` (#922). Every other `temp_dir()` call in a test file is still refused, and an admission that
  matches no call is refused. This pull request adds only that ruling and changes no code, test, row or
  assertion; the change that applies it lands with #919.
