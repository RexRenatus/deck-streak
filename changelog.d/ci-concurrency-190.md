### Changed

- A pull request's superseded run of the changelog and engine-measure workflows is now cancelled
  when a newer push arrives, as the main gate's already was. No push, tag, schedule or dispatch run
  is cancelled while it runs, and the newest run reports the same checks as before. The release
  workflow's runs of one tag still queue under GitHub's one pending run per group, as before.
