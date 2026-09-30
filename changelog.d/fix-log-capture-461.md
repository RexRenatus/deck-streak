### Fixed

- A test's log capture no longer loses a line that another thread reached first (SPEC-024,
  issue #461). Every capturing test goes through one helper that first installs a floor as the
  global default, so a thread holding no capture answers "sometimes" for a callsite it reaches after
  that. A test reads every source file the tests compile and fails on any install, dispatcher or
  callsite registration outside the helper.
