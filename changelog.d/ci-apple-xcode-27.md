### Changed

- The Apple jobs and both TestFlight app jobs build with Xcode 27.0 on GitHub's `xcode-27` image,
  and the harness tests on iOS 27.0.
- The harness's cold-start test ends the app after each pass's measurement, never inside the next.
