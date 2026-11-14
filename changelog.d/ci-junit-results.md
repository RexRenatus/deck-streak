### Added

- CI uploads the JUnit results of every job that runs tests as an artifact, `junit-<job>`,
  whatever the tests concluded: the Rust tests, Vitest, the browser suites and the Python stage,
  one test case per module. A failing Rust test runs once more, and a test that passes on that
  retry still fails the run and is marked flaky in the results; the mutation runs retry nothing.
