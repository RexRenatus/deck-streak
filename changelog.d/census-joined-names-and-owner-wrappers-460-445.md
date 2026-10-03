### Added

- The three table censuses read a reserved table name however its literals spell it (SPEC-324,
  #460): one shared reader decodes every crate's literals as the compiler does, follows includes
  and `#[path]` modules, pools the pieces across the workspace, and refuses each file holding a
  piece of a join that assembles `xp_ledger`, `xp_settlement` or `coin_ledger` in any case outside
  its owner. A join it cannot read beside part of the name, and an include it cannot name or read,
  are refused by name; an include joined onto the build-script output folder is counted and
  disclosed.
- A generated population plants every split of each name in nine forms, a fourteen-member family of
  literal spellings evaluated by the compiler, near misses one character short, and the real tree's
  own joins as controls; every census prints what it examined.

### Changed

- The settle census refuses progression's own operation (#445, ADR-197 round 9): a wrapper, a
  function pointer or a generic in progression's own code that calls `settle` is refused by name
  unless progression admits its file, which it admits none; its imports and re-exports stay
  accepted and are followed to their callers as before.
