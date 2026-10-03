### Added

- The gate runs the workspace's tests in the release profile the daemon ships from (stage
  `test-release`, CI job `release`), and clippy refuses `cfg!` and `option_env!` in workspace code
  unless a site allows one by name with its reason (SPEC-330, issue #473).
