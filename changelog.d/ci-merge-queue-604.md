### Changed

- Pull requests into dev now merge through the Mergify merge queue: a ready, green pull request
  labelled `queue` is tested with up to five others on one batch pull request and then merged with
  a merge commit, and the `fragment` check passes a batch of dependency updates alone.
