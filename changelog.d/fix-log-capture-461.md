### Fixed

- A test's log capture no longer loses a line that another thread reached first (SPEC-024,
  issue #461). Every capturing test now goes through one helper that registers a floor dispatcher
  before the capture, so a callsite's cached interest always includes the capture's; a test
  derives the population from the tree and fails on any capture that bypasses it.
