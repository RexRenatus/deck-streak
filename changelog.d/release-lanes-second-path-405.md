### Added

- A release tag whose push started no app release lane run is built by a manual dispatch of the
  lane at the tag's own ref, through the same checks, review and queue as its push; the lane
  refuses a run that a workflow's own token started (#733).
