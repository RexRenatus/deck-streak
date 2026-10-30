### Changed

- The real tree's settle census in `crates/progression/tests/xp_census.rs` may keep the units of lock-pinned
  registry and git packages between censuses, in a store of its own outside `target/`, by a ruling the owner signs
  in `docs/rulings/` (#920). Workspace member units are never kept, every planted tree's census stays cold, and
  each CI job starts with an empty store. This pull request adds only that ruling and changes no code, test, row or
  assertion; the change that applies it lands with #692.
