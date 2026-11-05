### Fixed

- The tests of the Python mutation runner no longer leave a `costs.jsonl` in the shared temporary
  directory: the two helpers that run the runner and read its report give each run a directory of
  its own, removed when the test ends.
