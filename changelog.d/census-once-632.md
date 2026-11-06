### Changed

- The xp census runs once per run, in a `census` job of its own, instead of in both the test and
  the release stage, and the pull request run's Rust leg ceiling falls from 185 to 184 for the one
  more job (SPEC-632).
