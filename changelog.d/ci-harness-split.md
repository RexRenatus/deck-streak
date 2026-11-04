### Changed

- The Swift harness job keeps its Debug tests and the card view planted suite, and a sibling job,
  `release-and-app`, runs the Release measurements, the size and imports of the Release app, the tests of the app,
  of the review screen and of bury and flag, and the unsigned archive, so each job finishes inside its own bound.
  A `verdict` job fails unless every job of the Apple build succeeded. No step, test, wait or bound is dropped or
  loosened, and each report names the other job for the rows it does not measure.
