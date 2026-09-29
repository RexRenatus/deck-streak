### Changed

- A pull request's superseded run of the changelog and engine-measure workflows is now cancelled
  when a newer push arrives, as the main gate's already was. Push, tag, schedule and dispatch runs
  are never cancelled, and the newest run reports the same checks as before.
