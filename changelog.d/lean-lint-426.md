### Added

- A warning Lean raises while it builds the Formal library now fails the build, and a guard in the
  python stage refuses `sorry`, `admit`, `axiom`, `native_decide`, `implemented_by`, `extern`,
  `unsafe`, `nolint`, a linter switched off, a zero heartbeat limit and any module that switches
  the warnings rule off, in the code of every Lean file under `formal/lean`, outside an empty
  allow-list (SPEC-426).
