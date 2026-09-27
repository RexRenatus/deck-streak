### Added

- A lint over every test file refuses, by file and line, a temporary file or directory that a test
  never removes.
- The pack runner refuses a pending pack whose rows all pass, and runs every deferred row in a pass
  of its own, refusing one that passes.
- The box-pack runner runs each phxd pack with the verb its catalog admits, judges the committed
  tree without the vendored rule code, and fails on any red row the wiring does not name.

### Changed

- The accessibility pack is enforced: every one of its blocking rows passes.

### Fixed

- Two guard tests no longer leave temporary directories behind.
