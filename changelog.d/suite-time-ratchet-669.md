### Added

- A census holds each test suite's run time to a recorded bound, at warn. It lists every suite CI
  runs by its runner's own unit (each Rust test binary, each crate's doc tests, each Python test
  module, each web unit and browser test file and each Swift test target), reads each suite's time
  from the stage logs CI uploads, and reports a suite over its bound by more than its margin, a
  suite with no time recorded and a suite now faster than its bound. A listing entry that names no
  current suite fails (SPEC-669).
