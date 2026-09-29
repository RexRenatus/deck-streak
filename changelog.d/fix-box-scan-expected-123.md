### Changed

- The box-run driver's proxy scan now admits an expected red row that names an open issue, as a pack's entry does. The row is counted, not failed, and the expectation goes stale when its issue closes or its row stops reading red; any other red row still fails the run.
