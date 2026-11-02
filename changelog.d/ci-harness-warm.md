### Changed

- The Swift harness job builds each of its two test steps once, then boots each simulator,
  launches the harness and ends it once before the tests, each simulator inside its own time
  budget, and runs the tests without building again. No test, wait, retry or bound changes.
- The harness report records, for each simulator, the warm's boot wait, launch and end, the UI
  test runner's bootstrap, the first-launch gap and the longest end of the app in each session.
