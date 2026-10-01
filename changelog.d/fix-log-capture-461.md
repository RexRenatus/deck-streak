### Fixed

- A test's log capture no longer loses a line that another thread reached first (SPEC-024,
  issue #461). Every capturing test goes through one helper that first installs a floor as the
  global default, so a thread holding no capture answers "sometimes" for a callsite it reaches after
  that. A test reads every source file under the crates and tools, and those they bring in, and fails on any name from its fixed lists
  that installs or registers outside the helper, and the helper refuses a capture nested inside
  another on one thread.
