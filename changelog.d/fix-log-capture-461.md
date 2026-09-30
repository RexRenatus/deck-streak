### Fixed

- A test's log capture no longer loses a line that another thread reached first (SPEC-024,
  issue #461). Every capturing test goes through one helper that first installs a floor as the
  global default, so a thread with no capture never caches a callsite as never. A test reads the
  install functions from the tree and fails on any call outside the helper.
