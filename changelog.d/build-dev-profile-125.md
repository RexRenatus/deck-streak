### Changed

- Development and test builds keep line tables only in the workspace's crates and no debuginfo in
  dependencies, which makes the build directory several times smaller with the same test results and
  backtraces that still name each file and line.
