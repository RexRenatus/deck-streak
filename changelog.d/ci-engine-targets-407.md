### Changed

- The two engine legs of the gate now build only the two test targets they run, in place of every
  test target of the workspace. They still build under the whole workspace's package scope, so no
  dependency is rebuilt, and the tests each slice runs are the same as before. The target flags are
  derived from the one definition of the engine set, so they cannot drift from it.
